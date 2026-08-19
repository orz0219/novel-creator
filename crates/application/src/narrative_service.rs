//! Narrative Service - 叙事管理的业务逻辑层
//!
//! 负责叙事节点的创建、更新、删除（软删除）。
//! 所有操作通过 application service，不直接操作数据库。

use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// Narrative Service - 叙事管理服务
pub struct NarrativeService {
    pool: PgPool,
}

impl NarrativeService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 列出项目的叙事节点（排除已删除的）
    pub async fn list_nodes(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>> {
        let rows: Vec<(String, String, String, String, Option<String>, String, Option<String>, String, i32, String, String, String)> = sqlx::query_as(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes::text, sort_order, status, created_at::text, updated_at::text \
             FROM narrative_node WHERE project_id = $1 AND status != 'Deleted' ORDER BY sort_order"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list narrative nodes")?;

        Ok(rows.into_iter().map(|(id, pid, wid, nt, par, title, desc, attrs, ord, st, cr, up)| {
            serde_json::json!({"id": id, "project_id": pid, "world_id": wid, "node_type": nt, "parent_id": par, "title": title, "description": desc, "attributes": serde_json::from_str::<serde_json::Value>(&attrs).unwrap_or(serde_json::json!({})), "sort_order": ord, "status": st, "created_at": cr, "updated_at": up})
        }).collect())
    }

    /// 获取单个叙事节点
    pub async fn get_node(&self, id: Uuid) -> Result<Option<serde_json::Value>> {
        let row: Option<(String, String, String, String, Option<String>, String, Option<String>, String, i32, String, String, String)> = sqlx::query_as(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes::text, sort_order, status, created_at::text, updated_at::text \
             FROM narrative_node WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get narrative node")?;

        Ok(row.map(|(id, pid, wid, nt, par, title, desc, attrs, ord, st, cr, up)| {
            serde_json::json!({"id": id, "project_id": pid, "world_id": wid, "node_type": nt, "parent_id": par, "title": title, "description": desc, "attributes": serde_json::from_str::<serde_json::Value>(&attrs).unwrap_or(serde_json::json!({})), "sort_order": ord, "status": st, "created_at": cr, "updated_at": up})
        }))
    }

    /// 创建叙事节点
    pub async fn create_node(
        &self,
        project_id: Uuid,
        node_type: &str,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = Uuid::new_v4();
        let world_id: (String,) = sqlx::query_as("SELECT id FROM world WHERE project_id = $1 LIMIT 1")
            .bind(project_id)
            .fetch_one(&self.pool)
            .await
            .context("Failed to get world for project")?;

        let sort_order: (i32,) = sqlx::query_as(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM narrative_node WHERE project_id = $1 AND parent_id IS NOT DISTINCT FROM $2"
        )
        .bind(project_id)
        .bind(parent_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to get sort order")?;

        sqlx::query(
            "INSERT INTO narrative_node (id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Draft')"
        )
        .bind(&id)
        .bind(project_id)
        .bind(&world_id.0)
        .bind(node_type)
        .bind(parent_id)
        .bind(title)
        .bind(description)
        .bind(attributes)
        .bind(sort_order.0)
        .execute(&self.pool)
        .await
        .context("Failed to create narrative node")?;

        self.get_node(id).await?
            .ok_or_else(|| anyhow::anyhow!("Node disappeared after creation"))
    }

    /// 更新叙事节点（不能更新已删除的节点）
    pub async fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<serde_json::Value> {
        // 检查节点是否存在且未删除
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM narrative_node WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to check node status")?;

        match exists {
            Some((s,)) if s == "Deleted" => {
                return Err(anyhow::anyhow!("Cannot update deleted narrative node"));
            }
            None => {
                return Err(anyhow::anyhow!("Narrative node not found"));
            }
            _ => {}
        }

        if let Some(t) = title {
            sqlx::query("UPDATE narrative_node SET title=$1, updated_at=NOW() WHERE id=$2")
                .bind(t).bind(id).execute(&self.pool).await?;
        }
        if let Some(d) = description {
            sqlx::query("UPDATE narrative_node SET description=$1, updated_at=NOW() WHERE id=$2")
                .bind(d).bind(id).execute(&self.pool).await?;
        }
        if let Some(s) = status {
            sqlx::query("UPDATE narrative_node SET status=$1, updated_at=NOW() WHERE id=$2")
                .bind(s).bind(id).execute(&self.pool).await?;
        }

        self.get_node(id).await?
            .ok_or_else(|| anyhow::anyhow!("Node disappeared after update"))
    }

    /// 软删除叙事节点
    pub async fn delete_node(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "UPDATE narrative_node SET status = 'Deleted', updated_at = NOW() WHERE id = $1 AND status != 'Deleted'"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to delete narrative node")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Narrative node not found or already deleted"));
        }

        Ok(())
    }
}
