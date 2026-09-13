//! 数据库结构代号（schema generation）机制验证。
//!
//! 背景：迁移按文件名记录，改了内容但文件名不变会被判定为「已应用」而跳过，
//! 旧库会停在旧结构上。因此引入代号比对，不一致就重建。
//!
//! 三个场景都必须验证：全新库、代号一致（不重建）、代号不一致（重建）。

use std::path::PathBuf;
use uuid::Uuid;

use sqlite_engine::engine::{BootstrapConfig, Engine};

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("novel-schema-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 全新库：应正常建表并写入当前代号
#[tokio::test]
async fn test_fresh_database_gets_current_generation() {
    let dir = tmpdir("fresh");
    let db = dir.join("t.db");

    let engine = Engine::build(db.to_str().unwrap(), BootstrapConfig::default())
        .await
        .expect("全新库组装失败");

    let gen: i64 = sqlx::query_scalar("SELECT generation FROM schema_version ORDER BY rowid DESC LIMIT 1")
        .fetch_one(&engine.state.pool)
        .await
        .expect("未写入 schema 代号");
    assert_eq!(gen, sqlite_db::embedded::SCHEMA_GENERATION as i64);
    println!("✅ 全新库代号: {}", gen);

    // 不应有「已重建」标记
    assert!(!dir.join("schema_rebuilt").exists(), "全新库不应留下重建标记");

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 代号一致：重复启动不应重建、数据应保留
#[tokio::test]
async fn test_matching_generation_keeps_data() {
    let dir = tmpdir("match");
    let db = dir.join("t.db");
    let path = db.to_str().unwrap().to_string();

    let pid;
    {
        let engine = Engine::build(&path, BootstrapConfig::default()).await.unwrap();
        let created = sqlite_db::application_ports::DbProjectRepositoryPort::new(
            engine.state.pool.clone(),
        );
        use domain::ports::ProjectRepositoryPort;
        let p = created.create_project("代号一致测试", None, None).await.unwrap();
        pid = p["id"].as_str().unwrap().to_string();
        drop(engine);
    }

    // 再次启动：应保留数据
    let engine = Engine::build(&path, BootstrapConfig::default()).await.unwrap();
    let repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
    use domain::ports::ProjectRepositoryPort;
    let list = repo.list_projects().await.unwrap();
    assert_eq!(list.len(), 1, "代号一致时不应重建，数据应保留");
    assert_eq!(list[0]["id"].as_str().unwrap(), pid);
    println!("✅ 代号一致：数据保留（{} 个项目）", list.len());

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 代号不一致：应重建（数据清空）并留下标记
#[tokio::test]
async fn test_mismatched_generation_rebuilds() {
    let dir = tmpdir("mismatch");
    let db = dir.join("t.db");
    let path = db.to_str().unwrap().to_string();

    // 先建库并写入数据
    {
        let engine = Engine::build(&path, BootstrapConfig::default()).await.unwrap();
        let repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
        use domain::ports::ProjectRepositoryPort;
        repo.create_project("将被清掉的项目", None, None).await.unwrap();
        assert_eq!(repo.list_projects().await.unwrap().len(), 1);

        // 手动把代号改成一个过期的值，模拟「代码的结构变了、库里还是旧的」
        sqlx::query("UPDATE schema_version SET generation = 0")
            .execute(&engine.state.pool)
            .await
            .unwrap();
        drop(engine);
    }

    // 再启动：应触发重建
    let engine = Engine::build(&path, BootstrapConfig::default()).await.unwrap();
    let repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
    use domain::ports::ProjectRepositoryPort;
    let list = repo.list_projects().await.unwrap();
    assert!(list.is_empty(), "代号不一致时应重建，旧数据应被清空，实际还有 {} 条", list.len());
    println!("✅ 代号不一致：已重建，旧数据已清空");

    let gen: i64 = sqlx::query_scalar("SELECT generation FROM schema_version ORDER BY rowid DESC LIMIT 1")
        .fetch_one(&engine.state.pool)
        .await
        .unwrap();
    assert_eq!(gen, sqlite_db::embedded::SCHEMA_GENERATION as i64);

    // 应留下标记，供前端提示用户
    let marker = dir.join("schema_rebuilt");
    assert!(marker.exists(), "重建后应写出 schema_rebuilt 标记");
    println!("✅ 重建标记已写入: {}", std::fs::read_to_string(&marker).unwrap());

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 老数据库没有 schema_version 表：应被识别为需要重建
#[tokio::test]
async fn test_legacy_database_without_version_table_rebuilds() {
    let dir = tmpdir("legacy");
    let db = dir.join("t.db");
    let path = db.to_str().unwrap().to_string();

    {
        let engine = Engine::build(&path, BootstrapConfig::default()).await.unwrap();
        let repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
        use domain::ports::ProjectRepositoryPort;
        repo.create_project("老库数据", None, None).await.unwrap();
        // 模拟引入本机制之前的老库：把版本表删掉
        sqlx::query("DROP TABLE schema_version").execute(&engine.state.pool).await.unwrap();
        drop(engine);
    }

    let engine = Engine::build(&path, BootstrapConfig::default()).await.unwrap();
    let repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
    use domain::ports::ProjectRepositoryPort;
    assert!(repo.list_projects().await.unwrap().is_empty(), "老库应被重建");
    println!("✅ 老库（无版本表）已识别并重建");

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}
