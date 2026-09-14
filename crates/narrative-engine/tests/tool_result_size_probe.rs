//! 读工具返回体积探针（**只读**：只调用 list_* / get_*，绝不写开发库）
//!
//! 目的：不靠估算，量出每个读工具在**真实数据量**下的返回字符数，
//! 为「列表有界化 + 结果体积预算」的改造提供依据，并在改造后当回归测试用。
//!
//! 与其它集成测试的区别：它们连 `_test` 测试库（干净、小），本探针刻意连
//! **开发库**——要看的就是真实项目的数据量。
//!
//! 运行：cargo test -p narrative-engine --test tool_result_size_probe -- --nocapture

use std::sync::Arc;

use agent::ToolRegistry;
use anyhow::{Context, Result};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// 从项目根的 `.env` 读开发库地址（读不到就报错，不用默认值兜底）。
fn dev_database_url() -> String {
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    let text = std::fs::read_to_string(&env_path)
        .unwrap_or_else(|e| panic!("读不到 {}：{}", env_path.display(), e));
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("DATABASE_URL=") {
            return v.trim().to_string();
        }
    }
    panic!("{} 里没有 DATABASE_URL", env_path.display());
}

struct Measurement {
    name: String,
    chars: usize,
    items: Option<usize>,
    note: String,
    /// data 首条的字段名（用于给「列表投影该保留哪些字段」提供依据）
    first_keys: Vec<String>,
}

async fn measure(
    registry: &Arc<ToolRegistry>,
    tool_name: &str,
    display: &str,
    input: serde_json::Value,
) -> Measurement {
    let tool = registry
        .get(tool_name)
        .unwrap_or_else(|| panic!("工具 {} 未注册", tool_name));
    match tool.execute(input).await {
        Ok(v) => {
            // runtime 是这么把结果写进上下文的：pretty JSON
            let text = serde_json::to_string_pretty(&v).unwrap_or_default();
            let items = v
                .get("data")
                .and_then(|d| d.as_array())
                .map(|a| a.len());
            let first_keys = v
                .get("data")
                .and_then(|d| d.as_array())
                .and_then(|a| a.first())
                .and_then(|x| x.as_object())
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            Measurement {
                name: display.to_string(),
                chars: text.chars().count(),
                items,
                note: "ok".to_string(),
                first_keys,
            }
        }
        Err(e) => Measurement {
            name: display.to_string(),
            chars: 0,
            items: None,
            note: format!("调用失败：{}", e),
            first_keys: Vec::new(),
        },
    }
}

#[tokio::test]
async fn report_read_tool_result_sizes() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&dev_database_url())
        .await
        .context("连开发库失败（探针需要真实数据；请确认 .env 的 DATABASE_URL 可用）")?;

    // 取数据量最大的项目 = 最坏情况
    let (project_id, entity_count): (Uuid, i64) = sqlx::query_as(
        "SELECT p.id, count(e.id) FROM project p LEFT JOIN entity e ON e.project_id = p.id \
         GROUP BY p.id ORDER BY count(e.id) DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .context("开发库里一个项目都没有，探针无从测起")?;

    let world_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM world WHERE project_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .context("该项目没有世界")?;

    // 描述最长的几个实体（最坏情况）
    let entities: Vec<(String, Uuid)> = sqlx::query_as(
        "SELECT et.name, e.id FROM entity e JOIN entity_type et ON et.id = e.entity_type_id \
         WHERE e.project_id = $1 AND e.status != 'Deleted' \
         ORDER BY length(coalesce(e.description, '')) DESC LIMIT 8",
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await?;

    let node_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM narrative_node WHERE project_id = $1 LIMIT 1")
            .bind(project_id)
            .fetch_optional(&pool)
            .await?;

    println!("\n===== 读工具返回体积实测（开发库真实数据）=====");
    println!(
        "项目 {}：实体 {} 个，世界 {}",
        project_id, entity_count, world_id
    );

    let registry = Arc::new(ToolRegistry::new());
    narrative_engine::agent_tools::register_all_domain_tools(&registry, &pool);

    let mut out: Vec<Measurement> = Vec::new();
    let pid = project_id.to_string();
    let wid = world_id.to_string();

    out.push(measure(&registry, "list_projects", "list_projects", json!({})).await);
    out.push(measure(&registry, "get_project", "get_project", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "get_project_status", "get_project_status", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "get_project_index", "get_project_index", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "get_main_world", "get_main_world", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "list_entities", "list_entities", json!({ "world_id": wid })).await);
    out.push(
        measure(
            &registry,
            "list_entities",
            "list_entities(Character)",
            json!({ "world_id": wid, "entity_type": "Character" }),
        )
        .await,
    );
    out.push(measure(&registry, "list_entity_types", "list_entity_types", json!({})).await);
    out.push(measure(&registry, "list_relations", "list_relations", json!({ "world_id": wid })).await);
    out.push(measure(&registry, "list_nodes", "list_nodes", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "list_storylines", "list_storylines", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "list_foreshadows", "list_foreshadows", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "list_rules", "list_rules", json!({ "world_id": wid })).await);
    out.push(measure(&registry, "list_events", "list_events", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "list_facts", "list_facts", json!({ "project_id": pid })).await);
    out.push(measure(&registry, "list_snapshots", "list_snapshots", json!({ "project_id": pid })).await);

    if let Some(nid) = node_id {
        out.push(measure(&registry, "get_node", "get_node", json!({ "id": nid.to_string() })).await);
    }
    for (etype, id) in &entities {
        let (tool, key) = match etype.as_str() {
            "Character" => ("get_character_profile", "character"),
            "Location" => ("get_location_profile", "location"),
            "Faction" => ("get_faction_profile", "faction"),
            "golden_finger" => ("get_golden_finger", "golden_finger"),
            _ => continue,
        };
        out.push(measure(&registry, tool, tool, json!({ "id": id.to_string() })).await);
        println!("   （取的是描述最长的 {}：{}）", etype, key);
    }
    if let Some((_, id)) = entities.iter().find(|(t, _)| t == "Character") {
        out.push(
            measure(&registry, "get_character_state", "get_character_state", json!({ "id": id.to_string() })).await,
        );
        out.push(measure(&registry, "get_entity", "get_entity", json!({ "id": id.to_string() })).await);
    }

    // ---- 体积构成分解：钱花在哪个字段上 ----
    let seg = |x: &serde_json::Value| {
        serde_json::to_string_pretty(x)
            .map(|t| t.chars().count())
            .unwrap_or(0)
    };
    if let Some((_, cid)) = entities.iter().find(|(t, _)| t == "Character") {
        let v = registry
            .get("get_character_profile")
            .unwrap()
            .execute(json!({ "id": cid.to_string() }))
            .await?;
        println!("\n== get_character_profile 构成（字符）==");
        println!(
            "  data: {}  |  readonly: {}  |  writable_fields: {}  |  整包: {}",
            seg(&v["data"]),
            seg(&v["readonly"]),
            seg(&v["writable_fields"]),
            seg(&v)
        );
    }
    {
        let v = registry
            .get("list_entities")
            .unwrap()
            .execute(json!({ "world_id": wid }))
            .await?;
        let arr = v["data"].as_array().cloned().unwrap_or_default();
        let mut per_key: std::collections::BTreeMap<String, usize> = Default::default();
        for item in &arr {
            if let Some(obj) = item.as_object() {
                for (k, val) in obj {
                    *per_key.entry(k.clone()).or_default() += seg(val);
                }
            }
        }
        let mut sorted: Vec<_> = per_key.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        println!("\n== list_entities 各字段合计（{} 条）==", arr.len());
        for (k, n) in sorted.iter().take(8) {
            println!("  {:<22} {}", k, n);
        }
    }

    out.sort_by(|a, b| b.chars.cmp(&a.chars));
    println!(
        "\n{:<28} {:>9} {:>7}  {}",
        "工具", "字符数", "条目数", "备注"
    );
    println!("{}", "-".repeat(78));
    let mut total = 0usize;
    for m in &out {
        total += m.chars;
        println!(
            "{:<28} {:>9} {:>7}  {}",
            m.name,
            m.chars,
            m.items.map(|n| n.to_string()).unwrap_or_else(|| "-".into()),
            m.note
        );
    }
    println!("{}", "-".repeat(78));
    println!("合计：{} 字符（{} 次调用）", total, out.len());
    println!("最大单次：{} 字符\n", out.first().map(|m| m.chars).unwrap_or(0));

    println!("== 列表类工具的 data 首条字段（投影依据）==");
    for m in &out {
        if !m.first_keys.is_empty() {
            println!("  {:<26} {}", m.name, m.first_keys.join(", "));
        }
    }
    println!();

    Ok(())
}
