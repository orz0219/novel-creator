//! SQLite 基础数据播种（seed）
//!
//! 与 PostgreSQL 版 main.rs 的 seed 语义一致：把必需的 entity_type 补齐。
//! PG 用 `ON CONFLICT (name) DO NOTHING`，SQLite 用 `INSERT OR IGNORE`（依赖 name 唯一约束）。
//!
//! 这是运行时必需数据：缺了它 create_entity 会直接报 "Entity type not found"。

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use uuid::Uuid;

/// 系统内置实体类型（必须与 crates/narrative-engine/src/main.rs 保持一致）
pub const BUILTIN_ENTITY_TYPES: [&str; 7] = [
    "Character",
    "Location",
    "Faction",
    "Item",
    "Creature",
    "Organization",
    "golden_finger",
];

/// 补齐内置实体类型。幂等：已存在的不会重复插入。
pub async fn seed_entity_types(pool: &SqlitePool) -> Result<()> {
    for name in BUILTIN_ENTITY_TYPES {
        sqlx::query(
            "INSERT OR IGNORE INTO entity_type (id, name, description) VALUES ($1, $2, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(format!("{} entity type", name))
        .execute(pool)
        .await
        .with_context(|| format!("Failed to seed entity type '{}'", name))?;
    }
    Ok(())
}

/// 一次性完成数据库初始化：建表迁移 + 基础数据
pub async fn initialize(pool: &SqlitePool, migrations_dir: &str) -> Result<Vec<String>> {
    let applied = crate::migration::run_migrations(pool, migrations_dir).await?;
    seed_entity_types(pool).await?;
    Ok(applied)
}

/// 用编译期内嵌的迁移完成数据库初始化（手机端用）
pub async fn initialize_embedded(pool: &SqlitePool) -> Result<Vec<String>> {
    let applied = crate::migration::apply_embedded_migrations(pool).await?;
    seed_entity_types(pool).await?;
    Ok(applied)
}
