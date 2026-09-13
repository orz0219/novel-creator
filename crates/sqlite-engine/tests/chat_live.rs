//! 真实对话链路验证（需要可用的 AI 网关配置）。
//!
//! 用环境变量提供配置，缺失时跳过（不静默失败，会明确打印跳过原因）：
//!   LIVE_API_KEY / LIVE_BASE_URL / LIVE_MODEL
//!
//! 验证内容：建项目 → 建会话 → 走 SSE 聊天端点 → 收到 token 与 done 事件。

use std::path::PathBuf;
use std::time::Duration;

use sqlite_engine::engine::{BootstrapConfig, Engine};
use uuid::Uuid;

#[tokio::test]
async fn test_live_chat_stream() {
    let api_key = match std::env::var("LIVE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            eprintln!("跳过：未设置 LIVE_API_KEY（这是预期内的跳过，不是通过）");
            return;
        }
    };
    let base_url = std::env::var("LIVE_BASE_URL")
        .unwrap_or_else(|_| "https://opencode.ai/zen/go/v1".to_string());
    let model = std::env::var("LIVE_MODEL").unwrap_or_else(|_| "mimo-v2.5".to_string());

    let dir = std::env::temp_dir().join(format!("novel-live-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("live.db");

    let engine = Engine::build(
        db_path.to_str().unwrap(),
        BootstrapConfig { api_key, base_url: base_url.clone(), model: model.clone(), ..Default::default() },
    )
    .await
    .expect("引擎组装失败");

    let app = sqlite_engine::api::router(engine.state.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .no_proxy()
        .build()
        .unwrap();
    let base = format!("http://127.0.0.1:{}", port);

    // 建项目
    let project: serde_json::Value = client
        .post(format!("{}/api/v1/projects", base))
        .json(&serde_json::json!({"name": "真机对话测试"}))
        .send().await.unwrap().json().await.unwrap();
    let pid = project["id"].as_str().unwrap().to_string();

    // 建会话
    let session: serde_json::Value = client
        .post(format!("{}/api/v1/agent/session", base))
        .json(&serde_json::json!({"project_id": pid}))
        .send().await.unwrap().json().await.unwrap();
    let sid = session["session_id"].as_str().unwrap().to_string();
    println!("会话: {}", sid);

    // 发消息，读 SSE 流
    let resp = client
        .post(format!("{}/api/v1/agent/chat", base))
        .json(&serde_json::json!({"session_id": sid, "message": "回复两个字：收到"}))
        .send().await.expect("聊天请求失败");
    println!("HTTP 状态: {}", resp.status());
    assert!(resp.status().is_success(), "聊天端点返回非 2xx");

    let text = resp.text().await.unwrap();
    println!("SSE 原始流（前 600 字符）:\n{}", &text[..text.len().min(600)]);

    let mut tokens = String::new();
    for block in text.split("\n\n") {
        let mut ev = String::new();
        let mut data = String::new();
        for line in block.split('\n') {
            if let Some(v) = line.strip_prefix("event:") { ev = v.trim().to_string() }
            else if let Some(v) = line.strip_prefix("data:") { data.push_str(v.trim_start()) }
        }
        if ev == "token" { tokens.push_str(&data) }
        if ev == "error" { panic!("引擎报错事件: {}", data) }
    }

    println!("收到 token 拼接结果: {:?}", tokens);
    assert!(!tokens.is_empty(), "没有收到任何 token");
    assert!(text.contains("event: done"), "没有收到 done 事件");

    let _ = std::fs::remove_dir_all(&dir);
    println!("✅ 完整对话链路打通");
}
