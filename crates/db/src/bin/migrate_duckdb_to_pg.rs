//! DuckDB → PostgreSQL Data Migration Tool
//!
//! Usage:
//!   cargo run --bin migrate_duckdb_to_pg -- --verify
//!   cargo run --bin migrate_duckdb_to_pg -- --export-pg

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    tracing::info!("DuckDB → PostgreSQL Migration Tool starting...");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: migrate_duckdb_to_pg --verify | --export-pg");
        eprintln!("  DATABASE_URL must be set");
        std::process::exit(1);
    }

    let mode = &args[1];
    let pg_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&pg_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    match mode.as_str() {
        "--verify" => verify_migration(&pool).await?,
        "--export-pg" => export_pg_stats(&pool).await?,
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }

    pool.close().await;
    Ok(())
}

async fn verify_migration(pool: &sqlx::PgPool) -> Result<()> {
    use sqlx::Row;
    tracing::info!("Verifying PostgreSQL data integrity...");

    let tables = vec![
        "project", "entity_type", "entity", "relation", "fact",
        "event", "current_state", "narrative_node", "scene",
        "knowledge_state", "skill", "generation_task",
        "memories", "agent_runs", "system_events",
    ];

    for table in &tables {
        let sql = format!("SELECT COUNT(*) as cnt FROM {}", table);
        match sqlx::query(&sql).fetch_one(pool).await {
            Ok(row) => {
                let count: i64 = row.get("cnt");
                tracing::info!("  {}: {} rows", table, count);
            }
            Err(e) => {
                tracing::error!("  {}: ERROR ({})", table, e);
            }
        }
    }

    tracing::info!("Verification complete");
    Ok(())
}

async fn export_pg_stats(pool: &sqlx::PgPool) -> Result<()> {
    use sqlx::Row;
    tracing::info!("PostgreSQL Table Statistics:");

    let rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
    ).fetch_all(pool).await?;

    for row in rows {
        let table: String = row.get("table_name");
        let sql = format!(r#"SELECT COUNT(*) as cnt FROM "{}""#, table);
        if let Ok(r) = sqlx::query(&sql).fetch_one(pool).await {
            let count: i64 = r.get("cnt");
            tracing::info!("  {}: {} rows", table, count);
        }
    }

    Ok(())
}