//! Generation Service - 生成任务管理的业务逻辑层
//!
//! 负责生成任务的创建、查询、取消。
//! 所有操作通过 application service，不直接操作数据库。

use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// Generation Service - 生成任务管理服务
pub struct GenerationService {
    pool: PgPool,
}

impl GenerationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 列出项目的生成任务
    pub async fn list_tasks(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<i32>, String)>(
            "SELECT id, task_type, status, COALESCE(model, ''), target_id, result::text, context_tokens, created_at::text \
             FROM generation_task WHERE project_id = $1 ORDER BY created_at DESC"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list generation tasks")?;

        Ok(rows.into_iter().map(|(id, ttype, status, model, target, result, tokens, created)| {
            serde_json::json!({"id": id, "type": ttype, "status": status, "model": model, "target_id": target, "result": result, "context_tokens": tokens, "parameters": {}, "created_at": created, "updated_at": created})
        }).collect())
    }

    /// 获取单个生成任务
    pub async fn get_task(&self, id: Uuid) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<i32>, String)>(
            "SELECT id, task_type, status, COALESCE(model, ''), target_id, result::text, context_tokens, created_at::text \
             FROM generation_task WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get generation task")?;

        Ok(row.map(|(id, ttype, status, model, target, result, tokens, created)| {
            serde_json::json!({"id": id, "type": ttype, "status": status, "model": model, "target_id": target, "result": result, "context_tokens": tokens, "parameters": {}, "created_at": created, "updated_at": created})
        }))
    }

    /// 创建生成任务
    pub async fn create_task(
        &self,
        project_id: Uuid,
        task_type: &str,
        target_id: Option<Uuid>,
        model: Option<&str>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO generation_task (id, project_id, task_type, target_id, model, parameters, status) \
             VALUES ($1, $2, $3, $4, $5, $6, 'Pending')"
        )
        .bind(&id)
        .bind(project_id)
        .bind(task_type)
        .bind(target_id)
        .bind(model)
        .bind(parameters)
        .execute(&self.pool)
        .await
        .context("Failed to create generation task")?;

        self.get_task(id).await?
            .ok_or_else(|| anyhow::anyhow!("Task disappeared after creation"))
    }

    /// 取消生成任务（只允许取消 Pending 或 Running 状态的任务）
    pub async fn cancel_task(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "UPDATE generation_task SET status = 'Cancelled' WHERE id = $1 AND status IN ('Pending', 'Running')"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to cancel generation task")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Cannot cancel task: task not found or not in cancellable state"));
        }

        Ok(())
    }
}
