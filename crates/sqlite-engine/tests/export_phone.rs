//! 验证手机端项目导出：格式与电脑端一致、内容完整。
//!
//! 电脑端的导出格式见 `crates/db/src/export.rs`，两者必须能互相导入，
//! 所以这里的断言就是在守那个「契约」。

use std::path::PathBuf;
use uuid::Uuid;

use sqlite_engine::engine::{BootstrapConfig, Engine};

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("novel-exp-{}-{}", tag, Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn test_phone_export_matches_desktop_format() {
    let dir = tmpdir("fmt");
    let db = dir.join("t.db");

    let engine = Engine::build(db.to_str().unwrap(), BootstrapConfig::default())
        .await
        .expect("引擎组装失败");

    use domain::ports::{EntityRepositoryPort, ProjectRepositoryPort};

    // 造点数据：项目 → 主世界 → 角色
    let proj_repo = sqlite_db::application_ports::DbProjectRepositoryPort::new(engine.state.pool.clone());
    let created = proj_repo
        .create_project("手机导出测试", Some("验证导出格式"), None)
        .await
        .unwrap();
    let pid = created["id"].as_str().unwrap().to_string();

    let wid: Uuid = sqlx::query_scalar("SELECT id FROM world WHERE project_id = $1 AND is_main = 1")
        .bind(Uuid::parse_str(&pid).unwrap())
        .fetch_one(&engine.state.pool)
        .await
        .expect("主世界不存在");

    let ent_repo = sqlite_db::application_ports::DbEntityRepositoryPort::new(engine.state.pool.clone());
    ent_repo
        .create_entity(wid, "Character", "张三", Some("主角"), Some("一个普通人"))
        .await
        .unwrap();

    // 导出
    let exp = sqlite_db::export::export_project(&engine.state.pool, &pid)
        .await
        .expect("导出失败");

    // ---- 格式契约：这些字段名与类型必须与电脑端一致 ----
    assert_eq!(exp.format_version, 2, "format_version 必须与电脑端一致（2）");
    assert_eq!(exp.root_project_id, pid, "root_project_id 应为标准 uuid 文本");
    assert!(!exp.exported_at.is_empty(), "exported_at 不能为空");
    assert!(exp.tables.contains_key("project"), "必须含 project 表");
    assert!(!exp.insert_order.is_empty(), "insert_order 不能为空");

    let proj = &exp.tables["project"];
    assert_eq!(proj.rows.len(), 1);
    assert!(!proj.columns.is_empty(), "columns 不能为空");
    assert!(
        proj.column_types.contains_key("id"),
        "column_types 必须给出 id 的类型（导入端据此决定绑定方式）"
    );
    assert_eq!(
        proj.column_types.get("id").map(|s| s.as_str()),
        Some("uuid"),
        "uuid 列的类型必须标注为 uuid"
    );
    assert_eq!(proj.rows[0]["id"].as_str().unwrap(), pid);

    // 角色应被带出来（它没有 project_id，靠 entity→world→project 关联）
    assert!(exp.tables.contains_key("entity"), "角色所在表应被导出");
    let names: Vec<String> = exp.tables["entity"]
        .rows
        .iter()
        .filter_map(|r| r["name"].as_str().map(|s| s.to_string()))
        .collect();
    assert!(names.iter().any(|n| n == "张三"), "导出的角色里应有张三: {:?}", names);

    // 时间列必须已转成文本（电脑端导入时按字符串处理）
    let created_at = exp.tables["project"].rows[0]["created_at"].clone();
    let s = created_at.as_str().expect("时间应为字符串");
    assert!(s.contains('T') && (s.contains('+') || s.ends_with('Z')), "时间应为 RFC3339: {}", s);
    println!("✅ 时间格式: {}", s);

    // 能被序列化成 JSON 并通过打印检查
    let json = serde_json::to_string(&exp).unwrap();
    println!("导出 JSON 大小: {} bytes，表数: {}", json.len(), exp.tables.len());
    println!("导出的表: {:?}", {
        let mut v: Vec<&String> = exp.tables.keys().collect();
        v.sort();
        v
    });

    println!("✅ 手机端导出格式与电脑端契约一致");

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}
