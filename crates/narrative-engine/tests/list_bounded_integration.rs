//! 列表读取「有界化」回归测试
//!
//! 锁住三件事（改造前它们全都不成立）：
//! 1. `list_*` 默认只返回 [`agent::LIST_DEFAULT_LIMIT`] 条，并给出 `total` / `has_more` / `next_offset`；
//! 2. 列表只给「目录页」字段——**不含正文**（description / config / premise）；
//! 3. 超过硬上限**报错**，不静默夹紧。
//!
//! 背景：实测 `list_projects` 曾一次返回 1172 个项目的全字段 = **425,018 字符**，
//! `list_entities` 返回 33 个实体的全字段 = **36,555 字符**（description 占 51%）。
//! 这些字符每轮请求都要重发，最终把模型拖到思考 116 秒并撞上网关超时。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry, LIST_DEFAULT_LIMIT, LIST_MAX_LIMIT};
use anyhow::Result;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbProjectRepositoryPort, DbWorldRepositoryPort};
use narrative_engine::agent_tools::register_all_domain_tools;
use serde_json::json;
use uuid::Uuid;

fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

fn chars(v: &serde_json::Value) -> usize {
    serde_json::to_string_pretty(v).unwrap().chars().count()
}

#[tokio::test]
async fn list_entities_is_bounded_and_reports_total() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("list-bounded-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("project id")).unwrap();
    let world = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())))
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界");

    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    // 造 25 个实体，其中每个都带长正文 —— 正是改造前把结果撑爆的东西
    let long_text = "这是一段很长的实体正文，用来复现「列表把正文一起返回」的问题。".repeat(20);
    for i in 0..25 {
        tool(&registry, "create_character")
            .execute(json!({
                "world_id": world.id.to_string(),
                "name": format!("测试角色{:02}", i),
                "description": long_text
            }))
            .await?;
    }

    // 1) 默认有界：只回 20 条 + 总量
    let page = tool(&registry, "list_entities")
        .execute(json!({ "world_id": world.id.to_string() }))
        .await?;
    assert_eq!(page["returned"], json!(LIST_DEFAULT_LIMIT));
    assert_eq!(page["total"], json!(25));
    assert_eq!(page["has_more"], json!(true));
    assert_eq!(page["next_offset"], json!(LIST_DEFAULT_LIMIT));
    assert_eq!(page["data"].as_array().unwrap().len(), LIST_DEFAULT_LIMIT);

    // 2) 目录页不含正文：25 条长正文一次就 3 万+ 字符，这是体积的主要来源
    assert!(
        page["data"][0].get("description").is_none(),
        "列表不该带 description：{}",
        page["data"][0]
    );
    assert!(page["data"][0].get("attributes").is_none());
    assert!(page["data"][0].get("created_at").is_none());
    assert!(page["data"][0]["name"].is_string(), "目录页要有名字");
    assert!(
        chars(&page) < 4000,
        "25 个实体的第一页应远小于 4000 字符，实测 {}",
        chars(&page)
    );

    // 3) 翻页拿剩下的
    let second = tool(&registry, "list_entities")
        .execute(json!({ "world_id": world.id.to_string(), "offset": LIST_DEFAULT_LIMIT }))
        .await?;
    assert_eq!(second["returned"], json!(5));
    assert_eq!(second["has_more"], json!(false));
    assert!(second.get("next_offset").is_none(), "读完了就不该再给 next_offset");

    // 4) 超过硬上限：报错，不静默夹紧
    let err = tool(&registry, "list_entities")
        .execute(json!({
            "world_id": world.id.to_string(),
            "limit": LIST_MAX_LIMIT + 1
        }))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("超过单次上限"),
        "超上限应报错：{}",
        err
    );

    Ok(())
}

#[tokio::test]
async fn list_projects_returns_catalog_fields_only() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let long_premise = "前提设定很长，改造前它会被 list_projects 一起返回。".repeat(50);
    for i in 0..3 {
        project_service
            .create_project(&format!("list-bounded-p{}", i), None, None)
            .await?;
        let _ = &long_premise;
    }

    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    let page = tool(&registry, "list_projects")
        .execute(json!({}))
        .await?;
    let items = page["data"].as_array().expect("data 数组");
    assert!(!items.is_empty());
    assert!(items.len() <= LIST_DEFAULT_LIMIT);
    assert!(page["total"].as_u64().unwrap() >= 3, "total 应是库里的真实项目数");
    for item in items {
        assert!(item.get("config").is_none(), "目录页不该带 config 全文");
        assert!(item.get("premise").is_none(), "目录页不该带 premise 全文");
        assert!(item.get("description").is_none());
        assert!(item["id"].is_string());
        assert!(item["name"].is_string());
    }
    assert!(
        chars(&page) < 6000,
        "一页项目目录应远小于 6000 字符，实测 {}",
        chars(&page)
    );

    Ok(())
}

/// 其余列表工具（关系 / 剧情线 / 规则 / 事件 / 事实 / 快照 / 节点 / 伏笔）同样有界。
///
/// 它们原先一律是「全量 + 全字段」，实测 list_relations 一次 10,299 字符、
/// list_storylines 5,317、list_rules 4,042。
#[tokio::test]
async fn other_lists_are_bounded_and_carry_envelope() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("list-bounded-others", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("project id")).unwrap();
    let world = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())))
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界");

    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);
    let pid = project_id.to_string();
    let wid = world.id.to_string();

    // 每个列表工具都必须给出信封，且体积受控（不管里面有多少条）
    let cases = [
        ("list_relations", json!({ "world_id": wid })),
        ("list_storylines", json!({ "project_id": pid })),
        ("list_rules", json!({ "world_id": wid })),
        ("list_events", json!({ "project_id": pid })),
        ("list_facts", json!({ "project_id": pid })),
        ("list_snapshots", json!({ "project_id": pid })),
        ("list_nodes", json!({ "project_id": pid })),
        ("list_foreshadows", json!({ "project_id": pid })),
    ];
    for (name, input) in cases {
        let out = tool(&registry, name).execute(input).await?;
        assert!(out["total"].is_number(), "{} 应给 total：{}", name, out);
        assert!(out["returned"].is_number(), "{} 应给 returned", name);
        assert!(out["has_more"].is_boolean(), "{} 应给 has_more", name);
        assert!(out["data"].is_array(), "{} 的 data 应是数组", name);
        assert!(
            out["hint"].is_string(),
            "{} 应告诉调用方还有没有、怎么继续",
            name
        );
        assert!(
            chars(&out) < 6000,
            "{} 的体积应受控，实测 {} 字符",
            name,
            chars(&out)
        );
    }

    // 超上限同样报错（不是静默夹紧）
    let err = tool(&registry, "list_relations")
        .execute(json!({ "world_id": wid, "limit": 9999 }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("超过单次上限"), "{}", err);

    Ok(())
}

/// 档案读取：`writable_fields` 默认只给字段名，`include_schema: true` 才给完整结构。
///
/// 实测完整结构 5,944 字符/次，占单次档案返回的 43%。
#[tokio::test]
async fn profile_schema_is_on_demand() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("profile-schema-ondemand", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("project id")).unwrap();
    let world = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())))
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界");

    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);
    let created = tool(&registry, "create_character")
        .execute(json!({ "world_id": world.id.to_string(), "name": "林夜" }))
        .await?;
    let id = created["data"]["id"].as_str().expect("character id").to_string();

    let small = tool(&registry, "get_character_profile")
        .execute(json!({ "id": id }))
        .await?;
    assert!(
        small["writable_fields"]["names"].is_array(),
        "默认应给字段名清单：{}",
        small["writable_fields"]
    );
    assert!(
        small["writable_fields"].get("identity").is_none(),
        "默认不该把完整结构塞进来"
    );

    let big = tool(&registry, "get_character_profile")
        .execute(json!({ "id": id, "include_schema": true }))
        .await?;
    assert!(
        big["writable_fields"]["identity"].is_object(),
        "显式要求时应给完整结构"
    );
    assert!(
        chars(&big) > chars(&small),
        "完整结构应当更大：{} vs {}",
        chars(&big),
        chars(&small)
    );

    Ok(())
}

/// 全景索引：一次调用回答「项目进行到什么情况」，体积 ≤3K。
///
/// 原先这个问题要靠 batch_call 批量拉全量，实测产生 78,616 字符的工具结果。
#[tokio::test]
async fn project_index_is_small_and_complete() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("project-index-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("project id")).unwrap();
    let world = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())))
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界");

    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);
    tool(&registry, "create_character")
        .execute(json!({ "world_id": world.id.to_string(), "name": "王久财" }))
        .await?;
    tool(&registry, "create_foreshadow")
        .execute(json!({
            "project_id": project_id.to_string(),
            "name": "神像的来历"
        }))
        .await?;

    let out = tool(&registry, "get_project_index")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;

    assert!(out["project"]["id"].is_string(), "应给项目骨架");
    assert!(
        out["entities"]["total"].as_u64().unwrap() >= 1,
        "实体计数应包含刚建的角色：{}",
        out["entities"]
    );
    assert_eq!(out["entities"]["by_type"]["Character"], json!(1));
    assert!(out["foreshadows"]["by_status"].is_object(), "应给伏笔状态分布");
    assert_eq!(out["foreshadows"]["unlinked"], json!(1), "未挂线的伏笔数");
    assert!(out["storylines"]["items"].is_array());
    assert!(out["nodes"]["by_type"].is_object());
    assert!(out["recent_events"].is_array());
    assert!(
        chars(&out) < 3000,
        "全景索引应 ≤3000 字符，实测 {}",
        chars(&out)
    );

    Ok(())
}
