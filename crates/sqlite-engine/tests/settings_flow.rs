//! 验证「设置页 → 引擎」这条链路真的通。
//!
//! 背景：手机端设置页最初把 AI 字段存成 `base_url` / `api_key` / `model`，
//! 而引擎读的是电脑端用的 `aiBaseUrl` / `aiApiKey` / `defaultModel`，
//! 于是出现「设置页提示保存成功，但引擎仍用旧配置」——保存后不生效。
//! 这里用真实的 HTTP 接口守住这个契约，避免再退回去。

use std::path::PathBuf;
use uuid::Uuid;

use sqlite_engine::engine::{BootstrapConfig, Engine};

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("novel-set-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 只组装引擎（不监听端口）。
async fn build(dir: &PathBuf) -> Engine {
    let db = dir.join("t.db");
    Engine::build(db.to_str().unwrap(), BootstrapConfig {
        api_key: "boot-key".to_string(),
        base_url: "https://boot.example/v1".to_string(),
        model: "boot-model".to_string(),
        max_output_tokens: 12_345,
    })
    .await
    .expect("引擎组装失败")
}

/// 起一个真监听端口的引擎（首次启动会用上面的 bootstrap 配置写入设置）。
async fn serve(dir: &PathBuf, port: u16) -> Engine {
    let engine = build(dir).await;

    let e = engine.clone();
    tokio::spawn(async move {
        if let Err(err) = e.serve(port).await {
            eprintln!("引擎提前退出: {}", err);
        }
    });
    // 等端口就绪
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    for _ in 0..50 {
        if client
            .get(format!("http://127.0.0.1:{}/api/v1/health", port))
            .send()
            .await
            .is_ok()
        {
            return engine;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("引擎在 5 秒内未就绪");
}

/// 全新库：bootstrap 写入的必须是电脑端字段名，且能被引擎读到。
#[tokio::test]
async fn bootstrap_writes_desktop_field_names() {
    let dir = tmpdir("boot");
    let port = 19081;
    let engine = serve(&dir, port).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let settings: serde_json::Value = client
        .get(format!("http://127.0.0.1:{}/api/v1/settings", port))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    println!("bootstrap 后的设置: {}", settings);
    assert_eq!(settings["aiBaseUrl"], "https://boot.example/v1");
    assert_eq!(settings["aiApiKey"], "boot-key");
    assert_eq!(settings["defaultModel"], "boot-model");
    assert_eq!(settings["maxOutputTokens"], 12_345);

    // 引擎实际取到的配置也应该是这些值（证明字段名对得上）
    let ready: serde_json::Value = client
        .get(format!("http://127.0.0.1:{}/api/v1/engine/ready", port))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("engine/ready: {}", ready);
    assert_eq!(ready["provider_configured"], true);
    assert_eq!(ready["model"], "boot-model");
    assert_eq!(ready["base_url"], "https://boot.example/v1");

    // config 是「当前生效值」，手机端设置页靠它把内置密钥显示出来
    // （密钥不在库里，只有这里能拿到）
    assert_eq!(ready["config"]["aiApiKey"], "boot-key");
    assert_eq!(ready["config"]["aiBaseUrl"], "https://boot.example/v1");
    assert_eq!(ready["config"]["defaultModel"], "boot-model");
    assert_eq!(ready["config"]["maxOutputTokens"], 12_345);
    assert_eq!(
        ready["config"]["contextLimit"],
        domain::model_catalog::DEFAULT_CONTEXT_LIMIT
    );

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 设置页保存后必须立即对引擎生效（这是之前失效的那条链路）。
#[tokio::test]
async fn saved_settings_take_effect_immediately() {
    let dir = tmpdir("save");
    let port = 19082;
    let engine = serve(&dir, port).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    // 模拟手机设置页点「保存」时提交的内容
    let saved: serde_json::Value = client
        .put(format!("http://127.0.0.1:{}/api/v1/settings", port))
        .json(&serde_json::json!({
            "aiBaseUrl": "https://opencode.ai/zen/go/v1",
            "aiApiKey": "sk-user-typed",
            "defaultModel": "deepseek-flash",
            "contextLimits": { "deepseek-flash": 300_000 },
            "maxOutputTokens": 30_000,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(saved["defaultModel"], "deepseek-flash");

    let ready: serde_json::Value = client
        .get(format!("http://127.0.0.1:{}/api/v1/engine/ready", port))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("保存后 engine/ready: {}", ready);
    assert_eq!(ready["model"], "deepseek-flash", "保存的模型必须立即生效");
    assert_eq!(ready["base_url"], "https://opencode.ai/zen/go/v1");
    assert_eq!(ready["provider_configured"], true);

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 早期手机端写下的 snake_case 字段必须被迁移，而不是被静默丢弃。
#[tokio::test]
async fn legacy_snake_case_fields_are_migrated() {
    let dir = tmpdir("legacy");
    let port = 19083;

    // 先建库，再把设置改写成早期的 snake_case 形态（不监听端口，避免占住 19083）
    {
        let engine = build(&dir).await;
        sqlx::query("UPDATE app_settings SET settings = $1 WHERE id = 'default'")
            .bind(
                serde_json::json!({
                    "base_url": "https://legacy.example/v1",
                    "api_key": "legacy-key",
                    "model": "legacy-model",
                    "fontSize": 14,
                })
                .to_string(),
            )
            .execute(&engine.state.pool)
            .await
            .unwrap();
        drop(engine);
    }

    // 重新启动引擎（同一 DB）→ 迁移应在 bootstrap 时发生
    let engine = serve(&dir, port).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let settings: serde_json::Value = client
        .get(format!("http://127.0.0.1:{}/api/v1/settings", port))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("迁移后的设置: {}", settings);

    assert_eq!(settings["aiBaseUrl"], "https://legacy.example/v1");
    assert_eq!(settings["aiApiKey"], "legacy-key");
    assert_eq!(settings["defaultModel"], "legacy-model");
    // 旧字段应被移除，避免两份配置并存造成困惑
    assert!(settings.get("base_url").is_none(), "旧字段 base_url 应已移除");
    assert!(settings.get("api_key").is_none(), "旧字段 api_key 应已移除");
    assert!(settings.get("model").is_none(), "旧字段 model 应已移除");
    // 无关字段必须原样保留
    assert_eq!(settings["fontSize"], 14, "与 AI 无关的字段不能被丢掉");

    let ready: serde_json::Value = client
        .get(format!("http://127.0.0.1:{}/api/v1/engine/ready", port))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ready["base_url"], "https://legacy.example/v1");
    assert_eq!(ready["model"], "legacy-model");

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}
