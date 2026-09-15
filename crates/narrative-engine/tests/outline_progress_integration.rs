//! `outline_progress` 工具的集成测试
//!
//! 这个工具是「AI 自查细纲进度」的唯一低成本手段：`list_nodes` 的投影里没有
//! description，翻不出空壳章。因此它的判定必须与引导流程的校验口径一致：
//! 否则会出现「工具说可以开始写正文了，点推进却报场景缺字段」这种无人能查的分叉。
//!
//! 需要 PostgreSQL。`testkit::test_pool()` 会自动派生并使用独立的 `_test` 库。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbProjectRepositoryPort, DbWorldRepositoryPort};
use narrative_engine::agent_tools::register_all_domain_tools;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

async fn setup() -> Result<(PgPool, Arc<ToolRegistry>, Uuid)> {
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
        .create_project(&format!("outline-progress-{}", Uuid::new_v4()), None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("项目 id")).unwrap();
    Ok((pool, registry, project_id))
}

fn node_id(v: &Value) -> Uuid {
    Uuid::parse_str(v["data"]["id"].as_str().expect("节点 id")).unwrap()
}

async fn create_node(
    registry: &Arc<ToolRegistry>,
    project_id: Uuid,
    args: Value,
) -> Result<Value> {
    let mut payload = args;
    payload["project_id"] = json!(project_id.to_string());
    tool(registry, "create_node").execute(payload).await
}

async fn progress(registry: &Arc<ToolRegistry>, project_id: Uuid) -> Result<Value> {
    tool(registry, "outline_progress")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await
}

fn suggestion_action(report: &Value) -> &str {
    report["next_suggestion"]["action"]
        .as_str()
        .expect("next_suggestion.action 必须是字符串")
}

/// 建一个字段齐全的场景（objective / conflict / pov_character_id / location_id 都非空）。
async fn create_ready_scene(
    registry: &Arc<ToolRegistry>,
    project_id: Uuid,
    chapter_id: Uuid,
    title: &str,
) -> Result<Value> {
    create_node(
        registry,
        project_id,
        json!({
            "node_type": "Scene",
            "parent_id": chapter_id.to_string(),
            "title": title,
            "attributes": {
                "objective": "让主角拿到第一笔钱",
                "conflict": "钱来路无法解释",
                "pov_character_id": Uuid::new_v4().to_string(),
                "location_id": Uuid::new_v4().to_string(),
            }
        }),
    )
    .await
}

/// 主线：空项目 → 建卷 → 建弧 → 建章 → 补一句话事件 → 展开场景 → 就绪。
/// 每一步都断言工具给出的"下一步建议"确实指向该做的那件事。
#[tokio::test]
async fn outline_progress_walks_from_empty_to_writing_ready() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;

    // 1) 空项目：先建卷
    let report = progress(&registry, project_id).await?;
    assert_eq!(report["counts"]["volumes"], 0);
    assert_eq!(suggestion_action(&report), "先建卷");

    // 2) 只有卷：弧下面没章 → 先给弧补章
    let volume = create_node(
        &registry,
        project_id,
        json!({ "node_type": "Volume", "title": "卷一 · 破庙立神" }),
    )
    .await?;
    let volume_id = node_id(&volume);
    let arc = create_node(
        &registry,
        project_id,
        json!({
            "node_type": "Arc",
            "parent_id": volume_id.to_string(),
            "title": "弧一 · 落脚破庙"
        }),
    )
    .await?;
    let arc_id = node_id(&arc);

    let report = progress(&registry, project_id).await?;
    assert_eq!(report["arcs_without_chapter_total"], 1);
    assert_eq!(suggestion_action(&report), "给这些弧补章");

    // 3) 章建好了但没写一句话事件 → 空壳章优先
    let chapter = create_node(
        &registry,
        project_id,
        json!({
            "node_type": "Chapter",
            "parent_id": arc_id.to_string(),
            "title": "第 1 章 · 无人的裂缝"
        }),
    )
    .await?;
    let chapter_id = node_id(&chapter);

    let report = progress(&registry, project_id).await?;
    assert_eq!(report["counts"]["chapters"], 1);
    assert_eq!(report["hollow_chapters_total"], 1);
    assert_eq!(suggestion_action(&report), "给这些章补一句话事件");
    assert_eq!(
        report["hollow_chapters"][0]["title"], "第 1 章 · 无人的裂缝",
        "空壳章要指名道姓，AI 才知道补哪一章"
    );
    assert_eq!(
        report["hollow_chapters"][0]["parent_title"], "弧一 · 落脚破庙",
        "要带父弧标题，否则同名章分不清"
    );

    // 4) 补上一句话事件 → 转向展开场景
    tool(&registry, "revise_node")
        .execute(json!({
            "project_id": project_id.to_string(),
            "id": chapter_id.to_string(),
            "description": "深夜加班回家，王久财在消防通道的拐角消失。"
        }))
        .await?;

    let report = progress(&registry, project_id).await?;
    assert_eq!(report["hollow_chapters_total"], 0);
    assert_eq!(report["chapters_without_scene_total"], 1);
    assert_eq!(suggestion_action(&report), "把接下来要写的几章展开成场景");
    // 建议里要给出具体是哪几章，并且带上场景必填字段清单
    assert_eq!(report["next_suggestion"]["chapters"][0]["title"], "第 1 章 · 无人的裂缝");
    assert_eq!(
        report["next_suggestion"]["scene_required_fields"],
        json!(["objective", "conflict", "pov_character_id", "location_id"])
    );

    // 5) 场景建了但属性不全 → 仍然算"没有就绪的场景"（与 beats.scenes 的校验同口径）
    create_node(
        &registry,
        project_id,
        json!({
            "node_type": "Scene",
            "parent_id": chapter_id.to_string(),
            "title": "场景 1（只填了标题）"
        }),
    )
    .await?;
    let report = progress(&registry, project_id).await?;
    assert_eq!(report["counts"]["scenes"], 1);
    assert_eq!(report["counts"]["scenes_missing_required_fields"], 1);
    assert_eq!(suggestion_action(&report), "把接下来要写的几章展开成场景");
    let reason = report["next_suggestion"]["reason"]
        .as_str()
        .expect("reason 必须是字符串");
    assert!(
        reason.contains("缺少必填属性"),
        "要说清是字段不全而不是没有场景：{}",
        reason
    );

    // 6) 补齐到 3 个字段齐全的场景 → 轮到正文写作（且场景总数门槛来自流程定义）
    assert_eq!(report["scene_min_required"], 3);
    create_ready_scene(&registry, project_id, chapter_id, "场景 2").await?;
    create_ready_scene(&registry, project_id, chapter_id, "场景 3").await?;
    create_ready_scene(&registry, project_id, chapter_id, "场景 4").await?;

    let report = progress(&registry, project_id).await?;
    assert_eq!(report["counts"]["scenes"], 4);
    assert_eq!(report["counts"]["scenes_missing_required_fields"], 1);
    // 还有一个字段不全的场景 → 仍不该说"可以进正文"
    assert_eq!(suggestion_action(&report), "把接下来要写的几章展开成场景");

    // 把那个残缺场景补全
    let stale_scene_id = {
        let listed = tool(&registry, "list_nodes")
            .execute(json!({
                "project_id": project_id.to_string(),
                "parent_id": chapter_id.to_string(),
                "node_type": "Scene"
            }))
            .await?;
        listed["data"]
            .as_array()
            .expect("list_nodes 返回数组")
            .iter()
            .find(|n| n["title"] == "场景 1（只填了标题）")
            .map(|n| Uuid::parse_str(n["id"].as_str().unwrap()).unwrap())
            .expect("应能找回只填标题的场景")
    };
    tool(&registry, "revise_node")
        .execute(json!({
            "project_id": project_id.to_string(),
            "id": stale_scene_id.to_string(),
            "attributes": {
                "objective": "补齐目标",
                "conflict": "补齐冲突",
                "pov_character_id": Uuid::new_v4().to_string(),
                "location_id": Uuid::new_v4().to_string(),
            }
        }))
        .await?;

    let report = progress(&registry, project_id).await?;
    assert_eq!(report["counts"]["scenes_missing_required_fields"], 0);
    assert_eq!(suggestion_action(&report), "可以进入正文写作");

    Ok(())
}

/// 重要故事线没挂到任何节点时，建议必须指向"挂线"，而不是急着展开场景。
#[tokio::test]
async fn outline_progress_points_at_unattached_storylines() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;

    // 建一条 Main 线（没有任何节点挂它）
    tool(&registry, "create_storyline")
        .execute(json!({
            "project_id": project_id.to_string(),
            "name": "信仰成神·两界螺旋",
            "importance": "Main",
            "tone": "light",
            "visibility": "visible"
        }))
        .await?;

    let volume = create_node(
        &registry,
        project_id,
        json!({ "node_type": "Volume", "title": "卷一" }),
    )
    .await?;
    let arc = create_node(
        &registry,
        project_id,
        json!({
            "node_type": "Arc",
            "parent_id": node_id(&volume).to_string(),
            "title": "弧一"
        }),
    )
    .await?;
    let chapter = create_node(
        &registry,
        project_id,
        json!({
            "node_type": "Chapter",
            "parent_id": node_id(&arc).to_string(),
            "title": "第 1 章",
            "description": "一句话事件"
        }),
    )
    .await?;
    assert!(node_id(&chapter).to_string().len() == 36);

    let report = progress(&registry, project_id).await?;
    assert_eq!(report["storylines_without_node_total"], 1);
    assert_eq!(suggestion_action(&report), "把重要故事线挂到节点上");
    assert_eq!(
        report["storylines_without_node"][0]["name"],
        "信仰成神·两界螺旋"
    );

    Ok(())
}
