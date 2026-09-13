//! 删除项目必须真的能删掉（并级联清理所有子表）。
//!
//! 这条链路之前是坏的：SQLite 的表缺 `ON DELETE CASCADE`，
//! `DELETE FROM project` 会一路撞外键，手机上点「删除项目」直接报
//! `FOREIGN KEY constraint failed`。
//!
//! 修复分两步，本测试同时守住两步：
//!   1. 新库：`001_init.sql` 的外键直接带 CASCADE
//!   2. 老库：`002_fix_cascade_fks.sql` 重建那些缺动作的表
//!
//! 所以这里**故意分两次建库**：先用只含 001 的迁移建库并写入数据，
//! 再补跑 002，最后验证删除能成功、且关联数据都被清掉。

use anyhow::Result;
use domain::ports::{EntityRepositoryPort, ProjectRepositoryPort};
use sqlite_db::application_ports::{DbEntityRepositoryPort, DbProjectRepositoryPort};
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

async fn open(tag: &str) -> Result<(Database, PathBuf)> {
    let dir = std::env::temp_dir().join(format!("novel-del-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("test.db");
    let db = Database::open(&format!("sqlite://{}?mode=rwc", db_path.display())).await?;
    Ok((db, dir))
}

/// 造一个「有内容」的项目：主世界 + 角色 + 剧情线 + 会话
async fn seed_project(db: &Database, name: &str) -> Result<Uuid> {
    let proj_repo = DbProjectRepositoryPort::new(db.pool().clone());
    let created = proj_repo.create_project(name, Some("删除测试"), None).await?;
    let pid = Uuid::parse_str(created["id"].as_str().unwrap())?;

    let wid: Uuid = sqlx::query_scalar("SELECT id FROM world WHERE project_id = $1 AND is_main = 1")
        .bind(pid)
        .fetch_one(db.pool())
        .await?;

    let ent_repo = DbEntityRepositoryPort::new(db.pool().clone());
    let ent = ent_repo.create_entity(wid, "Character", "林秋", None, None).await?;
    let eid = Uuid::parse_str(ent["id"].as_str().unwrap())?;

    // 角色扩展表（character_drive 等）也塞一行：它们在缺 CASCADE 时正是拦路的表
    sqlx::query("INSERT INTO character_drive (id, entity_id, motivation) VALUES (randomblob(16), $1, $2)")
        .bind(eid)
        .bind("想找回妹妹")
        .execute(db.pool())
        .await?;

    Ok(pid)
}

async fn count_rows(db: &Database, sql: &str, pid: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar(sql).bind(pid).fetch_one(db.pool()).await?)
}

/// 新建的库（001 已带 CASCADE）删项目应当成功并清空子表。
#[tokio::test]
async fn delete_project_cascades_on_fresh_database() -> Result<()> {
    let (db, dir) = open("fresh").await?;
    sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;

    let pid = seed_project(&db, "待删除项目").await?;
    let before = count_rows(&db, "SELECT count(*) FROM entity WHERE project_id = $1", pid).await?;
    assert!(before > 0, "应有实体");

    let proj_repo = DbProjectRepositoryPort::new(db.pool().clone());
    proj_repo.delete_project(pid).await?;

    let left = count_rows(&db, "SELECT count(*) FROM project WHERE id = $1", pid).await?;
    assert_eq!(left, 0, "项目应被删除");
    let ents = count_rows(&db, "SELECT count(*) FROM entity WHERE project_id = $1", pid).await?;
    assert_eq!(ents, 0, "实体应被级联删除");
    let worlds = count_rows(&db, "SELECT count(*) FROM world WHERE project_id = $1", pid).await?;
    assert_eq!(worlds, 0, "世界应被级联删除");

    // character_drive 没有 project_id，按 entity 关联
    let drives: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM character_drive cd WHERE NOT EXISTS (SELECT 1 FROM entity e WHERE e.id = cd.entity_id)",
    )
    .fetch_one(db.pool())
    .await?;
    assert_eq!(drives, 0, "角色扩展表不应留下孤儿行");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// 老库（只建过 001 的旧结构）在补跑 002 之后也要能删项目，且数据不丢。
#[tokio::test]
async fn cascade_migration_fixes_legacy_database() -> Result<()> {
    let (db, dir) = open("legacy").await?;

    // 1) 先只跑 001：还原「老库」的结构（外键缺 CASCADE）
    let only_001 = dir.join("only001");
    std::fs::create_dir_all(&only_001)?;
    let init = std::fs::read_to_string(
        PathBuf::from(migrations_dir()).join("001_init.sql"),
    )?;
    // 把「新 001」里的 CASCADE 去掉，模拟修复前的老结构
    let legacy_ddl = init.replace(" ON DELETE CASCADE", "");
    std::fs::write(only_001.join("001_init.sql"), &legacy_ddl)?;

    sqlite_db::seed::initialize(db.pool(), only_001.to_str().unwrap()).await?;
    let pid = seed_project(&db, "老库项目").await?;

    // 老结构下删除应当失败——这证明测试真的复现了问题
    let proj_repo = DbProjectRepositoryPort::new(db.pool().clone());
    let failed = proj_repo.delete_project(pid).await;
    assert!(
        failed.is_err(),
        "老结构下删除本应失败（外键拦截），若这里成功说明模拟没生效"
    );
    println!("老结构下删除失败（符合预期）: {}", failed.unwrap_err());

    // 2) 补跑真正的外键修复迁移
    let fix = std::fs::read_to_string(
        PathBuf::from(migrations_dir()).join("002_fix_cascade_fks.sql"),
    )?;
    let fix_dir = dir.join("fix");
    std::fs::create_dir_all(&fix_dir)?;
    std::fs::write(fix_dir.join("002_fix_cascade_fks.sql"), fix)?;
    let applied = sqlite_db::seed::initialize(db.pool(), fix_dir.to_str().unwrap()).await?;
    println!("补跑的迁移: {:?}", applied);
    assert!(
        applied.iter().any(|m| m.contains("002_fix_cascade")),
        "外键修复迁移应被执行"
    );

    // 3) 数据没丢
    let ents = count_rows(&db, "SELECT count(*) FROM entity WHERE project_id = $1", pid).await?;
    assert_eq!(ents, 1, "重建表不能丢数据");
    let drives: i64 = sqlx::query_scalar("SELECT count(*) FROM character_drive")
        .fetch_one(db.pool())
        .await?;
    assert_eq!(drives, 1, "角色扩展表的数据也不能丢");

    // 4) 现在删得掉了
    proj_repo.delete_project(pid).await?;
    let left = count_rows(&db, "SELECT count(*) FROM project WHERE id = $1", pid).await?;
    assert_eq!(left, 0, "迁移后应能删除项目");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
