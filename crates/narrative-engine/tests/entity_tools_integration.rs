//! Entity 领域工具集成测试
//!
//! 需要 PostgreSQL 环境（读取 DATABASE_URL）。验证 create → revise → get →
//! retire（逻辑删除）与 create_relation → end_relation（语义化结束）全链路，
//! 并直接查库确认：retire 后行仍在但 status='Deleted'；end_relation 后
//! valid_until 被设置——均非物理 DELETE。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use application::entity_service::EntityService;
use application::mutation::MutationCommitter;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbEntityRepositoryPort, DbProjectRepositoryPort, DbWorldRepositoryPort};
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use narrative_engine::agent_tools::register_entity_tools;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;


fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

#[tokio::test]
async fn entity_tools_create_revise_retire_logical_delete() -> Result<()> {
    let pool = testkit::test_pool().await?;

    // 项目 + 主世界
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let project = project_service
        .create_project("tool-test-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(project["id"].as_str().expect("project id")).unwrap();

    let world_service = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())));
    let world = world_service
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界应已创建");
    let world_id = world.id;

    // Entity service + 工具注册
    let entity_service = Arc::new(EntityService::new(
        Arc::new(DbEntityRepositoryPort::new(pool.clone())),
        Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(
            pool.clone(),
        )))),
        Arc::new(DbProjectResolverPort::new(pool.clone())),
        "user",
    ));
    let registry = Arc::new(ToolRegistry::new());
    register_entity_tools(&registry, entity_service);

    // 1) 创建角色
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world_id.to_string(), "name": "林夜" }))
        .await?;
    assert!(created["ok"].as_bool().is_some_and(|b| b));
    let char_id = Uuid::parse_str(created["data"]["id"].as_str().expect("char id")).unwrap();

    // 2) 修改角色名
    let revised = tool(&registry, "revise_entity")
        .execute(json!({ "id": char_id.to_string(), "name": "林夜·改" }))
        .await?;
    assert_eq!(revised["data"]["name"], "林夜·改");

    // 3) 读取确认修改
    let got = tool(&registry, "get_entity")
        .execute(json!({ "id": char_id.to_string() }))
        .await?;
    assert_eq!(got["data"]["name"], "林夜·改");

    // 4) 创建一个地点作为关系目标
    let loc = tool(&registry, "create_location")
        .execute(json!({ "world_id": world_id.to_string(), "name": "黑石城" }))
        .await?;
    let loc_id = Uuid::parse_str(loc["data"]["id"].as_str().expect("loc id")).unwrap();

    // 5) 创建关系
    let rel = tool(&registry, "create_relation")
        .execute(json!({
            "source_entity_id": char_id.to_string(),
            "target_entity_id": loc_id.to_string(),
            "relation_type": "enemy"
        }))
        .await?;
    let rel_id = Uuid::parse_str(rel["data"]["id"].as_str().expect("rel id")).unwrap();

    // 6) 逻辑删除角色（retire_entity）
    let retired = tool(&registry, "retire_entity")
        .execute(json!({ "id": char_id.to_string() }))
        .await?;
    assert!(retired["ok"].as_bool().is_some_and(|b| b));

    // 7) 删除后读取应失败（软删除：get_entity 过滤 status='Deleted'）
    let after = tool(&registry, "get_entity")
        .execute(json!({ "id": char_id.to_string() }))
        .await;
    assert!(after.is_err(), "逻辑删除后 get_entity 应返回错误");

    // 8) 直接查库：行仍存在，但 status='Deleted'（非物理删除）
    let status: String = sqlx::query_scalar("SELECT status FROM entity WHERE id = $1")
        .bind(char_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(status, "Deleted", "retire_entity 应为软删除，行仍存在");

    // 9) 语义化结束关系（end_relation）
    let ended = tool(&registry, "end_relation")
        .execute(json!({ "id": rel_id.to_string() }))
        .await?;
    assert!(ended["ok"].as_bool().is_some_and(|b| b));

    // 10) 直接查库：关系行仍存在，valid_until 已设置（非物理删除）
    let valid_until: Option<String> = sqlx::query_scalar(
        "SELECT valid_until::text FROM relation WHERE id = $1",
    )
    .bind(rel_id)
    .fetch_one(&pool)
    .await?;
    assert!(
        valid_until.is_some(),
        "end_relation 应设 valid_until 而非物理删除"
    );

    Ok(())
}
