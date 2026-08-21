//! Application 层端口的数据库实现（P3）。
//!
//! GenerationRepositoryPort 的具体 SQL 实现。原本这些查询位于
//! application::generation_service 中；P3 把它们下移到 db（依赖倒置：
//! application 依赖端口，db 实现端口）。

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use domain::ports::{
    ApprovalRepositoryPort, ContextSnapshotRepositoryPort, EntityRepositoryPort,
    ForeshadowRepositoryPort, GenerationRepositoryPort, HistoryRepositoryPort,
    NarrativeRepositoryPort, NarrativeStateWritePort, ProjectRepositoryPort, ProposalRepositoryPort,
    RuleRepositoryPort, SnapshotRepositoryPort, StorylineRepositoryPort, TimelineRepositoryPort,
    WorldRepositoryPort,
};
use domain::*;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbGenerationRepositoryPort {
    pool: PgPool,
}

impl DbGenerationRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GenerationRepositoryPort for DbGenerationRepositoryPort {
    async fn list_tasks(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<i32>, String)>(
            "SELECT id::text, task_type, status, COALESCE(model, ''), target_id::text, result::text, context_tokens, created_at::text              FROM generation_task WHERE project_id = $1 ORDER BY created_at DESC"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list generation tasks")?;

        Ok(rows
            .into_iter()
            .map(|(id, ttype, status, model, target, result, tokens, created)| {
                serde_json::json!({
                    "id": id, "type": ttype, "status": status, "model": model,
                    "target_id": target, "result": result, "context_tokens": tokens,
                    "parameters": {}, "created_at": created, "updated_at": created
                })
            })
            .collect())
    }

    async fn get_task(&self, id: Uuid) -> Result<Option<Value>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<i32>, String)>(
            "SELECT id::text, task_type, status, COALESCE(model, ''), target_id::text, result::text, context_tokens, created_at::text              FROM generation_task WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get generation task")?;

        Ok(row.map(|(id, ttype, status, model, target, result, tokens, created)| {
            serde_json::json!({
                "id": id, "type": ttype, "status": status, "model": model,
                "target_id": target, "result": result, "context_tokens": tokens,
                "parameters": {}, "created_at": created, "updated_at": created
            })
        }))
    }

    async fn create_task(
        &self,
        project_id: Uuid,
        task_type: &str,
        target_id: Option<Uuid>,
        model: Option<&str>,
        parameters: Value,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO generation_task (id, project_id, task_type, target_id, model, parameters, status)              VALUES ($1, $2, $3, $4, $5, $6, 'Pending')",
        )
        .bind(&id)
        .bind(project_id)
        .bind(task_type)
        .bind(target_id)
        .bind(model)
        .bind(&parameters)
        .execute(&self.pool)
        .await
        .context("Failed to create generation task")?;

        self.get_task(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task disappeared after creation"))
    }

    async fn cancel_task(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "UPDATE generation_task SET status = 'Cancelled' WHERE id = $1 AND status IN ('Pending', 'Running') AND project_id = (SELECT project_id FROM generation_task WHERE id = $2)"
        )
        .bind(&id)
        .bind(&id)
        .execute(&self.pool)
        .await
        .context("Failed to cancel generation task")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Cannot cancel task: task not found or not in cancellable state"
            ));
        }

        Ok(())
    }

    async fn get_task_struct(&self, id: Uuid) -> Result<Option<domain::generation::GenerationTask>> {
        crate::repos::generation_repo::TaskRepo::new(self.pool.clone())
            .get_by_id(id)
            .await
    }

    async fn get_skill_by_id(&self, id: Uuid) -> Result<Option<domain::generation::Skill>> {
        crate::repos::generation_repo::SkillRepo::new(self.pool.clone())
            .get_by_id(id)
            .await
    }

    async fn update_task_output(&self, id: Uuid, output: serde_json::Value) -> Result<()> {
        crate::repos::generation_repo::TaskRepo::new(self.pool.clone())
            .update_output(id, output)
            .await
    }

    async fn create_run(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        context_snapshot_id: Option<Uuid>,
        llm_model: &str,
        provider: Option<&str>,
        prompt_sent: &str,
        response_received: &str,
        token_usage: Option<serde_json::Value>,
        latency_ms: Option<i64>,
        reproducibility: domain::generation::ReproducibilityMeta,
    ) -> Result<()> {
        crate::repos::generation_repo::RunRepo::new(self.pool.clone())
            .create(
                project_id,
                task_id,
                context_snapshot_id,
                llm_model,
                provider,
                prompt_sent,
                response_received,
                token_usage,
                latency_ms,
                &reproducibility,
            )
            .await
    }
}

pub struct DbNarrativeRepositoryPort {
    pool: PgPool,
}

impl DbNarrativeRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NarrativeRepositoryPort for DbNarrativeRepositoryPort {
    async fn list_nodes(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(Uuid, Uuid, Uuid, String, Option<Uuid>, String, Option<String>, Option<String>, String, i32, String, String, String)> =
            sqlx::query_as(
                "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes::text, sort_order, status, created_at::text, updated_at::text                  FROM narrative_node WHERE project_id = $1 AND status != 'Deleted' ORDER BY sort_order"
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list narrative nodes")?;

        Ok(rows
            .into_iter()
            .map(|(id, pid, wid, nt, par, title, desc, content, attrs, ord, st, cr, up)| {
                serde_json::json!({
                    "id": id, "project_id": pid, "world_id": wid, "node_type": nt,
                    "parent_id": par, "title": title, "description": desc, "content": content,
                    "attributes": serde_json::from_str::<Value>(&attrs).unwrap_or(serde_json::json!({})),
                    "sort_order": ord, "status": st, "created_at": cr, "updated_at": up
                })
            })
            .collect())
    }

    async fn get_node(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(Uuid, Uuid, Uuid, String, Option<Uuid>, String, Option<String>, Option<String>, String, i32, String, String, String)> =
            sqlx::query_as(
                "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes::text, sort_order, status, created_at::text, updated_at::text                  FROM narrative_node WHERE id = $1"
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get narrative node")?;

        Ok(row.map(|(id, pid, wid, nt, par, title, desc, content, attrs, ord, st, cr, up)| {
            serde_json::json!({
                "id": id, "project_id": pid, "world_id": wid, "node_type": nt,
                "parent_id": par, "title": title, "description": desc, "content": content,
                "attributes": serde_json::from_str::<Value>(&attrs).unwrap_or(serde_json::json!({})),
                "sort_order": ord, "status": st, "created_at": cr, "updated_at": up
            })
        }))
    }

    async fn create_node(
        &self,
        project_id: Uuid,
        node_type: &str,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
        attributes: Value,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        let world_id: (Uuid,) = sqlx::query_as("SELECT id FROM world WHERE project_id = $1 LIMIT 1")
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
            "INSERT INTO narrative_node (id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Draft')"
        )
        .bind(&id)
        .bind(project_id)
        .bind(&world_id.0)
        .bind(node_type)
        .bind(parent_id)
        .bind(title)
        .bind(description)
        .bind(&attributes)
        .bind(sort_order.0)
        .execute(&self.pool)
        .await
        .context("Failed to create narrative node")?;

        self.get_node(id).await?
            .ok_or_else(|| anyhow::anyhow!("Node disappeared after creation"))
    }

    async fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<Value> {
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
            sqlx::query("UPDATE narrative_node SET title=$1, updated_at=NOW() WHERE id=$2 AND project_id = (SELECT project_id FROM narrative_node WHERE id = $3)")
                .bind(t).bind(id).bind(id).execute(&self.pool).await?;
        }
        if let Some(d) = description {
            sqlx::query("UPDATE narrative_node SET description=$1, updated_at=NOW() WHERE id=$2 AND project_id = (SELECT project_id FROM narrative_node WHERE id = $3)")
                .bind(d).bind(id).bind(id).execute(&self.pool).await?;
        }
        if let Some(s) = status {
            sqlx::query("UPDATE narrative_node SET status=$1, updated_at=NOW() WHERE id=$2 AND project_id = (SELECT project_id FROM narrative_node WHERE id = $3)")
                .bind(s).bind(id).bind(id).execute(&self.pool).await?;
        }

        self.get_node(id).await?
            .ok_or_else(|| anyhow::anyhow!("Node disappeared after update"))
    }

    async fn delete_node(&self, id: Uuid) -> Result<()> {
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

// ---------------------------------------------------------------------------
// ApprovalRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbApprovalRepositoryPort {
    pool: PgPool,
}

impl DbApprovalRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApprovalRepositoryPort for DbApprovalRepositoryPort {
    async fn create(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        content: Value,
    ) -> Result<ApprovalRecord> {
        crate::repos::approval_repo::ApprovalRepo::new(self.pool.clone())
            .create(project_id, target_type, target_id, proposed_by, content)
            .await
    }

    async fn approve(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        crate::repos::approval_repo::ApprovalRepo::new(self.pool.clone())
            .approve(record_id, reviewer_id, comment)
            .await
    }

    async fn reject(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        crate::repos::approval_repo::ApprovalRepo::new(self.pool.clone())
            .reject(record_id, reviewer_id, comment)
            .await
    }

    async fn list_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>> {
        crate::repos::approval_repo::ApprovalRepo::new(self.pool.clone())
            .list_pending(project_id)
            .await
    }
}

// ---------------------------------------------------------------------------
// ProposalRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbProposalRepositoryPort {
    pool: PgPool,
}

impl DbProposalRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ProposedChangeRow {
    id: Uuid,
    project_id: Uuid,
    task_id: Option<Uuid>,
    change_type: String,
    target_entity_id: Uuid,
    description: String,
    payload: Option<serde_json::Value>,
    status: String,
    created_at: chrono::DateTime<Utc>,
    resolved_at: Option<chrono::DateTime<Utc>>,
}

impl From<ProposedChangeRow> for ProposedChange {
    fn from(r: ProposedChangeRow) -> Self {
        ProposedChange {
            id: r.id,
            project_id: r.project_id,
            task_id: r.task_id,
            change_type: crate::ser::parse_proposed_change_type(&r.change_type),
            target_entity_id: r.target_entity_id,
            description: r.description,
            payload: r.payload.unwrap_or_default(),
            status: crate::ser::parse_proposed_change_status(&r.status),
            created_at: r.created_at,
            resolved_at: r.resolved_at,
        }
    }
}

#[async_trait]
impl ProposalRepositoryPort for DbProposalRepositoryPort {
    async fn list_proposals(&self, project_id: Uuid) -> Result<Vec<ProposedChange>> {
        let rows = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at \
             FROM proposed_change WHERE project_id = $1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list proposals")?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_proposal(&self, id: Uuid) -> Result<Option<ProposedChange>> {
        let row = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at \
             FROM proposed_change WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get proposal")?;
        Ok(row.map(Into::into))
    }

    async fn create_proposal(
        &self,
        project_id: Uuid,
        task_id: Option<Uuid>,
        change_type: ProposedChangeType,
        target_entity_id: Uuid,
        description: &str,
        payload: Value,
    ) -> Result<ProposedChange> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let ct_str = crate::ser::proposed_change_type_str(&change_type);
        sqlx::query(
            "INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'Draft', $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(task_id)
        .bind(&ct_str)
        .bind(target_entity_id)
        .bind(description)
        .bind(&payload)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create proposal")?;
        Ok(ProposedChange {
            id,
            project_id,
            task_id,
            change_type,
            target_entity_id,
            description: description.to_string(),
            payload,
            status: ProposedChangeStatus::Draft,
            created_at: now,
            resolved_at: None,
        })
    }

    async fn approve_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        let current = self
            .get_proposal(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;
        // 批准即提交：允许从 Draft 直接批准（隐式完成校验），
        // 同时保留 Valid -> Approved 等既有流转（can_transition_to 不变）。
        if !(current
            .status
            .can_transition_to(&ProposedChangeStatus::Approved)
            || current.status == ProposedChangeStatus::Draft)
        {
            return Err(anyhow::anyhow!(
                "Invalid state transition: {} -> {}",
                current.status.description(),
                ProposedChangeStatus::Approved.description()
            ));
        }
        let status_str = crate::ser::proposed_change_status_str(&current.status);
        let result = sqlx::query(
            "UPDATE proposed_change SET status = 'Approved', resolved_at = NOW() WHERE id = $1 AND status = $2 AND project_id = (SELECT project_id FROM proposed_change WHERE id = $3)",
        )
        .bind(id)
        .bind(&status_str)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to approve proposal")?;
        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Concurrent modification: proposal status changed during update"
            ));
        }
        self.get_proposal(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Proposal disappeared after update"))
    }

    async fn reject_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        let current = self
            .get_proposal(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;
        if !(current
            .status
            .can_transition_to(&ProposedChangeStatus::Rejected)
            || current.status == ProposedChangeStatus::Draft)
        {
            return Err(anyhow::anyhow!(
                "Invalid state transition: {} -> {}",
                current.status.description(),
                ProposedChangeStatus::Rejected.description()
            ));
        }
        let status_str = crate::ser::proposed_change_status_str(&current.status);
        let result = sqlx::query(
            "UPDATE proposed_change SET status = 'Rejected', resolved_at = NOW() WHERE id = $1 AND status = $2 AND project_id = (SELECT project_id FROM proposed_change WHERE id = $3)",
        )
        .bind(id)
        .bind(&status_str)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to reject proposal")?;
        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Concurrent modification: proposal status changed during update"
            ));
        }
        self.get_proposal(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Proposal disappeared after update"))
    }
}

// ---------------------------------------------------------------------------
// TimelineRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbTimelineRepositoryPort {
    pool: PgPool,
}

impl DbTimelineRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TimelineRepositoryPort for DbTimelineRepositoryPort {
    async fn list_events_by_project(&self, project_id: Uuid) -> Result<Vec<Event>> {
        crate::repos::event_repo::EventRepo::new(self.pool.clone())
            .list_by_project(project_id)
            .await
    }
}

// ---------------------------------------------------------------------------
// StorylineRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbStorylineRepositoryPort {
    pool: PgPool,
}

impl DbStorylineRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StorylineRepositoryPort for DbStorylineRepositoryPort {
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Storyline>> {
        crate::repos::storyline_repo::StorylineRepo::new(self.pool.clone())
            .list_by_project(project_id)
            .await
    }

    async fn list_storylines(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, Option<String>, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, name, description, status, importance, created_at::text, updated_at::text FROM storyline WHERE project_id=$1",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list storylines")?;
        Ok(rows
            .into_iter()
            .map(|(id, name, desc, st, imp, cr, up)| {
                serde_json::json!({
                    "id": id, "project_id": project_id.to_string(), "name": name,
                    "description": desc, "status": st, "importance": imp,
                    "created_at": cr, "updated_at": up
                })
            })
            .collect())
    }

    async fn create_storyline(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: &str,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO storyline (id, project_id, name, description, status, importance) VALUES ($1,$2,$3,$4,'Planned',$5)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(importance)
        .execute(&self.pool)
        .await
        .context("Failed to create storyline")?;
        Ok(serde_json::json!({
            "id": id.to_string(),
            "project_id": project_id.to_string(),
            "name": name,
            "status": "Planned"
        }))
    }

    async fn update_storyline(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        sqlx::query(
            "UPDATE storyline SET name=$1, description=$2, updated_at=NOW() WHERE id=$3 AND project_id = (SELECT project_id FROM storyline WHERE id = $4)",
        )
        .bind(name)
        .bind(description)
        .bind(id)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update storyline")?;
        Ok(serde_json::json!({ "id": id.to_string(), "updated": true }))
    }

    async fn delete_storyline(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM storyline WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete storyline")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ForeshadowRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbForeshadowRepositoryPort {
    pool: PgPool,
}

impl DbForeshadowRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ForeshadowRepositoryPort for DbForeshadowRepositoryPort {
    async fn list_foreshadows(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, Option<String>, String, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, name, description, status, importance, hint_level, created_at::text, updated_at::text FROM foreshadowing WHERE project_id=$1",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list foreshadows")?;
        Ok(rows
            .into_iter()
            .map(|(id, name, desc, st, imp, hint, cr, up)| {
                serde_json::json!({
                    "id": id, "project_id": project_id.to_string(), "name": name,
                    "description": desc, "status": st, "importance": imp,
                    "hint_level": hint, "related_entity_ids": [],
                    "created_at": cr, "updated_at": up
                })
            })
            .collect())
    }

    async fn create_foreshadow(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: &str,
        hint_level: &str,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO foreshadowing (id, project_id, name, description, status, importance, hint_level) VALUES ($1,$2,$3,$4,'Planned',$5,$6)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(importance)
        .bind(hint_level)
        .execute(&self.pool)
        .await
        .context("Failed to create foreshadow")?;
        Ok(serde_json::json!({
            "id": id.to_string(),
            "project_id": project_id.to_string(),
            "name": name,
            "status": "Planned"
        }))
    }

    async fn update_foreshadow(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        sqlx::query(
            "UPDATE foreshadowing SET name=$1, description=$2, updated_at=NOW() WHERE id=$3 AND project_id = (SELECT project_id FROM foreshadowing WHERE id = $4)",
        )
        .bind(name)
        .bind(description)
        .bind(id)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update foreshadow")?;
        Ok(serde_json::json!({ "id": id.to_string(), "updated": true }))
    }

    async fn delete_foreshadow(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM foreshadowing WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete foreshadow")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WorldRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbWorldRepositoryPort {
    pool: PgPool,
}

impl DbWorldRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorldRepositoryPort for DbWorldRepositoryPort {
    async fn create_world(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World> {
        crate::repos::world_repo::WorldRepo::new(self.pool.clone())
            .create(project_id, name, description, world_rules, is_main)
            .await
    }

    async fn get_world(&self, world_id: Uuid) -> Result<Option<World>> {
        crate::repos::world_repo::WorldRepo::new(self.pool.clone())
            .get_by_id(world_id)
            .await
    }

    async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        crate::repos::world_repo::WorldRepo::new(self.pool.clone())
            .get_main_world(project_id)
            .await
    }

    async fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World> {
        crate::repos::world_repo::WorldRepo::new(self.pool.clone())
            .ensure_main_world(project_id, project_name)
            .await
    }

    async fn create_entity(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: Value,
    ) -> Result<Entity> {
        let type_repo = crate::repos::entity_repo::EntityTypeRepo::new(self.pool.clone());
        let entity_type = type_repo.ensure(entity_type_name, None).await?;
        let entity_repo = crate::repos::entity_repo::EntityRepo::new(self.pool.clone());
        entity_repo
            .create(project_id, world_id, entity_type.id, name, summary, description, attributes)
            .await
    }

    async fn get_entity(&self, project_id: Uuid, entity_id: Uuid) -> Result<Option<Entity>> {
        crate::repos::entity_repo::EntityRepo::new(self.pool.clone())
            .get_by_id_with_project(project_id, entity_id)
            .await
    }

    async fn list_entities(&self, project_id: Uuid) -> Result<Vec<Entity>> {
        crate::repos::entity_repo::EntityRepo::new(self.pool.clone())
            .list_by_project(project_id)
            .await
    }

    async fn list_entities_by_type(
        &self,
        project_id: Uuid,
        entity_type_name: &str,
    ) -> Result<Vec<Entity>> {
        let type_repo = crate::repos::entity_repo::EntityTypeRepo::new(self.pool.clone());
        let entity_type = type_repo
            .get_by_name(entity_type_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Entity type not found: {}", entity_type_name))?;
        crate::repos::entity_repo::EntityRepo::new(self.pool.clone())
            .list_by_type(project_id, entity_type.id)
            .await
    }

    async fn create_relation(
        &self,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: Value,
    ) -> Result<Relation> {
        crate::repos::entity_repo::RelationRepo::new(self.pool.clone())
            .create(
                project_id,
                source_entity_id,
                target_entity_id,
                relation_type,
                description,
                attributes,
            )
            .await
    }

    async fn list_relations(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<Relation>> {
        crate::repos::entity_repo::RelationRepo::new(self.pool.clone())
            .list_by_entity(project_id, entity_id)
            .await
    }

    async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact> {
        crate::repos::entity_repo::FactRepo::new(self.pool.clone())
            .create(project_id, content, category, certainty, related_entity_ids)
            .await
    }

    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Fact>> {
        crate::repos::entity_repo::FactRepo::new(self.pool.clone())
            .list_by_project(project_id)
            .await
    }

    async fn set_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: Value,
    ) -> Result<CurrentState> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;
        let current = crate::repos::state_repo::StateRepo::get_current_state_tx(
            &mut *tx, project_id, entity_id, state_key,
        )
        .await?;
        let expected_version = current.as_ref().map(|s| s.version);
        let old_value = current.map(|s| s.state_value);
        crate::repos::state_repo::StateRepo::record_change_tx(
            &mut *tx, project_id, None, "SET", entity_id, state_key, old_value,
            state_value.clone(), Some("system"),
        )
        .await?;
        let state = crate::repos::state_repo::StateRepo::upsert_state_tx(
            &mut *tx, project_id, entity_id, state_key, state_value, expected_version,
        )
        .await?;
        tx.commit().await.context("Failed to commit transaction")?;
        Ok(state)
    }

    async fn get_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        crate::repos::state_repo::StateRepo::new(self.pool.clone())
            .get_current_state(project_id, entity_id, state_key)
            .await
    }

    async fn list_entity_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>> {
        crate::repos::state_repo::StateRepo::new(self.pool.clone())
            .list_current_states(project_id, entity_id)
            .await
    }

    async fn upsert_resource(
        &self,
        project_id: Uuid,
        location_id: Uuid,
        resource_name: &str,
        quantity: Option<f64>,
        production_rate: Option<f64>,
        controlled_by: Option<Uuid>,
    ) -> Result<ResourceState> {
        crate::repos::state_repo::StateRepo::new(self.pool.clone())
            .upsert_resource(
                project_id,
                location_id,
                resource_name,
                quantity,
                production_rate,
                controlled_by,
            )
            .await
    }

    async fn list_resources(&self, location_id: Uuid) -> Result<Vec<ResourceState>> {
        crate::repos::state_repo::StateRepo::new(self.pool.clone())
            .list_resources_by_location(location_id)
            .await
    }

    async fn record_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        involved_entity_ids: &[Uuid],
        state_changes: Vec<StateChange>,
    ) -> Result<Event> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;
        let id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO event (id, project_id, name, description, event_type, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(event_type)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .context("Failed to insert event")?;

        for entity_id in involved_entity_ids {
            sqlx::query("INSERT INTO event_entity (id, event_id, entity_id) VALUES ($1, $2, $3)")
                .bind(Uuid::new_v4())
                .bind(id)
                .bind(entity_id)
                .execute(&mut *tx)
                .await
                .context("Failed to insert event_entity")?;
        }

        for change in &state_changes {
            let current = crate::repos::state_repo::StateRepo::get_current_state_tx(
                &mut *tx,
                project_id,
                change.target_entity_id,
                &change.state_key,
            )
            .await?;
            let expected_version = current.as_ref().map(|s| s.version);
            let old_value = current.map(|s| s.state_value);
            crate::repos::state_repo::StateRepo::record_change_tx(
                &mut *tx,
                project_id,
                Some(id),
                "EVENT",
                change.target_entity_id,
                &change.state_key,
                old_value,
                change.new_value.clone(),
                Some("event"),
            )
            .await?;
            crate::repos::state_repo::StateRepo::upsert_state_tx(
                &mut *tx,
                project_id,
                change.target_entity_id,
                &change.state_key,
                change.new_value.clone(),
                expected_version,
            )
            .await?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(Event {
            id,
            project_id,
            name: name.to_string(),
            description: description.to_string(),
            event_type: event_type.map(|s| s.to_string()),
            timestamp: None,
            event_time: None,
            duration: None,
            involved_entity_ids: involved_entity_ids.to_vec(),
            state_changes,
            created_at: now,
            updated_at: now,
        })
    }

    async fn get_or_create_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        let world_repo = crate::repos::world_repo::WorldRepo::new(self.pool.clone());
        if let Some(w) = world_repo.get_main_world(project_id).await? {
            return Ok(Some(w));
        }
        let name: Option<(String,)> = sqlx::query_as("SELECT name FROM project WHERE id = $1")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to load project for auto world creation")?;
        let name = match name {
            Some((n,)) => n,
            None => return Ok(None),
        };
        Ok(Some(world_repo.ensure_main_world(project_id, &name).await?))
    }

    async fn update_main_world(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        world_rules: Option<&str>,
    ) -> Result<World> {
        if let Some(name) = name {
            sqlx::query(
                "UPDATE world SET name = $1, updated_at = NOW() WHERE project_id = $2 AND is_main = true",
            )
            .bind(name)
            .bind(project_id)
            .execute(&self.pool)
            .await
            .context("Failed to update world name")?;
        }
        if let Some(description) = description {
            sqlx::query(
                "UPDATE world SET description = $1, updated_at = NOW() WHERE project_id = $2 AND is_main = true",
            )
            .bind(description)
            .bind(project_id)
            .execute(&self.pool)
            .await
            .context("Failed to update world description")?;
        }
        if let Some(world_rules) = world_rules {
            sqlx::query(
                "UPDATE world SET world_rules = $1, updated_at = NOW() WHERE project_id = $2 AND is_main = true",
            )
            .bind(world_rules)
            .bind(project_id)
            .execute(&self.pool)
            .await
            .context("Failed to update world rules")?;
        }
        crate::repos::world_repo::WorldRepo::new(self.pool.clone())
            .get_main_world(project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Main world not found for project {}", project_id))
    }
}
// ---------------------------------------------------------------------------
// ProjectRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbProjectRepositoryPort {
    pool: PgPool,
}

impl DbProjectRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepositoryPort for DbProjectRepositoryPort {
    async fn list_projects(&self) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, Option<String>, Option<String>, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, name, description, language, COALESCE(status, 'Concept'), COALESCE(config::text, '{}'), created_at::text, updated_at::text FROM project ORDER BY updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .context("Failed to list projects")?;

        Ok(rows
            .into_iter()
            .map(|(id, name, desc, lang, status, config, created, updated)| {
                serde_json::json!({
                    "id": id, "name": name, "description": desc, "language": lang,
                    "status": status,
                    "config": serde_json::from_str::<Value>(&config).unwrap_or_default(),
                    "default_params": {}, "created_at": created, "updated_at": updated
                })
            })
            .collect())
    }

    async fn get_project(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(String, String, Option<String>, Option<String>, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, name, description, language, COALESCE(status, 'Concept'), COALESCE(config::text, '{}'), created_at::text, updated_at::text FROM project WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get project")?;

        Ok(row.map(|(id, name, desc, lang, status, config, created, updated)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "language": lang,
                "status": status,
                "config": serde_json::from_str::<Value>(&config).unwrap_or_default(),
                "default_params": {}, "created_at": created, "updated_at": updated
            })
        }))
    }

    async fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        language: Option<&str>,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO project (id, name, description, language, status, config) VALUES ($1, $2, $3, $4, 'Concept', '{}')",
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(language)
        .execute(&self.pool)
        .await
        .context("Failed to create project")?;

        // Auto-create main world (best-effort, mirrors host semantics).
        let world_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO world (id, project_id, name, description, config, is_main) VALUES ($1, $2, $3, $4, '{}', true)",
        )
        .bind(&world_id)
        .bind(&id)
        .bind(name)
        .bind(description)
        .execute(&self.pool)
        .await
        .ok();

        self.get_project(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project disappeared after creation"))
    }

    async fn update_project(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<Value> {
        if let Some(name) = name {
            sqlx::query("UPDATE project SET name = $1, updated_at = NOW() WHERE id = $2")
                .bind(name)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(description) = description {
            sqlx::query("UPDATE project SET description = $1, updated_at = NOW() WHERE id = $2")
                .bind(description)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(status) = status {
            sqlx::query("UPDATE project SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(status)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        self.get_project(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found"))
    }

    async fn delete_project(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM project WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete project")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RuleRepositoryPort (canon_rule)
// ---------------------------------------------------------------------------

pub struct DbRuleRepositoryPort {
    pool: PgPool,
}

impl DbRuleRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RuleRepositoryPort for DbRuleRepositoryPort {
    async fn list_rules(&self, world_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, String, String, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, project_id::text, world_id::text, COALESCE(rule_level, ''), rule_content, COALESCE(affected_scope, ''), enforcement, created_at::text, updated_at::text FROM canon_rule WHERE world_id = $1 ORDER BY created_at",
            )
            .bind(world_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list rules")?;

        Ok(rows
            .into_iter()
            .map(|(id, pid, wid, level, content, scope, enforce, cr, up)| {
                serde_json::json!({
                    "id": id, "project_id": pid, "world_id": wid,
                    "rule_level": level, "rule_content": content,
                    "affected_scope": scope, "enforcement": enforce,
                    "created_at": cr, "updated_at": up
                })
            })
            .collect())
    }

    async fn create_rule(
        &self,
        world_id: Uuid,
        rule_content: &str,
        rule_level: Option<&str>,
        affected_scope: Option<&str>,
        enforcement: Option<&str>,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        let project_id: (Uuid,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1")
            .bind(world_id)
            .fetch_one(&self.pool)
            .await
            .context("Failed to resolve project for world")?;

        sqlx::query(
            "INSERT INTO canon_rule (id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(&project_id.0)
        .bind(world_id)
        .bind(rule_level.unwrap_or("RULE-2"))
        .bind(rule_content)
        .bind(affected_scope.unwrap_or("general"))
        .bind(enforcement.unwrap_or("Allow"))
        .execute(&self.pool)
        .await
        .context("Failed to create rule")?;

        self.get_rule(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Rule disappeared after creation"))
    }

    async fn get_rule(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(String, String, String, String, String, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, project_id::text, world_id::text, COALESCE(rule_level, ''), rule_content, COALESCE(affected_scope, ''), enforcement, created_at::text, updated_at::text FROM canon_rule WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get rule")?;

        Ok(row.map(|(id, pid, wid, level, content, scope, enforce, cr, up)| {
            serde_json::json!({
                "id": id, "project_id": pid, "world_id": wid,
                "rule_level": level, "rule_content": content,
                "affected_scope": scope, "enforcement": enforce,
                "created_at": cr, "updated_at": up
            })
        }))
    }

    async fn update_rule(
        &self,
        id: Uuid,
        rule_content: Option<&str>,
        rule_level: Option<&str>,
    ) -> Result<Value> {
        let maybe_project_id: Option<(Uuid,)> =
            sqlx::query_as("SELECT project_id FROM canon_rule WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to check rule")?;
        let project_id = match maybe_project_id {
            Some((pid,)) => pid,
            None => return Err(anyhow::anyhow!("Rule not found")),
        };

        if let Some(content) = rule_content {
            sqlx::query(
                "UPDATE canon_rule SET rule_content = $1, updated_at = NOW() WHERE id = $2 AND project_id = $3",
            )
            .bind(content)
            .bind(id)
            .bind(&project_id)
            .execute(&self.pool)
            .await?;
        }
        if let Some(level) = rule_level {
            sqlx::query(
                "UPDATE canon_rule SET rule_level = $1, updated_at = NOW() WHERE id = $2 AND project_id = $3",
            )
            .bind(level)
            .bind(id)
            .bind(&project_id)
            .execute(&self.pool)
            .await?;
        }

        self.get_rule(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Rule disappeared after update"))
    }

    async fn delete_rule(&self, id: Uuid) -> Result<()> {
        let rows = sqlx::query(
            "DELETE FROM canon_rule WHERE id = $1 AND project_id = (SELECT project_id FROM canon_rule WHERE id = $2)",
        )
        .bind(&id)
        .bind(&id)
        .execute(&self.pool)
        .await
        .context("Failed to delete rule")?;
        if rows.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Rule not found"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// HistoryRepositoryPort (event / fact)
// ---------------------------------------------------------------------------

pub struct DbHistoryRepositoryPort {
    pool: PgPool,
}

impl DbHistoryRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HistoryRepositoryPort for DbHistoryRepositoryPort {
    async fn list_events(&self, project_id: Uuid, limit: i64) -> Result<Vec<Value>> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String)>(
            "SELECT id::text, name, description, event_type, timestamp, created_at::text FROM event WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list events")?;

        Ok(rows
            .into_iter()
            .map(|(id, name, desc, etype, ts, created)| {
                serde_json::json!({
                    "id": id, "name": name, "description": desc,
                    "event_type": etype, "timestamp": ts, "created_at": created
                })
            })
            .collect())
    }

    async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO event (id, project_id, name, description) VALUES ($1, $2, $3, $4)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .execute(&self.pool)
        .await
        .context("Failed to create event")?;
        Ok(serde_json::json!({
            "id": id.to_string(), "name": name, "description": description
        }))
    }

    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
            "SELECT id::text, content, category, certainty, created_at::text FROM fact WHERE project_id = $1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list facts")?;

        Ok(rows
            .into_iter()
            .map(|(id, content, cat, cert, created)| {
                serde_json::json!({
                    "id": id,
                    "project_id": project_id.to_string(),
                    "content": content,
                    "category": cat,
                    "certainty": cert,
                    "created_at": created,
                    "updated_at": created
                })
            })
            .collect())
    }

    async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO fact (id, project_id, content, category, certainty) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(content)
        .bind(category)
        .bind(certainty)
        .execute(&self.pool)
        .await
        .context("Failed to create fact")?;
        Ok(serde_json::json!({
            "id": id.to_string(),
            "project_id": project_id.to_string(),
            "content": content,
            "category": category,
            "certainty": certainty
        }))
    }
}

// ---------------------------------------------------------------------------
// SnapshotRepositoryPort (novel_state_snapshot)
// ---------------------------------------------------------------------------

pub struct DbSnapshotRepositoryPort {
    pool: PgPool,
}

impl DbSnapshotRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SnapshotRepositoryPort for DbSnapshotRepositoryPort {
    async fn list_snapshots(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            String,
            String,
        )> = sqlx::query_as(
            "SELECT id::text, scene_id::text, story_time, world_summary, main_character_state, current_location, active_threads_count, unresolved_foreshadows_count, known_characters_count, known_locations_count, state_data::text, created_at::text FROM novel_state_snapshot WHERE project_id = $1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list snapshots")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    _scene,
                    story_time,
                    summary,
                    _char_state,
                    location,
                    threads,
                    foreshadows,
                    chars,
                    locs,
                    state_data,
                    created,
                )| {
                    let state_json: Value =
                        serde_json::from_str(&state_data).unwrap_or(serde_json::json!({}));
                    let name = state_json
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("快照");
                    let progress = state_json
                        .get("progress")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "story_time": story_time.unwrap_or_default(),
                        "world_summary": summary.unwrap_or_default(),
                        "current_location": location.unwrap_or_default(),
                        "active_threads_count": threads.unwrap_or(0),
                        "unresolved_foreshadows_count": foreshadows.unwrap_or(0),
                        "known_characters_count": chars.unwrap_or(0),
                        "known_locations_count": locs.unwrap_or(0),
                        "progress": progress,
                        "created_at": created
                    })
                },
            )
            .collect())
    }

    async fn create_snapshot(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        story_time: Option<&str>,
        world_summary: Option<&str>,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        let state_data = serde_json::json!({
            "name": name.unwrap_or("手动快照"),
            "progress": ""
        });

        sqlx::query(
            "INSERT INTO novel_state_snapshot (id, project_id, story_time, world_summary, state_data, active_threads_count, unresolved_foreshadows_count, known_characters_count, known_locations_count) VALUES ($1, $2, $3, $4, $5, 0, 0, 0, 0)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(story_time.unwrap_or("now"))
        .bind(world_summary.unwrap_or(""))
        .bind(&state_data)
        .execute(&self.pool)
        .await
        .context("Failed to create snapshot")?;

        Ok(serde_json::json!({
            "id": id.to_string(),
            "name": name.unwrap_or("手动快照"),
            "story_time": story_time.unwrap_or_default(),
            "world_summary": world_summary.unwrap_or_default(),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn delete_snapshot(&self, id: Uuid) -> Result<()> {
        let rows = sqlx::query(
            "DELETE FROM novel_state_snapshot WHERE id = $1 AND project_id = (SELECT project_id FROM novel_state_snapshot WHERE id = $2)",
        )
        .bind(&id)
        .bind(&id)
        .execute(&self.pool)
        .await
        .context("Failed to delete snapshot")?;
        if rows.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Snapshot not found"));
        }
        Ok(())
    }

    async fn find_snapshot(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(Uuid, Uuid, Option<String>, String, String, Option<String>, Option<String>, i32, i32, i32, i32, Option<String>)> =
            sqlx::query_as(
                "SELECT id, project_id, scene_id::text, story_time, world_summary, main_character_state, current_location, \
                        COALESCE(active_threads_count,0), COALESCE(unresolved_foreshadows_count,0), \
                        COALESCE(known_characters_count,0), COALESCE(known_locations_count,0), \
                        state_data::text \
                 FROM novel_state_snapshot WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to find snapshot")?;

        Ok(row.map(
            |(id, project_id, scene_id, story_time, world_summary, main_character_state, current_location, threads, foreshadows, chars, locs, state_data)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "project_id": project_id.to_string(),
                    "scene_id": scene_id,
                    "story_time": story_time,
                    "world_summary": world_summary,
                    "main_character_state": main_character_state,
                    "current_location": current_location,
                    "active_threads_count": threads,
                    "unresolved_foreshadows_count": foreshadows,
                    "known_characters_count": chars,
                    "known_locations_count": locs,
                    "state_data": state_data.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                })
            },
        ))
    }
}

// ---------------------------------------------------------------------------
// NarrativeStateWritePort（快照恢复等状态回写）
// ---------------------------------------------------------------------------

/// narrative_state 幂等写入的数据库实现。
pub struct DbNarrativeStateWritePort {
    pool: PgPool,
}

impl DbNarrativeStateWritePort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NarrativeStateWritePort for DbNarrativeStateWritePort {
    async fn upsert_state(
        &self,
        project_id: Uuid,
        dimension: domain::narrative::StateDimension,
        state_key: &str,
        state_value: Value,
    ) -> Result<()> {
        crate::repos::narrative_state_repo::NarrativeStateRepo::new(self.pool.clone())
            .upsert(project_id, dimension, state_key, state_value)
            .await
    }
}

// ---------------------------------------------------------------------------
// EntityRepositoryPort
// ---------------------------------------------------------------------------

pub struct DbEntityRepositoryPort {
    pool: PgPool,
}

impl DbEntityRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn parse_json(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(serde_json::json!({}))
    }
}

#[async_trait]
impl EntityRepositoryPort for DbEntityRepositoryPort {
    async fn list_entities(
        &self,
        world_id: Uuid,
        entity_type: Option<&str>,
    ) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, Option<String>, Option<String>, String, i32, String, String)> =
            if let Some(t) = entity_type {
                sqlx::query_as(
                    "SELECT e.id::text, e.name, et.name, e.summary, e.description, e.attributes::text, e.version, e.created_at::text, e.updated_at::text FROM entity e JOIN entity_type et ON e.entity_type_id = et.id WHERE e.world_id = $1 AND et.name = $2 AND e.status != 'Deleted' ORDER BY e.name",
                )
                .bind(world_id)
                .bind(t)
                .fetch_all(&self.pool)
                .await
                .context("Failed to list entities")?
            } else {
                sqlx::query_as(
                    "SELECT e.id::text, e.name, et.name, e.summary, e.description, e.attributes::text, e.version, e.created_at::text, e.updated_at::text FROM entity e JOIN entity_type et ON e.entity_type_id = et.id WHERE e.world_id = $1 AND e.status != 'Deleted' ORDER BY e.name",
                )
                .bind(world_id)
                .fetch_all(&self.pool)
                .await
                .context("Failed to list entities")?
            };

        let wid = world_id.to_string();
        Ok(rows
            .into_iter()
            .map(|(id, name, etype, summary, desc, attrs, ver, created, updated)| {
                serde_json::json!({
                    "id": id, "world_id": wid, "entity_type_id": etype, "name": name,
                    "summary": summary, "description": desc,
                    "attributes": Self::parse_json(&attrs), "version": ver,
                    "created_by": "user", "created_at": created, "updated_at": updated
                })
            })
            .collect())
    }

    async fn get_entity(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            i32,
            String,
            String,
            String,
        )> = sqlx::query_as(
            "SELECT e.id::text, e.project_id::text, e.world_id::text, e.name, e.summary, e.description, e.attributes::text, e.version, e.created_by, e.created_at::text, e.updated_at::text FROM entity e WHERE e.id = $1 AND e.status != 'Deleted'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get entity")?;

        Ok(row.map(|(id, pid, wid, name, summary, desc, attrs, ver, created_by, created, updated)| {
            serde_json::json!({
                "id": id, "project_id": pid, "world_id": wid, "name": name,
                "summary": summary, "description": desc,
                "attributes": Self::parse_json(&attrs), "version": ver,
                "created_by": created_by, "created_at": created, "updated_at": updated
            })
        }))
    }

    async fn create_entity(
        &self,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        let world: (Uuid,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1")
            .bind(world_id)
            .fetch_one(&self.pool)
            .await
            .context("Failed to resolve project for world")?;
        let etype: (Uuid,) = sqlx::query_as("SELECT id FROM entity_type WHERE name = $1")
            .bind(entity_type_name)
            .fetch_one(&self.pool)
            .await
            .with_context(|| format!("Entity type not found: {}", entity_type_name))?;

        sqlx::query(
            "INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'user')",
        )
        .bind(&id)
        .bind(&world.0)
        .bind(world_id)
        .bind(&etype.0)
        .bind(name)
        .bind(summary)
        .bind(description)
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await
        .context("Failed to create entity")?;

        self.get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Entity disappeared after creation"))
    }

    async fn update_entity(
        &self,
        id: Uuid,
        name: Option<&str>,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: Option<&Value>,
    ) -> Result<Value> {
        if let Some(name) = name {
            sqlx::query(
                "UPDATE entity SET name = $1, version = version + 1, updated_at = NOW() WHERE id = $2 AND status != 'Deleted' AND project_id = (SELECT project_id FROM entity WHERE id = $3)",
            )
            .bind(name)
            .bind(id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        if let Some(summary) = summary {
            sqlx::query(
                "UPDATE entity SET summary = $1, version = version + 1, updated_at = NOW() WHERE id = $2 AND status != 'Deleted' AND project_id = (SELECT project_id FROM entity WHERE id = $3)",
            )
            .bind(summary)
            .bind(id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        if let Some(description) = description {
            sqlx::query(
                "UPDATE entity SET description = $1, version = version + 1, updated_at = NOW() WHERE id = $2 AND status != 'Deleted' AND project_id = (SELECT project_id FROM entity WHERE id = $3)",
            )
            .bind(description)
            .bind(id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        if let Some(attributes) = attributes {
            sqlx::query(
                "UPDATE entity SET attributes = $1, version = version + 1, updated_at = NOW() WHERE id = $2 AND status != 'Deleted' AND project_id = (SELECT project_id FROM entity WHERE id = $3)",
            )
            .bind(attributes)
            .bind(id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        self.get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Entity not found"))
    }

    async fn delete_entity(&self, id: Uuid) -> Result<Value> {
        // 软删除：仅标记 Deleted；RETURNING project_id 同时保证 project 作用域。
        let maybe_project_id: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE entity SET status = 'Deleted', version = version + 1, updated_at = NOW() \
             WHERE id = $1 AND status = 'Active' \
             RETURNING project_id",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to delete entity")?;

        match maybe_project_id {
            Some((project_id,)) => Ok(serde_json::json!({
                "deleted": true,
                "id": id.to_string(),
                "project_id": project_id
            })),
            None => Err(anyhow::anyhow!("Entity not found or already deleted")),
        }
    }

    async fn list_relations(&self, world_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, String, Option<String>, String, String, String)> =
            sqlx::query_as(
                "SELECT r.id::text, r.source_entity_id::text, r.target_entity_id::text, r.relation_type, r.description, r.attributes::text, r.created_at::text, r.updated_at::text FROM relation r JOIN entity e ON r.source_entity_id = e.id WHERE e.world_id = $1 AND r.valid_until IS NULL",
            )
            .bind(world_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list relations")?;

        Ok(rows
            .into_iter()
            .map(|(id, src, tgt, rtype, desc, attrs, created, updated)| {
                serde_json::json!({
                    "id": id, "source_entity_id": src, "target_entity_id": tgt,
                    "relation_type": rtype, "description": desc,
                    "attributes": Self::parse_json(&attrs),
                    "created_at": created, "updated_at": updated
                })
            })
            .collect())
    }

    async fn create_relation(
        &self,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        // 校验 source / target 实体存在且同属一个 project，避免跨项目关系。
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT project_id FROM entity WHERE id = $1 OR id = $2",
        )
        .bind(source_entity_id)
        .bind(target_entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to check relation entities")?;

        if rows.len() != 2 {
            return Err(anyhow::anyhow!(
                "Cannot create relation: source or target entity does not exist"
            ));
        }
        if rows[0].0 != rows[1].0 {
            return Err(anyhow::anyhow!(
                "Cannot create relation: source and target entities belong to different projects"
            ));
        }
        let project_id = &rows[0].0;

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO relation (id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes) VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(source_entity_id)
        .bind(target_entity_id)
        .bind(relation_type)
        .bind(description)
        .execute(&self.pool)
        .await
        .context("Failed to create relation")?;

        Ok(serde_json::json!({
            "id": id.to_string(),
            "source_entity_id": source_entity_id.to_string(),
            "target_entity_id": target_entity_id.to_string(),
            "relation_type": relation_type
        }))
    }

    async fn delete_relation(&self, id: Uuid) -> Result<()> {
        // 带 project 作用域：子查询确保只删除属于同 project 的关系。
        let rows = sqlx::query(
            "DELETE FROM relation WHERE id = $1 AND project_id = (SELECT project_id FROM relation WHERE id = $2)",
        )
        .bind(&id)
        .bind(&id)
        .execute(&self.pool)
        .await
        .context("Failed to delete relation")?;
        if rows.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Relation not found"));
        }
        Ok(())
    }

    async fn get_character_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let repo = crate::repos::character_repo::CharacterProfileRepo::new(self.pool.clone());
        let profile = repo.get_by_entity(id).await?;
        let drive = crate::repos::character_repo::CharacterDriveRepo::new(self.pool.clone())
            .get_by_entity(id)
            .await?;
        let conflicts = crate::repos::character_repo::CharacterConflictRepo::new(self.pool.clone())
            .list_by_entity(id)
            .await?;
        let relationships =
            crate::repos::character_repo::CharacterRelationshipRepo::new(self.pool.clone())
                .list_by_entity(id)
                .await?;
        let secrets = crate::repos::character_repo::CharacterSecretRepo::new(self.pool.clone())
            .list_by_entity(id)
            .await?;
        let capabilities =
            crate::repos::character_repo::CharacterCapabilityRepo::new(self.pool.clone())
                .get_by_entity(id)
                .await?;
        let arc = crate::repos::character_repo::CharacterArcRepo::new(self.pool.clone())
            .get_by_entity(id)
            .await?;
        let extension = crate::repos::character_repo::CharacterExtensionRepo::new(self.pool.clone())
            .get_by_entity(id)
            .await?;

        Ok(profile.map(|p| {
            serde_json::json!({
                "id": p.id.to_string(),
                "entity_id": p.entity_id.to_string(),
                "name": p.name,
                "aliases": p.aliases,
                "age_range": p.age.map(|a| a.as_str()),
                "gender": p.gender.map(|g| g.as_str()),
                "identity": p.identity,
                "appearance": p.appearance,
                "background_origin": p.background_origin,
                "social_position": p.social_position,
                "core_personality": p.core_personality,
                "values": p.values,
                "role_in_story": p.role_in_story.map(|r| r.as_str()),
                "narrative_necessity": p.narrative_necessity,
                "drive": drive,
                "conflicts": conflicts,
                "relationships": relationships,
                "secrets": secrets,
                "capabilities": capabilities,
                "arc_potential": arc,
                "extension": extension,
                "created_at": p.created_at.to_rfc3339(),
                "updated_at": p.updated_at.to_rfc3339()
            })
        }))
    }

    async fn get_character_state(&self, id: Uuid) -> Result<Option<Value>> {
        let repo = crate::repos::character_repo::CharacterStateRepo::new(self.pool.clone());
        let state = repo.get_by_entity(id).await?;
        Ok(state.map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "entity_id": s.entity_id.to_string(),
                "location": s.location,
                "physical_state": s.physical_state,
                "mental_state": s.mental_state,
                "resource_state": s.resource_state,
                "social_state": s.social_state,
                "flags": s.flags,
                "extra": s.extra,
                "created_at": s.created_at.to_rfc3339(),
                "updated_at": s.updated_at.to_rfc3339()
            })
        }))
    }

    async fn update_character_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        use domain::character::*;
        let now = Utc::now();
        let s = |k: &str| profile.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        let age_range: Option<AgeRange> =
            profile.get("age_range").and_then(|v| v.as_str()).map(AgeRange::from_str);
        let gender: Option<Gender> =
            profile.get("gender").and_then(|v| v.as_str()).map(Gender::from_str);
        let role: Option<StoryRole> = profile
            .get("role_in_story")
            .and_then(|v| v.as_str())
            .map(StoryRole::from_str);
        let aliases: Vec<String> = profile
            .get("aliases")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let social_position: Option<SocialPosition> = profile
            .get("social_position")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let narrative_necessity: Option<NarrativeNecessity> = profile
            .get("narrative_necessity")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let existing: Option<(Uuid,)> =
            sqlx::query_as("SELECT id::text FROM character_profile WHERE entity_id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .context("check character profile")?;
        match existing {
            Some((pid,)) => {
                sqlx::query("UPDATE character_profile SET name=$1, aliases=$2, age_range=$3, gender=$4, identity=$5, appearance=$6, background_origin=$7, social_position=$8, core_personality=$9, \"values\"=$10, role_in_story=$11, narrative_necessity=$12, extra=$13, updated_at=$14 WHERE id=$15")
                    .bind(s("name"))
                    .bind(serde_json::to_value(&aliases).unwrap_or(serde_json::Value::Null))
                    .bind(age_range.map(|a| a.as_str()))
                    .bind(gender.map(|g| g.as_str()))
                    .bind(s("identity"))
                    .bind(s("appearance"))
                    .bind(s("background_origin"))
                    .bind(serde_json::to_value(&social_position).unwrap_or(serde_json::Value::Null))
                    .bind(s("core_personality"))
                    .bind(s("values"))
                    .bind(role.map(|r| r.as_str()))
                    .bind(serde_json::to_value(&narrative_necessity).unwrap_or(serde_json::Value::Null))
                    .bind(profile.get("extra").cloned().unwrap_or(serde_json::Value::Null))
                    .bind(now)
                    .bind(pid)
                    .execute(&self.pool).await.context("update character profile")?;
            }
            None => {
                sqlx::query("INSERT INTO character_profile (id, entity_id, name, aliases, age_range, gender, identity, appearance, background_origin, social_position, core_personality, \"values\", role_in_story, narrative_necessity, extra, created_at, updated_at) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
                    .bind(id)
                    .bind(s("name"))
                    .bind(serde_json::to_value(&aliases).unwrap_or(serde_json::Value::Null))
                    .bind(age_range.map(|a| a.as_str()))
                    .bind(gender.map(|g| g.as_str()))
                    .bind(s("identity"))
                    .bind(s("appearance"))
                    .bind(s("background_origin"))
                    .bind(serde_json::to_value(&social_position).unwrap_or(serde_json::Value::Null))
                    .bind(s("core_personality"))
                    .bind(s("values"))
                    .bind(role.map(|r| r.as_str()))
                    .bind(serde_json::to_value(&narrative_necessity).unwrap_or(serde_json::Value::Null))
                    .bind(profile.get("extra").cloned().unwrap_or(serde_json::Value::Null))
                    .bind(now)
                    .bind(now)
                    .execute(&self.pool).await.context("insert character profile")?;
            }
        }

        // Sync drive / capabilities / arc / extension if present
        if let Some(d) = profile.get("drive") {
            if let Ok(drive) = serde_json::from_value::<CharacterDrive>(d.clone()) {
                crate::repos::character_repo::CharacterDriveRepo::new(self.pool.clone())
                    .upsert(id, &drive)
                    .await?;
            }
        }
        if let Some(c) = profile.get("capabilities") {
            if let Ok(cap) = serde_json::from_value::<CharacterCapability>(c.clone()) {
                crate::repos::character_repo::CharacterCapabilityRepo::new(self.pool.clone())
                    .upsert(id, &cap)
                    .await?;
            }
        }
        if let Some(a) = profile.get("arc_potential") {
            if let Ok(arc) = serde_json::from_value::<CharacterArcPotential>(a.clone()) {
                crate::repos::character_repo::CharacterArcRepo::new(self.pool.clone())
                    .upsert(id, &arc)
                    .await?;
            }
        }
        if let Some(e) = profile.get("extension") {
            if let Ok(ext) = serde_json::from_value::<CharacterExtension>(e.clone()) {
                crate::repos::character_repo::CharacterExtensionRepo::new(self.pool.clone())
                    .upsert(id, &ext)
                    .await?;
            }
        }

        let mut out = profile.clone();
        out["entity_id"] = serde_json::json!(id.to_string());
        Ok(out)
    }

    async fn update_character_state(&self, id: Uuid, state: Value) -> Result<Value> {
        let now = Utc::now();
        let s = |k: &str| state.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        let flags: Vec<String> = state
            .get("flags")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let extra: Value = state
            .get("extra")
            .cloned()
            .unwrap_or(serde_json::json!(null));

        let existing: Option<(Uuid,)> =
            sqlx::query_as("SELECT id::text FROM character_state WHERE entity_id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .context("check character state")?;
        match existing {
            Some((pid,)) => {
                sqlx::query("UPDATE character_state SET location=$1, physical_state=$2, mental_state=$3, resource_state=$4, social_state=$5, flags=$6, extra=$7, updated_at=$8 WHERE id=$9")
                    .bind(s("location"))
                    .bind(s("physical_state"))
                    .bind(s("mental_state"))
                    .bind(s("resource_state"))
                    .bind(s("social_state"))
                    .bind(&flags)
                    .bind(&extra)
                    .bind(now)
                    .bind(pid)
                    .execute(&self.pool).await.context("update character state")?;
            }
            None => {
                sqlx::query("INSERT INTO character_state (id, entity_id, location, physical_state, mental_state, resource_state, social_state, flags, extra, created_at, updated_at) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
                    .bind(id)
                    .bind(s("location"))
                    .bind(s("physical_state"))
                    .bind(s("mental_state"))
                    .bind(s("resource_state"))
                    .bind(s("social_state"))
                    .bind(&flags)
                    .bind(&extra)
                    .bind(now)
                    .bind(now)
                    .execute(&self.pool).await.context("insert character state")?;
            }
        }
        let mut out = state.clone();
        out["entity_id"] = serde_json::json!(id.to_string());
        Ok(out)
    }

    async fn get_location_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(
            Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>, Option<String>, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
        )> = sqlx::query_as(
            "SELECT lp.geography, lp.appearance, lp.population, lp.economy, lp.rules, lp.history, lp.narrative_usage, \
             li.location_type, li.size, li.climate, li.era, li.accessibility \
             FROM location_profile lp FULL OUTER JOIN location_identity li ON li.entity_id = lp.entity_id \
             WHERE COALESCE(lp.entity_id, li.entity_id) = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get location profile")?;
        Ok(row.map(|r| {
            serde_json::json!({
                "geography": r.0, "appearance": r.1, "population": r.2, "economy": r.3,
                "rules": r.4, "history": r.5, "narrative_usage": r.6,
                "location_type": r.7, "size": r.8, "climate": r.9, "era": r.10, "accessibility": r.11
            })
        }))
    }

    async fn upsert_location_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        let s = |k: &str| profile.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        let geography = s("geography");
        let appearance = s("appearance");
        let population = s("population");
        let economy = s("economy");
        let rules = s("rules");
        let history = s("history");
        let narrative_usage = s("narrative_usage");
        let location_type = s("location_type");
        let size = s("size");
        let climate = s("climate");
        let era = s("era");
        let accessibility = s("accessibility");

        let lp: Option<(String,)> = sqlx::query_as("SELECT id::text FROM location_profile WHERE entity_id = $1")
            .bind(id).fetch_optional(&self.pool).await.context("chk loc profile")?;
        match lp {
            Some((pid,)) => {
                sqlx::query("UPDATE location_profile SET geography=$1, appearance=$2, population=$3, economy=$4, rules=$5, history=$6, narrative_usage=$7 WHERE id=$8")
                    .bind(&geography).bind(&appearance).bind(&population).bind(&economy).bind(&rules).bind(&history).bind(&narrative_usage).bind(pid)
                    .execute(&self.pool).await.context("upd loc profile")?;
            }
            None => {
                sqlx::query("INSERT INTO location_profile (id, entity_id, geography, appearance, population, economy, rules, history, narrative_usage) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8)")
                    .bind(id).bind(&geography).bind(&appearance).bind(&population).bind(&economy).bind(&rules).bind(&history).bind(&narrative_usage)
                    .execute(&self.pool).await.context("ins loc profile")?;
            }
        }
        let li: Option<(String,)> = sqlx::query_as("SELECT id::text FROM location_identity WHERE entity_id = $1")
            .bind(id).fetch_optional(&self.pool).await.context("chk loc identity")?;
        match li {
            Some((pid,)) => {
                sqlx::query("UPDATE location_identity SET location_type=$1, size=$2, climate=$3, era=$4, accessibility=$5 WHERE id=$6")
                    .bind(&location_type).bind(&size).bind(&climate).bind(&era).bind(&accessibility).bind(pid)
                    .execute(&self.pool).await.context("upd loc identity")?;
            }
            None => {
                sqlx::query("INSERT INTO location_identity (id, entity_id, location_type, size, climate, era, accessibility) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6)")
                    .bind(id).bind(&location_type).bind(&size).bind(&climate).bind(&era).bind(&accessibility)
                    .execute(&self.pool).await.context("ins loc identity")?;
            }
        }
        let mut out = profile.clone();
        out["entity_id"] = serde_json::json!(id.to_string());
        Ok(out)
    }

    async fn get_faction_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(
            Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>, Option<String>, Option<String>,
        )> = sqlx::query_as(
            "SELECT goals, leader, \"values\", resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi FROM faction_profile WHERE entity_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get faction profile")?;
        Ok(row.map(|r| {
            serde_json::json!({
                "goals": r.0, "leader": r.1, "values": r.2, "resources": r.3,
                "territory": r.4, "members": r.5, "enemies": r.6, "allies": r.7,
                "internal_conflicts": r.8, "secrets": r.9, "modus_operandi": r.10
            })
        }))
    }

    async fn upsert_faction_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        let s = |k: &str| profile.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        let goals = s("goals");
        let leader = s("leader");
        let values = s("values");
        let resources = s("resources");
        let territory = s("territory");
        let members = s("members");
        let enemies = s("enemies");
        let allies = s("allies");
        let internal_conflicts = s("internal_conflicts");
        let secrets = s("secrets");
        let modus_operandi = s("modus_operandi");
        let existing: Option<(String,)> = sqlx::query_as("SELECT id::text FROM faction_profile WHERE entity_id = $1")
            .bind(id).fetch_optional(&self.pool).await.context("chk faction profile")?;
        match existing {
            Some((pid,)) => {
                sqlx::query("UPDATE faction_profile SET goals=$1, leader=$2, \"values\"=$3, resources=$4, territory=$5, members=$6, enemies=$7, allies=$8, internal_conflicts=$9, secrets=$10, modus_operandi=$11 WHERE id=$12")
                    .bind(&goals).bind(&leader).bind(&values).bind(&resources).bind(&territory).bind(&members).bind(&enemies).bind(&allies).bind(&internal_conflicts).bind(&secrets).bind(&modus_operandi).bind(pid)
                    .execute(&self.pool).await.context("upd faction profile")?;
            }
            None => {
                sqlx::query("INSERT INTO faction_profile (id, entity_id, goals, leader, \"values\", resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
                    .bind(id).bind(&goals).bind(&leader).bind(&values).bind(&resources).bind(&territory).bind(&members).bind(&enemies).bind(&allies).bind(&internal_conflicts).bind(&secrets).bind(&modus_operandi)
                    .execute(&self.pool).await.context("ins faction profile")?;
            }
        }
        let mut out = profile.clone();
        out["entity_id"] = serde_json::json!(id.to_string());
        Ok(out)
    }

    async fn get_character_knowledge(&self, id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT ks.id::text, ks.knowledge_level, COALESCE(ks.source, ''), f.content FROM knowledge_state ks JOIN fact f ON ks.fact_id = f.id WHERE ks.subject_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get character knowledge")?;
        Ok(rows
            .into_iter()
            .map(|(id, level, source, content)| {
                serde_json::json!({ "id": id, "fact": content, "level": level, "source": source })
            })
            .collect())
    }

    async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT r.id::text, r.relation_type, e2.name, e2.id::text, r.description FROM relation r JOIN entity e2 ON r.target_entity_id = e2.id WHERE r.source_entity_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get character relationships")?;
        Ok(rows
            .into_iter()
            .map(|(id, rtype, name, tid, desc)| {
                serde_json::json!({
                    "id": id, "type": rtype, "target": name,
                    "target_id": tid, "description": desc
                })
            })
            .collect())
    }
}

/// ContextSnapshot 仓储端口的数据库实现（提案 十二）。
pub struct DbContextSnapshotRepositoryPort {
    pool: PgPool,
}

impl DbContextSnapshotRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ContextSnapshotRepositoryPort for DbContextSnapshotRepositoryPort {
    async fn save(&self, package: &domain::generation::ContextPackage) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let mut pkg = package.clone();
        pkg.id = id;
        crate::repos::context_snapshot_repo::ContextSnapshotRepo::new(self.pool.clone())
            .save(&pkg)
            .await?;
        Ok(id)
    }
}

/// 全局应用设置仓储（设置页持久化）。单全球行（id='default'），settings 存 JSONB。
pub struct DbSettingsRepositoryPort {
    pool: PgPool,
}

impl DbSettingsRepositoryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 读取全局设置；无记录时返回空对象 {}。
    pub async fn get_settings(&self) -> Result<Value> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT settings::text FROM app_settings WHERE id = 'default'",
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to read app settings")?;

        match row {
            Some((s,)) => Ok(serde_json::from_str::<Value>(&s).unwrap_or_else(|_| serde_json::json!({}))),
            None => Ok(serde_json::json!({})),
        }
    }

    /// 覆盖写入全局设置（upsert）。
    pub async fn upsert_settings(&self, settings: Value) -> Result<Value> {
        sqlx::query(
            "INSERT INTO app_settings (id, settings, updated_at) VALUES ('default', $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET settings = EXCLUDED.settings, updated_at = NOW()",
        )
        .bind(&settings)
        .execute(&self.pool)
        .await
        .context("Failed to upsert app settings")?;
        Ok(settings)
    }
}
