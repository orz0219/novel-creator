//! 引擎组装与启动（手工维护）
//!
//! 职责：把 SQLite 数据库 + 各端口实现 + Agent 运行时组装成 `AppState`，
//! 并启动只监听 127.0.0.1 的 axum 服务。
//!
//! 与电脑端 `narrative-engine/src/main.rs` 的差异：
//!   - 数据库换成 SQLite 本地文件
//!   - 默认值不读环境变量（手机上无 .env），改由 BootstrapConfig 传入
//!   - 只监听 127.0.0.1（单机，不对外暴露）
//!   - 启动逻辑封装成函数，供 Tauri 在进程内调用

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePoolOptions;

use agent::{AgentRuntime, AskQuestionTool, EchoTool, ToolRegistry};
use domain::ports::{AiSettingsPort, GuideProgressPort, PromptRepositoryPort};
use infrastructure::llm::{InfraLlmPort, LlmClient, OpenAiCompatibleProvider};
use sqlite_db::ai_settings::{AiSettingsDefaults, DbAiSettingsPort};
use sqlite_db::guide_progress::DbGuideProgressPort;
use sqlite_db::repos::memory_repo::MemoryRepo;
use sqlite_db::repos::prompt_repo::PromptRepo;
use sqlite_db::repos::session_repo::SessionRepo;

use crate::state::AppState;

/// 首次启动时的 AI 配置。
///
/// 真源始终是数据库里的 `app_settings`（设置页可改）；
/// 这里的值只在「从未配置过」时写入一次，之后不再覆盖用户设置。
///
/// 各字段与电脑端设置页的字段一一对应（见 `db/ai_settings.rs` 的字段约定）：
/// 接口地址 / 密钥 / 模型 / 单次输出上限。手机上不注入 `contextLimits`，
/// 上下文上限直接走内置模型目录（`domain::model_catalog`）。
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    /// 单次输出 token 上限（电脑端设置页的「单次输出上限」）
    pub max_output_tokens: u32,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://opencode.ai/zen/go/v1".to_string(),
            model: "deepseek-flash".to_string(),
            max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        }
    }
}

/// 组装好的引擎
#[derive(Clone)]
pub struct Engine {
    pub state: AppState,
}

impl Engine {
    /// 打开数据库、跑内嵌迁移、播种，并以给定配置组装服务。
    ///
    /// 手机端没有可用的迁移文件目录，迁移 SQL 在编译期内嵌。
    ///
    /// 若现有数据库的结构代号与当前代码不一致，会**清空重建**（丢数据），
    /// 原因见 `sqlite_db::embedded::SCHEMA_GENERATION` 的说明。
    pub async fn build(db_path: &str, bootstrap: BootstrapConfig) -> Result<Self> {
        ensure_schema_generation(db_path).await?;

        let url = format!("sqlite://{}?mode=rwc", db_path);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&url)
            .await
            .with_context(|| format!("无法打开 SQLite 数据库: {}", db_path))?;

        // SQLite 默认不开外键约束，必须显式打开，否则级联删除不生效
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .context("开启外键约束失败")?;

        // 建表（内嵌迁移）+ 内置实体类型
        sqlite_db::seed::initialize_embedded(&pool).await?;

        // 记录本次建库的结构代号，供下次启动比对
        sqlx::query("CREATE TABLE IF NOT EXISTS schema_version (generation INTEGER NOT NULL, applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .context("创建 schema_version 表失败")?;
        sqlx::query("INSERT INTO schema_version (generation) VALUES ($1)")
            .bind(sqlite_db::embedded::SCHEMA_GENERATION as i64)
            .execute(&pool)
            .await
            .context("写入 schema 代号失败")?;

        // 首次启动写入 AI 默认配置（已配置过则不动）
        bootstrap_ai_settings(&pool, &bootstrap).await?;

        let defaults = AiSettingsDefaults {
            base_url: bootstrap.base_url.clone(),
            api_key: if bootstrap.api_key.is_empty() {
                None
            } else {
                Some(bootstrap.api_key.clone())
            },
            model: bootstrap.model.clone(),
            ..AiSettingsDefaults::from_env()
        };
        let ai_settings: Arc<dyn AiSettingsPort> =
            Arc::new(DbAiSettingsPort::new(pool.clone(), defaults));

        // 对话 / 生成 / 抽取共用同一个 LLM 端口，设置页改完立即生效
        let mut llm_client = LlmClient::new("opencode".to_string());
        llm_client.add_provider(Arc::new(OpenAiCompatibleProvider::new(
            ai_settings.clone(),
        )));
        let llm = Arc::new(InfraLlmPort::new(llm_client, ai_settings.clone()));

        let agent_tools = Arc::new(ToolRegistry::new());
        agent_tools.register(Arc::new(EchoTool));
        agent_tools.register(Arc::new(AskQuestionTool));
        // 领域工具：与电脑端同一套注册逻辑
        crate::agent_tools::register_all_domain_tools(&agent_tools, &pool);

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

        // 告知是否已配置 AI 网关（前端据此决定要不要弹首次配置引导）
        let provider_configured = ai_settings
            .load()
            .await
            .map(|cfg| !cfg.api_key.as_deref().unwrap_or("").trim().is_empty())
            .unwrap_or(false);
        tracing::info!(
            "AI 网关配置状态: {}",
            if provider_configured { "已配置" } else { "未配置（App 会显示首次配置引导）" }
        );

        let state = AppState::new(pool, agent, ai_settings);
        Ok(Self { state })
    }

    /// 在 127.0.0.1:port 上提供服务（阻塞直到停止）
    pub async fn serve(self, port: u16) -> Result<()> {
        let app = crate::api::router(self.state);

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("无法绑定本地端口 {}", port))?;

        tracing::info!("引擎已启动: http://127.0.0.1:{}", port);

        axum::serve(listener, app).await.context("HTTP 服务异常退出")
    }
}

/// 检查（必要时重建）数据库的 schema 代号。
///
/// 情况有三种：
///   1. 数据库文件不存在 → 全新库，什么都不做
///   2. 代号一致 → 什么都不做
///   3. 代号不一致 → 删除库文件并重建，同时写出一个标记文件
///      （`schema_rebuilt`），供前端读取并提示用户数据已重置
async fn ensure_schema_generation(db_path: &str) -> Result<()> {
    let path = std::path::Path::new(db_path);
    if !path.exists() {
        tracing::info!("全新数据库，将按当前结构创建（schema generation {}）",
            sqlite_db::embedded::SCHEMA_GENERATION);
        return Ok(());
    }

    let url = format!("sqlite://{}?mode=rwc", db_path);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .with_context(|| format!("检查数据库版本时无法打开 {}", db_path))?;

    let existing: Option<i64> = match sqlx::query_scalar::<_, i64>(
        "SELECT generation FROM schema_version ORDER BY rowid DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            // 表不存在 = 老数据库（早于本机制引入）
            tracing::warn!("读取 schema 代号失败（可能是老库）: {}", e);
            None
        }
    };
    pool.close().await;

    let current = sqlite_db::embedded::SCHEMA_GENERATION as i64;
    if existing == Some(current) {
        tracing::info!("数据库结构代号匹配: {}", current);
        return Ok(());
    }

    tracing::warn!(
        "数据库结构已变更（库内 {} / 代码 {}），将重建数据库。原有数据会丢失。",
        existing.map(|v| v.to_string()).unwrap_or_else(|| "无".into()),
        current
    );

    // 删除库文件及其 WAL/SHM 附属文件
    for suffix in ["", "-wal", "-shm"] {
        let p = format!("{}{}", db_path, suffix);
        if std::path::Path::new(&p).exists() {
            std::fs::remove_file(&p).with_context(|| format!("删除旧数据库失败: {}", p))?;
        }
    }

    // 写标记，让 App 能提示用户「数据已重置」
    if let Some(dir) = path.parent() {
        let _ = std::fs::write(dir.join("schema_rebuilt"), current.to_string());
    }

    Ok(())
}

/// 单次输出 token 上限的默认值，与电脑端 `AiSettingsDefaults::from_env` 一致。
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 22_000;

/// 首次启动写入 AI 配置；已有配置则保持不动（仅做旧字段名迁移）。
///
/// 字段名必须与电脑端设置页一致（camelCase）：`DbAiSettingsPort` 读的是
/// `aiBaseUrl` / `aiApiKey` / `defaultModel` / `maxOutputTokens` / `contextLimits`，
/// 若写成 snake_case，设置页保存的配置对引擎完全不可见。
async fn bootstrap_ai_settings(pool: &sqlx::SqlitePool, cfg: &BootstrapConfig) -> Result<()> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT settings FROM app_settings WHERE id = 'default'")
            .fetch_optional(pool)
            .await
            .context("读取 AI 设置失败")?;

    let settings = match existing {
        Some((raw,)) => migrate_legacy_ai_keys(&raw)?,
        None => serde_json::json!({
            "aiBaseUrl": cfg.base_url,
            "aiApiKey": cfg.api_key,
            "defaultModel": cfg.model,
            "contextLimit": domain::model_catalog::DEFAULT_CONTEXT_LIMIT,
            "maxOutputTokens": cfg.max_output_tokens,
        }),
    };

    sqlx::query(
        "INSERT INTO app_settings (id, settings) VALUES ('default', $1) \
         ON CONFLICT (id) DO UPDATE SET settings = EXCLUDED.settings",
    )
    .bind(settings.to_string())
    .execute(pool)
    .await
    .context("写入 AI 设置失败")?;

    Ok(())
}

/// 把早期手机端写下的 snake_case AI 字段名迁移成电脑端的 camelCase。
///
/// 背景：手机端最初的设置页存的是 `base_url` / `api_key` / `model`，
/// 而引擎读的是 `aiBaseUrl` / `aiApiKey` / `defaultModel`，
/// 于是「设置页保存成功但不生效」。这里把旧字段平移过去。
/// camelCase 侧已有值时不覆盖（以用户后来改的为准）。
fn migrate_legacy_ai_keys(raw: &str) -> Result<serde_json::Value> {
    let mut value: serde_json::Value = serde_json::from_str(raw)
        .with_context(|| format!("app_settings.settings 不是合法 JSON: {}", raw))?;
    let Some(obj) = value.as_object_mut() else {
        anyhow::bail!("app_settings.settings 不是 JSON 对象: {}", raw);
    };

    let pairs = [
        ("base_url", "aiBaseUrl"),
        ("api_key", "aiApiKey"),
        ("model", "defaultModel"),
    ];
    let mut moved: Vec<String> = Vec::new();
    for (old, new) in pairs {
        if let Some(v) = obj.remove(old) {
            obj.entry(new.to_string()).or_insert(v);
            moved.push(format!("{} → {}", old, new));
        }
    }
    if !moved.is_empty() {
        tracing::info!("已迁移手机端旧 AI 配置字段名: {}", moved.join(", "));
    }
    Ok(value)
}
