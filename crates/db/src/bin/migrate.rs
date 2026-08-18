//! Database migration runner binary

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string());

    println!("Connecting to PostgreSQL...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Running migrations...");
    let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
    let executed = db::migration::run_migrations(&pool, migrations_dir).await?;

    if executed.is_empty() {
        println!("No new migrations to apply.");
    } else {
        println!("Applied {} migrations:", executed.len());
        for m in &executed {
            println!("  - {}", m);
        }
    }

    // Validate schema
    let missing = db::schema::validate_schema(&pool).await?;
    if missing.is_empty() {
        println!("Schema validation passed!");
    } else {
        println!("Missing tables: {:?}", missing);
    }

    pool.close().await;
    Ok(())
}
