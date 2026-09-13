//! 导出/导入的边界：本机级配置（`app_settings`）不得被带进项目文件，
//! 也不得被项目文件覆盖。
//!
//! 起因是一个真实事故：手机端把电脑端导出的项目导进来之后，手机上自己填的
//! AI 配置（模型 / 密钥）被悄悄替换成了电脑端那份，用户毫无察觉。
//! 根因是 `app_settings` 既没有 `project_id` 也没有关联列，被导出逻辑当成
//! 「全局字典表」（原意是带出 `entity_type` 这类字典）整表带走了。

use anyhow::Result;
use domain::ports::ProjectRepositoryPort;
use sqlite_db::application_ports::DbProjectRepositoryPort;
use sqlite_db::connection::Database;
use std::path::PathBuf;
use uuid::Uuid;

fn migrations_dir() -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .join("db")
        .join("migrations_sqlite")
        .to_string_lossy()
        .to_string()
}

async fn setup(tag: &str) -> Result<(Database, PathBuf)> {
    let dir = std::env::temp_dir().join(format!("novel-iso-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    let db = Database::open(&url).await?;
    sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;
    Ok((db, dir))
}

/// 写一份本机 AI 配置，返回写入内容
async fn write_local_settings(db: &Database, model: &str) -> Result<()> {
    let settings = serde_json::json!({
        "aiBaseUrl": "https://local.example/v1",
        "aiApiKey": "local-secret",
        "defaultModel": model,
    });
    sqlx::query(
        "INSERT INTO app_settings (id, settings) VALUES ('default', $1) \
         ON CONFLICT (id) DO UPDATE SET settings = EXCLUDED.settings",
    )
    .bind(settings.to_string())
    .execute(db.pool())
    .await?;
    Ok(())
}

async fn read_local_settings(db: &Database) -> Result<serde_json::Value> {
    let raw: String =
        sqlx::query_scalar("SELECT settings FROM app_settings WHERE id = 'default'")
            .fetch_one(db.pool())
            .await?;
    Ok(serde_json::from_str(&raw)?)
}

/// 导出的项目文件里不能包含 app_settings。
#[tokio::test]
async fn export_excludes_local_settings_table() -> Result<()> {
    let (db, dir) = setup("export").await?;
    write_local_settings(&db, "local-model").await?;

    let repo = DbProjectRepositoryPort::new(db.pool().clone());
    let created = repo
        .create_project("导出隔离测试", Some("验证 app_settings 不外泄"), None)
        .await?;
    let pid = created["id"].as_str().unwrap().to_string();

    let exp = sqlite_db::export::export_project(db.pool(), &pid).await?;

    assert!(
        !exp.tables.contains_key("app_settings"),
        "app_settings 是设备级配置，绝不能出现在项目导出文件里（当前表: {:?}）",
        exp.tables.keys().collect::<Vec<_>>()
    );
    assert!(!exp.insert_order.contains(&"app_settings".to_string()));

    // 项目数据本身仍然要完整导出
    assert!(exp.tables.contains_key("project"));
    println!("导出的表: {:?}", {
        let mut v: Vec<&String> = exp.tables.keys().collect();
        v.sort();
        v
    });

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// 导入一份「含 app_settings」的旧格式文件时，本机配置必须保持不变。
#[tokio::test]
async fn import_does_not_overwrite_local_settings() -> Result<()> {
    let (db, dir) = setup("import").await?;
    write_local_settings(&db, "local-model").await?;

    let repo = DbProjectRepositoryPort::new(db.pool().clone());
    let created = repo
        .create_project("导入隔离测试", None, None)
        .await?;
    let pid = created["id"].as_str().unwrap().to_string();

    // 造一份导出数据，并**人为塞进** app_settings（模拟旧版本导出的文件）
    let exp = sqlite_db::export::export_project(db.pool(), &pid).await?;
    let mut payload = serde_json::to_value(&exp)?;
    payload["tables"]["app_settings"] = serde_json::json!({
        "columns": ["id", "settings"],
        "column_types": {},
        "rows": [{
            "id": "default",
            "settings": serde_json::json!({
                "aiBaseUrl": "https://from-file.example/v1",
                "aiApiKey": "file-secret",
                "defaultModel": "file-model",
            }).to_string(),
        }],
    });
    payload["insert_order"]
        .as_array_mut()
        .expect("insert_order 应为数组")
        .push(serde_json::json!("app_settings"));

    let summary = sqlite_db::import::import_from_json(db.pool(), &payload.to_string()).await?;
    println!(
        "导入摘要: 表 {} 行 {} 跳过 {}",
        summary.tables_written, summary.rows_written, summary.skipped_rows
    );

    let after = read_local_settings(&db).await?;
    println!("导入后的本机设置: {}", after);
    assert_eq!(
        after["defaultModel"], "local-model",
        "本机 AI 配置不允许被导入文件覆盖"
    );
    assert_eq!(after["aiApiKey"], "local-secret", "本机密钥不允许被导入文件覆盖");
    assert_eq!(after["aiBaseUrl"], "https://local.example/v1");

    // 项目数据本身必须已经导入
    let name: String = sqlx::query_scalar("SELECT name FROM project WHERE id = $1")
        .bind(Uuid::parse_str(&pid)?)
        .fetch_one(db.pool())
        .await?;
    assert_eq!(name, "导入隔离测试");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
