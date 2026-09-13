//! 项目数据导入（电脑端导出文件 → 手机本地 SQLite）（手工维护）
//!
//! 设计要点（每条都由实测踩坑得出）：
//!   - **列名驱动**：插入显式写列名，不依赖列顺序，避免 PG/SQLite 列序差异串列。
//!   - **uuid 必须绑成 16 字节 BLOB**：导出文件里是文本，导入时用 sqlx 的 `Uuid`
//!     绑定；若当字符串写入，读回时会报 `invalid length: expected 16 bytes`。
//!   - **数值按 i64/f64 绑定**：SQLite 是弱类型，若数字存成 TEXT，
//!     读取 `i32`/`f64` 时会类型不匹配。
//!   - **整体一个事务**：失败全回滚，不留半份数据。
//!   - **只清空本次涉及的表**：未涉及的表保持原样。

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
pub struct TableExport {
    pub columns: Vec<String>,
    #[serde(default)]
    pub column_types: HashMap<String, String>,
    #[serde(default)]
    pub rows: Vec<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectExport {
    pub format_version: i32,
    pub root_project_id: String,
    #[allow(dead_code)]
    pub exported_at: String,
    pub tables: HashMap<String, TableExport>,
    pub insert_order: Vec<String>,
}

/// 导入结果摘要，用于回显给用户
#[derive(Debug, serde::Serialize)]
pub struct ImportSummary {
    pub project_id: String,
    pub project_name: String,
    pub tables_written: usize,
    pub rows_written: usize,
    /// 因源数据引用无效而跳过的行数
    pub skipped_rows: usize,
}

/// 从导出的 JSON 文本导入。
pub async fn import_from_json(pool: &SqlitePool, json: &str) -> Result<ImportSummary> {
    let export: ProjectExport =
        serde_json::from_str(json).context("导出文件不是合法 JSON 或结构不匹配")?;

    if export.format_version > crate::embedded::EXPORT_FORMAT_VERSION {
        anyhow::bail!(
            "导出文件版本 {} 比当前 App 支持的 {} 更新，请先升级 App",
            export.format_version,
            crate::embedded::EXPORT_FORMAT_VERSION
        );
    }

    // 目标库实际存在的列
    let mut existing_cols: HashMap<String, HashSet<String>> = HashMap::new();
    for t in export.tables.keys() {
        if crate::export::NON_PROJECT_TABLES.contains(&t.as_str()) {
            continue; // 本机级配置不接受导入（见 export.rs 的常量说明）
        }
        let info: Vec<(i64, String)> = sqlx::query_as(&format!("PRAGMA table_info({})", quote(t)))
            .fetch_all(pool)
            .await
            .with_context(|| format!("读取 {} 的列失败", t))?;
        let set: HashSet<String> = info.into_iter().map(|(_, name)| name).collect();
        if set.is_empty() {
            anyhow::bail!("导出文件里的表 `{}` 在当前 App 中不存在", t);
        }
        existing_cols.insert(t.clone(), set);
    }

    let mut summary = ImportSummary {
        project_id: export.root_project_id.clone(),
        project_name: String::new(),
        tables_written: 0,
        rows_written: 0,
        skipped_rows: 0,
    };

    let mut tx = pool.begin().await.context("开启导入事务失败")?;

    // 1. 清空本次涉及的表（倒序：先清子表）
    for t in export.insert_order.iter().rev() {
        if !export.tables.contains_key(t) {
            continue;
        }
        if crate::export::NON_PROJECT_TABLES.contains(&t.as_str()) {
            continue;
        }
        sqlx::query(&format!("DELETE FROM {}", quote(t)))
            .execute(&mut *tx)
            .await
            .with_context(|| format!("清空 {} 失败", t))?;
    }

    // 2. 按拓扑顺序插入
    for t in &export.insert_order {
        let Some(tbl) = export.tables.get(t) else {
            continue;
        };
        if crate::export::NON_PROJECT_TABLES.contains(&t.as_str()) {
            continue;
        }
        let rows = if tbl.rows.is_empty() {
            continue;
        } else {
            &tbl.rows
        };

        let existing = &existing_cols[t];
        let cols: Vec<&String> = tbl.columns.iter().filter(|c| existing.contains(*c)).collect();
        if cols.is_empty() {
            continue;
        }

        let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("${}", i)).collect();
        let col_list: Vec<String> = cols.iter().map(|c| quote(c)).collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote(t),
            col_list.join(", "),
            placeholders.join(", ")
        );

        let mut skipped = 0usize;
        for row in rows {
            let mut q = sqlx::query(&sql);
            for col in &cols {
                let v = row.get(*col).cloned().unwrap_or(Value::Null);
                let udt = tbl
                    .column_types
                    .get(*col)
                    .map(|s| s.as_str())
                    .unwrap_or("text");
                q = bind_value(q, v, udt)
                    .with_context(|| format!("绑定 {}.{} 失败", t, col))?;
            }
            match q.execute(&mut *tx).await {
                Ok(_) => summary.rows_written += 1,
                Err(e) => {
                    // 源数据可能有悬空引用（实测电脑端库里 storyline_scene 有 47 行
                    // 指向不存在的 scene）。这类行跳过并计数，不让整个项目导不进来。
                    tracing::warn!("跳过 {}.{} 的一行: {}", t, sql, e);
                    skipped += 1;
                    summary.skipped_rows += 1;
                }
            }
        }
        if skipped > 0 {
            tracing::warn!("表 {} 跳过 {} 行（源数据引用无效）", t, skipped);
        }
        summary.tables_written += 1;
    }

    // 3. 记录项目名，供回显
    if let Some(name) = sqlx::query_scalar::<_, String>("SELECT name FROM project WHERE id = $1")
        .bind(
            uuid::Uuid::parse_str(&export.root_project_id)
                .context("导出文件里的项目 id 不是合法 uuid")?,
        )
        .fetch_optional(&mut *tx)
        .await
        .context("读取导入后的项目名失败")?
    {
        summary.project_name = name;
    }

    tx.commit().await.context("提交导入事务失败")?;

    let violations: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await
        .context("外键完整性检查失败")?;
    if !violations.is_empty() {
        tracing::warn!("导入后存在 {} 处外键违例（来自源数据的悬空引用）", violations.len());
    }

    Ok(summary)
}

/// 按列类型把 JSON 值绑成 SQLite 能正确存取的形态。
///
/// 关键：
///   - uuid → `Uuid`（落成 16 字节 BLOB，与 sqlx 解码一致）
///   - 数值 → i64 / f64（保持数值类型，避免读取 i32/f64 时类型不匹配）
///   - 其余 → 字符串（SQLite 是弱类型，TEXT 可被 String / DateTime 解码）
fn bind_value<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    v: Value,
    udt: &str,
) -> Result<sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>> {
    if v.is_null() {
        return Ok(q.bind(Option::<String>::None));
    }

    match udt {
        "uuid" => {
            let s = v.as_str().context("uuid 列的值不是字符串")?;
            let u = uuid::Uuid::parse_str(s).with_context(|| format!("非法 uuid: {}", s))?;
            Ok(q.bind(u))
        }
        "int2" | "int4" | "int8" => {
            if let Some(i) = v.as_i64() {
                Ok(q.bind(i))
            } else if let Some(f) = v.as_f64() {
                Ok(q.bind(f as i64))
            } else if let Some(b) = v.as_bool() {
                Ok(q.bind(if b { 1i64 } else { 0i64 }))
            } else {
                let s = v.to_string();
                Ok(q.bind(s))
            }
        }
        "float4" | "float8" | "numeric" => {
            if let Some(f) = v.as_f64() {
                Ok(q.bind(f))
            } else if let Some(i) = v.as_i64() {
                Ok(q.bind(i as f64))
            } else {
                Ok(q.bind(v.to_string()))
            }
        }
        "bool" => {
            if let Some(b) = v.as_bool() {
                Ok(q.bind(if b { 1i64 } else { 0i64 }))
            } else {
                Ok(q.bind(v.to_string()))
            }
        }
        "json" | "jsonb" => {
            // 存成 JSON 文本，与 sqlx 的 json 解码兼容
            Ok(q.bind(v.to_string()))
        }
        _ => {
            // text / varchar / timestamptz 等统一存文本
            if let Some(s) = v.as_str() {
                Ok(q.bind(s.to_string()))
            } else {
                Ok(q.bind(v.to_string()))
            }
        }
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
