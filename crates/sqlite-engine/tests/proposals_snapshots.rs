//! 提案审批与快照回滚的 HTTP 契约测试。
//!
//! 手机端「更多」页现在能直接批准/拒绝 AI 提案、新建/恢复/删除快照，
//! 这些按钮背后就是下面这几个接口。测试守住两件事：
//!   1. 接口真的能改状态（不是只返回 200 却没落库）
//!   2. 返回/列表里的字段够手机端渲染（`id` / `status` / `changes[].description`）
//!
//! 前端对状态的处理见 `mobile/dist/js/app.js` 的 `isPendingProposal()`，
//! 它按后端 `ProposedChangeStatus::description()` 给出的英文名判断可操作性——
//! 这个测试会一并把「哪些状态真的能批准」钉住。

use std::path::PathBuf;
use std::time::Duration;

use sqlite_engine::engine::{BootstrapConfig, Engine};
use uuid::Uuid;

struct TestServer {
    base: String,
    dir: PathBuf,
    state: sqlite_engine::state::AppState,
}

impl TestServer {
    async fn start(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("novel-prop-{}-{}", tag, Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("engine.db");

        let engine = Engine::build(db_path.to_str().unwrap(), BootstrapConfig::default())
            .await
            .expect("引擎组装失败");

        let app = sqlite_engine::api::router(engine.state.clone());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(200)).await;

        Self {
            base: format!("http://127.0.0.1:{}", addr.port()),
            dir,
            state: engine.state.clone(),
        }
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            // 环境里可能有 http_proxy（构建时用），本地请求必须绕过代理
            .no_proxy()
            .build()
            .unwrap()
    }

    /// 建一个项目，返回 (project_id, world_id)
    async fn new_project(&self, name: &str) -> (Uuid, Uuid) {
        let pid: serde_json::Value = self
            .client()
            .post(format!("{}/api/v1/projects", self.base))
            .json(&serde_json::json!({ "name": name, "description": "测试" }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let pid = Uuid::parse_str(pid["id"].as_str().unwrap()).unwrap();

        let wid: Uuid = sqlx::query_scalar("SELECT id FROM world WHERE project_id = $1 AND is_main = 1")
            .bind(pid)
            .fetch_one(&self.state.pool)
            .await
            .expect("主世界不存在");
        (pid, wid)
    }

    /// 造一条提案（直接写库，模拟 AI/抽取流程产出的待审批变更）
    async fn insert_proposal(
        &self,
        project_id: Uuid,
        target: Uuid,
        status: &str,
        change_type: &str,
        payload: serde_json::Value,
        description: &str,
    ) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO proposed_change \
             (id, project_id, change_type, target_entity_id, description, payload, status) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(change_type)
        .bind(target)
        .bind(description)
        .bind(payload.to_string())
        .bind(status)
        .execute(&self.state.pool)
        .await
        .expect("写入提案失败");
        id
    }

    async fn proposal_status(&self, id: Uuid) -> String {
        sqlx::query_scalar::<_, String>("SELECT status FROM proposed_change WHERE id = $1")
            .bind(id)
            .fetch_one(&self.state.pool)
            .await
            .expect("读提案状态失败")
    }

    async fn cleanup(self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 快照：新建 → 列表 → 恢复 → 删除，每步都验真。
#[tokio::test]
async fn snapshot_create_list_restore_delete() {
    let srv = TestServer::start("snap").await;
    let c = srv.client();
    let (pid, _wid) = srv.new_project("快照测试").await;

    // 列表一开始是空的
    let list: serde_json::Value = c
        .get(format!("{}/api/v1/projects/{}/snapshots", srv.base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = list.as_array().expect("快照列表应为数组");
    assert!(arr.is_empty(), "新项目不应有快照: {}", list);

    // 新建（手机端「新建快照」走的就是这个）
    let created: serde_json::Value = c
        .post(format!("{}/api/v1/projects/{}/snapshots", srv.base, pid))
        .json(&serde_json::json!({
            "name": "第二卷完稿",
            "story_time": "第三年春",
            "world_summary": "主角刚拿下北境商路",
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("创建的快照: {}", created);
    let snap_id = created["id"].as_str().expect("快照必须有 id").to_string();

    // 手机端靠 name 显示标题，缺失就会退化成 uuid 前 8 位
    let list: serde_json::Value = c
        .get(format!("{}/api/v1/projects/{}/snapshots", srv.base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1, "应有 1 个快照: {}", list);
    assert_eq!(arr[0]["name"], "第二卷完稿", "列表必须带 name 供手机端显示");

    // 恢复（幂等：把快照的宏观状态回写到 narrative_state）
    let restored: serde_json::Value = c
        .post(format!("{}/api/v1/snapshots/{}/restore", srv.base, snap_id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("恢复结果: {}", restored);
    assert!(
        restored.get("restored_keys").is_some() || restored.get("project_id").is_some(),
        "恢复接口应返回摘要: {}",
        restored
    );

    // 状态确实写进去了（不是只返回 200）
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM narrative_state WHERE project_id = $1 AND state_key = 'story_time'",
    )
    .bind(pid)
    .fetch_one(&srv.state.pool)
    .await
    .unwrap();
    assert_eq!(n, 1, "恢复后 narrative_state 里应有 story_time");

    // 恢复幂等：再恢复一次不应报错、也不应产生重复行
    let again = c
        .post(format!("{}/api/v1/snapshots/{}/restore", srv.base, snap_id))
        .send()
        .await
        .unwrap();
    assert!(again.status().is_success(), "重复恢复应当成功（幂等）");
    let n2: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM narrative_state WHERE project_id = $1 AND state_key = 'story_time'",
    )
    .bind(pid)
    .fetch_one(&srv.state.pool)
    .await
    .unwrap();
    assert_eq!(n2, 1, "重复恢复不应产生重复状态行");

    // 删除
    let del: serde_json::Value = c
        .delete(format!("{}/api/v1/snapshots/{}", srv.base, snap_id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(del["deleted"], true);

    let list: serde_json::Value = c
        .get(format!("{}/api/v1/projects/{}/snapshots", srv.base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(list.as_array().unwrap().is_empty(), "删除后列表应为空");

    // 恢复一个不存在的快照必须明确报错（不静默成功）
    let missing = c
        .post(format!(
            "{}/api/v1/snapshots/{}/restore",
            srv.base,
            Uuid::new_v4()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404, "不存在的快照应返回 404");

    srv.cleanup().await;
}

/// 提案：列表字段够渲染、批准会真的落库、拒绝会改状态、已处理的不再是待审批。
#[tokio::test]
async fn proposal_accept_and_reject_actually_persist() {
    let srv = TestServer::start("prop").await;
    let c = srv.client();
    let (pid, wid) = srv.new_project("提案测试").await;

    // 一个可被批准的实体（EntityUpdate 要指向真实实体）
    // 走仓储接口建实体：entity 的类型是 entity_type_id（外键），不是裸名字
    use domain::ports::EntityRepositoryPort;
    let ent_repo =
        sqlite_db::application_ports::DbEntityRepositoryPort::new(srv.state.pool.clone());
    let created_entity = ent_repo
        .create_entity(wid, "Character", "张三", Some("主角"), Some("一个普通人"))
        .await
        .expect("创建实体失败");
    let ent_id = Uuid::parse_str(created_entity["id"].as_str().expect("实体应有 id")).unwrap();

    let to_update = srv
        .insert_proposal(
            pid,
            ent_id,
            "Valid",
            "EntityUpdate",
            serde_json::json!({
                "type": "EntityUpdate",
                "data": { "name": "张三丰", "attributes": null }
            }),
            "把角色名改为张三丰",
        )
        .await;
    let to_reject = srv
        .insert_proposal(
            pid,
            wid,
            "Valid",
            "KnowledgeUpdate",
            serde_json::json!({
                "type": "KnowledgeUpdate",
                "data": { "fact_content": "北境商路已开通", "certainty": "Certain" }
            }),
            "新增事实：北境商路已开通",
        )
        .await;

    // ---- 列表：手机端渲染需要的字段 ----
    let list: serde_json::Value = c
        .get(format!("{}/api/v1/projects/{}/proposals", srv.base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = list.as_array().expect("提案列表应为数组");
    assert_eq!(arr.len(), 2, "应有 2 条提案: {}", list);
    for p in arr {
        assert!(p["id"].is_string(), "提案必须有 id: {}", p);
        assert!(p["status"].is_string(), "提案必须有 status: {}", p);
        assert!(
            p["changes"].as_array().map(|a| !a.is_empty()).unwrap_or(false),
            "提案必须带 changes，手机端用它显示描述: {}",
            p
        );
        assert!(
            p["changes"][0]["description"].is_string(),
            "changes[0].description 要能给手机端当标题: {}",
            p
        );
    }
    // 手机端 isPendingProposal() 认这些状态为「可操作」
    assert_eq!(arr[0]["status"], "Valid");

    // ---- 批准：状态与正典数据都要真的变 ----
    let accepted: serde_json::Value = c
        .post(format!("{}/api/v1/proposals/{}/accept", srv.base, to_update))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("批准结果: {}", accepted);
    assert_eq!(accepted["id"], to_update.to_string());
    let status = accepted["status"].as_str().unwrap().to_string();
    assert!(
        matches!(status.as_str(), "Approved" | "Committed" | "Applied"),
        "批准后状态应为已批准/已提交，实为 {}",
        status
    );

    // 实体名真的改了（批准 = 写入 Canon，不是只翻状态）
    let name: String = sqlx::query_scalar("SELECT name FROM entity WHERE id = $1")
        .bind(ent_id)
        .fetch_one(&srv.state.pool)
        .await
        .unwrap();
    assert_eq!(name, "张三丰", "批准后实体名应已更新");

    // ---- 拒绝：状态落库 ----
    let rejected: serde_json::Value = c
        .post(format!("{}/api/v1/proposals/{}/reject", srv.base, to_reject))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("拒绝结果: {}", rejected);
    assert_eq!(srv.proposal_status(to_reject).await, "Rejected");
    // 被拒绝的提案不应产生事实数据
    let facts: i64 = sqlx::query_scalar("SELECT count(*) FROM fact WHERE project_id = $1")
        .bind(pid)
        .fetch_one(&srv.state.pool)
        .await
        .unwrap();
    assert_eq!(facts, 0, "拒绝的提案不应写入事实");

    // ---- 再查列表：状态已持久化，手机端据此不再显示按钮 ----
    let list: serde_json::Value = c
        .get(format!("{}/api/v1/projects/{}/proposals", srv.base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let statuses: Vec<String> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["status"].as_str().unwrap().to_string())
        .collect();
    println!("处理后的状态: {:?}", statuses);
    assert!(statuses.iter().any(|s| s == "Rejected"));
    assert!(
        !statuses.iter().any(|s| s == "Valid"),
        "处理过的提案不应仍停在 Valid: {:?}",
        statuses
    );

    // 批准一条不存在的提案必须报错（不静默成功）
    let bad = c
        .post(format!(
            "{}/api/v1/proposals/{}/accept",
            srv.base,
            Uuid::new_v4()
        ))
        .send()
        .await
        .unwrap();
    assert!(
        !bad.status().is_success(),
        "批准不存在的提案不应返回成功"
    );

    srv.cleanup().await;
}

/// 提交失败时，提案必须保持原状态（可重试），不能停在「已批准」。
///
/// 这是实测踩到的坑：`approve_proposal` 原来先改状态再提交 Canon，
/// 一旦提交失败（例如此处目标实体不存在 → `fact_entity` 外键失败），
/// 提案就变成「已批准但数据没落库」——界面上看起来批过了，用户不会再收到
/// 任何提示，状态机也不允许再批准，连重试都做不到。
#[tokio::test]
async fn failed_commit_leaves_proposal_retryable() {
    let srv = TestServer::start("retry").await;
    let c = srv.client();
    let (pid, _wid) = srv.new_project("失败重试测试").await;

    // target_entity_id 指向一个不存在的实体 → 提交时外键必然失败
    let bad = srv
        .insert_proposal(
            pid,
            Uuid::new_v4(),
            "Valid",
            "KnowledgeUpdate",
            serde_json::json!({
                "type": "KnowledgeUpdate",
                "data": { "fact_content": "这条不该被写入", "certainty": "Certain" }
            }),
            "目标实体不存在，提交必然失败",
        )
        .await;

    let resp = c
        .post(format!("{}/api/v1/proposals/{}/accept", srv.base, bad))
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap();
    println!("批准失败响应: {} {}", status, body);
    assert!(status >= 400, "提交失败时接口不应返回成功");

    // 关键断言：状态没有被改成 Approved
    let after = srv.proposal_status(bad).await;
    println!("失败后的提案状态: {}", after);
    assert_eq!(
        after, "Valid",
        "提交失败后提案必须保持原状态（否则用户无法重试，且会误以为已生效）"
    );

    // 也不能因为失败而写进任何事实数据
    let facts: i64 = sqlx::query_scalar("SELECT count(*) FROM fact WHERE project_id = $1")
        .bind(pid)
        .fetch_one(&srv.state.pool)
        .await
        .unwrap();
    assert_eq!(facts, 0, "失败的提交不应留下数据");

    srv.cleanup().await;
}

/// 手机端把「非待审批」的状态一律显示为已处理，这里钉住状态名，
/// 防止后端改名字后前端按钮消失或误显（前端见 app.js 的 isPendingProposal）。
#[tokio::test]
async fn proposal_status_names_match_frontend_expectations() {
    use domain::validation::ProposedChangeStatus;

    // 手机端认为可批准的状态
    let actionable = ["Draft", "Validating", "Valid", "PendingApproval"];
    // 手机端认为不可再操作的状态
    let done = [
        "Approved",
        "Committed",
        "Applied",
        "Invalid",
        "Rejected",
        "Conflicted",
        "Expired",
        "Failed",
    ];

    let all = [
        ProposedChangeStatus::Draft,
        ProposedChangeStatus::Validating,
        ProposedChangeStatus::Valid,
        ProposedChangeStatus::Approved,
        ProposedChangeStatus::PendingApproval,
        ProposedChangeStatus::Committed,
        ProposedChangeStatus::Applied,
        ProposedChangeStatus::Invalid,
        ProposedChangeStatus::Rejected,
        ProposedChangeStatus::Conflicted,
        ProposedChangeStatus::Expired,
        ProposedChangeStatus::Failed,
    ];

    for st in all {
        let name = st.description().to_string();
        let expect_actionable = actionable.contains(&name.as_str());
        let expect_done = done.contains(&name.as_str());
        assert!(
            expect_actionable ^ expect_done,
            "状态 {} 必须被前端明确归为「可操作」或「已处理」之一，\
             否则手机端会显示错误的按钮（见 mobile/dist/js/app.js）",
            name
        );
    }
    println!("✅ 12 个状态名都被前端覆盖");
}
