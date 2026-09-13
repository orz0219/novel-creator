//! 验证整库备份（VACUUM INTO）真的能产出一份可用、可恢复的快照。
//!
//! 手机端「备份手机数据」依赖它：备份文件必须能被当作正常数据库打开，
//! 且内容与备份时一致。

use std::path::PathBuf;
use uuid::Uuid;

use sqlite_engine::engine::{BootstrapConfig, Engine};

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("novel-backup-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn test_vacuum_into_produces_usable_snapshot() {
    let dir = tmpdir("vacuum");
    let db = dir.join("novel.db");
    let backup = dir.join("backup.db");

    // 建库并写入一条数据
    let engine = Engine::build(db.to_str().unwrap(), BootstrapConfig::default())
        .await
        .expect("引擎组装失败");

    use domain::ports::ProjectRepositoryPort;
    let repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
    let created = repo.create_project("备份验证项目", Some("这条数据要能在备份里找到"), None)
        .await
        .unwrap();
    let pid = created["id"].as_str().unwrap().to_string();

    // 执行备份（与 lib.rs 里 backup_database 用的是同一条语句）
    let size_before = std::fs::metadata(&db).unwrap().len();
    sqlx::query("VACUUM INTO ?")
        .bind(backup.to_string_lossy().to_string())
        .execute(&engine.state.pool)
        .await
        .expect("VACUUM INTO 失败");

    assert!(backup.exists(), "备份文件没有生成");
    let size_after = std::fs::metadata(&backup).unwrap().len();
    println!("原库 {} bytes → 备份 {} bytes", size_before, size_after);
    assert!(size_after > 0, "备份文件是空的");

    // 目标已存在时再备份应该报错（调用方需要先删除，这是已知行为）
    let again = sqlx::query("VACUUM INTO ?")
        .bind(backup.to_string_lossy().to_string())
        .execute(&engine.state.pool)
        .await;
    assert!(again.is_err(), "目标存在时 VACUUM INTO 应报错，让调用方处理");
    println!("✅ 目标已存在时会报错（调用方需先清理同名文件）");

    drop(engine);

    // 把备份当作正常数据库打开，确认数据在
    let engine2 = Engine::build(backup.to_str().unwrap(), BootstrapConfig::default())
        .await
        .expect("备份文件无法作为数据库打开");

    let repo2 = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine2.state.pool.clone());
    let list = repo2.list_projects().await.expect("从备份读项目失败");
    println!("备份里的项目数: {}", list.len());
    assert_eq!(list.len(), 1, "备份里应有 1 个项目");
    assert_eq!(list[0]["id"].as_str().unwrap(), pid, "项目 id 应与备份前一致");
    assert_eq!(list[0]["name"].as_str().unwrap(), "备份验证项目");
    println!("✅ 备份可被正常打开，数据完整一致");

    drop(engine2);
    let _ = std::fs::remove_dir_all(&dir);
}
