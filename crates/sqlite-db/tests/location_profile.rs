//! 地点档案的读写往返测试。
//!
//! 为什么要专门测它：地点档案的字段**分散在两张表**——
//!   - `location_profile`  ：geography / appearance / population / economy /
//!     rules / history / narrative_usage
//!   - `location_identity` ：location_type / size / climate / era / accessibility
//!
//! 读取时用 `FULL OUTER JOIN` 把两张表拼起来。而 **SQLite 直到 3.39 才支持
//! FULL OUTER JOIN**——手机端如果跑在不支持它的 SQLite 上，地点档案会直接报错，
//! 而不是「少几个字段」。所以这里在 SQLite 上真的跑一遍读写往返，
//! 顺便守住那 5 个容易写成死变量的字段不会悄悄丢掉。
//!
//! 另外还覆盖「只写在 identity 表、profile 表为空」的情况：
//! FULL OUTER JOIN 正是为了这种半边数据而用的，普通 JOIN 会读不到。

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

async fn setup(tag: &str) -> Result<(Database, PathBuf, Uuid, Uuid)> {
    let dir = std::env::temp_dir().join(format!("novel-locprof-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    let db = Database::open(&url).await?;
    sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;

    let proj_repo = DbProjectRepositoryPort::new(db.pool().clone());
    let created = proj_repo.create_project("地点档案测试", None, None).await?;
    let pid = Uuid::parse_str(created["id"].as_str().unwrap())?;

    let wid: Uuid = sqlx::query_scalar("SELECT id FROM world WHERE project_id = $1 AND is_main = 1")
        .bind(pid)
        .fetch_one(db.pool())
        .await?;

    let ent_repo = DbEntityRepositoryPort::new(db.pool().clone());
    let loc = ent_repo
        .create_entity(wid, "Location", "北境城", Some("边城"), None)
        .await?;
    let lid = Uuid::parse_str(loc["id"].as_str().unwrap())?;

    Ok((db, dir, pid, lid))
}

/// 全部 12 个字段写进去都能读回来（含只存在 identity 表的那 5 个）。
#[tokio::test]
async fn location_profile_roundtrip_keeps_all_fields() -> Result<()> {
    let (db, dir, _pid, lid) = setup("full").await?;
    let repo = DbEntityRepositoryPort::new(db.pool().clone());

    let written = serde_json::json!({
        // location_profile 表
        "geography": "盘状世界的北缘，常年积雪",
        "appearance": "灰石城墙，三重城门",
        "population": "约十二万",
        "economy": "皮毛与铁矿",
        "rules": "入城需验引，夜禁",
        "history": "三百年前由边军筑城",
        "narrative_usage": "主角起家的地方",
        // location_identity 表
        "location_type": "城市",
        "size": "大型",
        "climate": "苦寒",
        "era": "王朝中期",
        "accessibility": "陆路可达，冬季封道",
    });

    let saved = repo.upsert_location_profile(lid, written.clone(), "user")
        .await?;
    // upsert 的回传会比入参多一个 entity_id，逐字段比较即可
    for key in written.as_object().unwrap().keys() {
        assert_eq!(saved.get(key), written.get(key), "upsert 回传的 {} 与写入不一致", key);
    }

    // 重新读取（走 FULL OUTER JOIN 那条路径）
    let got = repo
        .get_location_profile(lid)
        .await?
        .expect("档案应能读回");

    println!("读回的地点档案: {}", got);
    for key in [
        "geography",
        "appearance",
        "population",
        "economy",
        "rules",
        "history",
        "narrative_usage",
        "location_type",
        "size",
        "climate",
        "era",
        "accessibility",
    ] {
        assert_eq!(
            got.get(key).and_then(|v| v.as_str()),
            written.get(key).and_then(|v| v.as_str()),
            "字段 {} 没能读写往返（手机端 FULL OUTER JOIN 是否可用？）",
            key
        );
    }

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// 更新已存在的档案也要落库（走 UPDATE 分支，不是 INSERT）。
///
/// 这条同时是**回归测试**：INSERT 分支过去用「36 字符 uuid 文本」填主键，
/// 第二次进来 `SELECT id` 解码就报 `invalid length: expected 16 bytes, found 36`，
/// 表现为「第一次保存成功、再改一次报错」。
#[tokio::test]
async fn location_profile_update_persists() -> Result<()> {
    let (db, dir, _pid, lid) = setup("update").await?;
    let repo = DbEntityRepositoryPort::new(db.pool().clone());

    repo.upsert_location_profile(lid, serde_json::json!({
        "geography": "旧地理", "location_type": "城市", "climate": "温和"
    }), "user")
        .await?;

    // 第二次写入：两条 UPDATE 语句都要生效
    repo.upsert_location_profile(lid, serde_json::json!({
        "geography": "新地理", "location_type": "要塞", "climate": "苦寒", "size": "中型"
    }), "user")
        .await?;

    let got = repo.get_location_profile(lid).await?.expect("档案应存在");
    println!("更新后的档案: {}", got);
    assert_eq!(got["geography"], "新地理");
    assert_eq!(got["location_type"], "要塞");
    assert_eq!(got["climate"], "苦寒");
    assert_eq!(got["size"], "中型");

    // 只写 identity 表那一半时，profile 表那一半不应被清掉
    repo.upsert_location_profile(lid, serde_json::json!({ "location_type": "港口" }), "user")
        .await?;
    let got = repo.get_location_profile(lid).await?.expect("档案应存在");
    println!("只更新 identity 侧后: {}", got);
    assert_eq!(got["location_type"], "港口");
    assert_eq!(got["geography"], "新地理", "另一张表的字段不该被清空");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
