//! 阶段弧线（arc_stages）跨实体集成测试：人物 / 势力 / 地点。
//!
//! 需要 PostgreSQL（读取 DATABASE_URL）。
//! 重点验收：**小说的 AI 能不能读到这些字段** ——
//! 不仅写进库，还要出现在 get_*_profile 的返回、writable_fields、以及系统提示词里。

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
use narrative_engine::agent_tools::{
    register_all_domain_tools, register_character_tools, register_entity_tools,
    register_faction_profile_tools, register_location_profile_tools, BulkLocationProfileTool,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;


fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

#[tokio::test]
async fn arc_stages_readable_for_faction_and_location() -> Result<()> {
    let pool = testkit::test_pool().await?;

    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let project = project_service
        .create_project("arc-stage-multi-entity", None, None)
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
    register_character_tools(&registry, entity_service.clone());
    register_faction_profile_tools(&registry, entity_service.clone());
    register_location_profile_tools(&registry, entity_service.clone());
    registry.register(Arc::new(BulkLocationProfileTool::new(entity_service.clone())));

    // ---------- 系统提示词：AI 的工具清单里必须能看到 arc_stages ----------
    let prompt = agent::prompt::build_system_prompt(
        "你是小说创作助手。",
        "characters.protagonist",
        &registry.list(),
        &[],
    );
    assert!(
        prompt.contains("arc_stages: array<"),
        "arc_stages 应出现在 LLM 系统提示词的工具列表里"
    );
    assert!(
        prompt.contains("screen_weight: enum(Light|Medium|Heavy)"),
        "screen_weight 的枚举取值应暴露给 LLM"
    );

    // ---------- 势力 ----------
    let faction = tool(&registry, "create_faction")
        .execute(json!({ "world_id": world_id.to_string(), "name": "黑炎教团" }))
        .await?;
    let faction_id = Uuid::parse_str(faction["data"]["id"].as_str().expect("faction id")).unwrap();

    tool(&registry, "update_faction_profile")
        .execute(json!({
            "id": faction_id.to_string(),
            "goals": "扩张教权",
            "arc_stages": [
                { "stage": "前期", "role": "传闻中的小教团", "screen_weight": "轻",
                  "status": "三十号人，缩在一座破庙里" },
                { "stage": "中期", "role": "主角的靠山", "screen_weight": "Medium",
                  "goal": "借主角的异世界物资扩张", "entry_trigger": "与主角结盟",
                  "status": "三百人，占三座城，盟友是主角团" },
                { "stage": "后期", "role": "主要对手", "screen_weight": "Heavy",
                  "status": "三千人，半壁江山，与主角决裂" }
            ]
        }))
        .await?;

    // AI 能读到：data.arc_stages + writable_fields.arc_stages
    let got_faction = tool(&registry, "get_faction_profile")
        .execute(json!({ "id": faction_id.to_string() }))
        .await?;
    let f_stages = got_faction["data"]["arc_stages"]
        .as_array()
        .unwrap_or_else(|| panic!("势力档案应能读到 arc_stages: {got_faction}"));
    assert_eq!(f_stages.len(), 3, "势力应读到三个阶段: {got_faction}");
    assert_eq!(f_stages[1]["stage"], "中期");
    assert_eq!(f_stages[1]["screen_weight"], "Medium");
    assert_eq!(
        f_stages[1]["status"], "三百人，占三座城，盟友是主角团",
        "阶段现状快照必须能读到"
    );
    assert!(
        got_faction["writable_fields"]["names"]
            .as_array()
            .map(|a| a.iter().any(|v| v == "arc_stages"))
            .unwrap_or(false),
        "势力 writable_fields 的字段名清单应含 arc_stages: {}",
        got_faction["writable_fields"]
    );

    // 势力阶段 merge：只改一段、不动其它
    tool(&registry, "update_faction_profile")
        .execute(json!({
            "id": faction_id.to_string(),
            "arc_stages": [{ "stage": "后期", "status": "两千人，被主角团击溃" }],
            "arc_stages_mode": "merge"
        }))
        .await?;
    let got_faction2 = tool(&registry, "get_faction_profile")
        .execute(json!({ "id": faction_id.to_string() }))
        .await?;
    let f2 = got_faction2["data"]["arc_stages"].as_array().unwrap();
    assert_eq!(f2.len(), 3, "merge 后仍应有三段");
    assert_eq!(f2[0]["role"], "传闻中的小教团", "未提及的前期应保留");
    assert_eq!(f2[2]["status"], "两千人，被主角团击溃");
    assert_eq!(f2[2]["role"], "主要对手", "merge 只改 status，role 应保留");

    // ---------- 地点 ----------
    let loc = tool(&registry, "create_location")
        .execute(json!({ "world_id": world_id.to_string(), "name": "破庙" }))
        .await?;
    let loc_id = Uuid::parse_str(loc["data"]["id"].as_str().expect("loc id")).unwrap();

    tool(&registry, "update_location_profile")
        .execute(json!({
            "id": loc_id.to_string(),
            "location_type": "庙宇",
            "arc_stages": [
                { "stage": "前期", "role": "主角藏身处", "screen_weight": "Light",
                  "status": "漏雨的破庙，无人问津" },
                { "stage": "中期", "role": "教团总部", "screen_weight": "Medium",
                  "status": "翻修过，人来人往" },
                { "stage": "后期", "role": "两军对垒的战场", "screen_weight": "Heavy",
                  "entry_trigger": "教团与帝国开战" }
            ]
        }))
        .await?;

    let got_loc = tool(&registry, "get_location_profile")
        .execute(json!({ "id": loc_id.to_string() }))
        .await?;
    let l_stages = got_loc["data"]["arc_stages"]
        .as_array()
        .unwrap_or_else(|| panic!("地点档案应能读到 arc_stages: {got_loc}"));
    assert_eq!(l_stages.len(), 3, "地点应读到三个阶段: {got_loc}");
    assert_eq!(l_stages[0]["role"], "主角藏身处");
    assert_eq!(l_stages[2]["screen_weight"], "Heavy");
    assert!(
        got_loc["writable_fields"]["names"]
            .as_array()
            .map(|a| a.iter().any(|v| v == "arc_stages"))
            .unwrap_or(false),
        "地点 writable_fields 的字段名清单应含 arc_stages: {}",
        got_loc["writable_fields"]
    );

    // 批量地点路径也要支持阶段弧线（AI 常用 bulk 补齐档案）
    let bulk = tool(&registry, "bulk_update_location_profiles")
        .execute(json!({ "profiles": [{
            "id": loc_id.to_string(),
            "arc_stages": [{ "stage": "终章", "role": "战争遗迹", "screen_weight": "Light" }],
            "arc_stages_mode": "merge"
        }]}))
        .await?;
    assert!(bulk["ok"].as_bool().unwrap_or(false), "bulk 应成功: {bulk}");
    let got_loc2 = tool(&registry, "get_location_profile")
        .execute(json!({ "id": loc_id.to_string() }))
        .await?;
    let l2 = got_loc2["data"]["arc_stages"].as_array().unwrap();
    assert_eq!(l2.len(), 4, "bulk merge 应新增「终章」且保留原有三段: {got_loc2}");
    assert_eq!(l2[0]["role"], "主角藏身处", "bulk merge 不应覆盖未提及阶段");

    // ---------- 直接查库确认三实体共用一张表 ----------
    let faction_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entity_arc_stage WHERE entity_id = $1")
            .bind(faction_id)
            .fetch_one(&pool)
            .await?;
    let loc_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entity_arc_stage WHERE entity_id = $1")
            .bind(loc_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!((faction_rows, loc_rows), (3, 4));

    Ok(())
}

/// 剧情线也有阶段弧线（与人物 / 势力 / 地点**同构**）。
///
/// 回归自实际反馈：三条副线都写了明确的阶段推进（破庙 → 聚落 → 村落 → 社区化），
/// 但剧情线没有 arc_stages，只能塞进 description 当散文——面板没有阶段视图，
/// 也无法按「推进到哪个阶段」筛选。
#[tokio::test]
async fn storyline_supports_arc_stages() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("storyline-arc-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("project id")).unwrap();
    let pid = project_id.to_string();

    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    // 创建时就能写阶段：字符串简写 / 中文戏份 / stage_role 等价键都要能用
    let sl = tool(&registry, "create_storyline")
        .execute(json!({
            "project_id": pid,
            "name": "《破庙居委会》",
            "arc_stages": [
                "破庙期",
                { "stage": "聚落期", "stage_role": "自组织雏形", "screen_weight": "中" },
                { "stage": "村落期", "goal": "与旧秩序对接", "screen_weight": "Heavy" }
            ]
        }))
        .await?;
    let stages = sl["data"]["arc_stages"]
        .as_array()
        .unwrap_or_else(|| panic!("创建后应能读回阶段弧线：{sl}"));
    assert_eq!(stages.len(), 3);
    assert_eq!(stages[0]["stage"], json!("破庙期"), "字符串简写应归一化为对象");
    assert_eq!(stages[1]["role"], json!("自组织雏形"), "stage_role 应归一化到 role");
    assert_eq!(stages[1]["screen_weight"], json!("Medium"), "中文戏份应规范化");
    assert_eq!(stages[2]["screen_weight"], json!("Heavy"));

    let sid = sl["data"]["id"].as_str().expect("storyline id").to_string();

    // merge：只更新一段，未提及的段与子字段都保留
    let merged = tool(&registry, "revise_storyline")
        .execute(json!({
            "id": sid,
            "arc_stages": [{ "stage": "村落期", "status": "三村联合，议事成形" }],
            "arc_stages_mode": "merge"
        }))
        .await?;
    let stages = merged["data"]["arc_stages"].as_array().expect("merge 后应有阶段");
    assert_eq!(stages.len(), 3, "未提及的阶段必须保留：{}", merged);
    let village = stages
        .iter()
        .find(|s| s["stage"] == json!("村落期"))
        .expect("村落期仍在");
    assert_eq!(village["status"], json!("三村联合，议事成形"));
    assert_eq!(
        village["goal"],
        json!("与旧秩序对接"),
        "merge 不该清掉未传的子字段"
    );

    // 只传 remove_arc_stages：按阶段名删除，其余保留
    let removed = tool(&registry, "revise_storyline")
        .execute(json!({
            "id": sid,
            "remove_arc_stages": ["破庙期"],
            "arc_stages_mode": "merge"
        }))
        .await?;
    let stages = removed["data"]["arc_stages"].as_array().expect("删完后读取");
    assert_eq!(stages.len(), 2, "破庙期应被删除：{}", removed);
    assert!(stages.iter().all(|s| s["stage"] != json!("破庙期")));

    // 只改名：阶段与描述都不能被清掉
    let renamed = tool(&registry, "revise_storyline")
        .execute(json!({ "id": sid, "name": "《破庙居委会》（改名）" }))
        .await?;
    assert_eq!(renamed["data"]["name"], json!("《破庙居委会》（改名）"));
    assert_eq!(
        renamed["data"]["arc_stages"].as_array().unwrap().len(),
        2,
        "只改名不该动阶段弧线"
    );

    // 按 id 读单条（比 list_storylines 拉全部省上下文），且要带阶段弧线
    let one = tool(&registry, "get_storyline")
        .execute(json!({ "id": sid }))
        .await?;
    assert_eq!(one["data"]["id"], json!(sid));
    assert_eq!(
        one["data"]["arc_stages"].as_array().unwrap().len(),
        2,
        "单条读取也要带阶段弧线：{}",
        one
    );

    Ok(())
}
