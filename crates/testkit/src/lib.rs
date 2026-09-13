//! 集成测试共享工具：把测试数据写进**独立的测试库**，绝不碰开发库。
//!
//! 背景（实测事故）：这些测试会真的建项目 / 世界 / 实体 / 叙事节点。
//! 原来它们直连 `DATABASE_URL` 指向的库——也就是开发库，于是每跑一次
//! `cargo test --workspace`，开发库里就多出一批 `Test Project …`，
//! 而且没人清理（实测一天积了 75 个项目、400 多行子表数据）。
//!
//! 现在的规则：
//!   1. 库名加 `_test` 后缀（`novel_engine` → `novel_engine_test`）
//!   2. 库不存在就建（建库要在 `postgres` 库里执行 `CREATE DATABASE`）
//!   3. 建完保证迁移已经跑过
//!
//! 因此测试可以反复跑，开发库始终保持干净。

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_URL: &str = "postgresql://novel:novel_pass@localhost:5432/novel_engine";

/// 拆出 (前缀含斜杠, 库名, 查询串)，例如
/// `postgresql://u:p@h:5432/novel_engine?sslmode=disable`
/// → (`postgresql://u:p@h:5432/`, `novel_engine`, `?sslmode=disable`)
fn split_url(url: &str) -> (String, String, String) {
    let (head, tail) = url
        .rsplit_once('/')
        .unwrap_or_else(|| panic!("DATABASE_URL 里没有库名: {}", url));
    match tail.split_once('?') {
        Some((db, q)) => (format!("{}/", head), db.to_string(), format!("?{}", q)),
        None => (format!("{}/", head), tail.to_string(), String::new()),
    }
}

/// 派生测试库 URL（已经是 `_test` 结尾就原样返回）
fn test_url() -> String {
    let base = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    let (head, db, query) = split_url(&base);
    if db.ends_with("_test") {
        return base;
    }
    format!("{}{}_test{}", head, db, query)
}

/// 把 URL 里的库名换成指定值（用于连 `postgres` 库建库）
fn url_with_db(url: &str, db: &str) -> String {
    let (head, _old, query) = split_url(url);
    format!("{}{}{}", head, db, query)
}

fn db_name_of(url: &str) -> String {
    split_url(url).1
}

/// 确保测试库存在（不存在则在 `postgres` 库里 CREATE DATABASE）
async fn ensure_database(test_url: &str) -> Result<()> {
    let admin_url = url_with_db(test_url, "postgres");
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&admin_url)
        .await
        .with_context(|| format!("连不上管理库 {}（建测试库需要它）", admin_url))?;

    let name = db_name_of(test_url);
    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(&name)
        .fetch_optional(&admin)
        .await
        .context("查询 pg_database 失败")?;

    if exists.is_none() {
        // CREATE DATABASE 不能用预处理参数（也不能放在事务里），只能拼串；
        // 库名来自我们自己的 DATABASE_URL，这里仍做双引号转义。
        let quoted = name.replace('"', "\"\"");
        if let Err(e) = sqlx::raw_sql(&format!("CREATE DATABASE \"{}\"", quoted))
            .execute(&admin)
            .await
        {
            // 多个测试二进制会并发建同一个库：撞上「已存在」是预期情况，其余错误要报出来
            let duplicate = e
                .as_database_error()
                .map(|d| d.code().as_deref() == Some("42P04"))
                .unwrap_or(false);
            if !duplicate {
                return Err(e).with_context(|| format!("创建测试库 {} 失败", name));
            }
        } else {
            println!("已创建测试库 {}", name);
        }
    }

    admin.close().await;
    Ok(())
}

/// 迁移目录未跑过时跑一遍（幂等）
async fn ensure_migrated(pool: &PgPool) -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ 目录")
        .join("db")
        .join("migrations");
    let applied = db::migration::run_migrations(pool, dir.to_str().expect("迁移目录不是合法路径"))
        .await
        .context("测试库迁移失败")?;
    if !applied.is_empty() {
        println!("测试库应用了 {} 个迁移", applied.len());
    }
    Ok(())
}

/// 集成测试统一的连接入口：测试库 + 已迁移。
pub async fn test_pool() -> Result<PgPool> {
    let url = test_url();
    ensure_database(&url).await?;

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(20))
        .connect(&url)
        .await
        .with_context(|| format!("连不上测试库 {}", url))?;

    ensure_migrated(&pool).await?;
    Ok(pool)
}

/// 打印一下当前用的是哪个库，测试输出里能直接看清（避免误以为在写开发库）
pub async fn test_db_name(pool: &PgPool) -> String {
    sqlx::query_scalar::<_, String>("SELECT current_database()")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "?".to_string())
}

/// 只拿测试库的 URL（库不存在会先建好）。
///
/// 给仍用 `db::connection::Database` 包装的测试用——它只能从 URL 构造，
/// 不接受已有连接池。
pub async fn test_database_url() -> Result<String> {
    let url = test_url();
    ensure_database(&url).await?;
    Ok(url)
}
