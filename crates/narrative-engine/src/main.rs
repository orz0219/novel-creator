//! Novel Engine - HTTP Server Entry Point
//!
//! Starts the Axum HTTP server that bridges frontend API calls to backend services.

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;

mod state;
mod api;

use state::AppState;
use agent::{AgentRuntime, ToolRegistry, EchoTool, AskQuestionTool};
use infrastructure::llm::{InfraLlmPort, LlmClient, OpenAiCompatibleProvider};
use db::repos::prompt_repo::PromptRepo;
use db::repos::session_repo::SessionRepo;
use db::repos::memory_repo::MemoryRepo;
use db::ai_settings::DbAiSettingsPort;
use db::guide_progress::DbGuideProgressPort;
use domain::ports::{AiSettingsPort, GuideProgressPort, PromptRepositoryPort};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载 .env（若存在），便于本地配置 OPENCODE_* 等环境变量
    let _ = dotenvy::dotenv();

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
    let entity_types = ["Character", "Location", "Faction", "Item", "Creature", "Organization", "golden_finger"];
    for et in &entity_types {
        let result = sqlx::query(
            "INSERT INTO entity_type (id, name, description) VALUES ($1, $2, $3) ON CONFLICT (name) DO NOTHING"
        )
        .bind(uuid::Uuid::new_v4())
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

    // 构建 Agent 运行时：会话/记忆用内存；基础工具（echo/ask_question）+ P2 真实领域工具。
    // 运行时 AI 配置：真源是「设置」页写入的 app_settings，环境变量仅作为未配置时的默认值。
    // 对话 / 生成 / 抽取三处 LLM 共用同一个端口，设置页保存后全部立即生效。
    let ai_settings: Arc<dyn AiSettingsPort> = Arc::new(DbAiSettingsPort::new(
        pool.clone(),
        db::ai_settings::AiSettingsDefaults::from_env(),
    ));
    let mut llm_client = LlmClient::new("opencode".to_string());
    llm_client.add_provider(Arc::new(OpenAiCompatibleProvider::new(ai_settings.clone())));
    let llm = Arc::new(InfraLlmPort::new(llm_client, ai_settings.clone()));

    let agent_tools = Arc::new(ToolRegistry::new());
    // 基础工具
    agent_tools.register(Arc::new(EchoTool));
    agent_tools.register(Arc::new(AskQuestionTool));
    // P2 真实领域工具（Entity + Narrative / Storyline / Foreshadow / Rule /
    // Snapshot / Project / World / History 聚合；覆盖 C/U/D+R，D 为逻辑删除）
    narrative_engine::agent_tools::register_all_domain_tools(&agent_tools, &pool);
    let agent_sessions: Arc<dyn domain::agent_store::SessionStore> =
        Arc::new(SessionRepo::new(pool.clone()));
    let agent_memory: Arc<dyn domain::agent_store::AgentMemory> =
        Arc::new(MemoryRepo::new(pool.clone()));
    let prompt_store: Arc<dyn PromptRepositoryPort> = Arc::new(PromptRepo::new(pool.clone()));
    // 引导进度真源：project.config.current_step（项目级，多会话共享）
    let guide_progress: Arc<dyn GuideProgressPort> =
        Arc::new(DbGuideProgressPort::new(pool.clone()));
    let agent = Arc::new(AgentRuntime::new(
        llm,
        agent_tools,
        agent_sessions,
        agent_memory,
        prompt_store,
        agent::DEFAULT_SYSTEM_PROMPT_BASE.to_string(),
        ai_settings.clone(),
        guide_progress,
    ));

    // Create application state
    let state = AppState::new(pool, agent, ai_settings);

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