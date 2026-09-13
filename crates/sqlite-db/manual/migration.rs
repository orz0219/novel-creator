//! SQLite 迁移执行器（手工维护）
//!
//! 与 PostgreSQL 版的差异：
//!   - 没有 advisory lock，改用单连接 + `BEGIN IMMEDIATE` 事务保证串行
//!   - `_migrations` 表用 `INTEGER PRIMARY KEY AUTOINCREMENT`（PG 的 SERIAL 不兼容）
//!   - 记录迁移用 `INSERT OR IGNORE`，配合 name 唯一约束实现幂等
//!
//! 迁移目录：crates/db/migrations_sqlite/（由 tmp/gen_sqlite_schema.py 生成）

use anyhow::{Context, Result};
use sqlx::{SqliteConnection, SqlitePool};

/// 运行所有 migration（按文件名排序，跳过已记录的）
pub async fn run_migrations(pool: &SqlitePool, migrations_dir: &str) -> Result<Vec<String>> {
    let mut conn = pool
        .acquire()
        .await
        .context("Failed to acquire database connection")?;
    run_migrations_on_conn(&mut conn, migrations_dir).await
}

/// 在指定连接上执行迁移。
///
/// SQLite 是单写者模型，同一时刻只有一个连接能持有写事务，
/// 因此「读取已应用列表 → 执行 → 记录」整个过程放在一个
/// `BEGIN IMMEDIATE` 事务里即可保证并发安全。
pub async fn run_migrations_on_conn(
    conn: &mut SqliteConnection,
    migrations_dir: &str,
) -> Result<Vec<String>> {
    let mut executed = Vec::new();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&mut *conn)
    .await
    .context("Failed to create _migrations table")?;

    let dir = std::path::Path::new(migrations_dir);
    if !dir.exists() {
        tracing::info!("未找到迁移目录: {}", migrations_dir);
        return Ok(executed);
    }

    let mut files: Vec<String> = std::fs::read_dir(dir)
        .context(format!("Failed to read migrations dir: {}", migrations_dir))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "sql")
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    files.sort();

    // 迁移期间必须关掉外键检查，且**必须在事务之外**执行：
    // `PRAGMA foreign_keys` 在事务内是空操作。
    //
    // 为什么非关不可：SQLite 改不了外键约束，只能「建新表 → 拷数据 → 删旧表 → 改名」。
    // 而 `DROP TABLE X` 在 foreign_keys=ON 时会对所有引用 X 且带 ON DELETE CASCADE 的
    // 表**立即级联删除**——实测重建 `entity` 会把 `character_profile` / `character_state`
    // 等表清空，然后那些表再"重建"时拷到的已经是空表，**数据静默丢失**。
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await
        .context("关闭外键约束失败（迁移需要在无外键检查下重建表）")?;

    let applied: Vec<String> = sqlx::query_scalar("SELECT name FROM _migrations ORDER BY id")
        .fetch_all(&mut *conn)
        .await
        .context("Failed to query applied migrations")?;

    for file in files {
        if applied.contains(&file) {
            continue;
        }

        let path = dir.join(&file);
        let sql = std::fs::read_to_string(&path)
            .context(format!("Failed to read migration: {}", path.display()))?;

        tracing::info!("Applying migration: {}", file);

        // BEGIN IMMEDIATE：立即获取写锁，避免并发 runner 交错
        let mut tx = conn
            .begin()
            .await
            .context("Failed to begin migration transaction")?;

        sqlx::raw_sql(&sql)
            .execute(&mut *tx)
            .await
            .context(format!("Failed to apply migration: {}", file))?;

        sqlx::query("INSERT OR IGNORE INTO _migrations (name) VALUES ($1)")
            .bind(&file)
            .execute(&mut *tx)
            .await
            .context("Failed to record migration")?;

        tx.commit()
            .await
            .context("Failed to commit migration transaction")?;

        executed.push(file);
    }

    // 迁移跑完做一次完整性自检，再把外键打开。
    //
    // 重建表期间没有任何约束兜底，万一 COPY 漏了行就会留下孤儿数据。
    // 与其等到运行时某个查询莫名少数据，不如在这里当场报出来。
    let orphans = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut *conn)
        .await
        .context("外键自检失败")?;
    if !orphans.is_empty() {
        anyhow::bail!(
            "迁移后检测到 {} 处外键违规（迁移可能漏拷了行，请勿继续使用该库）",
            orphans.len()
        );
    }

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await
        .context("重新开启外键约束失败")?;

    Ok(executed)
}

/// 列出已应用的迁移
pub async fn list_applied(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT name FROM _migrations ORDER BY id")
        .fetch_all(pool)
        .await
        .context("Failed to list applied migrations")?;
    Ok(rows)
}

use sqlx::Acquire;

/// 应用编译期内嵌的迁移（手机端用得上的入口）
pub async fn apply_embedded_migrations(pool: &SqlitePool) -> Result<Vec<String>> {
    let mut conn = pool
        .acquire()
        .await
        .context("Failed to acquire database connection")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&mut *conn)
    .await
    .context("Failed to create _migrations table")?;

    // 同 run_migrations：迁移要在无外键检查下跑（重建表时 DROP TABLE 会级联删数据），
    // 而且 PRAGMA 必须在事务之外。
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await
        .context("关闭外键约束失败（迁移需要在无外键检查下重建表）")?;

    let applied: Vec<String> = sqlx::query_scalar("SELECT name FROM _migrations ORDER BY id")
        .fetch_all(&mut *conn)
        .await
        .context("Failed to query applied migrations")?;

    let mut executed = Vec::new();
    for (name, sql) in crate::embedded::EMBEDDED_MIGRATIONS {
        if applied.iter().any(|a| a == name) {
            continue;
        }
        tracing::info!("Applying embedded migration: {}", name);

        let mut tx = conn
            .begin()
            .await
            .context("Failed to begin migration transaction")?;

        sqlx::raw_sql(sql)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to apply embedded migration: {}", name))?;

        sqlx::query("INSERT OR IGNORE INTO _migrations (name) VALUES ($1)")
            .bind(*name)
            .execute(&mut *tx)
            .await
            .context("Failed to record migration")?;

        tx.commit()
            .await
            .context("Failed to commit migration transaction")?;

        executed.push((*name).to_string());
    }

    // 迁移完先自检外键完整性，再打开外键检查
    let orphans = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut *conn)
        .await
        .context("外键自检失败")?;
    if !orphans.is_empty() {
        anyhow::bail!(
            "迁移后检测到 {} 处外键违规（迁移可能漏拷了行，请勿继续使用该库）",
            orphans.len()
        );
    }

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await
        .context("重新开启外键约束失败")?;

    Ok(executed)
}
