//! 结构化字段校验集成测试
//!
//! 需要 PostgreSQL 环境（读取 DATABASE_URL）。锁住一条原则：
//! **字段名 / 子字段名写错必须报错**，不允许"写进陌生键 → 落库时被 serde 丢掉 →
//! 工具还回 ok: true"这种静默丢数据。
//!
//! 实测踩到过：`arc_potential: {"initial_state": ...}`（真实字段名是
//! `starting_state`）返回 ok: true，读回来三个字段全是 null。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbProjectRepositoryPort, DbWorldRepositoryPort};
use serde_json::json;
use uuid::Uuid;

fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

/// 建项目 + 主世界 + 实体服务，注册档案相关工具，返回 (registry, world_id)。
async fn setup() -> Result<(Arc<ToolRegistry>, Uuid, Uuid)> {
    let pool = testkit::test_pool().await?;

    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let project = project_service
        .create_project("field-validation-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(project["id"].as_str().expect("project id")).unwrap();

    let world_service = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())));
    let world = world_service
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界应已创建");

    // 用与线上同一套注册逻辑，避免"测试里少注册了某个工具"造成的假绿
    let registry = Arc::new(ToolRegistry::new());
    narrative_engine::agent_tools::register_all_domain_tools(&registry, &pool);

    Ok((registry, world.id, project_id))
}

/// 人物档案：`arc_potential` 子字段名写错必须报错（真实踩过的坑）。
#[tokio::test]
async fn character_profile_rejects_unknown_subfield() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world_id.to_string(), "name": "林夜" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("character id").to_string();

    let err = tool(&registry, "update_character_profile")
        .execute(json!({
            "id": id,
            "arc_potential": {
                "initial_state": "被困在旧身份里",
                "turning_point": "发现真相",
                "final_state": "成为执剑人"
            }
        }))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("initial_state"), "错误里应点名写错的子字段：{}", msg);
    assert!(
        msg.contains("starting_state"),
        "错误里应给出允许的子字段清单：{}",
        msg
    );

    // 没写进去：档案里的 arc_potential 仍是空
    let after = tool(&registry, "get_character_profile")
        .execute(json!({ "id": id }))
        .await?;
    assert!(
        after["data"]["arc_potential"].is_null(),
        "子字段名写错时不能把值写进陌生键：{}",
        after
    );

    // 写对的子字段能正常落库
    tool(&registry, "update_character_profile")
        .execute(json!({
            "id": id,
            "arc_potential": { "starting_state": "被困在旧身份里" }
        }))
        .await?;
    let ok = tool(&registry, "get_character_profile")
        .execute(json!({ "id": id }))
        .await?;
    assert_eq!(
        ok["data"]["arc_potential"]["starting_state"],
        json!("被困在旧身份里")
    );

    Ok(())
}

/// 人物档案：`drive` / `capabilities` 的子字段名同样要拦。
#[tokio::test]
async fn character_profile_rejects_unknown_drive_subfield() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world_id.to_string(), "name": "周浩" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("character id").to_string();

    let err = tool(&registry, "update_character_profile")
        .execute(json!({
            "id": id,
            "drive": { "primary_goal": "翻案", "motive": "替父亲洗清冤屈" }
        }))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("motive"), "{}", msg);
    assert!(msg.contains("motivation"), "{}", msg);

    let err = tool(&registry, "update_character_profile")
        .execute(json!({
            "id": id,
            "capabilities": { "skill": ["侦查"] }
        }))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("skill"), "{}", msg);
    assert!(msg.contains("skills"), "{}", msg);

    Ok(())
}

/// 地点档案：顶层字段名写错原先完全无声（值写进陌生键、落库被丢弃）。
#[tokio::test]
async fn location_profile_rejects_unknown_field() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_location")
        .execute(json!({ "world_id": world_id.to_string(), "name": "天柱山脉" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("location id").to_string();

    let err = tool(&registry, "update_location_profile")
        .execute(json!({ "id": id, "climte": "常年积雪" }))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("climte"), "错误里应点名写错的字段：{}", msg);
    assert!(msg.contains("climate"), "错误里应给出可写字段清单：{}", msg);

    // 字段名写对则正常写入
    tool(&registry, "update_location_profile")
        .execute(json!({ "id": id, "climate": "常年积雪" }))
        .await?;
    let after = tool(&registry, "get_location_profile")
        .execute(json!({ "id": id }))
        .await?;
    assert_eq!(after["data"]["climate"], json!("常年积雪"));

    Ok(())
}

/// 地点档案：`arc_stages` 元素里的子字段名写错要拦。
#[tokio::test]
async fn location_profile_rejects_unknown_arc_stage_subfield() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_location")
        .execute(json!({ "world_id": world_id.to_string(), "name": "破庙" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("location id").to_string();

    let err = tool(&registry, "update_location_profile")
        .execute(json!({
            "id": id,
            "arc_stages": [{ "stage": "前期", "roles": "主角据点" }]
        }))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("roles"), "{}", msg);
    assert!(msg.contains("role"), "{}", msg);

    Ok(())
}

/// 势力档案：子字段名写错要拦。
#[tokio::test]
async fn faction_profile_rejects_unknown_arc_stage_subfield() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_faction")
        .execute(json!({ "world_id": world_id.to_string(), "name": "炼器神宗" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("faction id").to_string();

    let err = tool(&registry, "update_faction_profile")
        .execute(json!({
            "id": id,
            "arc_stages": [{ "stage": "中期", "goal_s": "独占灵脉" }]
        }))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("goal_s"), "{}", msg);
    assert!(msg.contains("goal"), "{}", msg);

    Ok(())
}

/// 批量地点工具：字段契约与单条一致（含 aliases / secrets），写错字段名逐条报错。
///
/// 回归自实际缺口：批量补档案时 `aliases` / `secrets` 根本不在契约里，
/// 想批量补别名就只能退回去逐条调单条工具。
#[tokio::test]
async fn bulk_location_profiles_supports_aliases_and_rejects_unknown_field() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;

    let a = tool(&registry, "create_location")
        .execute(json!({ "world_id": world_id.to_string(), "name": "天柱山脉" }))
        .await?;
    let a_id = a["data"]["id"].as_str().expect("location id").to_string();
    let b = tool(&registry, "create_location")
        .execute(json!({ "world_id": world_id.to_string(), "name": "无尽之海" }))
        .await?;
    let b_id = b["data"]["id"].as_str().expect("location id").to_string();

    let out = tool(&registry, "bulk_update_location_profiles")
        .execute(json!({
            "profiles": [
                {
                    "id": a_id,
                    "aliases": ["盘脊"],
                    "secrets": "那尊无名神像原本属于谁"
                },
                { "id": b_id, "climte": "写错的字段名" }
            ]
        }))
        .await?;

    assert_eq!(out["ok"], json!(false), "有失败项时整体不能报成功：{}", out);
    let failures = out["failures"].as_array().expect("failures array");
    assert_eq!(failures.len(), 1, "只有写错字段的那条应失败：{}", out);
    assert!(
        failures[0]["error"]
            .as_str()
            .is_some_and(|e| e.contains("climte") && e.contains("climate")),
        "错误里应点名写错字段并给出可写字段：{}",
        failures[0]
    );

    // 成功那条真的写进去了
    let after = tool(&registry, "get_location_profile")
        .execute(json!({ "id": a_id }))
        .await?;
    assert_eq!(after["data"]["aliases"], json!(["盘脊"]));
    assert_eq!(after["data"]["secrets"], json!("那尊无名神像原本属于谁"));

    Ok(())
}

/// `append: true`：长文本分次写，第二次不必把第一段重发一遍。
///
/// 这正对长文本被单次输出上限截断的卡点（实测 `EOF while parsing`，
/// json 只写到一半）；有了追加语义，模型可以一小段一小段地写。
#[tokio::test]
async fn revise_entity_appends_text_instead_of_overwriting() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world_id.to_string(), "name": "柳如烟" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("character id").to_string();

    tool(&registry, "revise_entity")
        .execute(json!({ "id": id, "description": "第一段：出身戏班。" }))
        .await?;
    tool(&registry, "revise_entity")
        .execute(json!({
            "id": id,
            "description": "第二段：被卷入宫闱。",
            "append": true
        }))
        .await?;

    let after = tool(&registry, "get_entity")
        .execute(json!({ "id": id }))
        .await?;
    assert_eq!(
        after["data"]["description"],
        json!("第一段：出身戏班。第二段：被卷入宫闱。"),
        "追加写不能把第一段覆盖掉"
    );

    Ok(())
}

/// 人物档案的长文本字段同样支持 append（`background_origin` 这类字段常写很长）。
#[tokio::test]
async fn character_profile_appends_text_field() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world_id.to_string(), "name": "吴敬之" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("character id").to_string();

    tool(&registry, "update_character_profile")
        .execute(json!({ "id": id, "background_origin": "寒门出身。" }))
        .await?;
    tool(&registry, "update_character_profile")
        .execute(json!({
            "id": id,
            "background_origin": "科举入仕后掌刑部。",
            "append": true
        }))
        .await?;

    let after = tool(&registry, "get_character_profile")
        .execute(json!({ "id": id }))
        .await?;
    assert_eq!(
        after["data"]["background_origin"],
        json!("寒门出身。科举入仕后掌刑部。"),
        "追加写不能覆盖旧内容"
    );

    Ok(())
}

/// 章节正文（叙事节点的 content）支持 append——它是系统里最长的文本。
#[tokio::test]
async fn revise_node_appends_content() -> Result<()> {
    let (registry, _world_id, project_id) = setup().await?;
    let node = tool(&registry, "create_node")
        .execute(json!({
            "project_id": project_id.to_string(),
            "node_type": "Chapter",
            "title": "第1章",
            "content": "开篇：雨夜入城。"
        }))
        .await?;
    let id = node["data"]["id"].as_str().expect("node id").to_string();

    tool(&registry, "revise_node")
        .execute(json!({
            "id": id,
            "content": "第二段：巷中遇刺。",
            "append": true
        }))
        .await?;

    let after = tool(&registry, "get_node")
        .execute(json!({ "id": id }))
        .await?;
    assert_eq!(
        after["data"]["content"],
        json!("开篇：雨夜入城。第二段：巷中遇刺。"),
        "章节正文追加写不能覆盖开头"
    );

    Ok(())
}

/// `arc_stages` 的 `role` 有等价写法 `stage_role`。
///
/// 起因：同一个字段在人物 / 势力 / 地点三处共用（这个设计是对的、不该拆成三套），
/// 但「role」这个词写在势力 / 地点上读起来别扭。做法是**给一个更中性的等价键**，
/// 而不是原地改列名——原地改会让已有数据、前端和模型已经记住的写法一起失效。
#[tokio::test]
async fn arc_stages_accepts_stage_role_alias() -> Result<()> {
    let (registry, world_id, _project_id) = setup().await?;
    let created = tool(&registry, "create_faction")
        .execute(json!({ "world_id": world_id.to_string(), "name": "炼器神宗" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("faction id").to_string();

    // 用更中性的 stage_role 写势力阶段
    tool(&registry, "update_faction_profile")
        .execute(json!({
            "id": id,
            "arc_stages": [{ "stage": "中期", "stage_role": "主角靠山", "screen_weight": "Heavy" }]
        }))
        .await?;

    let after = tool(&registry, "get_faction_profile")
        .execute(json!({ "id": id }))
        .await?;
    assert_eq!(
        after["data"]["arc_stages"][0]["role"],
        json!("主角靠山"),
        "stage_role 应归一化到 role 落库：{}",
        after
    );

    // 老的 role 写法仍然可用
    tool(&registry, "update_faction_profile")
        .execute(json!({
            "id": id,
            "arc_stages": [{ "stage": "前期", "role": "传闻中的小教团" }],
            "arc_stages_mode": "merge"
        }))
        .await?;

    // 两个都传且不一致：报错，而不是静默挑一个
    let err = tool(&registry, "update_faction_profile")
        .execute(json!({
            "id": id,
            "arc_stages": [{ "stage": "后期", "role": "主要对手", "stage_role": "背景板" }]
        }))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("只传一个"),
        "role 与 stage_role 冲突时应报错：{}",
        err
    );

    Ok(())
}
