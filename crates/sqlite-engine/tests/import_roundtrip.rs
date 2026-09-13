//! 导出 → 导入 往返验证。
//!
//! 用真实 PostgreSQL 数据导出成 JSON，再导入到手机端同构的 SQLite，
//! 然后核对关键表的行数与字段，确保迁移不丢数据、不串列、类型正确。

use std::path::PathBuf;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("novel-rt-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn test_export_import_roundtrip() {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("跳过：未设置 DATABASE_URL");
            return;
        }
    };

    // 1) 从 PG 导出真实项目
    let pg = PgPoolOptions::new().max_connections(4).connect(&url).await.unwrap();
    let pid: String = sqlx::query_scalar(
        "SELECT p.id::text FROM project p \
         LEFT JOIN entity e ON e.project_id = p.id \
         GROUP BY p.id ORDER BY count(e.id) DESC, p.updated_at DESC LIMIT 1",
    )
    .fetch_one(&pg)
    .await
    .unwrap();
    let export = db::export::export_project(&pg, &pid).await.expect("导出失败");
    let json = serde_json::to_string(&export).unwrap();
    println!("导出: {} 表 / {} 行 / {:.2} MB", export.tables.len(),
        export.tables.values().map(|t| t.rows.len()).sum::<usize>(),
        json.len() as f64 / 1024.0 / 1024.0);

    // 2) 起手机端同构引擎（SQLite）
    let dir = tmpdir("rt");
    let db_path = dir.join("phone.db");
    let engine = sqlite_engine::engine::Engine::build(
        db_path.to_str().unwrap(),
        Default::default(),
    ).await.expect("引擎组装失败");

    let app = sqlite_engine::api::router(engine.state.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(120)).build().unwrap();

    // 3) 导入
    let resp = client
        .post(format!("{}/api/v1/import", base))
        .header("Content-Type", "application/json")
        .body(json.clone())
        .send().await.expect("导入请求失败");
    let status = resp.status();
    let body = resp.text().await.unwrap();
    println!("导入响应 {}: {}", status, &body[..body.len().min(400)]);
    assert!(status.is_success(), "导入失败");
    let summary: serde_json::Value = serde_json::from_str(&body).unwrap();
    println!("导入摘要: {}", summary);

    // 4) 核对
    let proj: serde_json::Value = client
        .get(format!("{}/api/v1/projects/{}", base, pid))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(proj["id"].as_str().unwrap(), pid, "项目 id 应保持一致");
    let src_name = export.tables["project"].rows[0]["name"].as_str().unwrap();
    assert_eq!(proj["name"].as_str().unwrap(), src_name, "项目名应一致");
    println!("✅ 项目: {}", proj["name"]);

    // 角色数量对比
    let src_chars = export.tables.get("entity")
        .map(|t| t.rows.iter().filter(|r| {
            r.get("status").and_then(|v| v.as_str()).map(|s| s != "Deleted").unwrap_or(true)
        }).count())
        .unwrap_or(0);
    let world: serde_json::Value = client
        .get(format!("{}/api/v1/projects/{}/world", base, pid))
        .send().await.unwrap().json().await.unwrap();
    let wid = world["id"].as_str().unwrap();
    let chars: Vec<serde_json::Value> = client
        .get(format!("{}/api/v1/worlds/{}/characters", base, wid))
        .send().await.unwrap().json().await.unwrap();
    println!("源实体数: {}, 导入后角色数: {}", src_chars, chars.len());
    if !chars.is_empty() {
        println!("首个角色: {}", chars[0]["name"]);
        // 详情接口会解码 uuid，能走通说明类型正确
        let cid = chars[0]["id"].as_str().unwrap();
        let detail: serde_json::Value = client
            .get(format!("{}/api/v1/characters/{}", base, cid))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(detail["id"].as_str().unwrap(), cid, "角色 id 读回应一致");
        println!("✅ 角色详情读回正常: {}", detail["name"]);
    }

    // 会话与消息（最容易因类型出错的部分）
    let sessions: Vec<serde_json::Value> = client
        .get(format!("{}/api/v1/agent/sessions?project_id={}", base, pid))
        .send().await.unwrap().json().await.unwrap();
    let src_msgs = export.tables.get("agent_messages").map(|t| t.rows.len()).unwrap_or(0);
    let mut got_msgs = 0usize;
    for s in &sessions {
        let sid = s["id"].as_str().unwrap();
        let detail: serde_json::Value = client
            .get(format!("{}/api/v1/agent/session/{}", base, sid))
            .send().await.unwrap().json().await.unwrap();
        got_msgs += detail["messages"].as_array().map(|a| a.len()).unwrap_or(0);
    }
    println!("源消息数: {}, 导入后会话数: {}, 消息数: {}", src_msgs, sessions.len(), got_msgs);
    assert_eq!(got_msgs, src_msgs, "消息数量应完全一致");

    // 剧情线
    let storylines: Vec<serde_json::Value> = client
        .get(format!("{}/api/v1/projects/{}/storylines", base, pid))
        .send().await.unwrap().json().await.unwrap();
    let src_sl = export.tables.get("storyline").map(|t| t.rows.len()).unwrap_or(0);
    println!("源剧情线: {}, 导入后: {}", src_sl, storylines.len());
    assert_eq!(storylines.len(), src_sl, "剧情线数量应一致");

    println!("✅ 导出→导入 往返验证通过");
    let _ = std::fs::remove_dir_all(&dir);
}
