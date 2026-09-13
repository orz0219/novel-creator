//! 导出功能验证：用真实 PostgreSQL 数据跑一次项目导出。
//!
//! 需要 DATABASE_URL 指向可用的库（本机开发库）。

use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_export_real_project() {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("跳过：未设置 DATABASE_URL");
            return;
        }
    };
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("连接数据库失败");

    // 挑一个数据量适中的项目（优先选有会话/角色的）
    let pid: String = sqlx::query_scalar(
        "SELECT p.id::text FROM project p \
         LEFT JOIN entity e ON e.project_id = p.id \
         GROUP BY p.id ORDER BY count(e.id) DESC, p.updated_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("取项目失败");
    println!("导出项目: {}", pid);

    let exp = db::export::export_project(&pool, &pid)
        .await
        .expect("导出失败");

    println!("format_version: {}", exp.format_version);
    println!("涉及表数: {}", exp.tables.len());
    let total_rows: usize = exp.tables.values().map(|t| t.rows.len()).sum();
    println!("总行数: {}", total_rows);
    println!("插入顺序（{}）: {:?}", exp.insert_order.len(), exp.insert_order);

    assert!(exp.tables.contains_key("project"), "必须含 project 表");
    let proj = &exp.tables["project"];
    assert_eq!(proj.rows.len(), 1, "project 应恰好 1 行");
    assert_eq!(
        proj.rows[0]["id"].as_str().unwrap(),
        pid,
        "project id 应为文本且等于源 id"
    );
    println!("project 行: {}", serde_json::to_string(&proj.rows[0]).unwrap());

    // 本机级配置不得外泄：手机端导入这份文件时，它上面的 AI 配置（含密钥）
    // 必须保持不变（详见 db::export::NON_PROJECT_TABLES）
    for t in db::export::NON_PROJECT_TABLES {
        assert!(
            !exp.tables.contains_key(*t),
            "表 {} 是本机级配置，不能出现在项目导出文件里",
            t
        );
        assert!(!exp.insert_order.contains(&t.to_string()));
    }

    // 插入顺序必须覆盖所有导出的表
    for t in exp.tables.keys() {
        assert!(exp.insert_order.contains(t), "表 {} 不在插入顺序里", t);
    }
    // project 必须排在依赖它的表之前
    let pos = |name: &str| exp.insert_order.iter().position(|x| x == name);
    if let (Some(p), Some(w)) = (pos("project"), pos("world")) {
        assert!(p < w, "project 应排在 world 之前");
    }

    // 打印每张表的行数，便于人工核对
    let mut list: Vec<(&String, usize)> = exp.tables.iter().map(|(k, v)| (k, v.rows.len())).collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    println!("各表行数（前 20）:");
    for (t, n) in list.iter().take(20) {
        println!("   {:32} {}", t, n);
    }

    // 全局表（如 entity_type）是否带上：没带的话手机端建档会失败
    let mut names: Vec<&String> = exp.tables.keys().collect();
    names.sort();
    println!("导出的全部表（{}）:", names.len());
    println!("  {:?}", names);
    for must in ["entity_type", "project", "world"] {
        println!("  含 {}: {}", must, exp.tables.contains_key(must));
    }

    // 关键关联表是否带上
    for must in ["scene", "narrative_node", "storyline_scene", "world_version", "agent_sessions"] {
        println!("  含 {:22} {}", must, exp.tables.contains_key(must));
    }
    if let Some(t) = exp.tables.get("narrative_node") { println!("narrative_node 行数: {}", t.rows.len()); }
    if let Some(t) = exp.tables.get("scene") { println!("scene 行数: {}", t.rows.len()); }

    // 序列化体积检查（手机端要能解析）
    let json = serde_json::to_string(&exp).unwrap();
    println!("导出 JSON 体积: {:.2} MB", json.len() as f64 / 1024.0 / 1024.0);
    assert!(json.len() > 100, "导出内容过小");

    // 导出两份必须一致（同样的输入产生同样结构）
    let exp2 = db::export::export_project(&pool, &pid).await.unwrap();
    assert_eq!(exp2.tables.len(), exp.tables.len(), "两次导出的表数应一致");
    println!("✅ 导出验证通过");
}
