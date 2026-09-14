//! 引导进度（`get_project_status`）集成测试。
//!
//! 锁定一个真实发生过的缺陷：前端自己数数量判断「血肉步」完成度，清单里漏掉了
//! `storylines.branches`（副线）——于是副线产物明明齐了，步骤条却**永远不打勾**，
//! 而且没有任何报错，只能靠人去猜。
//!
//! 现在完成度只由后端判定（`GET /projects/{id}/guide/status` 与 AI 看到的
//! `get_project_status` 是同一个工具，`confirm_step` 也是同一套校验），
//! 这个测试就锁住后端判定本身：副线建好并挂到主线之后，该步必须就绪。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbProjectRepositoryPort, DbWorldRepositoryPort};
use narrative_engine::agent_tools::register_all_domain_tools;
use serde_json::{json, Value};
use uuid::Uuid;

fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

/// 从 `get_project_status` 的返回里取某一步的报告
fn step<'a>(status: &'a Value, key: &str) -> &'a Value {
    status["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("steps 应为数组，实际：{}", status))
        .iter()
        .find(|s| s["key"] == key)
        .unwrap_or_else(|| panic!("steps 里应当有 '{}'，实际：{}", key, status))
}

async fn guide_status(registry: &Arc<ToolRegistry>, pid: Uuid) -> Result<Value> {
    // 走 execute_in_project：与真实调用（前端接口 / agent）同一条路径，project_id 由框架注入
    registry
        .execute_in_project(pid, "get_project_status", json!({}))
        .await
}

#[tokio::test]
async fn branches_step_becomes_ready_once_sub_storyline_is_attached() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    // 项目名带 uuid：测试库可能残留上一次运行的数据，固定名字会互相干扰
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project(&format!("guide-status-{}", Uuid::new_v4()), None, None)
        .await?;
    let pid = Uuid::parse_str(proj["id"].as_str().expect("proj id"))?;

    // 1) 副线是血肉步，且产物没建之前必须不就绪
    let before = guide_status(&registry, pid).await?;
    let branches = step(&before, "storylines.branches");
    assert_eq!(branches["group"], "flesh", "副线应当是血肉步");
    assert_eq!(branches["title"], "副线");
    assert_eq!(branches["ready"], false, "还没有副线时不该就绪");

    // 2) 建主线
    let main = tool(&registry, "create_storyline")
        .execute(json!({
            "project_id": pid.to_string(),
            "name": "主线",
            "importance": "Main"
        }))
        .await?;
    let main_id = main["data"]["id"].as_str().expect("main id").to_string();

    // 3) 建一条副线：create_storyline 传 parent_id 会同时写 storyline_relation
    let sub = tool(&registry, "create_storyline")
        .execute(json!({
            "project_id": pid.to_string(),
            "name": "副线·暗线",
            "importance": "Important",
            "tone": "dark",
            "visibility": "hidden",
            "parent_id": main_id
        }))
        .await?;
    assert!(sub["data"]["id"].is_string(), "副线应当创建成功：{}", sub);

    // 挂载关系确实落库——否则下一步的 ready 断言失败时会看不出真实原因
    let rels = tool(&registry, "list_storyline_relations")
        .execute(json!({ "project_id": pid.to_string() }))
        .await?;
    assert_eq!(
        rels["data"].as_array().map(|a| a.len()),
        Some(1),
        "应当有 1 条挂载关系：{}",
        rels
    );

    // 4) 副线齐了 → 必须就绪（用户看不到的那个对勾，判定就在这一步）
    let after = guide_status(&registry, pid).await?;
    let branches = step(&after, "storylines.branches");
    assert_eq!(
        branches["ready"], true,
        "副线建好并挂到主线后应当就绪，实际：{}",
        branches
    );
    assert!(
        branches["missing"].as_array().map(|a| a.is_empty()) == Some(true),
        "就绪时 missing 应为空，实际：{}",
        branches
    );

    Ok(())
}

#[tokio::test]
async fn guide_status_lists_every_step_with_group_and_ready() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project(&format!("guide-status-{}", Uuid::new_v4()), None, None)
        .await?;
    let pid = Uuid::parse_str(proj["id"].as_str().expect("proj id"))?;

    let status = guide_status(&registry, pid).await?;
    let steps = status["steps"].as_array().expect("steps 应为数组");

    // 步骤条渲染所需的字段一个都不能少：前端不再自备清单，缺一个就少一个节点
    for s in steps {
        assert!(s["key"].is_string(), "step 缺 key：{}", s);
        assert!(s["title"].is_string(), "step 缺 title：{}", s);
        assert!(
            s["group"] == "skeleton" || s["group"] == "flesh",
            "step group 只能是 skeleton / flesh：{}",
            s
        );
        assert!(s["ready"].is_boolean(), "step 缺 ready：{}", s);
        assert!(s["missing"].is_array(), "step 缺 missing：{}", s);
    }

    // 血肉步（可跳过、也不能按位置推断完成）一应俱全
    let flesh: Vec<&str> = steps
        .iter()
        .filter(|s| s["group"] == "flesh")
        .map(|s| s["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        flesh,
        vec![
            "world.map",
            "world.factions",
            "world.items",
            "characters.supporting",
            "storylines.branches"
        ],
        "血肉步清单（含副线）应与后端 STEPS 一致"
    );

    // 当前阶段必须是其中之一，且能取到标题（前端靠它显示"当前阶段"）
    let current = status["current_step"].as_str().expect("current_step");
    assert!(
        steps.iter().any(|s| s["key"] == current),
        "current_step '{}' 不在 steps 里",
        current
    );
    assert!(status["current_title"].is_string());

    Ok(())
}
