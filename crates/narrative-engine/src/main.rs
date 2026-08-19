//! Novel Engine - HTTP Server Entry Point
//!
//! Starts the Axum HTTP server that bridges frontend API calls to backend services.

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

mod state;
mod api;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,novel_engine=debug".into()),
        )
        .init();

    tracing::info!("Novel Engine starting...");

    // Connect to PostgreSQL - DATABASE_URL is required
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is required"))?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to PostgreSQL: {}", e))?;

    tracing::info!("PostgreSQL connected");

    // Run migrations
    let migrations_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("db")
        .join("migrations");

    if migrations_dir.exists() {
        match db::migration::run_migrations(&pool, migrations_dir.to_str().unwrap()).await {
            Ok(executed) => {
                if !executed.is_empty() {
                    tracing::info!("Migrations applied: {:?}", executed);
                }
            }
            Err(e) => {
                tracing::error!("Migration failed: {}. Server cannot start with inconsistent schema.", e);
                return Err(e);
            }
        }
    }

    // Seed entity types (idempotent using ON CONFLICT)
    let entity_types = ["Character", "Location", "Faction", "Item", "Creature", "Organization"];
    for et in &entity_types {
        let id = uuid::Uuid::new_v4().to_string();
        let result = sqlx::query(
            "INSERT INTO entity_type (id, name, description) VALUES ($1, $2, $3) ON CONFLICT (name) DO NOTHING"
        )
        .bind(&id)
        .bind(et)
        .bind(format!("{} entity type", et))
        .execute(&pool)
        .await;

        match result {
            Ok(_) => {
                tracing::debug!("Ensured entity type exists: {}", et);
            }
            Err(e) => {
                tracing::error!("Failed to seed entity type '{}': {}. Server cannot start without required seed data.", et, e);
                return Err(anyhow::anyhow!("Seed failed for entity type '{}': {}", et, e));
            }
        }
    }

    // Create application state
    let state = AppState::new(pool);

    // Build router
    let app = api::router(state);

    // Start server
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("Novel Engine listening on {}", bind_addr);
    tracing::info!("API base: http://{}/api/v1", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}