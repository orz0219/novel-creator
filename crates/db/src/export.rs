//! 项目数据导出（电脑端 → 手机端单向迁移）
//!
//! 设计要点：
//!   - **元数据驱动**，不手工枚举表：从 pg_catalog 读列类型，通用构造 SQL。
//!     否则加一张表就要改一次导出代码。
//!   - **按项目导出**，不是全量 dump：手机上只需要某个项目，全量会带上无
//!     关联的测试数据且文件过大。
//!   - **递归收集关联行**：从 project 出发沿外键链收集（agent_sessions →
//!     agent_messages 这类间接关联也不会漏）。
//!   - **uuid / 时间在导出时就转成文本**，避免 JSON 里出现二进制或本地格式。
//!
//! 输出 JSON 结构：
//! ```json
//! {
//!   "format_version": 1,
//!   "exported_at": "2026-09-12T...",
//!   "schema_generation": 1,
//!   "root_project_id": "uuid",
//!   "tables": {
//!     "project": { "columns": ["id","name",...], "rows": [ {"id":"...","name":"..."} ] },
//!     ...
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet, VecDeque};

/// 当前导出格式版本。手机端导入会校验它。
pub const EXPORT_FORMAT_VERSION: i32 = 2;

/// 不属于任何项目、导入时绝不能替换的表。
///
/// `app_settings` 是**本机（本服务）级**的全局配置——AI 网关地址、密钥、模型、
/// 界面偏好等，不是项目数据。它既没有 `project_id` 也没有任何关联列，
/// 早期版本因此把它当成「全局字典表」整表导出，结果是：
/// 用另一台设备的导出文件导入，会把本机的 AI 配置连同用户手填的密钥一起
/// 替换掉，而且没有任何提示。
pub const NON_PROJECT_TABLES: &[&str] = &["app_settings"];

#[derive(Debug, Serialize, Deserialize)]
pub struct TableExport {
    pub columns: Vec<String>,
    /// 列名 -> PostgreSQL 类型名（uuid / timestamptz / text / jsonb ...）
    ///
    /// 必须有：手机端 SQLite 是弱类型，需要用这个信息决定绑定类型。
    /// 尤其 uuid —— 只有用 sqlx 的 `Uuid` 绑定才会落成 16 字节 BLOB，
    /// 否则读回时会报 invalid length。
    #[serde(default)]
    pub column_types: std::collections::HashMap<String, String>,
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectExport {
    pub format_version: i32,
    pub exported_at: String,
    pub root_project_id: String,
    pub tables: HashMap<String, TableExport>,
    /// 本次导出涉及的表的写入顺序（外键拓扑序），手机端按此顺序插入
    pub insert_order: Vec<String>,
}

/// 判断某列是否参与「按列值反查」。
///
/// 规则：列名是 id 或以 `_id` 结尾，且不是主键本身。
/// 之所以动态判断而不是维护固定清单：库里关联列五花八门
/// （narrative_node_id、pov_character_id、parent_version_id ...），
/// 列清单一定会漏，漏了就会像 scene 那样整表导不出来。
fn is_link_column(name: &str, pk: &str) -> bool {
    if name == pk {
        return false;
    }
    name == "id" || name.ends_with("_id")
}

/// 外键边：从某表某列指向另一表
#[derive(Debug, Clone)]
struct FkEdge {
    table: String,
    column: String,
    ref_table: String,
}

/// 表的列元数据
#[derive(Debug, Clone)]
struct ColMeta {
    name: String,
    udt: String,
}

/// 导出指定项目的全部相关数据
pub async fn export_project(pool: &PgPool, project_id: &str) -> Result<ProjectExport> {
    let fks = load_foreign_keys(pool).await?;
    let all_tables = load_all_tables(pool).await?;
    // 反向索引：被引用的表 -> 引用它的 (表, 列)
    let mut incoming: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for e in &fks {
        incoming
            .entry(e.ref_table.clone())
            .or_default()
            .push((e.table.clone(), e.column.clone()));
    }

    // 收集策略（重要，经过实测修正）：
    //
    // 这个库里很多表只是「靠列值相等」关联，根本没有外键约束
    // （agent_sessions.project_id、world_version.world_id 都没有 FK），
    // 因此不能只靠外键链遍历 —— 会漏掉整条对话数据与世界版本历史。
    //
    // 采用「按列值反查 + 迭代闭包」：
    //   1. 先收集 project 自身，以及所有含 project_id 的表
    //   2. 把已收集行的主键放进 seen_ids
    //   3. 反复扫描尚未触达的表：凡是它的关联列取值命中 seen_ids，就收下这些行
    //   4. 直到一轮下来没有任何新增（闭包完成）
    //
    // 这样「world → world_version」「agent_sessions → agent_messages」这类
    // 没有 FK 的关联都能被正确带出。
    let mut collected: HashMap<String, Vec<String>> = HashMap::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    let root_id = normalize_uuid_text(project_id);

    let mut push = |table: &str, id: String,
                    seen: &mut HashSet<(String, String)>,
                    seen_ids: &mut HashSet<String>,
                    collected: &mut HashMap<String, Vec<String>>| {
        if seen.insert((table.to_string(), id.clone())) {
            seen_ids.insert(id.clone());
            collected.entry(table.to_string()).or_default().push(id);
        }
    };

    // 1. project 自身
    push("project", root_id.clone(), &mut seen, &mut seen_ids, &mut collected);

    // 2. 所有含 project_id 的表
    for t in &all_tables {
        let cols = load_columns(pool, t).await?;
        let Some(col) = cols.iter().find(|c| c.name == "project_id") else {
            continue;
        };
        let pk = primary_key(pool, t).await?;
        let ids = load_ids_by_value(pool, t, &pk, &col.name, &col.udt, &root_id).await?;
        for id in ids {
            push(t, id, &mut seen, &mut seen_ids, &mut collected);
        }
    }

    // 3. 迭代闭包：对所有「关联列」（*_id）做反查，直到没有新行
    loop {
        let mut added = 0usize;
        let ids_vec: Vec<String> = seen_ids.iter().cloned().collect();

        for t in &all_tables {
            if collected.get(t).map(|v| !v.is_empty()).unwrap_or(false) {
                continue; // 已收集过的表不再扫描（其行本身已在 seen_ids 中参与比较）
            }
            let cols = load_columns(pool, t).await?;
            let pk = primary_key(pool, t).await?;
            let link_cols: Vec<&ColMeta> = cols
                .iter()
                .filter(|c| c.udt == "uuid" && is_link_column(&c.name, &pk))
                .collect();
            if link_cols.is_empty() {
                continue;
            }

            let mut hits: Vec<String> = Vec::new();
            for col in link_cols {
                let sql = format!(
                    "SELECT {}::text FROM {} WHERE {}::text = ANY($1)",
                    quote_ident(&pk),
                    quote_ident(t),
                    quote_ident(&col.name)
                );
                let rows = sqlx::query(&sql)
                    .bind(&ids_vec)
                    .fetch_all(pool)
                    .await
                    .with_context(|| format!("反查 {}.{} 失败", t, col.name))?;
                for r in rows {
                    hits.push(r.get::<String, _>(0));
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

    // 组装每张表的数据
    let mut tables: HashMap<String, TableExport> = HashMap::new();

    for (table, ids) in &collected {
        let cols = load_columns(pool, table).await?;
        let pk = primary_key(pool, table).await?;
        let ddl = build_select(table, &pk, &cols);
        let mut rows_export = Vec::new();

        for id in ids {
            let uid = uuid::Uuid::parse_str(id)
                .with_context(|| format!("无效的 uuid: {}", id))?;
            let row = sqlx::query(&ddl)
                .bind(uid)
                .fetch_optional(pool)
                .await
                .with_context(|| format!("导出 {}.{} 失败", table, id))?;
            if let Some(r) = row {
                let mut obj = serde_json::Map::new();
                for c in &cols {
                    obj.insert(c.name.clone(), read_value(&r, &c.name, &c.udt)?);
                }
                rows_export.push(obj);
            }
        }

        if !rows_export.is_empty() {
            tables.insert(
                table.clone(),
                TableExport {
                    columns: cols.iter().map(|c| c.name.clone()).collect(),
                    column_types: cols
                        .iter()
                        .map(|c| (c.name.clone(), c.udt.clone()))
                        .collect(),
                    rows: rows_export,
                },
            );
        }
    }

    // 全局表与项目表的分界：
    //   带 project_id / world_id / entity_id / session_id 的表属于「项目内数据」，
    //   没被收集到就是该项目没有这类数据，不应导出；
    //   不带的（entity_type、standard_relation_type、skill 等）是全局字典表，
    //   手机端需要它们才能正常建档，必须整体带上。
    for t in &all_tables {
        if tables.contains_key(t) {
            continue;
        }
        if NON_PROJECT_TABLES.contains(&t.as_str()) {
            continue; // 本机级配置，不属于项目数据（见常量说明）
        }
        let cols = load_columns(pool, t).await?;
        let project_scoped = cols.iter().any(|c| {
            matches!(c.name.as_str(), "project_id" | "world_id" | "entity_id" | "session_id")
        });
        if project_scoped {
            continue; // 项目内数据但本项没有，跳过
        }
        let sql = build_select_all(t, &cols);
        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await
            .with_context(|| format!("导出全局表 {} 失败", t))?;
        let mut rows_export = Vec::new();
        for r in rows {
            let mut obj = serde_json::Map::new();
            for c in &cols {
                obj.insert(c.name.clone(), read_value(&r, &c.name, &c.udt)?);
            }
            rows_export.push(obj);
        }
        if !rows_export.is_empty() {
            tables.insert(
                t.clone(),
                TableExport {
                    columns: cols.iter().map(|c| c.name.clone()).collect(),
                    column_types: cols
                        .iter()
                        .map(|c| (c.name.clone(), c.udt.clone()))
                        .collect(),
                    rows: rows_export,
                },
            );
        }
    }

    // 插入顺序：外键拓扑序（被引用的表在前）
    let insert_order = topo_order(&tables.keys().cloned().collect::<Vec<_>>(), &fks);

    Ok(ProjectExport {
        format_version: EXPORT_FORMAT_VERSION,
        exported_at: chrono::Utc::now().to_rfc3339(),
        root_project_id: root_id,
        tables,
        insert_order,
    })
}

/// 把 uuid 规范成小写带连字符的文本
fn normalize_uuid_text(s: &str) -> String {
    match uuid::Uuid::parse_str(s) {
        Ok(u) => u.to_string(),
        Err(_) => s.to_string(),
    }
}

/// 查表的主键列名（绝大多数是 id，mutation_ledger 是 command_id）
async fn primary_key(pool: &PgPool, table: &str) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT sa.attname::text FROM pg_constraint con \
         JOIN pg_class c ON c.oid = con.conrelid \
         JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname='public' \
         JOIN pg_attribute sa ON sa.attrelid = con.conrelid AND sa.attnum = con.conkey[1] \
         WHERE con.contype='p' AND c.relname = $1",
    )
    .bind(table)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("读取 {} 的主键失败", table))?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| "id".to_string()))
}

/// 读取全部主键为 uuid 的表名
async fn load_all_tables(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT t.table_name::text FROM information_schema.tables t \
         WHERE t.table_schema='public' AND t.table_type='BASE TABLE' \
           AND t.table_name <> '_migrations' \
         ORDER BY t.table_name",
    )
    .fetch_all(pool)
    .await
    .context("读取表清单失败")?;
    Ok(rows.iter().map(|r| r.get::<String, _>(0)).collect())
}

/// 读取某表的列与类型
async fn load_columns(pool: &PgPool, table: &str) -> Result<Vec<ColMeta>> {
    let rows = sqlx::query(
        "SELECT a.attname::text, t.typname::text \
         FROM pg_attribute a \
         JOIN pg_class c ON c.oid = a.attrelid \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         JOIN pg_type t ON t.oid = a.atttypid \
         WHERE n.nspname='public' AND c.relname=$1 AND a.attnum>0 AND NOT a.attisdropped \
         ORDER BY a.attnum",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .with_context(|| format!("读取 {} 的列失败", table))?;

    Ok(rows
        .iter()
        .map(|r| ColMeta {
            name: r.get::<String, _>(0),
            udt: r.get::<String, _>(1),
        })
        .collect())
}

/// 读取全部外键关系
async fn load_foreign_keys(pool: &PgPool) -> Result<Vec<FkEdge>> {
    let rows = sqlx::query(
        "SELECT st.relname::text AS tbl, sa.attname::text AS col, \
                rt.relname::text AS ref_tbl, ra.attname::text AS ref_col \
         FROM pg_constraint con \
         JOIN pg_class st ON st.oid = con.conrelid \
         JOIN pg_class rt ON rt.oid = con.confrelid \
         JOIN pg_namespace sn ON sn.oid = st.relnamespace AND sn.nspname='public' \
         JOIN LATERAL unnest(con.conkey) AS k(att) ON true \
         JOIN LATERAL unnest(con.confkey) AS rf(refatt) ON true \
         JOIN pg_attribute sa ON sa.attrelid = con.conrelid AND sa.attnum = k.att \
         JOIN pg_attribute ra ON ra.attrelid = con.confrelid AND ra.attnum = rf.refatt \
         WHERE con.contype = 'f'",
    )
    .fetch_all(pool)
    .await
    .context("读取外键失败")?;

    let mut out = Vec::new();
    for r in rows {
        out.push(FkEdge {
            table: r.get::<String, _>(0),
            column: r.get::<String, _>(1),
            ref_table: r.get::<String, _>(2),
        });
    }
    Ok(out)
}

/// 按「某列 = 给定值」查主键集合。绑定类型按该列自身的类型决定。
async fn load_ids_by_value(
    pool: &PgPool,
    table: &str,
    pk: &str,
    column: &str,
    column_udt: &str,
    value: &str,
) -> Result<Vec<String>> {
    let sql = format!(
        "SELECT {}::text FROM {} WHERE {} = $1",
        quote_ident(pk),
        quote_ident(table),
        quote_ident(column)
    );
    let rows = if column_udt == "uuid" {
        let u = uuid::Uuid::parse_str(value)
            .with_context(|| format!("无效的 uuid: {}", value))?;
        sqlx::query(&sql).bind(u).fetch_all(pool).await
    } else {
        sqlx::query(&sql).bind(value).fetch_all(pool).await
    }
    .with_context(|| format!("收集 {}.{} 失败", table, column))?;

    Ok(rows.iter().map(|r| r.get::<String, _>(0)).collect())
}

/// 查某个表里「某列 = 给定值」的全部主键，返回 id 的文本形式。
///
/// 关键：绑定值必须与被比较列的类型一致（uuid 列绑 Uuid，否则 PG 会报
/// 「operator does not exist: uuid = text」），所以先查该列的类型再决定绑定方式。
async fn load_column_values(
    pool: &PgPool,
    table: &str,
    column: &str,
    _ref_table: &str,
    ref_id: &str,
) -> Result<Vec<String>> {
    let col_type = column_type(pool, table, column).await?;
    let pk = primary_key(pool, table).await?;
    let sql = format!(
        "SELECT {}::text FROM {} WHERE {} = $1",
        quote_ident(&pk),
        quote_ident(table),
        quote_ident(column)
    );

    let rows = if col_type == "uuid" {
        let u = uuid::Uuid::parse_str(ref_id)
            .with_context(|| format!("无效的 uuid: {}", ref_id))?;
        sqlx::query(&sql)
            .bind(u)
            .fetch_all(pool)
            .await
            .with_context(|| format!("收集 {}.{} 失败", table, column))?
    } else {
        sqlx::query(&sql)
            .bind(ref_id)
            .fetch_all(pool)
            .await
            .with_context(|| format!("收集 {}.{} 失败", table, column))?
    };

    Ok(rows.iter().map(|r| r.get::<String, _>(0)).collect())
}

async fn column_type(pool: &PgPool, table: &str, column: &str) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT t.typname::text FROM pg_attribute a \
         JOIN pg_class c ON c.oid=a.attrelid \
         JOIN pg_namespace n ON n.oid=c.relnamespace \
         JOIN pg_type t ON t.oid=a.atttypid \
         WHERE n.nspname='public' AND c.relname=$1 AND a.attname=$2",
    )
    .bind(table)
    .bind(column)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0).unwrap_or_default())
}

fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// 构造按主键查单行的 SELECT（主键是 uuid，绑定也用 Uuid）
fn build_select(table: &str, pk: &str, cols: &[ColMeta]) -> String {
    let mut parts = Vec::new();
    for c in cols {
        parts.push(expr_for(c));
    }
    format!(
        "SELECT {} FROM {} WHERE {} = $1",
        parts.join(", "),
        quote_ident(table),
        quote_ident(pk)
    )
}

fn build_select_all(table: &str, cols: &[ColMeta]) -> String {
    let mut parts = Vec::new();
    for c in cols {
        parts.push(expr_for(c));
    }
    format!("SELECT {} FROM {}", parts.join(", "), quote_ident(table))
}

/// 每列的 SELECT 表达式：统一产出「文本」，由 read_value 按类型还原。
///
/// - 文本类列直接取原值（避免被 to_json 加引号后再解析时产生歧义）
/// - uuid/json 用 ::text
/// - 其余（数值/布尔/时间）用 to_json(...)::text 以保留精确语义
fn expr_for(c: &ColMeta) -> String {
    let id = quote_ident(&c.name);
    // 必须显式 AS 出原名：像 to_json(x)::text 这种表达式，PG 会自动生成
    // 形如 "to_json" 的列名，导致后面按名字取值失败（no column found for name）。
    let alias = quote_ident(&c.name);
    let body = match c.udt.as_str() {
        "text" | "varchar" | "bpchar" | "char" | "name" => id,
        "uuid" | "json" | "jsonb" => format!("{}::text", id),
        _ => format!("to_json({})::text", id),
    };
    format!("{} AS {}", body, alias)
}

/// 把查询结果的一列读成 JSON 值。
///
/// 关键：不能对所有类型都用 `to_json(...)::text` 再解析 —— 那样
/// 文本列里形如 `123`、`true`、`[1,2]` 的内容会被误解析成数字/布尔/数组。
/// 因此文本列直接按字符串读取，只有非文本列才走「to_json 文本再解析」。
fn read_value(row: &sqlx::postgres::PgRow, column: &str, udt: &str) -> Result<serde_json::Value> {
    let raw: Option<String> = row
        .try_get(column)
        .with_context(|| format!("读取列 {} 失败", column))?;

    let Some(text) = raw else {
        return Ok(serde_json::Value::Null);
    };

    match udt {
        // 纯文本：原样保留，绝不做 JSON 解析
        "text" | "varchar" | "bpchar" | "char" | "name" => Ok(serde_json::Value::String(text)),
        // uuid 已 ::text，也是纯文本
        "uuid" => Ok(serde_json::Value::String(text)),
        // JSON 列：文本本身就是 JSON
        "json" | "jsonb" => {
            Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)))
        }
        // 其余（数值/布尔/时间等）：由 to_json(...)::text 产出，需要再解析
        _ => Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))),
    }
}

/// 外键拓扑排序（被引用的表排在前面）
fn topo_order(tables: &[String], fks: &[FkEdge]) -> Vec<String> {
    let set: HashSet<&String> = tables.iter().collect();
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for t in tables {
        deps.insert(t.clone(), HashSet::new());
    }
    for e in fks {
        if e.table == e.ref_table {
            continue; // 自引用不构成依赖
        }
        if set.contains(&e.table) && set.contains(&e.ref_table) {
            deps.get_mut(&e.table).unwrap().insert(e.ref_table.clone());
        }
    }

    let mut ordered = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    while ordered.len() < tables.len() {
        let mut progressed = false;
        let mut candidates: Vec<&String> = tables.iter().filter(|t| !done.contains(*t)).collect();
        candidates.sort();
        for t in candidates {
            let ready = deps.get(t).map(|d| d.iter().all(|x| done.contains(x))).unwrap_or(true);
            if ready {
                ordered.push(t.clone());
                done.insert(t.clone());
                progressed = true;
            }
        }
        if !progressed {
            // 有环（自引用已排除，这里兜底）：剩余表直接按名字排
            let mut rest: Vec<String> = tables.iter().filter(|t| !done.contains(*t)).cloned().collect();
            rest.sort();
            ordered.extend(rest);
            break;
        }
    }
    ordered
}
