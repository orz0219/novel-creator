//! Agent 记忆持久化（agent_memory 表）。

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use domain::agent_store::{AgentMemory, MemoryItem};

pub struct MemoryRepo {
    pool: PgPool,
}

impl MemoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgentMemory for MemoryRepo {
    async fn save(&self, project_id: Uuid, memory_type: &str, content: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_memory (project_id, memory_type, content) VALUES ($1, $2, $3)",
        )
        .bind(project_id)
        .bind(memory_type)
        .bind(content)
        .execute(&self.pool)
        .await
        .context("Failed to save agent memory")?;
        Ok(())
    }

    async fn list(&self, project_id: Uuid) -> Result<Vec<MemoryItem>> {
        let rows = sqlx::query_as::<_, MemoryRow>(
            "SELECT memory_type, content, created_at FROM agent_memory \
             WHERE project_id = $1 ORDER BY created_at ASC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list agent memory")?;
        Ok(rows.into_iter().map(|r| r.into_item()).collect())
    }

    async fn replace_by_type(
        &self,
        project_id: Uuid,
        memory_type: &str,
        content: &str,
    ) -> Result<()> {
        // 必须在一个事务里：先删后插之间失败会丢掉摘要，比留着旧摘要更糟
        let mut tx = self
            .pool
            .begin()
            .await
            .context("开启记忆替换事务失败")?;

        sqlx::query("DELETE FROM agent_memory WHERE project_id = $1 AND memory_type = $2")
            .bind(project_id)
            .bind(memory_type)
            .execute(&mut *tx)
            .await
            .context("删除旧记忆条目失败")?;

        sqlx::query(
            "INSERT INTO agent_memory (project_id, memory_type, content) VALUES ($1, $2, $3)",
        )
        .bind(project_id)
        .bind(memory_type)
        .bind(content)
        .execute(&mut *tx)
        .await
        .context("写入新记忆条目失败")?;

        tx.commit().await.context("提交记忆替换事务失败")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct MemoryRow {
    memory_type: String,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl MemoryRow {
    fn into_item(self) -> MemoryItem {
        MemoryItem {
            memory_type: self.memory_type,
            content: self.content,
            created_at: self.created_at,
        }
    }
}
