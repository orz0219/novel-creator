//! Phase B 领域工具集成测试
//!
//! 需要 PostgreSQL 环境（读取 DATABASE_URL）。验证 Narrative / Storyline / Rule /
//! Project / World / History 聚合工具的创建、修改、读取与逻辑删除（retire_node 后
//! get_node 失败，即软删除）全链路。

use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbProjectRepositoryPort, DbWorldRepositoryPort};
use narrative_engine::agent_tools::register_all_domain_tools;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;


fn tool(registry: &Arc<ToolRegistry>, name: &str) -> Arc<dyn AgentTool> {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("tool '{}' 未注册", name))
}

#[tokio::test]
async fn phase_b_tools_crud_logical_delete() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);

    // 1) 项目（经 ProjectService 创建并自动 ensure 主世界；create_project / list_projects 工具已移除）
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("pb-test-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();

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

    tool(&registry, "retire_node")
        .execute(json!({ "id": node_id.to_string() }))
        .await?;
    // 逻辑删除后不应再被读取到（已统一过滤 status='Deleted'，与 Entity 一致）
    let after = tool(&registry, "get_node")
        .execute(json!({ "id": node_id.to_string() }))
        .await;
    assert!(after.is_err(), "retire_node 后 get_node 应失败（软删除已过滤读取）");
    // 且底层确为软删（status='Deleted'），行仍存在而非物理 DELETE
    let nstatus: Option<Option<String>> = sqlx::query_scalar(
        "SELECT status FROM narrative_node WHERE id = $1",
    )
    .bind(node_id)
    .fetch_optional(&pool)
    .await?;
    assert!(
        matches!(nstatus, Some(Some(s)) if s == "Deleted"),
        "retire_node 应为软删除（status='Deleted'），行仍存在而非物理删除"
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

/// 回归：`batch_call` 的子调用必须与单次调用一样拿到框架注入的 `project_id`。
///
/// 实际故障：批处理里调 `list_foreshadows` 报「字段 'project_id' 缺失或不是合法
/// UUID」，而同一工具单独调用正常——因为子调用原先绕过了 `runtime::tool_execute`
/// 的注入。本用例故意**一项都不传** `project_id`，靠框架注入跑通。
#[tokio::test]
async fn batch_call_injects_project_id_into_domain_tool_items() -> Result<()> {
    let pool = testkit::test_pool().await?;
    let registry = Arc::new(ToolRegistry::new());
    register_all_domain_tools(&registry, &pool);
    registry.register(Arc::new(agent::BatchCallTool::new(registry.clone())));

    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("pb-batch-call-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();

    let out = registry
        .execute_in_project(
            project_id,
            "batch_call",
            json!({"calls": [
                {"tool": "list_foreshadows", "input": {}},
                {"tool": "list_nodes", "input": {}},
                {"tool": "get_main_world", "input": {}}
            ]}),
        )
        .await?;

    assert_eq!(out["ok"], json!(true), "批量读不应失败：{}", out);
    assert_eq!(out["executed"], json!(3));
    assert!(
        out["results"][0]["result"]["data"].is_array(),
        "list_foreshadows 应返回数组：{}",
        out
    );
    assert!(
        out["results"][1]["result"]["data"].is_array(),
        "list_nodes 应返回数组：{}",
        out
    );
    assert!(
        out["results"][2]["result"]["data"].is_object(),
        "get_main_world 应返回主世界：{}",
        out
    );

    // 反证：同一批调用若不经过注入（直接 execute），必须明确报错，
    // 而不是"悄悄当成没有项目过滤"继续跑。
    let raw = registry
        .execute(
            "batch_call",
            json!({"calls": [{"tool": "list_foreshadows", "input": {}}]}),
        )
        .await
        .unwrap_err();
    assert!(
        raw.to_string().contains("project_id"),
        "未经注入的批量调用应报 project_id 缺失：{}",
        raw
    );

    Ok(())
}

/// revise_foreshadow：只改归属时不必抄名字，也不会顺手把描述清空。
///
/// 回归自实际卡点：改 3 条伏笔的归属得把名字一起抄一遍，抄错就是一次误改名；
/// 而 description 原先是无条件写入，不传就等于清空。
#[tokio::test]
async fn revise_foreshadow_only_touches_provided_fields() -> Result<()> {
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
        .create_project("pb-revise-foreshadow-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();

    let fs = tool(&registry, "create_foreshadow")
        .execute(json!({
            "project_id": project_id.to_string(),
            "name": "神像的来历",
            "description": "破庙里那尊无名神像原本属于谁",
            "importance": "Important"
        }))
        .await?;
    let fs_id = fs["data"]["id"].as_str().expect("foreshadow id").to_string();

    let sl = tool(&registry, "create_storyline")
        .execute(json!({ "project_id": project_id.to_string(), "name": "神明的遗产" }))
        .await?;
    let sl_id = sl["data"]["id"].as_str().expect("storyline id").to_string();

    // 只挂线：name / description 都不传
    let revised = tool(&registry, "revise_foreshadow")
        .execute(json!({ "id": fs_id, "storyline_id": sl_id }))
        .await?;
    assert!(revised["ok"].as_bool().is_some_and(|b| b), "{}", revised);

    let list = tool(&registry, "list_foreshadows")
        .execute(json!({ "project_id": project_id.to_string() }))
        .await?;
    let item = list["data"]
        .as_array()
        .expect("foreshadows array")
        .iter()
        .find(|f| f["id"] == json!(fs_id))
        .expect("伏笔仍应在列表里");
    assert_eq!(item["name"], json!("神像的来历"), "没传 name 就不能改名");
    assert_eq!(
        item["description"],
        json!("破庙里那尊无名神像原本属于谁"),
        "没传 description 就不能清空"
    );
    assert_eq!(item["storyline_id"], json!(sl_id), "归属应改到新线上");

    // 什么都没给：报错，而不是"成功但什么都没变"
    let err = tool(&registry, "revise_foreshadow")
        .execute(json!({ "id": fs_id }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("至少要给"), "{}", err);

    Ok(())
}

/// 伏笔等级字段：写入即归一（中文可写），非法值报错——不再让自由文本落库。
///
/// 回归自实际数据污染：importance 里混着 `重要` / `Main` / `Major`，
/// hint_level 里混着 `低（前期只透风，不揭示）` / `中期显形`，
/// 前端因此无法按等级排序 / 过滤（根因是 HTTP API 直接透传字符串）。
#[tokio::test]
async fn foreshadow_enum_is_normalized_on_write() -> Result<()> {
    use application::foreshadow_service::ForeshadowService;
    use db::application_ports::DbForeshadowRepositoryPort;

    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("foreshadow-enum-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();

    let service = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(pool.clone())));

    // 中文写法照常可写，但**落库的是规范枚举**
    service
        .create_foreshadow(
            project_id,
            "神像的来历",
            Some("破庙里那尊无名神像原本属于谁"),
            "计划中",
            "重要",
            "低",
            None,
            None,
            None,
            // 节点锚与伏笔树（本轮新增）
            None,
            None,
            None,
            None,
        )
        .await?;

    let list = service.list_foreshadows(project_id).await?;
    let item = list
        .iter()
        .find(|f| f["name"] == serde_json::json!("神像的来历"))
        .expect("刚建的伏笔应在列表里");
    assert_eq!(item["importance"], serde_json::json!("Important"), "{}", item);
    assert_eq!(item["hint_level"], serde_json::json!("Subtle"), "{}", item);

    // 非法值：报错并给出合法取值，绝不静默写进去
    let err = service
        .create_foreshadow(
            project_id,
            "写坏的伏笔",
            None,
            "Planned",
            "超级重要",
            "Subtle",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("无法识别"), "{}", msg);
    assert!(msg.contains("Core"), "错误里应给出合法取值：{}", msg);

    Ok(())
}

/// 伏笔的备注 + 三个时间锚：写得进、读得出；revise 回显**完整对象**且不动未传字段。
///
/// 回归自实际反馈：伏笔原先只有 importance / hint_level / status 三个属性，
/// 「埋在哪一章、打算哪一章收」无处可写；而 revise 只回 `{"updated":true}`，
/// 改完看不到写进去的是什么，只能再 list 一遍全文。
#[tokio::test]
async fn foreshadow_note_and_time_anchors_roundtrip() -> Result<()> {
    use application::foreshadow_service::ForeshadowService;
    use db::application_ports::DbForeshadowRepositoryPort;

    let pool = testkit::test_pool().await?;
    let project_service = ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            pool.clone(),
        )))),
    );
    let proj = project_service
        .create_project("foreshadow-anchor-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();

    let service = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(pool.clone())));

    let created = service
        .create_foreshadow(
            project_id,
            "神像的来历",
            Some("破庙里那尊无名神像原本属于谁"),
            "Planned",
            "Important",
            "Subtle",
            Some("前期只透风，不揭示"),
            Some("第三卷"),
            Some("第五卷"),
            None,
            None,
            None,
            None,
        )
        .await?;

    // 创建就回显完整对象（不必再 list 一遍）
    assert_eq!(created["hint_note"], serde_json::json!("前期只透风，不揭示"));
    assert_eq!(created["introduced_at"], serde_json::json!("第三卷"));
    assert_eq!(created["expected_reveal_at"], serde_json::json!("第五卷"));
    assert_eq!(
        created["description"],
        serde_json::json!("破庙里那尊无名神像原本属于谁"),
        "创建回显应带描述，而不是只回 id / name"
    );

    let id = Uuid::parse_str(created["id"].as_str().expect("foreshadow id")).unwrap();

    // 只填「实际回收时机」：其余字段一律保持原值
    let updated = service
        .update_foreshadow(
            id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("第六卷"),
            None,
            None,
            None,
            None,
        )
        .await?;

    assert_eq!(updated["actual_reveal_at"], serde_json::json!("第六卷"));
    assert_eq!(
        updated["hint_note"],
        serde_json::json!("前期只透风，不揭示"),
        "没传的备注不能被清掉"
    );
    assert_eq!(updated["introduced_at"], serde_json::json!("第三卷"));
    assert_eq!(updated["expected_reveal_at"], serde_json::json!("第五卷"));
    assert_eq!(updated["name"], serde_json::json!("神像的来历"));
    assert_eq!(updated["importance"], serde_json::json!("Important"));

    Ok(())
}

/// revise_storyline / revise_event 回显**修改后的完整对象**。
///
/// 原先只回 `{"updated":true}`：调用方（AI）看不到写进去的是什么，只能再 list
/// 一遍全文——本项目的 storyline / 事件全文近万字，重复拉取代价很大。
#[tokio::test]
async fn revise_storyline_and_event_echo_full_object() -> Result<()> {
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
        .create_project("revise-echo-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();
    let pid = project_id.to_string();

    // 剧情线：改描述后应回显完整对象（含 name / status / tone / visibility）
    let sl = tool(&registry, "create_storyline")
        .execute(json!({ "project_id": pid, "name": "神明的遗产" }))
        .await?;
    let sl_id = sl["data"]["id"].as_str().expect("storyline id").to_string();
    let revised = tool(&registry, "revise_storyline")
        .execute(json!({
            "id": sl_id,
            "description": "待校准·留白：神格残片与神明的下落"
        }))
        .await?;
    assert_eq!(
        revised["data"]["description"],
        json!("待校准·留白：神格残片与神明的下落"),
        "应回显写入后的描述：{}",
        revised
    );
    assert_eq!(revised["data"]["name"], json!("神明的遗产"), "回显要带名字");
    assert!(revised["data"]["status"].is_string(), "回显要带状态：{}", revised);
    assert!(revised["data"]["visibility"].is_string());

    // 事件：改名字后应回显完整对象（含 attributes 结构化字段）
    let ev = tool(&registry, "create_event")
        .execute(json!({
            "project_id": pid,
            "name": "炼器神明失踪",
            "description": "千年之前，炼器神明忽然不见"
        }))
        .await?;
    let ev_id = ev["data"]["id"].as_str().expect("event id").to_string();
    let ev_revised = tool(&registry, "revise_event")
        .execute(json!({ "id": ev_id, "name": "炼器神明失踪（改）" }))
        .await?;
    assert_eq!(ev_revised["data"]["name"], json!("炼器神明失踪（改）"));
    assert_eq!(
        ev_revised["data"]["description"],
        json!("千年之前，炼器神明忽然不见"),
        "没传的字段不能被清掉"
    );
    assert!(ev_revised["data"]["attributes"].is_object(), "回显要带 attributes");

    Ok(())
}

/// ④ 事件的历史轴排序锚 `era_order`；⑤ 新增 Deity（神明）实体类型。
///
/// 回归自实际反馈：`when` 是中文自由文本（「远昔」「缓变」）没法比大小，历史轴一定排乱；
/// 而故事核心的「神明」此前没有类型，只能停在事件描述里当散文，挂不上关系边。
#[tokio::test]
async fn event_era_order_and_deity_type() -> Result<()> {
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
        .create_project("era-order-deity-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();
    let pid = project_id.to_string();
    let world = WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone())))
        .get_or_create_main_world(project_id)
        .await?
        .expect("主世界");
    let wid = world.id.to_string();

    // ⑤ Deity：中文别名「神明」可直接建实体
    let deity = tool(&registry, "create_entity")
        .execute(json!({ "world_id": wid, "entity_type": "神明", "name": "炼器神明" }))
        .await?;
    assert!(
        deity["ok"].as_bool().is_some_and(|b| b),
        "「神明」应能建成实体：{}",
        deity
    );

    let types = tool(&registry, "list_entity_types").execute(json!({})).await?;
    let list = types["data"].as_array().expect("类型清单");
    let d = list
        .iter()
        .find(|t| t["entity_type"] == json!("Deity"))
        .unwrap_or_else(|| panic!("类型清单应含 Deity：{types}"));
    assert!(
        d["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "神明" || a == "神祇"),
        "Deity 应带中文别名：{}",
        d
    );

    // ④ era_order：先建「纪元早」的事件，再建「纪元晚」的事件
    let early = tool(&registry, "create_event")
        .execute(json!({
            "project_id": pid,
            "name": "炼器神明失踪",
            "description": "千年之前，炼器神明忽然不见",
            "era_order": 1
        }))
        .await?;
    assert_eq!(
        early["data"]["era_order"],
        json!(1),
        "创建应回显 era_order：{}",
        early
    );
    tool(&registry, "create_event")
        .execute(json!({
            "project_id": pid,
            "name": "轮回渡开始积压",
            "description": "当下",
            "era_order": 9
        }))
        .await?;

    // 按历史轴排：era_order=1 在前（与创建顺序相反）
    let by_era = tool(&registry, "list_events")
        .execute(json!({ "project_id": pid, "order_by": "era" }))
        .await?;
    let era_names: Vec<&str> = by_era["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["name"].as_str())
        .collect();
    let p1 = era_names.iter().position(|n| *n == "炼器神明失踪").expect("事件在");
    let p2 = era_names.iter().position(|n| *n == "轮回渡开始积压").expect("事件在");
    assert!(
        p1 < p2,
        "按历史轴排序时 era_order=1 应在前：{:?}",
        era_names
    );

    // 默认（recent）仍按创建时间倒序：后创建的在前
    let by_recent = tool(&registry, "list_events")
        .execute(json!({ "project_id": pid }))
        .await?;
    let recent_names: Vec<&str> = by_recent["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["name"].as_str())
        .collect();
    let q1 = recent_names
        .iter()
        .position(|n| *n == "轮回渡开始积压")
        .unwrap();
    let q2 = recent_names
        .iter()
        .position(|n| *n == "炼器神明失踪")
        .unwrap();
    assert!(q1 < q2, "默认最近优先：{:?}", recent_names);

    // 非法 order_by：明确报错（不静默回退到 recent）
    let err = tool(&registry, "list_events")
        .execute(json!({ "project_id": pid, "order_by": "random" }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("order_by"), "{}", err);

    Ok(())
}

/// ⑥ 剧情线之间可以建**带类型的横向边**（驱动 / 依赖 / 交汇 / 对冲），并能查看与删除。
///
/// 回归自实际反馈：副线正文里写着「《破庙居委会》吃得越多 → 《现实线的建设》压力越大」，
/// 但 storyline_relation 只有 parent_id / child_id，只能表达挂载树，
/// 横向咬合关系无处可写，前端也画不出"线咬合图"。
#[tokio::test]
async fn storyline_relations_carry_type() -> Result<()> {
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
        .create_project("storyline-rel-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();
    let pid = project_id.to_string();

    let mut ids = Vec::new();
    for name in ["《破庙居委会》", "《现实线的建设》", "《旧秩序的墙》"] {
        let sl = tool(&registry, "create_storyline")
            .execute(json!({ "project_id": pid, "name": name, "importance": "Normal" }))
            .await?;
        ids.push(
            sl["data"]["id"]
                .as_str()
                .expect("storyline id")
                .to_string(),
        );
    }
    let (a, b, c) = (ids[0].clone(), ids[1].clone(), ids[2].clone());

    // 驱动（中文别名「驱动」应能解析）
    let r1 = tool(&registry, "relate_storylines")
        .execute(json!({
            "project_id": pid,
            "from_storyline_id": a,
            "to_storyline_id": b,
            "relation_type": "驱动"
        }))
        .await?;
    assert_eq!(
        r1["data"]["relation_type"],
        json!("Drives"),
        "中文「驱动」应归一化为 Drives：{}",
        r1
    );

    // 交汇与对冲（方向有意义）
    tool(&registry, "relate_storylines")
        .execute(json!({
            "project_id": pid,
            "from_storyline_id": b,
            "to_storyline_id": c,
            "relation_type": "Intersects"
        }))
        .await?;
    tool(&registry, "relate_storylines")
        .execute(json!({
            "project_id": pid,
            "from_storyline_id": c,
            "to_storyline_id": a,
            "relation_type": "Counters"
        }))
        .await?;

    // 查看：带类型 + 两端名字
    let listed = tool(&registry, "list_storyline_relations")
        .execute(json!({ "project_id": pid }))
        .await?;
    let edges = listed["data"].as_array().expect("关系数组");
    assert_eq!(edges.len(), 3, "应有 3 条边：{}", listed);
    let drives = edges
        .iter()
        .find(|e| e["relation_type"] == json!("Drives"))
        .expect("驱动边在");
    assert_eq!(drives["from_name"], json!("《破庙居委会》"));
    assert_eq!(drives["to_name"], json!("《现实线的建设》"));
    assert_eq!(listed["total"], json!(3));

    // 幂等：同向重复建边是**更新类型**，不是新增一条
    let again = tool(&registry, "relate_storylines")
        .execute(json!({
            "project_id": pid,
            "from_storyline_id": a,
            "to_storyline_id": b,
            "relation_type": "DependsOn"
        }))
        .await?;
    assert_eq!(again["data"]["relation_type"], json!("DependsOn"));
    let after = tool(&registry, "list_storyline_relations")
        .execute(json!({ "project_id": pid }))
        .await?;
    assert_eq!(after["total"], json!(3), "同向建边应更新而不是新增：{}", after);

    // 自连：报错（不是静默建一条自己指向自己的边）
    let err = tool(&registry, "relate_storylines")
        .execute(json!({
            "project_id": pid,
            "from_storyline_id": a,
            "to_storyline_id": a,
            "relation_type": "Drives"
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("自己"), "{}", err);

    // 删除一条边
    let edge_id = drives["id"].as_str().expect("edge id").to_string();
    tool(&registry, "unrelate_storylines")
        .execute(json!({ "id": edge_id }))
        .await?;
    let last = tool(&registry, "list_storyline_relations")
        .execute(json!({ "project_id": pid }))
        .await?;
    assert_eq!(last["total"], json!(2), "删边后应剩 2 条：{}", last);

    Ok(())
}

/// ⑦ 伏笔可以挂「埋点 / 回收点」节点锚，也可以挂父伏笔形成**伏笔树**；
/// 锚点指向不存在的目标时**直接报错**（不写悬空 uuid）。
///
/// 回归自实际反馈：伏笔原先只有 importance / hint_level / status，
/// 「这条钩子埋在哪一章、打算哪一章收」只能写在 description 里当散文，排章节时查不出来。
#[tokio::test]
async fn foreshadow_node_anchors_and_tree() -> Result<()> {
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
        .create_project("foreshadow-anchor-node-project", None, None)
        .await?;
    let project_id = Uuid::parse_str(proj["id"].as_str().expect("proj id")).unwrap();
    let pid = project_id.to_string();

    // 两个章节节点
    let mut chapters = Vec::new();
    for title in ["第3章·破庙", "第7章·回收"] {
        let node = tool(&registry, "create_node")
            .execute(json!({
                "project_id": pid,
                "node_type": "Chapter",
                "title": title
            }))
            .await?;
        chapters.push(node["data"]["id"].as_str().expect("node id").to_string());
    }
    let (ch3, ch7) = (chapters[0].clone(), chapters[1].clone());

    // 大伏笔：埋在第 3 章、计划第 7 章收
    let big = tool(&registry, "create_foreshadow")
        .execute(json!({
            "project_id": pid,
            "name": "神明的遗产",
            "description": "暗线总纲",
            "planted_node_id": ch3,
            "payoff_node_id": ch7
        }))
        .await?;
    assert_eq!(
        big["data"]["planted_node_id"],
        json!(ch3),
        "创建应回显埋点：{}",
        big
    );
    assert_eq!(big["data"]["payoff_node_id"], json!(ch7));
    let big_id = big["data"]["id"].as_str().expect("foreshadow id").to_string();

    // 小钩子挂在它下面（伏笔树）
    let small = tool(&registry, "create_foreshadow")
        .execute(json!({
            "project_id": pid,
            "name": "神像的来历",
            "parent_foreshadow_id": big_id
        }))
        .await?;
    assert_eq!(
        small["data"]["parent_foreshadow_id"],
        json!(big_id),
        "应能挂到父伏笔下：{}",
        small
    );
    let small_id = small["data"]["id"].as_str().expect("id").to_string();

    // 列表里能看到这三个锚点
    let listed = tool(&registry, "list_foreshadows")
        .execute(json!({ "project_id": pid }))
        .await?;
    let item = listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == json!(small_id))
        .expect("小钩子在列表里");
    assert_eq!(item["parent_foreshadow_id"], json!(big_id));

    // revise 只改回收点：父伏笔不能被清掉
    let revised = tool(&registry, "revise_foreshadow")
        .execute(json!({ "id": small_id, "payoff_node_id": ch7 }))
        .await?;
    assert_eq!(revised["data"]["payoff_node_id"], json!(ch7));
    assert_eq!(
        revised["data"]["parent_foreshadow_id"],
        json!(big_id),
        "没传的锚点不能被清掉：{}",
        revised
    );

    // 悬空节点：报错（不写悬空 uuid）
    let err = tool(&registry, "create_foreshadow")
        .execute(json!({
            "project_id": pid,
            "name": "悬空的钩子",
            "planted_node_id": Uuid::new_v4().to_string()
        }))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("叙事节点"), "{}", err);

    // 悬空父伏笔：报错
    let err2 = tool(&registry, "revise_foreshadow")
        .execute(json!({
            "id": big_id,
            "parent_foreshadow_id": Uuid::new_v4().to_string()
        }))
        .await
        .unwrap_err();
    assert!(err2.to_string().contains("父伏笔"), "{}", err2);

    Ok(())
}
