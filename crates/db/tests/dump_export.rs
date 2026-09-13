//! 把真实项目导出成文件，供手机端导入测试使用（仅本地调试用）。
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn dump_export_to_file() {
    let url = std::env::var("DATABASE_URL").expect("需要 DATABASE_URL");
    let pool = PgPoolOptions::new().max_connections(4).connect(&url).await.unwrap();
    let pid: String = sqlx::query_scalar(
        "SELECT p.id::text FROM project p LEFT JOIN entity e ON e.project_id=p.id \
         GROUP BY p.id ORDER BY count(e.id) DESC, p.updated_at DESC LIMIT 1",
    ).fetch_one(&pool).await.unwrap();
    let exp = db::export::export_project(&pool, &pid).await.unwrap();
    let out = "/tmp/novel-export.json";
    std::fs::write(out, serde_json::to_string(&exp).unwrap()).unwrap();
    println!("已导出到 {} （{} 表 / {} 行）", out, exp.tables.len(),
        exp.tables.values().map(|t| t.rows.len()).sum::<usize>());
}
