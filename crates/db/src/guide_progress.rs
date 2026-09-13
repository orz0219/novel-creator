//! `GuideProgressPort` 的 PostgreSQL 实现。
//!
//! 从 `project.config->>'current_step'` 读取引导进度——这是项目级真源
//! （由 `confirm_step` 工具写入 `project.config.current_step`），
//! 供 Agent 构建系统提示词时判断「当前处于哪一步」。

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use domain::ports::GuideProgressPort;

/// 读取项目引导进度的端口实现。
pub struct DbGuideProgressPort {
    pool: PgPool,
}

impl DbGuideProgressPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GuideProgressPort for DbGuideProgressPort {
    async fn current_step(&self, project_id: Uuid) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT config->>'current_step' FROM project WHERE id = $1")
                .bind(project_id)
                .fetch_optional(&self.pool)
                .await
                .context("读取项目引导进度失败")?;

        let (step,) = row.context(format!("项目不存在：{}", project_id))?;
        Ok(step.filter(|s| !s.trim().is_empty()))
    }
}
