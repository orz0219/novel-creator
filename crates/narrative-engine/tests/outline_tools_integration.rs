//! 细纲（narrative_node）工具集成测试
//!
//! 覆盖本次改造的核心能力：**能建树、也能动树**——
//! 换父移动（id 不变，伏笔锚点不脱钩）、兄弟重排（序号连续）、
//! 挂故事线与阶段、挂在场角色 / 地点 / 道具、批量建树（key / parent_key）、
//! 列表过滤与 child_count、事件挂到节点。
//!
//! 需要 PostgreSQL。`testkit::test_pool()` 会自动派生并使用独立的 `_test` 库
//! （开发库不会被写脏），并保证迁移已跑过。

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

/// 建项目 + 注册全部领域工具（每个测试用自己的项目，互不干扰）。
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
        .create_project(&format!("outline-{}", Uuid::new_v4()), None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("项目 id")).unwrap();
    Ok((pool, registry, project_id))
}

fn node_id(v: &Value) -> Uuid {
    Uuid::parse_str(v["data"]["id"].as_str().expect("节点 id")).unwrap()
}

async fn create(
    registry: &Arc<ToolRegistry>,
    project_id: Uuid,
    args: Value,
) -> Result<Value> {
    let mut payload = args;
    payload["project_id"] = json!(project_id.to_string());
    tool(registry, "create_node").execute(payload).await
}

/// 某个父节点下的 (title, sort_order)，按 sort_order 排序。
async fn children_of(
    registry: &Arc<ToolRegistry>,
    project_id: Uuid,
    parent_id: Uuid,
) -> Result<Vec<(String, i64)>> {
    let listed = tool(registry, "list_nodes")
        .execute(json!({
            "project_id": project_id.to_string(),
            "parent_id": parent_id.to_string(),
            "limit": 50,
        }))
        .await?;
    let mut rows: Vec<(String, i64)> = listed["data"]
        .as_array()
        .expect("data 数组")
        .iter()
        .map(|n| {
            (
                n["title"].as_str().unwrap_or_default().to_string(),
                n["sort_order"].as_i64().unwrap_or_default(),
            )
        })
        .collect();
    rows.sort_by_key(|(_, ord)| *ord);
    Ok(rows)
}

// ------------------------------------------------------------
// 节点类型：中文别名可用，拼错就报错，自定义要显式前缀
// ------------------------------------------------------------

#[tokio::test]
async fn create_node_type_accepts_aliases_and_rejects_typos() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;

    // 中文别名
    let vol = create(&registry, project_id, json!({ "node_type": "卷", "title": "卷一" })).await?;
    assert_eq!(vol["data"]["node_type"], json!("Volume"));

    // 显式自定义
    let custom = create(
        &registry,
        project_id,
        json!({ "node_type": "custom:过场", "title": "过场一" }),
    )
    .await?;
    assert_eq!(custom["data"]["node_type"], json!("custom:过场"));

    // 拼错：直接报错，不静默变成别的类型
    let err = create(
        &registry,
        project_id,
        json!({ "node_type": "foobar_test", "title": "错的类型" }),
    )
    .await
    .expect_err("未知 node_type 必须报错");
    let msg = err.to_string();
    assert!(msg.contains("node_type 不认识"), "{}", msg);
    assert!(msg.contains("custom:"), "错误里要提示自定义写法：{}", msg);
    Ok(())
}

// ------------------------------------------------------------
// 换父移动：id 不变（伏笔锚点不脱钩）
// ------------------------------------------------------------

#[tokio::test]
async fn revise_node_moves_subtree_keeping_id() -> Result<()> {
    let (pool, registry, project_id) = setup().await?;
    let vol_a = create(&registry, project_id, json!({ "node_type": "Volume", "title": "卷一" })).await?;
    let vol_b = create(&registry, project_id, json!({ "node_type": "Volume", "title": "卷二" })).await?;
    let (a_id, b_id) = (node_id(&vol_a), node_id(&vol_b));

    let chapter = create(
        &registry,
        project_id,
        json!({ "node_type": "Chapter", "title": "第 3 章", "parent_id": a_id.to_string() }),
    )
    .await?;
    let chapter_id = node_id(&chapter);

    // 移动：从卷一挪到卷二
    let moved = tool(&registry, "revise_node")
        .execute(json!({
            "id": chapter_id.to_string(),
            "parent_id": b_id.to_string(),
        }))
        .await?;
    assert_eq!(moved["data"]["parent_id"], json!(b_id.to_string()));
    assert_eq!(
        moved["data"]["id"],
        json!(chapter_id.to_string()),
        "移动不应改变节点 id（伏笔的 planted_node_id / payoff_node_id 才不会脱钩）"
    );

    // 库里确认（不是只改了回显）
    let (parent,): (Option<Uuid>,) =
        sqlx::query_as("SELECT parent_id FROM narrative_node WHERE id = $1")
            .bind(chapter_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(parent, Some(b_id));

    // 移到顶层
    let rooted = tool(&registry, "revise_node")
        .execute(json!({ "id": chapter_id.to_string(), "move_to_root": true }))
        .await?;
    assert!(rooted["data"]["parent_id"].is_null());
    Ok(())
}

#[tokio::test]
async fn revise_node_rejects_moving_under_own_descendant() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;
    let vol = create(&registry, project_id, json!({ "node_type": "Volume", "title": "卷一" })).await?;
    let vol_id = node_id(&vol);
    let chapter = create(
        &registry,
        project_id,
        json!({ "node_type": "Chapter", "title": "第 1 章", "parent_id": vol_id.to_string() }),
    )
    .await?;
    let chapter_id = node_id(&chapter);
    let scene = create(
        &registry,
        project_id,
        json!({ "node_type": "Scene", "title": "开场", "parent_id": chapter_id.to_string() }),
    )
    .await?;
    let scene_id = node_id(&scene);

    // 把卷挪到自己的孙节点下 → 必须报错（否则树成环）
    let err = tool(&registry, "revise_node")
        .execute(json!({ "id": vol_id.to_string(), "parent_id": scene_id.to_string() }))
        .await
        .expect_err("移动到自己子孙下必须报错");
    assert!(err.to_string().contains("成环"), "{}", err);
    Ok(())
}

// ------------------------------------------------------------
// 兄弟重排：序号保持 1..n 连续
// ------------------------------------------------------------

#[tokio::test]
async fn revise_node_reorders_siblings_compactly() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;
    let vol = create(&registry, project_id, json!({ "node_type": "Volume", "title": "卷一" })).await?;
    let vol_id = node_id(&vol);

    let mut ids = Vec::new();
    for i in 1..=3 {
        let n = create(
            &registry,
            project_id,
            json!({
                "node_type": "Chapter",
                "title": format!("第 {} 章", i),
                "parent_id": vol_id.to_string(),
            }),
        )
        .await?;
        ids.push(node_id(&n));
    }
    assert_eq!(
        children_of(&registry, project_id, vol_id).await?,
        vec![
            ("第 1 章".to_string(), 1),
            ("第 2 章".to_string(), 2),
            ("第 3 章".to_string(), 3),
        ]
    );

    // 把第 3 章移到第 1 位：其余兄弟自动顺移，序号仍连续
    tool(&registry, "revise_node")
        .execute(json!({ "id": ids[2].to_string(), "sort_order": 1 }))
        .await?;
    assert_eq!(
        children_of(&registry, project_id, vol_id).await?,
        vec![
            ("第 3 章".to_string(), 1),
            ("第 1 章".to_string(), 2),
            ("第 2 章".to_string(), 3),
        ]
    );
    Ok(())
}

#[tokio::test]
async fn create_node_inserts_at_explicit_position() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;
    let vol = create(&registry, project_id, json!({ "node_type": "Volume", "title": "卷一" })).await?;
    let vol_id = node_id(&vol);
    for i in [1, 2] {
        create(
            &registry,
            project_id,
            json!({ "node_type": "Chapter", "title": format!("第 {} 章", i), "parent_id": vol_id.to_string() }),
        )
        .await?;
    }
    // 插到第 1 位：原第 1、2 章顺移到 2、3
    create(
        &registry,
        project_id,
        json!({
            "node_type": "Chapter",
            "title": "楔子",
            "parent_id": vol_id.to_string(),
            "sort_order": 1,
        }),
    )
    .await?;
    assert_eq!(
        children_of(&registry, project_id, vol_id).await?,
        vec![
            ("楔子".to_string(), 1),
            ("第 1 章".to_string(), 2),
            ("第 2 章".to_string(), 3),
        ]
    );
    Ok(())
}

// ------------------------------------------------------------
// 挂载：故事线 / 阶段 / 在场实体 / 元数据
// ------------------------------------------------------------

#[tokio::test]
async fn create_node_mounts_storyline_entities_and_metadata() -> Result<()> {
    let (pool, registry, project_id) = setup().await?;

    let world = tool(&registry, "get_main_world")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    let world_id = world["data"]["id"].as_str().expect("world id").to_string();

    let mut entity_ids = Vec::new();
    for (kind, name) in [("Character", "林樊"), ("Location", "黑石城"), ("Item", "青铜灯")] {
        let e = tool(&registry, "create_entity")
            .execute(json!({ "world_id": world_id, "entity_type": kind, "name": name }))
            .await?;
        entity_ids.push(Uuid::parse_str(e["data"]["id"].as_str().expect("entity id")).unwrap());
    }
    let (lin, city, lamp) = (entity_ids[0], entity_ids[1], entity_ids[2]);

    let sl = tool(&registry, "create_storyline")
        .execute(json!({ "project_id": project_id.to_string(), "name": "主线", "importance": "Main" }))
        .await?;
    let sl_id = Uuid::parse_str(sl["data"]["id"].as_str().expect("storyline id")).unwrap();

    // 阶段表为空时不拦（先建树、后补阶段表的顺序是正常的）
    let scene = create(
        &registry,
        project_id,
        json!({
            "node_type": "Scene",
            "title": "黑石城初遇",
            "storyline_id": sl_id.to_string(),
            "arc_stage": "起",
            "participant_entity_ids": [lin.to_string()],
            "location_id": city.to_string(),
            "item_ids": [lamp.to_string()],
            "estimated_chapters": 3,
            "estimated_words": 12000,
            "story_time": "第三天黄昏",
        }),
    )
    .await?;
    let d = &scene["data"];
    assert_eq!(d["storyline_id"], json!(sl_id.to_string()));
    assert_eq!(d["storyline_name"], json!("主线"));
    assert_eq!(d["arc_stage"], json!("起"));
    assert_eq!(d["participant_entity_ids"], json!([lin.to_string()]));
    assert_eq!(d["location_id"], json!(city.to_string()));
    assert_eq!(d["item_ids"], json!([lamp.to_string()]));
    assert_eq!(d["estimated_chapters"], json!(3));
    assert_eq!(d["estimated_words"], json!(12000));
    assert_eq!(d["story_time"], json!("第三天黄昏"));

    // 阶段表有了之后，写错的阶段名要被拦下
    sqlx::query("UPDATE storyline SET arc_stages = $1 WHERE id = $2")
        .bind(json!([{ "stage": "起" }, { "stage": "承" }]))
        .bind(sl_id)
        .execute(&pool)
        .await?;
    let ok = create(
        &registry,
        project_id,
        json!({ "node_type": "Chapter", "title": "第 2 章", "storyline_id": sl_id.to_string(), "arc_stage": "承" }),
    )
    .await?;
    assert_eq!(ok["data"]["arc_stage"], json!("承"));

    let err = create(
        &registry,
        project_id,
        json!({ "node_type": "Chapter", "title": "第 3 章", "storyline_id": sl_id.to_string(), "arc_stage": "转" }),
    )
    .await
    .expect_err("阶段名对不上必须报错");
    assert!(err.to_string().contains("上没有这个阶段"), "{}", err);

    // 实体不存在 / 不属于本项目：直接报错，不写悬空引用
    let ghost = Uuid::new_v4();
    let err = create(
        &registry,
        project_id,
        json!({ "node_type": "Scene", "title": "幽灵场景", "participant_entity_ids": [ghost.to_string()] }),
    )
    .await
    .expect_err("不存在的实体必须报错");
    assert!(err.to_string().contains("participant_entity_ids"), "{}", err);
    Ok(())
}

#[tokio::test]
async fn revise_node_clears_and_replaces_mounts() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;
    let sl = tool(&registry, "create_storyline")
        .execute(json!({ "project_id": project_id.to_string(), "name": "暗线", "importance": "Important" }))
        .await?;
    let sl_id = Uuid::parse_str(sl["data"]["id"].as_str().expect("storyline id")).unwrap();

    let chapter = create(
        &registry,
        project_id,
        json!({ "node_type": "Chapter", "title": "第 1 章", "storyline_id": sl_id.to_string(), "arc_stage": "起" }),
    )
    .await?;
    let chapter_id = node_id(&chapter);

    // 解绑：storyline_id 与 arc_stage 一起清空
    let cleared = tool(&registry, "revise_node")
        .execute(json!({ "id": chapter_id.to_string(), "clear_storyline": true }))
        .await?;
    assert!(cleared["data"]["storyline_id"].is_null());
    assert!(cleared["data"]["arc_stage"].is_null());

    // 只给 arc_stage 不给 storyline_id：没有可挂的线，报错
    let err = tool(&registry, "revise_node")
        .execute(json!({ "id": chapter_id.to_string(), "arc_stage": "承" }))
        .await
        .expect_err("没有 storyline_id 时写 arc_stage 必须报错");
    assert!(err.to_string().contains("storyline_id"), "{}", err);
    Ok(())
}

// ------------------------------------------------------------
// 批量建树：key / parent_key
// ------------------------------------------------------------

#[tokio::test]
async fn bulk_create_nodes_builds_subtree_with_keys() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;

    let created = tool(&registry, "bulk_create_nodes")
        .execute(json!({
            "project_id": project_id.to_string(),
            "nodes": [
                { "key": "vol1", "node_type": "Volume", "title": "卷一" },
                { "key": "arc1", "node_type": "Arc", "title": "破庙弧", "parent_key": "vol1" },
                { "key": "ch1", "node_type": "Chapter", "title": "第 1 章", "parent_key": "arc1" },
                { "key": "ch2", "node_type": "Chapter", "title": "第 2 章", "parent_key": "arc1" }
            ]
        }))
        .await?;

    assert_eq!(created["ok"], json!(true), "{}", created);
    assert_eq!(created["created_count"], json!(4));

    let by_key: std::collections::BTreeMap<String, Uuid> = created["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["key"].as_str().unwrap_or_default().to_string(),
                Uuid::parse_str(c["id"].as_str().unwrap()).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        created["created"][2]["parent_id"],
        json!(by_key["arc1"].to_string()),
        "parent_key 应解析成同批次里更早那个节点的 id"
    );

    // 树形结构落到库里
    let (p1,): (Option<Uuid>,) = sqlx::query_as("SELECT parent_id FROM narrative_node WHERE id = $1")
        .bind(by_key["arc1"])
        .fetch_one(&_pool)
        .await?;
    assert_eq!(p1, Some(by_key["vol1"]));
    assert_eq!(
        children_of(&registry, project_id, by_key["arc1"]).await?,
        vec![("第 1 章".to_string(), 1), ("第 2 章".to_string(), 2)]
    );

    // parent_key 指向不存在的 key：该项失败并说明原因，其余项照常建成
    let partial = tool(&registry, "bulk_create_nodes")
        .execute(json!({
            "project_id": project_id.to_string(),
            "nodes": [
                { "key": "vol2", "node_type": "Volume", "title": "卷二" },
                { "key": "bad", "node_type": "Arc", "title": "挂空弧", "parent_key": "nope" }
            ]
        }))
        .await?;
    assert_eq!(partial["ok"], json!(false));
    assert_eq!(partial["created_count"], json!(1));
    assert!(
        partial["failures"][0]["error"]
            .as_str()
            .unwrap()
            .contains("找不到"),
        "{}",
        partial["failures"]
    );
    Ok(())
}

// ------------------------------------------------------------
// 列表：下钻 / 只看顶层 / 按类型筛 + child_count
// ------------------------------------------------------------

#[tokio::test]
async fn list_nodes_filters_and_reports_child_count() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;
    let vol = create(&registry, project_id, json!({ "node_type": "Volume", "title": "卷一" })).await?;
    let vol_id = node_id(&vol);
    let arc = create(
        &registry,
        project_id,
        json!({ "node_type": "Arc", "title": "破庙弧", "parent_id": vol_id.to_string() }),
    )
    .await?;
    let arc_id = node_id(&arc);
    for i in 1..=2 {
        create(
            &registry,
            project_id,
            json!({ "node_type": "Chapter", "title": format!("第 {} 章", i), "parent_id": arc_id.to_string() }),
        )
        .await?;
    }

    // 只看顶层：只有卷
    let roots = tool(&registry, "list_nodes")
        .execute(json!({ "project_id": project_id.to_string(), "roots_only": true }))
        .await?;
    assert_eq!(roots["total"], json!(1));
    assert_eq!(roots["data"][0]["title"], json!("卷一"));
    assert_eq!(
        roots["data"][0]["child_count"],
        json!(1),
        "顶层卷下面有 1 个弧"
    );
    // 过滤条件要回带，避免把 total 误读成项目全量
    assert_eq!(roots["filter"]["roots_only"], json!(true));

    // 下钻到卷：只有弧，且弧下面有 2 章
    let under_vol = tool(&registry, "list_nodes")
        .execute(json!({ "project_id": project_id.to_string(), "parent_id": vol_id.to_string() }))
        .await?;
    assert_eq!(under_vol["total"], json!(1));
    assert_eq!(under_vol["data"][0]["child_count"], json!(2));

    // 按类型筛
    let chapters = tool(&registry, "list_nodes")
        .execute(json!({ "project_id": project_id.to_string(), "node_type": "章" }))
        .await?;
    assert_eq!(chapters["total"], json!(2));

    // 拼错的类型要报错（不是静默返回空列表）
    let err = tool(&registry, "list_nodes")
        .execute(json!({ "project_id": project_id.to_string(), "node_type": "chaptr" }))
        .await
        .expect_err("未知 node_type 过滤应报错");
    assert!(err.to_string().contains("node_type 不认识"), "{}", err);

    // 互斥过滤条件
    let err = tool(&registry, "list_nodes")
        .execute(json!({
            "project_id": project_id.to_string(),
            "parent_id": vol_id.to_string(),
            "roots_only": true
        }))
        .await
        .expect_err("parent_id 与 roots_only 互斥");
    assert!(err.to_string().contains("不能同时给"), "{}", err);
    Ok(())
}

// ------------------------------------------------------------
// 事件 ↔ 节点
// ------------------------------------------------------------

#[tokio::test]
async fn create_event_links_node_and_rejects_unknown_node() -> Result<()> {
    let (_pool, registry, project_id) = setup().await?;
    let chapter = create(&registry, project_id, json!({ "node_type": "Chapter", "title": "第 1 章" })).await?;
    let chapter_id = node_id(&chapter);

    let event = tool(&registry, "create_event")
        .execute(json!({
            "project_id": project_id.to_string(),
            "name": "破庙立神",
            "description": "主角在破庙里立下神像",
            "narrative_node_id": chapter_id.to_string(),
        }))
        .await?;
    assert_eq!(
        event["data"]["narrative_node_id"],
        json!(chapter_id.to_string())
    );
    assert_eq!(event["data"]["narrative_node_title"], json!("第 1 章"));

    // list_events 也带出节点信息
    let listed = tool(&registry, "list_events")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    let hit = listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == json!("破庙立神"))
        .expect("应能列出刚建的事件");
    assert_eq!(hit["narrative_node_id"], json!(chapter_id.to_string()));

    // 挂到不存在的节点：直接报错
    let err = tool(&registry, "create_event")
        .execute(json!({
            "project_id": project_id.to_string(),
            "name": "幽灵事件",
            "description": "挂在不存在节点上",
            "narrative_node_id": Uuid::new_v4().to_string(),
        }))
        .await
        .expect_err("挂到不存在的节点必须报错");
    assert!(err.to_string().contains("叙事节点"), "{}", err);

    // 解绑
    let event_id = event["data"]["id"].as_str().unwrap().to_string();
    let cleared = tool(&registry, "revise_event")
        .execute(json!({ "id": event_id, "clear_narrative_node": true }))
        .await?;
    assert!(cleared["data"]["narrative_node_id"].is_null());
    Ok(())
}
