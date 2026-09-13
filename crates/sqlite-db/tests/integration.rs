//! sqlite-db 集成测试：验证迁移执行与端口真实读写。
//!
//! 这些测试用真实文件库（非内存库：连接池多连接会各自开独立内存库），
//! 目的是验证「自动转换生成的持久层」在运行时确实可用，而不只是能编译。

use anyhow::Result;
use domain::ports::{EntityRepositoryPort, ProjectRepositoryPort};
use sqlite_db::application_ports::{DbEntityRepositoryPort, DbProjectRepositoryPort};
use sqlite_db::connection::Database;
use std::path::PathBuf;
use uuid::Uuid;

/// 迁移目录：crates/db/migrations_sqlite
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

/// 建一个临时库并跑迁移
async fn setup(tag: &str) -> Result<(Database, PathBuf)> {
    let dir = std::env::temp_dir().join(format!("novel-sqlite-test-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let db = Database::open(&url).await?;
    let applied = sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;
    println!("应用迁移: {:?}", applied);
    Ok((db, dir))
}

async fn teardown(dir: PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_migrations_create_all_tables() -> Result<()> {
    let (db, dir) = setup("migrate").await?;

    let tables = sqlite_db::schema::list_tables(db.pool()).await?;
    println!("表数量: {}", tables.len());
    assert!(tables.len() >= 100, "表数量异常: {}", tables.len());

    // 关键表必须存在
    for t in [
        "project",
        "world",
        "entity",
        "entity_type",
        "agent_sessions",
        "agent_messages",
        "agent_prompts",
        "app_settings",
        "generation_task",
        "narrative_node",
        "proposed_change",
        "novel_state_snapshot",
    ] {
        assert!(tables.iter().any(|x| x == t), "缺少表: {}", t);
    }

    // 迁移幂等：再跑一次不应报错也不应重复应用
    let again = sqlite_db::migration::run_migrations(db.pool(), &migrations_dir()).await?;
    assert!(again.is_empty(), "迁移不幂等，重复应用了: {:?}", again);

    // schema 校验
    let missing = sqlite_db::schema::validate_schema(db.pool()).await?;
    println!("缺失的核心表: {:?}", missing);

    teardown(dir).await;
    Ok(())
}

#[tokio::test]
async fn test_project_crud() -> Result<()> {
    let (db, dir) = setup("project").await?;
    let repo = DbProjectRepositoryPort::new(db.pool().clone());

    // 建
    let created = repo
        .create_project("测试小说", Some("一句话简介"), Some("zh"))
        .await?;
    println!("创建项目: {}", created);
    let pid = Uuid::parse_str(created["id"].as_str().unwrap())?;
    assert_eq!(created["name"], "测试小说");

    // 读
    let got = repo.get_project(pid).await?;
    assert!(got.is_some(), "刚创建的项目读不到");
    assert_eq!(got.unwrap()["name"], "测试小说");

    // 列
    let all = repo.list_projects().await?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0]["name"], "测试小说");

    // 改
    let updated = repo
        .update_project(pid, Some("改名后"), None, Some("Active"), Some("前提"))
        .await?;
    assert_eq!(updated["name"], "改名后");
    assert_eq!(updated["status"], "Active");

    // 删（级联删除 world）
    repo.delete_project(pid).await?;
    assert!(repo.get_project(pid).await?.is_none(), "删除后仍能读到");

    teardown(dir).await;
    Ok(())
}

#[tokio::test]
async fn test_entity_and_world() -> Result<()> {
    let (db, dir) = setup("entity").await?;
    let project_repo = DbProjectRepositoryPort::new(db.pool().clone());
    let entity_repo = DbEntityRepositoryPort::new(db.pool().clone());

    let created = project_repo.create_project("实体测试", None, None).await?;
    let pid = Uuid::parse_str(created["id"].as_str().unwrap())?;

    // create_project 会自动建一个主世界，直接查出来
    let wid: Uuid = sqlx::query_scalar("SELECT id FROM world WHERE project_id = $1 AND is_main = 1")
        .bind(pid)
        .fetch_one(db.pool())
        .await
        .expect("主世界未自动创建");

    // 建角色
    let ch = entity_repo
        .create_entity(wid, "Character", "林寒", Some("主角"), Some("少年剑客"))
        .await?;
    println!("创建角色: {}", ch);
    let cid = Uuid::parse_str(ch["id"].as_str().unwrap())?;
    assert_eq!(ch["name"], "林寒");

    // 读回
    let got = entity_repo.get_entity(cid).await?;
    assert!(got.is_some());
    assert_eq!(got.unwrap()["name"], "林寒");

    // 列表（含类型过滤，验证 LOWER(...) 大小写不敏感分支）
    let list = entity_repo.list_entities(wid, Some("character")).await?;
    assert_eq!(list.len(), 1, "小写类型名过滤应命中");
    let list_upper = entity_repo.list_entities(wid, Some("Character")).await?;
    assert_eq!(list_upper.len(), 1, "首字母大写类型名过滤应命中");

    // 再建一个地点，测多次读写与中文
    let loc = entity_repo
        .create_entity(wid, "Location", "青云山", None, None)
        .await?;
    assert_eq!(loc["name"], "青云山");
    let all = entity_repo.list_entities(wid, None).await?;
    assert_eq!(all.len(), 2);

    teardown(dir).await;
    Ok(())
}

#[tokio::test]
async fn test_data_persists_across_reopen() -> Result<()> {
    let dir = std::env::temp_dir().join(format!("novel-sqlite-persist-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("persist.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let pid;
    {
        let db = Database::open(&url).await?;
        sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;
        let repo = DbProjectRepositoryPort::new(db.pool().clone());
        let created = repo.create_project("持久化测试", None, None).await?;
        pid = created["id"].as_str().unwrap().to_string();
        db.close().await;
    }

    // 重新打开同一个文件库
    {
        let db = Database::open(&url).await?;
        let repo = DbProjectRepositoryPort::new(db.pool().clone());
        let all = repo.list_projects().await?;
        assert_eq!(all.len(), 1, "重开后数据丢失");
        assert_eq!(all[0]["id"].as_str().unwrap(), pid);
        // 迁移应识别已应用，seed 应幂等
        let again = sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;
        assert!(again.is_empty());
        let types: Vec<String> =
            sqlx::query_scalar("SELECT name FROM entity_type ORDER BY name")
                .fetch_all(db.pool())
                .await?;
        assert_eq!(types.len(), 7, "entity_type 应恰好 7 条，实际 {}", types.len());
        db.close().await;
    }

    teardown(dir).await;
    Ok(())
}

/// 回归测试：数据库默认生成的 uuid 必须能被 Rust 侧 Uuid 正确读回。
///
/// 这个路径曾经漏测，导致手机端「发消息」直接失败：
///   - 省略默认值 → NOT NULL constraint failed: agent_messages.id
///   - 默认值产出 uuid 文本 → invalid length: expected 16 bytes, found 36
/// 这里把「依赖数据库默认 id」的写入 + 读回固定成测试。
#[tokio::test]
async fn test_db_generated_uuid_defaults() -> Result<()> {
    use domain::agent_store::{AgentSession, ChatMessage, SessionStore};
    use sqlite_db::repos::session_repo::SessionRepo;

    let (db, dir) = setup("defaults").await?;
    let repo = DbProjectRepositoryPort::new(db.pool().clone());
    let created = repo.create_project("默认值测试", None, None).await?;
    let pid = Uuid::parse_str(created["id"].as_str().unwrap())?;

    // 会话与消息表都用数据库默认值生成 id
    let session_repo = SessionRepo::new(db.pool().clone());
    let mut session = AgentSession::new(pid);
    session.messages.push(ChatMessage {
        role: "user".into(),
        content: "你好".into(),
        created_at: chrono::Utc::now(),
    });
    session_repo.create(session.clone()).await?;

    // 读回：这一步会解码 agent_messages.id，类型不符就会报错
    let got = session_repo
        .get(session.id)
        .await?
        .expect("刚创建的会话读不到");
    assert_eq!(got.messages.len(), 1);
    assert_eq!(got.messages[0].content, "你好");
    println!("✅ 数据库默认 uuid 生成 + 读回正常");

    // 再走一次「清空重写」路径（update 会把消息删掉再插入）
    let mut s2 = got.clone();
    s2.messages.push(ChatMessage {
        role: "assistant".into(),
        content: "收到".into(),
        created_at: chrono::Utc::now(),
    });
    session_repo.update(s2).await?;
    let got2 = session_repo.get(session.id).await?.unwrap();
    assert_eq!(got2.messages.len(), 2);
    println!("✅ 消息全量重写正常");

    drop(db);
    teardown(dir).await;
    Ok(())
}
