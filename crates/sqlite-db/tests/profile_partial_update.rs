//! 「未传的字段保持原值」是 agent 工具的**对外契约**，这里把它钉住。
//!
//! `update_character_profile` / `update_faction_profile` / `update_location_profile`
//! 的描述都明确告诉模型：「**只传需要设置的字段**，未传的保持原值」。
//! 模型就是照这句话调用的——它改一个「身份」，就真的只会传 `{"identity": "..."}`。
//!
//! 而这些 upsert 原来是**整行 SET**：模型只传一个字段时，其余字段会被写成 NULL。
//! 用户侧表现为「设定莫名其妙没了」，且没有任何提示。地点档案更严重——
//! 字段分散在 `location_profile` 与 `location_identity` 两张表，
//! 只改其中一张表的字段会把**另一张表的所有字段**一起清空。
//!
//! 本文件覆盖四个写入路径（角色档案 / 角色状态 / 地点档案 / 势力档案）。

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

struct Fixture {
    db: Database,
    dir: PathBuf,
    repo: DbEntityRepositoryPort,
    character: Uuid,
    location: Uuid,
    faction: Uuid,
}

async fn setup(tag: &str) -> Result<Fixture> {
    let dir = std::env::temp_dir().join(format!("novel-partial-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("test.db");
    let db = Database::open(&format!("sqlite://{}?mode=rwc", db_path.display())).await?;
    sqlite_db::seed::initialize(db.pool(), &migrations_dir()).await?;

    let proj_repo = DbProjectRepositoryPort::new(db.pool().clone());
    let created = proj_repo.create_project("部分更新测试", None, None).await?;
    let pid = Uuid::parse_str(created["id"].as_str().unwrap())?;
    let wid: Uuid = sqlx::query_scalar("SELECT id FROM world WHERE project_id = $1 AND is_main = 1")
        .bind(pid)
        .fetch_one(db.pool())
        .await?;

    let repo = DbEntityRepositoryPort::new(db.pool().clone());
    let character = Uuid::parse_str(
        repo.create_entity(wid, "Character", "林秋", None, None).await?["id"]
            .as_str()
            .unwrap(),
    )?;
    let location = Uuid::parse_str(
        repo.create_entity(wid, "Location", "北境城", None, None).await?["id"]
            .as_str()
            .unwrap(),
    )?;
    let faction = Uuid::parse_str(
        repo.create_entity(wid, "Faction", "商会", None, None).await?["id"]
            .as_str()
            .unwrap(),
    )?;

    Ok(Fixture { db, dir, repo, character, location, faction })
}

/// 角色档案：只传 identity，其余字段必须原样保留。
#[tokio::test]
async fn character_profile_partial_update_keeps_other_fields() -> Result<()> {
    let f = setup("char").await?;

    f.repo
        .update_character_profile(f.character, serde_json::json!({
            "name": "林秋",
            "identity": "行商",
            "age_range": "青年",
            "gender": "女",
            "role_in_story": "主角",
            "appearance": "瘦高，左眉有疤",
            "background_origin": "北境小户",
            "core_personality": "能忍，认死理",
            "values": "重诺",
            "aliases": ["秋娘"]
        }), "user")
        .await?;

    // 模型最典型的调用：只改一个字段
    f.repo
        .update_character_profile(f.character, serde_json::json!({ "identity": "商会头目" }), "user")
        .await?;

    let got = f.repo.get_character_profile(f.character).await?.expect("档案应存在");
    println!("只改 identity 之后: {}", got);
    assert_eq!(got["identity"], "商会头目", "改动的字段要生效");
    assert_eq!(got["name"], "林秋", "未传的 name 应保持");
    assert_eq!(got["appearance"], "瘦高，左眉有疤", "未传的 appearance 应保持");
    assert_eq!(got["core_personality"], "能忍，认死理", "未传的 core_personality 应保持");
    assert_eq!(got["background_origin"], "北境小户", "未传的 background_origin 应保持");
    assert_eq!(got["values"], "重诺", "未传的 values 应保持");
    assert_eq!(got["age_range"], "YoungAdult", "未传的 age_range 应保持");
    assert_eq!(got["gender"], "Female", "未传的 gender 应保持");
    assert_eq!(got["role_in_story"], "Protagonist", "未传的 role_in_story 应保持");
    assert_eq!(got["aliases"], serde_json::json!(["秋娘"]), "未传的 aliases 应保持");

    drop(f.db);
    let _ = std::fs::remove_dir_all(&f.dir);
    Ok(())
}

/// 角色状态：只传 location，其余状态字段必须原样保留。
#[tokio::test]
async fn character_state_partial_update_keeps_other_fields() -> Result<()> {
    let f = setup("state").await?;

    f.repo
        .update_character_state(f.character, serde_json::json!({
            "location": "北境城",
            "physical_state": "旧伤未愈",
            "mental_state": "警惕",
            "resource_state": "两匹马",
            "social_state": "与掌柜有旧",
            "flags": ["通缉"]
        }), "user")
        .await?;

    f.repo
        .update_character_state(f.character, serde_json::json!({ "location": "南港" }), "user")
        .await?;

    let got = f.repo.get_character_state(f.character).await?.expect("状态应存在");
    println!("只改 location 之后: {}", got);
    assert_eq!(got["location"], "南港");
    assert_eq!(got["physical_state"], "旧伤未愈", "未传的 physical_state 应保持");
    assert_eq!(got["mental_state"], "警惕");
    assert_eq!(got["resource_state"], "两匹马");
    assert_eq!(got["social_state"], "与掌柜有旧");
    assert_eq!(got["flags"], serde_json::json!(["通缉"]), "未传的 flags 应保持");

    drop(f.db);
    let _ = std::fs::remove_dir_all(&f.dir);
    Ok(())
}

/// 地点档案：字段横跨两张表，只改一张表的字段不能清空另一张表。
#[tokio::test]
async fn location_profile_partial_update_keeps_both_tables() -> Result<()> {
    let f = setup("loc").await?;

    f.repo
        .upsert_location_profile(f.location, serde_json::json!({
            "geography": "盘状世界北缘",
            "appearance": "灰石城墙",
            "population": "十二万",
            "economy": "皮毛与铁矿",
            "rules": "夜禁",
            "history": "三百年边军筑城",
            "narrative_usage": "主角起家处",
            "location_type": "城市",
            "size": "大型",
            "climate": "苦寒",
            "era": "王朝中期",
            "accessibility": "陆路可达"
        }), "user")
        .await?;

    // 只改 location_identity 表的字段
    f.repo
        .upsert_location_profile(f.location, serde_json::json!({ "location_type": "要塞" }), "user")
        .await?;

    let got = f.repo.get_location_profile(f.location).await?.expect("档案应存在");
    println!("只改 location_type 之后: {}", got);
    assert_eq!(got["location_type"], "要塞", "改动的字段要生效");
    assert_eq!(got["size"], "大型", "同表的未传字段应保持");
    assert_eq!(got["climate"], "苦寒");
    assert_eq!(got["geography"], "盘状世界北缘", "另一张表的字段不能被清空");
    assert_eq!(got["appearance"], "灰石城墙");
    assert_eq!(got["population"], "十二万");
    assert_eq!(got["economy"], "皮毛与铁矿");
    assert_eq!(got["rules"], "夜禁");
    assert_eq!(got["history"], "三百年边军筑城");
    assert_eq!(got["narrative_usage"], "主角起家处");

    // 反过来：只改 profile 表的字段，也不能清空 identity 表
    f.repo
        .upsert_location_profile(f.location, serde_json::json!({ "geography": "改过的地理" }), "user")
        .await?;
    let got = f.repo.get_location_profile(f.location).await?.expect("档案应存在");
    assert_eq!(got["geography"], "改过的地理");
    assert_eq!(got["location_type"], "要塞", "另一张表的字段不能被清空");
    assert_eq!(got["size"], "大型");
    assert_eq!(got["era"], "王朝中期");
    assert_eq!(got["accessibility"], "陆路可达");

    drop(f.db);
    let _ = std::fs::remove_dir_all(&f.dir);
    Ok(())
}

/// 势力档案：只传 goals，其余字段必须原样保留。
#[tokio::test]
async fn faction_profile_partial_update_keeps_other_fields() -> Result<()> {
    let f = setup("fac").await?;

    f.repo
        .upsert_faction_profile(f.faction, serde_json::json!({
            "goals": "垄断北境商路",
            "leader": "沈九",
            "values": "唯利",
            "resources": "车队二十",
            "territory": "北境三城",
            "members": "三百余人",
            "enemies": "边军",
            "allies": "南港船帮",
            "internal_conflicts": "老派与新派之争",
            "secrets": "私通阴面",
            "modus_operandi": "先礼后兵"
        }), "user")
        .await?;

    f.repo
        .upsert_faction_profile(f.faction, serde_json::json!({ "goals": "垄断两界贸易" }), "user")
        .await?;

    let got = f.repo.get_faction_profile(f.faction).await?.expect("档案应存在");
    println!("只改 goals 之后: {}", got);
    assert_eq!(got["goals"], "垄断两界贸易");
    assert_eq!(got["leader"], "沈九", "未传的 leader 应保持");
    assert_eq!(got["values"], "唯利");
    assert_eq!(got["resources"], "车队二十");
    assert_eq!(got["territory"], "北境三城");
    assert_eq!(got["members"], "三百余人");
    assert_eq!(got["enemies"], "边军");
    assert_eq!(got["allies"], "南港船帮");
    assert_eq!(got["internal_conflicts"], "老派与新派之争");
    assert_eq!(got["secrets"], "私通阴面");
    assert_eq!(got["modus_operandi"], "先礼后兵");

    drop(f.db);
    let _ = std::fs::remove_dir_all(&f.dir);
    Ok(())
}

/// 传空串是「显式清空」，要真的清掉（否则用户没法删掉写错的内容）。
#[tokio::test]
async fn explicit_empty_string_clears_field() -> Result<()> {
    let f = setup("clear").await?;

    f.repo
        .upsert_faction_profile(f.faction, serde_json::json!({
            "goals": "垄断北境商路", "leader": "沈九"
        }), "user")
        .await?;

    f.repo
        .upsert_faction_profile(f.faction, serde_json::json!({ "leader": "" }), "user")
        .await?;

    let got = f.repo.get_faction_profile(f.faction).await?.expect("档案应存在");
    println!("显式清空 leader 之后: {}", got);
    assert!(
        got["leader"].is_null() || got["leader"] == "",
        "传空串应能清空该字段，实为 {:?}",
        got["leader"]
    );
    assert_eq!(got["goals"], "垄断北境商路", "清空一个字段不应影响其他字段");

    drop(f.db);
    let _ = std::fs::remove_dir_all(&f.dir);
    Ok(())
}
