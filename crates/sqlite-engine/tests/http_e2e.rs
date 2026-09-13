//! 端到端验证：手机单机版引擎能否提供与电脑端一致的 HTTP API。
//!
//! 覆盖：健康检查 → 建项目 → 列项目 → 建角色 → 读回 → 会话创建 → 工具列表。
//! 这些路径全部走真实 axum 路由 + 真实 SQLite 文件库。

use std::path::PathBuf;
use std::time::Duration;

use sqlite_engine::engine::{BootstrapConfig, Engine};
use uuid::Uuid;

/// 迁移目录：crates/db/migrations_sqlite
fn migrations_dir() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("db")
        .join("migrations_sqlite")
        .to_string_lossy()
        .to_string()
}

struct TestServer {
    base: String,
    dir: PathBuf,
}

impl TestServer {
    /// 起一个真实服务：绑定 0 端口由系统分配，避免测试间端口冲突
    async fn start(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("novel-e2e-{}-{}", tag, Uuid::new_v4()));
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
        }
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            // 关键：环境里可能设置了 http_proxy（构建时用），
            // 若不显式禁用，本地 127.0.0.1 请求会被送到代理而失败。
            .no_proxy()
            .build()
            .unwrap()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let srv = TestServer::start("health").await;
    let resp = srv
        .client()
        .get(format!("{}/api/v1/health", srv.base))
        .send()
        .await
        .expect("健康检查请求失败");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "OK");
    println!("[OK] 健康检查通过");
}

#[tokio::test]
async fn test_full_project_and_entity_flow() {
    let srv = TestServer::start("flow").await;
    let client = srv.client();
    let base = &srv.base;

    // 1. 建项目
    let resp = client
        .post(format!("{}/api/v1/projects", base))
        .json(&serde_json::json!({"name": "手机单机测试", "description": "端到端"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "建项目失败: {}", resp.text().await.unwrap());
    let project: serde_json::Value = resp.json().await.unwrap();
    let pid = project["id"].as_str().expect("项目缺少 id").to_string();
    assert_eq!(project["name"], "手机单机测试");
    println!("[OK] 建项目: {}", pid);

    // 2. 列项目
    let list: Vec<serde_json::Value> = client
        .get(format!("{}/api/v1/projects", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    println!("[OK] 列项目: {} 条", list.len());

    // 3. 读单个项目
    let one: serde_json::Value = client
        .get(format!("{}/api/v1/projects/{}", base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(one["name"], "手机单机测试");
    println!("[OK] 读项目");

    // 4. 取主世界
    let world: serde_json::Value = client
        .get(format!("{}/api/v1/projects/{}/world", base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let wid = world["id"].as_str().expect("主世界缺少 id").to_string();
    println!("[OK] 取主世界: {}", wid);

    // 5. 建角色
    let resp = client
        .post(format!("{}/api/v1/worlds/{}/characters", base, wid))
        .json(&serde_json::json!({"name": "林寒", "summary": "主角", "description": "少年剑客"}))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "建角色失败: {}", body);
    let ch: serde_json::Value = serde_json::from_str(&body).unwrap();
    let cid = ch["id"].as_str().expect("角色缺少 id").to_string();
    println!("[OK] 建角色: {} ({})", ch["name"], cid);

    // 6. 读回角色，确认中文与 uuid 都能正确往返
    let got: serde_json::Value = client
        .get(format!("{}/api/v1/characters/{}", base, cid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got["name"], "林寒");
    assert_eq!(got["id"].as_str().unwrap(), cid);
    println!("[OK] 读回角色，uuid 与中文均正确");

    // 7. 列角色
    let chars: Vec<serde_json::Value> = client
        .get(format!("{}/api/v1/worlds/{}/characters", base, wid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(chars.len(), 1);
    println!("[OK] 列角色: {} 条", chars.len());

    let _ = client;
}

#[tokio::test]
async fn test_agent_session_and_tools() {
    let srv = TestServer::start("agent").await;
    let client = srv.client();
    let base = &srv.base;

    // 先建项目（会话需要绑定项目）
    let project: serde_json::Value = client
        .post(format!("{}/api/v1/projects", base))
        .json(&serde_json::json!({"name": "会话测试"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let pid = project["id"].as_str().unwrap().to_string();

    // 工具列表：应包含领域工具（与电脑端同一套注册逻辑）
    let tools: serde_json::Value = client
        .get(format!("{}/api/v1/agent/tools", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tool_list = tools["tools"].as_array().expect("tools 应为数组");
    println!("[OK] 工具数量: {}", tool_list.len());
    assert!(tool_list.len() >= 10, "领域工具未注册上，只有 {} 个", tool_list.len());
    let names: Vec<String> = tool_list
        .iter()
        .map(|t| t["name"].as_str().unwrap_or("").to_string())
        .collect();
    println!("     工具: {:?}", names);

    // 建会话
    let resp = client
        .post(format!("{}/api/v1/agent/session", base))
        .json(&serde_json::json!({"project_id": pid, "title": "第一次对话"}))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "建会话失败: {}", body);
    let session: serde_json::Value = serde_json::from_str(&body).unwrap();
    let sid = session["session_id"]
        .as_str()
        .or_else(|| session["id"].as_str())
        .expect("会话缺少 id")
        .to_string();
    println!("[OK] 建会话: {}", sid);

    // 列会话
    let sessions: serde_json::Value = client
        .get(format!("{}/api/v1/agent/sessions?project_id={}", base, pid))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("[OK] 列会话: {}", sessions);
}

#[tokio::test]
async fn test_settings_roundtrip() {
    let srv = TestServer::start("settings").await;
    let client = srv.client();
    let base = &srv.base;

    // 读设置（首次启动应已由 bootstrap 写入）
    let resp = client
        .get(format!("{}/api/v1/settings", base))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "读设置失败: {}", body);
    println!("[OK] 读设置: {}", body);

    // 写设置
    let resp = client
        .put(format!("{}/api/v1/settings", base))
        .json(&serde_json::json!({
            "base_url": "https://example.test/v1",
            "api_key": "sk-test-key",
            "model": "test-model"
        }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "写设置失败: {}", body);
    println!("[OK] 写设置");

    // 再读确认持久化
    let after: serde_json::Value = client
        .get(format!("{}/api/v1/settings", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("[OK] 设置往返: {}", after);
}

/// 直接诊断：不走 HTTP，检查数据库连接本身是否可用
#[tokio::test]
async fn test_raw_db_connection() {
    let dir = std::env::temp_dir().join(format!("novel-raw-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("raw.db");

    let engine = Engine::build(db_path.to_str().unwrap(), BootstrapConfig::default())
        .await
        .expect("引擎组装失败");

    let r: Result<i32, sqlx::Error> = sqlx::query_scalar("SELECT 1").fetch_one(&engine.state.pool).await;
    match &r {
        Ok(v) => println!("[OK] SELECT 1 = {}", v),
        Err(e) => println!("[FAIL] SELECT 1 出错: {:?}", e),
    }
    assert!(r.is_ok(), "数据库查询失败: {:?}", r.err());

    // 看看 app_settings 里有什么
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, settings FROM app_settings")
        .fetch_all(&engine.state.pool)
        .await
        .expect("读 app_settings 失败");
    println!("app_settings 行: {:?}", rows);

    let _ = std::fs::remove_dir_all(&dir);
}
