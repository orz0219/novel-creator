//! 人物阶段弧线（arc_stages）集成测试
//!
//! 需要 PostgreSQL 环境（读取 DATABASE_URL，默认本地 novel_engine）。
//! 覆盖：update_character_profile 写入 arc_stages / conflicts.phase →
//! get_character_profile 读回 → 直接查库确认落表。

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
use narrative_engine::agent_tools::{register_character_tools, register_entity_tools};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;


fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

#[tokio::test]
async fn character_arc_stages_roundtrip() -> Result<()> {
    let pool = testkit::test_pool().await?;

    // 项目 + 主世界
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let project = project_service
        .create_project("arc-stage-test-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(project["id"].as_str().expect("project id")).unwrap();

    let world_service = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())));
    let world = world_service
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界应已创建");
    let world_id = world.id;

    let entity_service = Arc::new(EntityService::new(
        Arc::new(DbEntityRepositoryPort::new(pool.clone())),
        Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(
            pool.clone(),
        )))),
        Arc::new(DbProjectResolverPort::new(pool.clone())),
        "user",
    ));
    let registry = Arc::new(ToolRegistry::new());
    register_entity_tools(&registry, entity_service.clone());
    register_character_tools(&registry, entity_service);

    // 关键：确认 arc_stages / screen_weight / conflicts.phase 真的会随工具列表
    // 拼进 LLM 的系统提示词——不然字段只落库、AI 根本看不到。
    let tools = registry.list();
    let prompt = agent::prompt::build_system_prompt(
        "你是小说创作助手。",
        "characters.protagonist",
        &tools,
        &[],
    );
    assert!(
        prompt.contains("arc_stages: array<"),
        "arc_stages 应以字段形式出现在工具列表里: {prompt}"
    );
    assert!(
        prompt.contains("screen_weight: enum(Light|Medium|Heavy)"),
        "screen_weight 的枚举取值应暴露给 LLM"
    );
    assert!(prompt.contains("entry_trigger"), "entry_trigger 应暴露给 LLM");
    assert!(
        prompt.contains("update_character_profile"),
        "update_character_profile 工具应已注册"
    );

    // 1) 建角色
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world_id.to_string(), "name": "周浩" }))
        .await?;
    let char_id = Uuid::parse_str(created["data"]["id"].as_str().expect("char id")).unwrap();

    // 2) 写入三个阶段 + 一条带 phase 的冲突（故意混用字符串 / 对象 / 中文戏份）
    let updated = tool(&registry, "update_character_profile")
        .execute(json!({
            "id": char_id.to_string(),
            "role_in_story": "盟友",
            "arc_stages": [
                { "stage": "前期", "role": "普通同事", "screen_weight": "轻", "goal": "上班、攒首付" },
                { "stage": "中期", "role": "第一个合伙人", "screen_weight": "Medium",
                  "goal": "参与进去，别被留在门外", "entry_trigger": "主角决定开公司" },
                { "stage": "后期", "role": "独当一面的人", "screen_weight": "Heavy",
                  "order": 9, "function": "落地种田流的情感内核" }
            ],
            "conflicts": [
                { "conflict_type": "内心", "description": "怕被排除在外", "phase": "中期" }
            ]
        }))
        .await?;
    assert!(updated["ok"].as_bool().is_some_and(|b| b), "update 应成功: {updated}");

    // 3) 读回：三个阶段 / 中文戏份已规范化为枚举 / 冲突带 phase
    let got = tool(&registry, "get_character_profile")
        .execute(json!({ "id": char_id.to_string() }))
        .await?;
    let stages = got["data"]["arc_stages"].as_array().expect("arc_stages 应为数组");
    assert_eq!(stages.len(), 3, "应写入三个阶段: {got}");
    assert_eq!(stages[0]["stage"], "前期");
    assert_eq!(stages[0]["screen_weight"], "Light", "中文「轻」应规范化为 Light");
    assert_eq!(stages[2]["screen_weight"], "Heavy");
    assert_eq!(stages[2]["order"], 9);
    assert_eq!(stages[1]["entry_trigger"], "主角决定开公司");

    let conflicts = got["data"]["conflicts"].as_array().expect("conflicts 应为数组");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0]["phase"], "中期", "冲突应保留生效阶段");

    // 3.5) writable_fields 默认只给**字段名清单**（完整结构实测 5,944 字符/次，
    //      改为按需：传 include_schema: true 才给）。新增字段必须出现在清单里。
    let names = got["writable_fields"]["names"]
        .as_array()
        .unwrap_or_else(|| panic!("默认应给字段名清单: {}", got["writable_fields"]));
    assert!(
        names.iter().any(|v| v == "arc_stages"),
        "字段清单应含 arc_stages: {names:?}"
    );
    assert!(
        names.iter().any(|v| v == "social_position"),
        "字段清单应含 social_position: {names:?}"
    );

    // 显式要完整结构时，conflicts 的元素字段不能被吞掉 —— 之前 compact_schema_value
    // 会把所有叫 description 的键删掉，导致模型以为冲突不用写描述。
    let got_full = tool(&registry, "get_character_profile")
        .execute(json!({ "id": char_id.to_string(), "include_schema": true }))
        .await?;
    let wf = &got_full["writable_fields"];
    assert!(
        wf["conflicts"]["items"]["properties"].get("phase").is_some(),
        "conflicts[].phase 应出现在完整结构里"
    );
    assert!(
        wf["conflicts"]["items"]["properties"].get("description").is_some(),
        "conflicts[].description 不能从完整结构里消失"
    );

    // 3.6) drive 只传 motivation、不传 urgency，应成功（urgency 默认 3，不是必填）
    tool(&registry, "update_character_profile")
        .execute(json!({
            "id": char_id.to_string(),
            "drive": { "motivation": "想跟着兄弟干出点名堂" },
            "social_position": { "rank": "合伙人" }
        }))
        .await
        .expect("drive 缺 urgency 不应报错");
    let got_drive = tool(&registry, "get_character_profile")
        .execute(json!({ "id": char_id.to_string() }))
        .await?;
    assert_eq!(
        got_drive["data"]["drive"]["urgency"], 3,
        "urgency 缺省应落成默认值 3"
    );
    assert_eq!(got_drive["data"]["social_position"]["rank"], "合伙人");

    // 4) merge 模式：只加一段、删一段，未提及的保留
    tool(&registry, "update_character_profile")
        .execute(json!({
            "id": char_id.to_string(),
            "arc_stages": [
                { "stage": "中期", "goal": "改后的中期目标" },
                { "stage": "终章", "role": "荣归", "screen_weight": "Light" }
            ],
            "arc_stages_mode": "merge",
            "remove_arc_stages": ["后期"]
        }))
        .await?;
    let got2 = tool(&registry, "get_character_profile")
        .execute(json!({ "id": char_id.to_string() }))
        .await?;
    let stages2 = got2["data"]["arc_stages"].as_array().expect("arc_stages 应为数组");
    assert_eq!(stages2.len(), 3, "merge 后应为 前期/中期/终章: {got2}");
    let names: Vec<&str> = stages2.iter().filter_map(|s| s["stage"].as_str()).collect();
    assert!(names.contains(&"前期"), "未提及的「前期」应保留");
    assert!(names.contains(&"终章"), "新增的「终章」应写入");
    assert!(!names.contains(&"后期"), "remove_arc_stages 应删掉「后期」");
    let mid = stages2
        .iter()
        .find(|s| s["stage"] == "中期")
        .expect("中期应存在");
    assert_eq!(mid["goal"], "改后的中期目标");
    assert_eq!(mid["role"], "第一个合伙人", "merge 只改 goal，role 应保留");

    // 5) 直接查库确认落表
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entity_arc_stage WHERE entity_id = $1")
            .bind(char_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 3);

    // 6) 回归 secrets 的「整组替换」：泛化 arc_stage 时 CharacterSecretRepo::replace_by_entity
    //    被重建过，这里跑一遍真实链路确认没坏。
    tool(&registry, "update_character_profile")
        .execute(json!({
            "id": char_id.to_string(),
            "secrets": [{ "content": "他其实早就知道真相", "importance": 4 }]
        }))
        .await?;
    let got_secret = tool(&registry, "get_character_profile")
        .execute(json!({ "id": char_id.to_string() }))
        .await?;
    let secrets = got_secret["data"]["secrets"]
        .as_array()
        .expect("secrets 应为数组");
    assert_eq!(secrets.len(), 1, "secrets 应整组替换");
    assert_eq!(secrets[0]["content"], "他其实早就知道真相");

    Ok(())
}
