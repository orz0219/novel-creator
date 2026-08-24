//! Phase B 领域工具集成测试
//!
//! 需要 PostgreSQL 环境（读取 DATABASE_URL）。验证 Narrative / Storyline / Rule /
//! Project / World / History 聚合工具的创建、修改、读取与逻辑删除（remove_node 后
//! get_node 失败，即软删除）全链路。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use narrative_engine::agent_tools::register_all_domain_tools;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn test_pool() -> Result<PgPool> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string()
    });
    Ok(PgPool::connect(&url).await?)
}

fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

#[tokio::test]
async fn phase_b_tools_crud_logical_delete() -> Result<()> {
    let pool = test_pool().await?;
    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    // 1) 项目
    let proj = tool(&registry, "create_project")
        .execute(json!({ "name": "pb-test-project" }))
        .await?;
    let project_id = Uuid::parse_str(proj["data"]["id"].as_str().expect("proj id")).unwrap();
    let list_p = tool(&registry, "list_projects").execute(json!({})).await?;
    assert!(
        list_p["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == proj["data"]["id"]),
        "list_projects 应含新建项目"
    );

    // 2) 叙事节点：create → revise → remove（逻辑删除）
    let node = tool(&registry, "create_node")
        .execute(json!({
            "project_id": project_id.to_string(),
            "node_type": "Chapter",
            "title": "第1章"
        }))
        .await?;
    let node_id = Uuid::parse_str(node["data"]["id"].as_str().expect("node id")).unwrap();

    let revised = tool(&registry, "revise_node")
        .execute(json!({ "id": node_id.to_string(), "title": "第1章·改" }))
        .await?;
    assert_eq!(revised["data"]["title"], "第1章·改");

    let list_n = tool(&registry, "list_nodes")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    assert!(
        list_n["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["id"] == node["data"]["id"]),
        "list_nodes 应含新建节点"
    );

    tool(&registry, "remove_node")
        .execute(json!({ "id": node_id.to_string() }))
        .await?;
    // 逻辑删除后不应再被读取到（已统一过滤 status='Deleted'，与 Entity 一致）
    let after = tool(&registry, "get_node")
        .execute(json!({ "id": node_id.to_string() }))
        .await;
    assert!(after.is_err(), "remove_node 后 get_node 应失败（软删除已过滤读取）");
    // 且底层确为软删（status='Deleted'），行仍存在而非物理 DELETE
    let nstatus: Option<Option<String>> = sqlx::query_scalar(
        "SELECT status FROM narrative_node WHERE id = $1",
    )
    .bind(node_id)
    .fetch_optional(&pool)
    .await?;
    assert!(
        matches!(nstatus, Some(Some(s)) if s == "Deleted"),
        "remove_node 应为软删除（status='Deleted'），行仍存在而非物理删除"
    );

    // 3) 剧情线
    let sl = tool(&registry, "create_storyline")
        .execute(json!({ "project_id": project_id.to_string(), "name": "主线" }))
        .await?;
    let sls = tool(&registry, "list_storylines")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    assert!(
        sls["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == sl["data"]["id"]),
        "list_storylines 应含新建剧情线"
    );

    // 4) 主世界 + 世界规则
    let world = tool(&registry, "get_main_world")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    let world_id = Uuid::parse_str(world["data"]["id"].as_str().expect("world id")).unwrap();
    let rule = tool(&registry, "create_rule")
        .execute(json!({ "world_id": world_id.to_string(), "rule_content": "禁止主角无故死亡" }))
        .await?;
    let rules = tool(&registry, "list_rules")
        .execute(json!({ "world_id": world_id.to_string() }))
        .await?;
    assert!(
        rules["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == rule["data"]["id"]),
        "list_rules 应含新建规则"
    );

    // 5) 历史事件
    let ev = tool(&registry, "create_event")
        .execute(json!({
            "project_id": project_id.to_string(),
            "name": "开篇",
            "description": "故事开始"
        }))
        .await?;
    assert!(ev["ok"].as_bool().is_some_and(|b| b));
    let evs = tool(&registry, "list_events")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    assert!(evs["data"].as_array().unwrap().len() >= 1);

    Ok(())
}
