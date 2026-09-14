//! 对话滚动摘要持久化（session_summary 表）。
//!
//! 每个项目恒定一份：`project_id` 是主键，写入走 `ON CONFLICT DO UPDATE`，
//! 因此重复收尾只会刷新那一份，不会堆积出多份摘要。

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use domain::session_summary::{SessionSummary, SessionSummaryPort, StoredSessionSummary};

pub struct SessionSummaryRepo {
    pool: PgPool,
}

impl SessionSummaryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    content: serde_json::Value,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl SessionSummaryPort for SessionSummaryRepo {
    async fn load(&self, project_id: Uuid) -> Result<Option<StoredSessionSummary>> {
        let row = sqlx::query_as::<_, SummaryRow>(
            "SELECT content, updated_at FROM session_summary WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("读取对话滚动摘要失败")?;

        row.map(|r| {
            let content: SessionSummary =
                serde_json::from_value(r.content).context("session_summary.content 不是合法摘要")?;
            Ok(StoredSessionSummary {
                content,
                updated_at: r.updated_at,
            })
        })
        .transpose()
    }

    async fn save(
        &self,
        project_id: Uuid,
        summary: &SessionSummary,
    ) -> Result<StoredSessionSummary> {
        let content = serde_json::to_value(summary).context("序列化对话摘要失败")?;

        let row = sqlx::query_as::<_, SummaryRow>(
            "INSERT INTO session_summary (project_id, content, updated_at) \
             VALUES ($1, $2, NOW()) \
             ON CONFLICT (project_id) DO UPDATE \
             SET content = EXCLUDED.content, updated_at = NOW() \
             RETURNING content, updated_at",
        )
        .bind(project_id)
        .bind(&content)
        .fetch_one(&self.pool)
        .await
        .context("写入对话滚动摘要失败")?;

        let content: SessionSummary =
            serde_json::from_value(row.content).context("回读摘要不是合法结构")?;
        Ok(StoredSessionSummary {
            content,
            updated_at: row.updated_at,
        })
    }
}
