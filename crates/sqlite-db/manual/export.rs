//! 项目数据导出（手机端 → 电脑端 / 备份）（手工维护）
//!
//! 与电脑端 `crates/db/src/export.rs` **保持相同的 JSON 格式**
//! （同样的 `format_version`、`tables[].columns/column_types/rows`、`insert_order`），
//! 这样手机导出的文件可以直接在电脑端导入，反之亦然。
//!
//! 与电脑端实现的差异只在数据来源与类型走查：
//!   - 列类型来自 `PRAGMA table_info`（SQLite 没有 pg_catalog）
//!   - uuid 列在库里是 16 字节 BLOB，导出时转成标准 uuid 文本
//!   - 时间列读成 `DateTime<Utc>` 再转 RFC3339 文本
//!
//! 收集策略同样采用「含 project_id 的表直取 + 对所有 `*_id` 列按值反查闭包」，
//! 因为手机端库里同样存在大量「靠列值关联、没有外键」的表。

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

/// 与电脑端 `db::export::EXPORT_FORMAT_VERSION` 保持一致
const EXPORT_FORMAT_VERSION: i32 = 2;

/// 不属于任何项目、导入时绝不能替换的表。
///
/// `app_settings` 是**本机级**的全局配置（AI 网关地址 / 密钥 / 模型 / 界面偏好）。
/// 它既没有 `project_id` 也没有关联列，早期实现把它当「全局字典表」整表导出，
/// 结果是手机导入电脑端导出的项目后，手机上自己填的 AI 配置（含密钥）
/// 会被电脑端那份悄悄替换掉——用户完全不知情。
pub const NON_PROJECT_TABLES: &[&str] = &["app_settings"];

#[derive(Debug, serde::Serialize)]
pub struct TableExport {
    pub columns: Vec<String>,
    pub column_types: HashMap<String, String>,
    pub rows: Vec<Map<String, Value>>,
}

#[derive(Debug, serde::Serialize)]
pub struct ProjectExport {
    pub format_version: i32,
    pub exported_at: String,
    pub root_project_id: String,
    pub tables: HashMap<String, TableExport>,
    pub insert_order: Vec<String>,
}

#[derive(Debug, Clone)]
struct ColInfo {
    name: String,
    decl_type: String,
    notnull: bool,
    pk: bool,
}

/// 从 SQLite 的类型亲和性推断「导出时按哪类处理」
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Uuid,
    DateTime,
    Int,
    Real,
    Text,
}

fn kind_of(decl: &str) -> Kind {
    let d = decl.to_uppercase();
    // 迁移生成时把 uuid 列声明为 TEXT，但要在读取时区分出来是不可能的，
    // 因此这里依赖「列名以 id 结尾」这一约定（与生成脚本的 UUID_LIKE 一致）。
    if d.contains("DATETIME") || d.contains("TIMESTAMP") {
        return Kind::DateTime;
    }
    if d.contains("INT") {
        return Kind::Int;
    }
    if d.contains("REAL") || d.contains("FLOA") || d.contains("DOUB") {
        return Kind::Real;
    }
    Kind::Text
}

/// uuid 列判定：与生成脚本保持一致（id / *_id / 别名.id）
fn is_uuid_column(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "id" || n.ends_with("_id") || n == "ids"
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

async fn table_columns(pool: &SqlitePool, table: &str) -> Result<Vec<ColInfo>> {
    let rows: Vec<(i64, String, String, i64, Option<String>, i64)> =
        sqlx::query_as(&format!("PRAGMA table_info({})", quote(table)))
            .fetch_all(pool)
            .await
            .with_context(|| format!("读取 {} 的列失败", table))?;
    Ok(rows
        .into_iter()
        .map(|(_cid, name, decl_type, notnull, _dflt, pk)| ColInfo {
            name,
            decl_type,
            notnull: notnull != 0,
            pk: pk != 0,
        })
        .collect())
}

async fn all_tables(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' \
         AND name NOT LIKE 'sqlite_%' AND name NOT IN ('_migrations','schema_version') \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context("读取表清单失败")?;
    Ok(rows)
}

async fn primary_key(pool: &SqlitePool, table: &str) -> Result<String> {
    let cols = table_columns(pool, table).await?;
    Ok(cols
        .iter()
        .find(|c| c.pk)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "id".to_string()))
}

/// 把一个 value 读成 JSON（uuid 转文本、时间转 RFC3339）
async fn read_row(
    pool: &SqlitePool,
    table: &str,
    cols: &[ColInfo],
    pk: &str,
    id: &str,
) -> Result<Option<Map<String, Value>>> {
    let sel = cols.iter().map(|c| quote(&c.name)).collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT {} FROM {} WHERE {} = ?",
        sel,
        quote(table),
        quote(pk)
    );

    // 主键实际存的是 BLOB（uuid）就用 Uuid 绑定，否则按字符串
    let pk_stores_blob = column_stores_blob(pool, table, pk).await.unwrap_or(false);
    let row = if pk_stores_blob {
        match uuid::Uuid::parse_str(id) {
            Ok(u) => sqlx::query(&sql).bind(u).fetch_optional(pool).await,
            // id 不是 uuid 文本时退化为字符串比较（不应发生，但不静默报错掩盖）
            Err(e) => {
                tracing::warn!("主键 {}={} 不是 uuid，改按文本比较: {}", pk, id, e);
                sqlx::query(&sql).bind(id).fetch_optional(pool).await
            }
        }
    } else {
        sqlx::query(&sql).bind(id).fetch_optional(pool).await
    }
    .with_context(|| format!("导出 {}.{} 失败", table, id))?;

    let Some(row) = row else { return Ok(None) };
    use sqlx::Row;

    let mut obj = Map::new();
    for c in cols {
        let kind = kind_of(&c.decl_type);
        let v: Value = if is_uuid_column(&c.name) {
            match row.try_get::<Option<uuid::Uuid>, _>(c.name.as_str()) {
                Ok(Some(u)) => Value::String(u.to_string()),
                Ok(None) => Value::Null,
                Err(e) => {
                    // 有些 *_id 列存的是普通字符串（如 entity_type_id 存类型名），按文本读
                    match row.try_get::<Option<String>, _>(c.name.as_str()) {
                        Ok(Some(s)) => Value::String(s),
                        Ok(None) => Value::Null,
                        Err(_) => return Err(anyhow::anyhow!("列 {} 读取失败: {}", c.name, e)),
                    }
                }
            }
        } else {
            match kind {
                Kind::DateTime => match row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(c.name.as_str()) {
                    Ok(Some(dt)) => Value::String(dt.to_rfc3339()),
                    Ok(None) => Value::Null,
                    Err(e) => return Err(anyhow::anyhow!("时间列 {} 读取失败: {}", c.name, e)),
                },
                Kind::Int => match row.try_get::<Option<i64>, _>(c.name.as_str()) {
                    Ok(Some(i)) => Value::Number(i.into()),
                    Ok(None) => Value::Null,
                    Err(e) => return Err(anyhow::anyhow!("整数列 {} 读取失败: {}", c.name, e)),
                },
                Kind::Real => match row.try_get::<Option<f64>, _>(c.name.as_str()) {
                    Ok(Some(f)) => serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                    Ok(None) => Value::Null,
                    Err(e) => return Err(anyhow::anyhow!("浮点列 {} 读取失败: {}", c.name, e)),
                },
                _ => match row.try_get::<Option<String>, _>(c.name.as_str()) {
                    Ok(Some(s)) => {
                        // jsonb 被声明成 TEXT，这里尝试还原成 JSON；不是 JSON 就保持文本
                        if c.decl_type.to_uppercase().contains("JSON") {
                            serde_json::from_str(&s).unwrap_or(Value::String(s))
                        } else {
                            Value::String(s)
                        }
                    }
                    Ok(None) => Value::Null,
                    Err(e) => return Err(anyhow::anyhow!("文本列 {} 读取失败: {}", c.name, e)),
                },
            }
        };
        obj.insert(c.name.clone(), v);
    }
    Ok(Some(obj))
}

/// 构造「列名 → 类型」映射。
///
/// uuid 的判定依据是**实际存储为 BLOB**，而不是列名或声明类型：
/// 像 `app_settings.id` 这种真实文本主键（值为 `default`）不能被误标为 uuid，
/// 否则导入端会拿它去解析 uuid 而失败。
async fn build_col_types(
    pool: &SqlitePool,
    table: &str,
    cols: &[ColInfo],
) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for c in cols {
        let ty = if is_uuid_column(&c.name)
            && column_stores_blob(pool, table, &c.name).await.unwrap_or(false)
        {
            "uuid".to_string()
        } else {
            match kind_of(&c.decl_type) {
                Kind::DateTime => "timestamptz".to_string(),
                Kind::Int => "int4".to_string(),
                Kind::Real => "float8".to_string(),
                _ => "text".to_string(),
            }
        };
        m.insert(c.name.clone(), ty);
    }
    m
}

/// 探测某列**实际存储**的类型：返回 true 表示存的是 BLOB（即 uuid）。
///
/// 为什么不能只看声明类型：迁移把 uuid 列声明成 `TEXT`，
/// 但 sqlx 的 `Uuid` 编码写入的是 16 字节 BLOB（SQLite 弱类型，不强制转换）。
/// 所以「声明 TEXT」≠「存的是文本」，必须按实际值判断。
async fn column_stores_blob(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
    let sql = format!(
        "SELECT typeof({}) FROM {} WHERE {} IS NOT NULL LIMIT 1",
        quote(column),
        quote(table),
        quote(column)
    );
    let t: Option<String> = sqlx::query_scalar(&sql)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("探测 {}.{} 的存储类型失败", table, column))?;
    Ok(t.as_deref() == Some("blob"))
}

/// 读某表的主键文本。
///
/// 关键：**不能对所有主键都套用 hex()**。多数表用 uuid（库里是 16 字节 BLOB，
/// hex 后正好 32 位十六进制），但像 `app_settings` 用的是普通文本主键
/// （值为 `default`），对它 hex 会得到 `64656661756C74` 这种乱码，
/// 之后按 uuid 解析就会失败。
async fn pk_text(sql: &str, pool: &SqlitePool, uuid_pk: bool, bind_uuid: Option<&str>) -> Result<Vec<String>> {
    let rows: Vec<Option<String>> = if let Some(v) = bind_uuid {
        let u = uuid::Uuid::parse_str(v).with_context(|| format!("非法 uuid: {}", v))?;
        sqlx::query_scalar(sql).bind(u).fetch_all(pool).await
    } else {
        sqlx::query_scalar(sql).bind(v_noop()).fetch_all(pool).await
    }
    .with_context(|| format!("查询主键失败: {}", sql))?;

    Ok(rows
        .into_iter()
        .flatten()
        .map(|v| {
            if uuid_pk && v.len() == 32 {
                uuid::Uuid::parse_str(&v).map(|u| u.to_string()).unwrap_or(v)
            } else {
                v
            }
        })
        .collect())
}

/// 占位（当前没有无参数绑定的分支）
fn v_noop() -> String {
    String::new()
}

/// 按「某列 = 某值」查主键文本列表
async fn ids_by_value(
    pool: &SqlitePool,
    table: &str,
    pk: &str,
    column: &str,
    value: &str,
) -> Result<Vec<String>> {
    let pk_is_uuid = is_uuid_column(&pk) && column_stores_blob(pool, table, &pk).await.unwrap_or(false);
    let col_is_uuid = is_uuid_column(column) && column_stores_blob(pool, table, column).await.unwrap_or(false);

    let sel = if pk_is_uuid { format!("hex({})", quote(pk)) } else { quote(pk) };
    let sql = format!(
        "SELECT {} FROM {} WHERE {} = ?",
        sel,
        quote(table),
        quote(column)
    );

    let rows: Vec<Option<String>> = if col_is_uuid {
        let u = uuid::Uuid::parse_str(value).with_context(|| format!("非法 uuid: {}", value))?;
        sqlx::query_scalar(&sql).bind(u).fetch_all(pool).await
    } else {
        sqlx::query_scalar(&sql).bind(value).fetch_all(pool).await
    }
    .with_context(|| format!("收集 {}.{} 失败", table, column))?;

    Ok(rows
        .into_iter()
        .flatten()
        .map(|h| {
            if pk_is_uuid && h.len() == 32 {
                uuid::Uuid::parse_str(&h).map(|u| u.to_string()).unwrap_or(h)
            } else {
                h
            }
        })
        .collect())
}

/// 导出整个项目（含关联数据），格式与电脑端一致
pub async fn export_project(pool: &SqlitePool, project_id: &str) -> Result<ProjectExport> {
    let pid = uuid::Uuid::parse_str(project_id)
        .context("项目 id 不是合法 uuid")?
        .to_string();

    let tables = all_tables(pool).await?;
    let mut collected: HashMap<String, Vec<String>> = HashMap::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    let mut push = |t: &str, id: String,
                    seen: &mut HashSet<(String, String)>,
                    seen_ids: &mut HashSet<String>,
                    collected: &mut HashMap<String, Vec<String>>| {
        if seen.insert((t.to_string(), id.clone())) {
            seen_ids.insert(id.clone());
            collected.entry(t.to_string()).or_default().push(id);
        }
    };

    // 1) project 自身
    push("project", pid.clone(), &mut seen, &mut seen_ids, &mut collected);

    // 2) 所有含 project_id 的表
    for t in &tables {
        let cols = table_columns(pool, t).await?;
        if !cols.iter().any(|c| c.name == "project_id") {
            continue;
        }
        let pk = primary_key(pool, t).await?;
        for id in ids_by_value(pool, t, &pk, "project_id", &pid).await? {
            push(t, id, &mut seen, &mut seen_ids, &mut collected);
        }
    }

    // 3) 按关联列反查闭包（补全没有外键的深层数据）
    loop {
        let mut added = 0usize;
        for t in &tables {
            if t == "project" || collected.get(t).map(|v| !v.is_empty()).unwrap_or(false) {
                continue;
            }
            let cols = table_columns(pool, t).await?;
            let pk = primary_key(pool, t).await?;
            let link_cols: Vec<&ColInfo> = cols
                .iter()
                .filter(|c| is_uuid_column(&c.name) && c.name != pk)
                .collect();
            if link_cols.is_empty() {
                continue;
            }

            let mut hits: Vec<String> = Vec::new();
            for col in link_cols {
                let sql = format!(
                    "SELECT hex({}) FROM {} WHERE {} IN (SELECT value FROM json_each(?))",
                    quote(&pk),
                    quote(t),
                    quote(&col.name)
                );
                // 用十六进制文本比较：与 seen_ids（标准 uuid 文本）统一形态
                let hex_list: Vec<String> = seen_ids
                    .iter()
                    .filter_map(|s| uuid::Uuid::parse_str(s).ok())
                    .map(|u| u.to_string())
                    .collect();
                let json_arr = serde_json::to_string(&hex_list).unwrap_or_else(|_| "[]".into());
                let rows: Vec<Option<String>> = sqlx::query_scalar(&sql)
                    .bind(&json_arr)
                    .fetch_all(pool)
                    .await
                    .with_context(|| format!("反查 {}.{} 失败", t, col.name))?;
                for h in rows.into_iter().flatten() {
                    if h.len() == 32 {
                        if let Ok(u) = uuid::Uuid::parse_str(&h) {
                            hits.push(u.to_string());
                        }
                    }
                }
            }

            hits.sort();
            hits.dedup();
            for id in hits {
                if seen.insert((t.clone(), id.clone())) {
                    seen_ids.insert(id.clone());
                    collected.entry(t.clone()).or_default().push(id);
                    added += 1;
                }
            }
        }
        if added == 0 {
            break;
        }
    }

    // 组装
    let mut out_tables: HashMap<String, TableExport> = HashMap::new();
    for (t, ids) in &collected {
        let cols = table_columns(pool, t).await?;
        let pk = primary_key(pool, t).await?;
        let col_types = build_col_types(pool, t, &cols).await;
        let mut rows = Vec::new();
        for id in ids {
            if let Some(obj) = read_row(pool, t, &cols, &pk, id).await? {
                rows.push(obj);
            }
        }
        if rows.is_empty() {
            continue;
        }
        out_tables.insert(
            t.clone(),
            TableExport {
                columns: cols.iter().map(|c| c.name.clone()).collect(),
                column_types: col_types,
                rows,
            },
        );
    }

    // 全局字典表（没有 project_id 也没有任何关联列的表，如 entity_type）
    for t in &tables {
        if out_tables.contains_key(t) {
            continue;
        }
        // 本机级配置不属于项目数据，不能被带进导出文件（见 NON_PROJECT_TABLES 说明）
        if NON_PROJECT_TABLES.contains(&t.as_str()) {
            continue;
        }
        let cols = table_columns(pool, t).await?;
        let project_scoped = cols.iter().any(|c| {
            matches!(
                c.name.as_str(),
                "project_id" | "world_id" | "entity_id" | "session_id"
            )
        });
        if project_scoped {
            continue;
        }
        let pk = primary_key(pool, t).await?;
        let col_types = build_col_types(pool, t, &cols).await;
        let pk_is_uuid = is_uuid_column(&pk) && column_stores_blob(pool, t, &pk).await.unwrap_or(false);
        let sel = if pk_is_uuid { format!("hex({})", quote(&pk)) } else { quote(&pk) };
        let raw: Vec<Option<String>> =
            sqlx::query_scalar(&format!("SELECT {} FROM {}", sel, quote(t)))
                .fetch_all(pool)
                .await
                .with_context(|| format!("读取全局表 {} 失败", t))?;
        let all: Vec<String> = raw
            .into_iter()
            .flatten()
            .map(|h| {
                if pk_is_uuid && h.len() == 32 {
                    uuid::Uuid::parse_str(&h).map(|u| u.to_string()).unwrap_or(h)
                } else {
                    h
                }
            })
            .collect();

        let mut rows = Vec::new();
        for id in all {
            if let Some(obj) = read_row(pool, t, &cols, &pk, &id).await? {
                rows.push(obj);
            }
        }
        if rows.is_empty() {
            continue;
        }
        out_tables.insert(
            t.clone(),
            TableExport {
                columns: cols.iter().map(|c| c.name.clone()).collect(),
                column_types: col_types,
                rows,
            },
        );
    }

    // 插入顺序：被引用的表在前。手机端没有外键元数据，按名字排序即可
    // （导入端会按同样的顺序插入；真正的父子关系由数据自身保证）
    let mut insert_order: Vec<String> = out_tables.keys().cloned().collect();
    insert_order.sort_by_key(|t| match t.as_str() {
        "project" => 0,
        "world" => 1,
        "entity" => 2,
        "entity_type" => 3,
        _ => 10,
    });

    Ok(ProjectExport {
        format_version: EXPORT_FORMAT_VERSION,
        exported_at: chrono::Utc::now().to_rfc3339(),
        root_project_id: pid,
        tables: out_tables,
        insert_order,
    })
}
