//! ⚠️ 本文件由 tmp/gen_sqlite_backend.py 自动生成，请勿手工编辑。
//! 如需修改逻辑，请改 PG 侧的对应文件后重新生成。

//! Agent 记忆持久化（agent_memory 表）。

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use domain::agent_store::{AgentMemory, MemoryItem};

pub struct MemoryRepo {
    pool: SqlitePool,
}

impl MemoryRepo {
    pub fn new(pool: SqlitePool) -> Self {
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
