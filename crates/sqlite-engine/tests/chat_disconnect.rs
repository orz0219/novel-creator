//! 验证：客户端中途断开时，助手回复仍然会被写入数据库。
//!
//! 背景：原实现把「生成 + 落库」写在给客户端的 SSE 流里，客户端一断流
//! 就被销毁，导致生成中止、回复永不落库（App 切后台就是这个场景）。
//! 修复后生成跑在独立后台任务里，断开只影响转发。
//!
//! 需要可用的 AI 网关配置（LIVE_API_KEY），未配置时跳过。

use std::path::PathBuf;
use std::time::Duration;

use sqlite_engine::engine::{BootstrapConfig, Engine};
use uuid::Uuid;

#[tokio::test]
async fn test_assistant_reply_survives_client_disconnect() {
    let api_key = match std::env::var("LIVE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            eprintln!("跳过：未设置 LIVE_API_KEY（需要真实网关才能验证）");
            return;
        }
    };
    let base_url = std::env::var("LIVE_BASE_URL")
        .unwrap_or_else(|_| "https://opencode.ai/zen/go/v1".to_string());
    let model = std::env::var("LIVE_MODEL").unwrap_or_else(|_| "mimo-v2.5".to_string());

    let dir = std::env::temp_dir().join(format!("novel-disc-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("t.db");

    let engine = Engine::build(
        db_path.to_str().unwrap(),
        BootstrapConfig { api_key, base_url, model, ..Default::default() },
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

    // 建项目 + 会话
    let project: serde_json::Value = client
        .post(format!("{}/api/v1/projects", base))
        .json(&serde_json::json!({"name": "断连测试"}))
        .send().await.unwrap().json().await.unwrap();
    let pid = project["id"].as_str().unwrap().to_string();
    let session: serde_json::Value = client
        .post(format!("{}/api/v1/agent/session", base))
        .json(&serde_json::json!({"project_id": pid}))
        .send().await.unwrap().json().await.unwrap();
    let sid = session["session_id"].as_str().unwrap().to_string();

    // 发消息后 2 秒就放弃响应（模拟客户端断开）
    {
        let req = client
            .post(format!("{}/api/v1/agent/chat", base))
            .json(&serde_json::json!({
                "session_id": sid,
                "message": "请用一句话说明你对这个项目的理解"
            }))
            .send();
        let _ = tokio::time::timeout(Duration::from_secs(2), req).await;
        println!("已在 2 秒后主动放弃响应（模拟切后台断连）");
    }

    // 等后台任务把回复写完
    tokio::time::sleep(Duration::from_secs(45)).await;

    let detail: serde_json::Value = client
        .get(format!("{}/api/v1/agent/session/{}", base, sid))
        .send().await.unwrap().json().await.unwrap();
    let msgs = detail["messages"].as_array().expect("messages 应为数组");
    println!("断开后数据库里的消息数: {}", msgs.len());
    for m in msgs {
        let role = m["role"].as_str().unwrap_or("?");
        let c = m["content"].as_str().unwrap_or("");
        println!("  [{}] {} 字 | {}", role, c.chars().count(),
            c.chars().take(60).collect::<String>());
    }

    let has_assistant = msgs.iter().any(|m| {
        m["role"].as_str() == Some("assistant")
            && m["content"].as_str().map(|c| !c.trim().is_empty()).unwrap_or(false)
    });
    assert!(
        has_assistant,
        "客户端断开后助手回复未被保存（修复未生效）"
    );
    println!("✅ 客户端断开后，助手回复仍然完整落库");

    let _ = std::fs::remove_dir_all(&dir);
}
