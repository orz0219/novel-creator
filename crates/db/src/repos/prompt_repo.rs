//! Agent 提示词持久化（agent_prompts 表）。

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use domain::ports::{AgentPromptConfig, PromptRepositoryPort};

pub struct PromptRepo {
    pool: PgPool,
}

impl PromptRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PromptRow {
    id: Uuid,
    scope: String,
    project_id: Option<Uuid>,
    system_prompt: String,
    updated_at: DateTime<Utc>,
}

#[async_trait]
impl PromptRepositoryPort for PromptRepo {
    async fn load(&self, scope: &str) -> Result<Option<AgentPromptConfig>> {
        let row = sqlx::query_as::<_, PromptRow>(
            "SELECT id, scope, project_id, system_prompt, updated_at FROM agent_prompts WHERE scope = $1",
        )
        .bind(scope)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to load agent prompt")?;

        Ok(row.map(|r| AgentPromptConfig {
            id: r.id,
            scope: r.scope,
            project_id: r.project_id,
            system_prompt: r.system_prompt,
            updated_at: r.updated_at,
        }))
    }

    async fn save(&self, config: &AgentPromptConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_prompts (id, scope, project_id, system_prompt, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, NOW(), NOW()) \
             ON CONFLICT (scope) DO UPDATE SET system_prompt = EXCLUDED.system_prompt, updated_at = NOW()",
        )
        .bind(config.id)
        .bind(&config.scope)
        .bind(config.project_id)
        .bind(&config.system_prompt)
        .execute(&self.pool)
        .await
        .context("Failed to save agent prompt")?;
        Ok(())
    }

    async fn delete(&self, scope: &str) -> Result<()> {
        sqlx::query("DELETE FROM agent_prompts WHERE scope = $1")
            .bind(scope)
            .execute(&self.pool)
            .await
            .context("Failed to delete agent prompt")?;
        Ok(())
    }
}
