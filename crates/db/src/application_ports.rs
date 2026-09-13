//! Application 层端口的数据库实现（P3）。
//!
//! GenerationRepositoryPort 的具体 SQL 实现。原本这些查询位于
//! application::generation_service 中；P3 把它们下移到 db（依赖倒置：
//! application 依赖端口，db 实现端口）。

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::ports::{
    ApprovalRepositoryPort, ContextSnapshotRepositoryPort, EntityRepositoryPort,
    ForeshadowRepositoryPort, GenerationRepositoryPort, HistoryRepositoryPort,
    NarrativeRepositoryPort, NarrativeStateWritePort, ProjectRepositoryPort, ProposalRepositoryPort,
    RuleRepositoryPort, SnapshotRepositoryPort, StorylineRepositoryPort, TimelineRepositoryPort,
    TraceQueryPort, WorldRepositoryPort,
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
        // 带上实体类型名：详情页有「类型」这一行，但返回里一直没有这个字段，
        // 于是那一行永远是空的（前端取 entity_type 取不到）。
        //
        // ⚠️ 两条硬约束（都踩过）：
        //   1. SQL 必须写成**单行**。生成 SQLite 版的脚本按 `id::text` 这类文本模式
        //      改写 SQL 与元组类型，用 `\` 续行的多行 SQL 会让规则失配。
        //   2. `sqlx::query_as(` 与 SQL 字符串之间**不能有注释**。生成脚本用
        //      `query_as\(\s*"..."` 匹配，注释会让它匹配不上，
        //      于是元组类型不会从 String 改成 Uuid，运行时直接报
        //      "String is not compatible with SQL type BLOB"。
        let row: Option<(Uuid, Uuid, Uuid, String, Option<Uuid>, String, Option<String>, Option<String>, String, i32, String, String, String)> =
            sqlx::query_as(
                "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes::text, sort_order, status, created_at::text, updated_at::text                  FROM narrative_node WHERE id = $1 AND status != 'Deleted'"
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
        let rows: Vec<(String, String, Option<String>, String, String, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id::text, name, description, status, importance, COALESCE(tone,'light'), COALESCE(visibility,'visible'), created_at::text, updated_at::text FROM storyline WHERE project_id=$1 ORDER BY importance, name",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list storylines")?;
        Ok(rows
            .into_iter()
            .map(|(id, name, desc, st, imp, tone, vis, cr, up)| {
                serde_json::json!({
                    "id": id, "project_id": project_id.to_string(), "name": name,
                    "description": desc, "status": st, "importance": imp,
                    "tone": tone, "visibility": vis,
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
        tone: &str,
        visibility: &str,
        parent_id: Option<Uuid>,
    ) -> Result<Value> {
        // 业务规则：
        //   - Main 主线必须 parent_id=None
        //   - Normal 副线 parent_id 可选（None 表示独立于任何 story line）
        //   - DB 唯一约束：UNIQUE(parent_id, child_id) 在 storyline_relation 表上
        if importance == "Main" && parent_id.is_some() {
            anyhow::bail!("Main 主线不能挂到其他 story line 下");
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO storyline (id, project_id, name, description, status, importance, tone, visibility) VALUES ($1,$2,$3,$4,'Planned',$5,$6,$7)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(importance)
        .bind(tone)
        .bind(visibility)
        .execute(&self.pool)
        .await
        .context("Failed to create storyline")?;

        // 如果传了 parent_id，建挂载关系
        if let Some(pid) = parent_id {
            sqlx::query(
                "INSERT INTO storyline_relation (project_id, parent_id, child_id) VALUES ($1, $2, $3)",
            )
            .bind(project_id)
            .bind(pid)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to create storyline_relation")?;
        }

        Ok(serde_json::json!({
            "id": id.to_string(),
            "project_id": project_id.to_string(),
            "name": name,
            "importance": importance,
            "tone": tone,
            "visibility": visibility,
            "parent_id": parent_id.map(|x| x.to_string()),
            "status": "Planned"
        }))
    }

    async fn update_storyline(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
        tone: Option<&str>,
        visibility: Option<&str>,
    ) -> Result<Value> {
        // 动态 SET 拼接（仅在 tone/visibility 提供时更新）
        let mut sql = String::from("UPDATE storyline SET name=$1, description=$2, updated_at=NOW()");
        if tone.is_some() { sql.push_str(", tone=$5"); }
        if visibility.is_some() { sql.push_str(", visibility=$6"); }
        sql.push_str(" WHERE id=$3 AND project_id = (SELECT project_id FROM storyline WHERE id = $4)");

        let mut q = sqlx::query(&sql)
            .bind(name)
            .bind(description)
            .bind(id)
            .bind(id);
        if let Some(t) = tone { q = q.bind(t); }
        if let Some(v) = visibility { q = q.bind(v); }
        q.execute(&self.pool).await.context("Failed to update storyline")?;
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

    async fn list_storyline_relations(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id::text, parent_id::text, child_id::text, created_at::text \
             FROM storyline_relation WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list storyline_relations")?;
        Ok(rows
            .into_iter()
            .map(|(id, parent, child, cr)| {
                serde_json::json!({
                    "id": id, "project_id": project_id.to_string(),
                    "parent_id": parent, "child_id": child,
                    "created_at": cr
                })
            })
            .collect())
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
        let rows: Vec<(String, String, Option<String>, Option<String>, String, String, String, String, Option<String>)> =
            sqlx::query_as(
                "SELECT id::text, name, description, language, COALESCE(status, 'Concept'), COALESCE(config::text, '{}'), created_at::text, updated_at::text, premise FROM project ORDER BY updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .context("Failed to list projects")?;

        Ok(rows
            .into_iter()
            .map(|(id, name, desc, lang, status, config, created, updated, premise)| {
                serde_json::json!({
                    "id": id, "name": name, "description": desc, "language": lang,
                    "status": status,
                    "config": serde_json::from_str::<Value>(&config).unwrap_or_default(),
                    "default_params": {}, "created_at": created, "updated_at": updated,
                    "premise": premise
                })
            })
            .collect())
    }

    async fn get_project(&self, id: Uuid) -> Result<Option<Value>> {
        let row: Option<(String, String, Option<String>, Option<String>, String, String, String, String, Option<String>)> =
            sqlx::query_as(
                "SELECT id::text, name, description, language, COALESCE(status, 'Concept'), COALESCE(config::text, '{}'), created_at::text, updated_at::text, premise FROM project WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get project")?;

        Ok(row.map(|(id, name, desc, lang, status, config, created, updated, premise)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "language": lang,
                "status": status,
                "config": serde_json::from_str::<Value>(&config).unwrap_or_default(),
                "default_params": {}, "created_at": created, "updated_at": updated,
                "premise": premise
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
        premise: Option<&str>,
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
        if let Some(premise) = premise {
            sqlx::query("UPDATE project SET premise = $1, updated_at = NOW() WHERE id = $2")
                .bind(premise)
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
// TraceQueryPort（AI 可追溯：generation_run / validation_run 只读视图）
// ---------------------------------------------------------------------------

/// AI 可追溯查询的数据库实现。
pub struct DbTraceQueryPort {
    pool: PgPool,
}

impl DbTraceQueryPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TraceQueryPort for DbTraceQueryPort {
    async fn list_generation_runs(&self, project_id: Uuid, limit: i64) -> Result<Vec<Value>> {
        let rows: Vec<(Uuid, Uuid, Option<Uuid>, String, Option<String>, String, String, Option<Value>, Option<i64>, Option<Value>, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT id, task_id, context_snapshot_id, llm_model, provider, prompt_sent, response_received, \
                        token_usage, latency_ms, reproducibility_meta, created_at \
                 FROM generation_run WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2",
            )
            .bind(project_id)
            .bind(limit.clamp(1, 200))
            .fetch_all(&self.pool)
            .await
            .context("Failed to list generation runs")?;

        Ok(rows
            .into_iter()
            .map(|(id, task_id, ctx_snapshot, model, provider, prompt, response, tokens, latency, repro, created)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "task_id": task_id.to_string(),
                    "context_snapshot_id": ctx_snapshot.map(|u| u.to_string()),
                    "llm_model": model,
                    "provider": provider,
                    "prompt_sent": prompt,
                    "response_received": response,
                    "token_usage": tokens,
                    "latency_ms": latency,
                    "reproducibility_meta": repro,
                    "created_at": created.to_rfc3339(),
                })
            })
            .collect())
    }

    async fn list_validation_runs(&self, project_id: Uuid, limit: i64) -> Result<Vec<Value>> {
        let rows: Vec<(Uuid, Uuid, i32, i32, i32, String, DateTime<Utc>, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                "SELECT id, task_id, changes_validated, changes_approved, changes_rejected, status, started_at, completed_at \
                 FROM validation_run WHERE project_id = $1 ORDER BY started_at DESC LIMIT $2",
            )
            .bind(project_id)
            .bind(limit.clamp(1, 200))
            .fetch_all(&self.pool)
            .await
            .context("Failed to list validation runs")?;

        let mut out = Vec::with_capacity(rows.len());
        for (id, task_id, validated, approved, rejected, status, started, completed) in rows {
            let issues: Vec<(Uuid, String, String, String, Option<String>)> = sqlx::query_as(
                "SELECT id, issue_type, severity, message, suggestion \
                 FROM validation_issue WHERE validation_run_id = $1 ORDER BY created_at",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list validation issues")?;

            out.push(serde_json::json!({
                "id": id.to_string(),
                "task_id": task_id.to_string(),
                "changes_validated": validated,
                "changes_approved": approved,
                "changes_rejected": rejected,
                "status": status,
                "started_at": started.to_rfc3339(),
                "completed_at": completed.map(|t| t.to_rfc3339()),
                "issues": issues.into_iter().map(|(iid, itype, sev, msg, sugg)| {
                    serde_json::json!({
                        "id": iid.to_string(),
                        "issue_type": itype,
                        "severity": sev,
                        "message": msg,
                        "suggestion": sugg,
                    })
                }).collect::<Vec<_>>(),
            }));
        }
        Ok(out)
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
        // attributes 可能是 NULL（直接 SQL 插的 entity 没设 attributes）：
        // 改成 Option<String> + 用 unwrap_or("{}") 兜底
        //
        // 类型过滤用 LOWER(...) 做大小写不敏感匹配：类型名在库中是 `Character` /
        // `Location` 这类首字母大写形式，而 LLM 调用工具时常写成小写（`location`）。
        // 精确匹配会静默返回空列表，看起来像"这个世界没有数据"。
        let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, i32, String, String)> =
            if let Some(t) = entity_type {
                sqlx::query_as(
                    "SELECT e.id::text, e.name, et.name, e.summary, e.description, e.attributes::text, e.version, e.created_at::text, e.updated_at::text FROM entity e JOIN entity_type et ON e.entity_type_id = et.id WHERE e.world_id = $1 AND LOWER(et.name) = LOWER($2) AND e.status != 'Deleted' ORDER BY e.name",
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
                    "attributes": Self::parse_json(attrs.as_deref().unwrap_or("{}")), "version": ver,
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
            Option<String>,
        )> = sqlx::query_as(
            "SELECT e.id::text, e.project_id::text, e.world_id::text, e.name, e.summary, e.description, e.attributes::text, e.version, e.created_by, e.created_at::text, e.updated_at::text, et.name FROM entity e LEFT JOIN entity_type et ON et.id = e.entity_type_id WHERE e.id = $1 AND e.status != 'Deleted'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get entity")?;

        Ok(row.map(|(id, pid, wid, name, summary, desc, attrs, ver, created_by, created, updated, etype)| {
            serde_json::json!({
                "id": id, "project_id": pid, "world_id": wid, "name": name,
                "summary": summary, "description": desc,
                // 类型名（Character / Location / Faction / Item …），详情页「类型」行用它
                "entity_type": etype,
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
        // 改动前留档（版本历史的来源）。走的是与 MutationCommitter 同一条
        // `EntityRepo::snapshot_tx`，保证「谁改的改动」都进得了历史。
        crate::repos::entity_repo::EntityRepo::snapshot_tx(&self.pool, id).await?;

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
        // 顺带把两端实体的**名字**查出来。
        //
        // 原先只返回 uuid，前端拿不到名字就只能显示关系类型
        // （一列「LocatedAt」「MemberOf」），用户根本看不出这条关系说的是谁和谁——
        // 而关系列表的全部价值就在于此。
        let rows: Vec<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            String,
        )> = sqlx::query_as(
            "SELECT r.id::text, r.source_entity_id::text, r.target_entity_id::text, \
                    src.name, tgt.name, r.relation_type, r.description, r.attributes::text, \
                    r.created_at::text, r.updated_at::text \
             FROM relation r \
             JOIN entity src ON r.source_entity_id = src.id \
             JOIN entity tgt ON r.target_entity_id = tgt.id \
             WHERE src.world_id = $1 AND r.valid_until IS NULL \
             ORDER BY src.name, tgt.name",
        )
        .bind(world_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list relations")?;

        Ok(rows
            .into_iter()
            .map(
                |(id, src, tgt, src_name, tgt_name, rtype, desc, attrs, created, updated)| {
                    serde_json::json!({
                        "id": id, "source_entity_id": src, "target_entity_id": tgt,
                        // 名字缺失时回退到 uuid 前 8 位，前端至少能看出是两条不同的记录
                        "source_name": src_name,
                        "target_name": tgt_name,
                        "relation_type": rtype, "description": desc,
                        "attributes": Self::parse_json(&attrs),
                        "created_at": created, "updated_at": updated
                    })
                },
            )
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
        let arc_stages =
            crate::repos::arc_stage_repo::EntityArcStageRepo::new(self.pool.clone())
                .list_by_entity(id)
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
                "arc_stages": arc_stages,
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

    async fn update_character_profile(&self, id: Uuid, profile: Value, actor: &str) -> Result<Value> {
        use domain::character::*;
        let now = Utc::now();
        let s = |k: &str| profile.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());

        /*
         * 枚举必须严格解析。原先三个字段一律走 from_str，未知取值被静默降级
         * （「男」→ Other、「青年」→ Unknown），库里留下错误值却没人察觉：
         * 「填了男」和「确实填了 Other」在数据上完全无法区分。
         *
         * 但「严格」不等于「只认英文」——这是中文小说系统，中文写法会被
         * parse() 明确映射到规范值；只有真正无法识别的取值才报错。
         */
        let age_range = match profile.get("age_range").and_then(|v| v.as_str()) {
            Some(raw) => Some(AgeRange::parse(raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "age_range 无法识别：{}（可传中文如「青年」「中年」，或规范值 {}）",
                    raw,
                    AgeRange::ALL.join(" / ")
                )
            })?),
            None => None,
        };
        let gender = match profile.get("gender").and_then(|v| v.as_str()) {
            Some(raw) => Some(Gender::parse(raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "gender 无法识别：{}（可传中文如「男」「女」，或规范值 {}）",
                    raw,
                    Gender::ALL.join(" / ")
                )
            })?),
            None => None,
        };
        let role = match profile.get("role_in_story").and_then(|v| v.as_str()) {
            Some(raw) => Some(StoryRole::parse(raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "role_in_story 无法识别：{}（可传中文如「主角」「反派」，或规范值 {}）",
                    raw,
                    StoryRole::ALL.join(" / ")
                )
            })?),
            None => None,
        };

        // 结构化字段：类型不符时报错，不静默丢弃。
        // 原先的 .ok() 会把非法输入变成空值，调用方以为写成功了。
        let aliases: Vec<String> = match profile.get("aliases") {
            Some(v) if !v.is_null() => serde_json::from_value(v.clone())
                .map_err(|e| anyhow::anyhow!("aliases 应为字符串数组：{}", e))?,
            _ => Vec::new(),
        };
        let social_position: Option<SocialPosition> = match profile.get("social_position") {
            Some(v) if !v.is_null() => Some(serde_json::from_value(v.clone()).map_err(|e| {
                anyhow::anyhow!("social_position 应为对象（rank / authority_level / social_access）：{}", e)
            })?),
            _ => None,
        };
        let narrative_necessity: Option<NarrativeNecessity> = match profile.get("narrative_necessity") {
            Some(v) if !v.is_null() => Some(
                serde_json::from_value(v.clone())
                    .map_err(|e| anyhow::anyhow!("narrative_necessity 应为对象：{}", e))?,
            ),
            _ => None,
        };

        // 先取出旧值：**本次没传的字段要保持原值**。
        //
        // agent 工具 `update_character_profile` 的描述明确写着「只传需要设置的字段，
        // 未传的保持原值」，模型就是照这句话调用的。原先这里整行 SET，只改
        // identity 一个字段就会把外貌 / 性格 / 背景 / 别名全部清成 NULL——
        // 用户侧表现为「设定莫名其妙没了」，而且没有任何提示。
        let existing: Option<(
            Uuid,
            Option<String>,
            serde_json::Value,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            serde_json::Value,
            Option<String>,
            Option<String>,
            Option<String>,
            serde_json::Value,
            serde_json::Value,
        )> = sqlx::query_as(
            "SELECT id, name, aliases, age_range, gender, identity, appearance, \
             background_origin, social_position, core_personality, \"values\", role_in_story, \
             narrative_necessity, extra FROM character_profile WHERE entity_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("check character profile")?;
        match existing {
            Some((
                pid,
                old_name,
                old_aliases,
                old_age,
                old_gender,
                old_identity,
                old_appearance,
                old_background,
                old_social,
                old_personality,
                old_values,
                old_role,
                old_necessity,
                old_extra,
            )) => {
                // 「传了就更新，没传就用旧值」——空串是有效输入（表示清空），null 一律视为未传
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                let aliases_final = match profile.get("aliases") {
                    Some(v) if !v.is_null() => {
                        serde_json::to_value(&aliases).unwrap_or(serde_json::Value::Null)
                    }
                    _ => old_aliases,
                };
                let social_final = match profile.get("social_position") {
                    Some(v) if !v.is_null() => {
                        serde_json::to_value(&social_position).unwrap_or(serde_json::Value::Null)
                    }
                    _ => old_social,
                };
                let necessity_final = match profile.get("narrative_necessity") {
                    Some(v) if !v.is_null() => serde_json::to_value(&narrative_necessity)
                        .unwrap_or(serde_json::Value::Null),
                    _ => old_necessity,
                };
                sqlx::query("UPDATE character_profile SET name=$1, aliases=$2, age_range=$3, gender=$4, identity=$5, appearance=$6, background_origin=$7, social_position=$8, core_personality=$9, \"values\"=$10, role_in_story=$11, narrative_necessity=$12, extra=$13, updated_at=$14 WHERE id=$15")
                    .bind(kept(s("name"), old_name))
                    .bind(aliases_final)
                    .bind(kept(age_range.map(|a| a.as_str().to_string()), old_age))
                    .bind(kept(gender.map(|g| g.as_str().to_string()), old_gender))
                    .bind(kept(s("identity"), old_identity))
                    .bind(kept(s("appearance"), old_appearance))
                    .bind(kept(s("background_origin"), old_background))
                    .bind(social_final)
                    .bind(kept(s("core_personality"), old_personality))
                    .bind(kept(s("values"), old_values))
                    .bind(kept(role.map(|r| r.as_str().to_string()), old_role))
                    .bind(necessity_final)
                    .bind(profile.get("extra").cloned().unwrap_or(old_extra))
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

        /*
         * 同步人物扩展表（drive / capabilities / arc / extension / conflicts / secrets）。
         *
         * 原先这里一律是 `if let Ok(x) = from_value(...)`：反序列化失败就**静默跳过**，
         * 调用方以为写成功了，实际什么都没落库——模型传字符串形态的 drive 时
         * 正是这样被无声丢掉的。现在改为失败即报错，并把期望的形态写进消息。
         */
        if let Some(d) = profile.get("drive") {
            if !d.is_null() {
                // 字符串形态是模型最自然的写法（只给一段动机描述），
                // 记为 motivation；对象形态则按字段解析。
                let drive: CharacterDrive = match d {
                    Value::String(s) if !s.trim().is_empty() => CharacterDrive {
                        motivation: Some(s.trim().to_string()),
                        ..Default::default()
                    },
                    _ => serde_json::from_value(d.clone()).map_err(|e| {
                        anyhow::anyhow!(
                            "drive 格式不对：{}（应为一句话描述，或含 primary_goal / motivation / fear / weakness / desire / contradiction 等字段的对象）",
                            e
                        )
                    })?,
                };
                crate::repos::character_repo::CharacterDriveRepo::new(self.pool.clone())
                    .upsert(id, &drive)
                    .await?;
            }
        }
        if let Some(c) = profile.get("capabilities") {
            if !c.is_null() {
                let cap: CharacterCapability = serde_json::from_value(c.clone())
                    .map_err(|e| anyhow::anyhow!("capabilities 格式不对：{}（应为对象，含 skills / limitations 两个字符串数组）", e))?;
                crate::repos::character_repo::CharacterCapabilityRepo::new(self.pool.clone())
                    .upsert(id, &cap)
                    .await?;
            }
        }
        if let Some(a) = profile.get("arc_potential") {
            if !a.is_null() {
                let arc: CharacterArcPotential = serde_json::from_value(a.clone()).map_err(|e| {
                    anyhow::anyhow!(
                        "arc_potential 格式不对：{}（应为对象，可含 starting_state / possible_change / resistance）",
                        e
                    )
                })?;
                crate::repos::character_repo::CharacterArcRepo::new(self.pool.clone())
                    .upsert(id, &arc)
                    .await?;
            }
        }
        if let Some(e) = profile.get("extension") {
            if !e.is_null() {
                let ext: CharacterExtension = serde_json::from_value(e.clone())
                    .map_err(|e| anyhow::anyhow!("extension 格式不对：{}", e))?;
                crate::repos::character_repo::CharacterExtensionRepo::new(self.pool.clone())
                    .upsert(id, &ext)
                    .await?;
            }
        }

        // 冲突：整组替换。数组元素可以是对象（{conflict_type, description}），
        // 也接受纯字符串（按 Internal 存），因为模型很自然地只给一段描述。
        if let Some(c) = profile.get("conflicts") {
            if !c.is_null() {
                let arr = c
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("conflicts 应为数组"))?;
                let mut items = Vec::with_capacity(arr.len());
                for (i, item) in arr.iter().enumerate() {
                    let description = match item {
                        Value::String(s) => s.trim().to_string(),
                        Value::Object(_) => item
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                        _ => String::new(),
                    };
                    if description.is_empty() {
                        anyhow::bail!(
                            "conflicts[{}] 缺少描述（元素应为字符串，或含 description 的对象）",
                            i
                        );
                    }
                    let conflict_type = match item.get("conflict_type").and_then(|v| v.as_str()) {
                        Some(raw) => ConflictType::parse(raw).ok_or_else(|| {
                            anyhow::anyhow!(
                                "conflicts[{}].conflict_type 无法识别：{}（可传 Internal / External / Relationship / Ideology，或中文「内在」「外在」「关系」「理念」）",
                                i,
                                raw
                            )
                        })?,
                        None => ConflictType::Internal,
                    };
                    let now = Utc::now();
                    items.push(CharacterConflict {
                        // 有 id 时保留，供 conflicts_mode=merge 的局部更新维持引用稳定。
                        id: item
                            .get("id")
                            .and_then(|v| v.as_str())
                            .and_then(|s| Uuid::parse_str(s).ok())
                            .unwrap_or_else(Uuid::new_v4),
                        entity_id: id,
                        conflict_type,
                        description,
                        target_entity_id: item
                            .get("target_entity_id")
                            .and_then(|v| v.as_str())
                            .and_then(|s| Uuid::parse_str(s).ok()),
                        resolution_status: item
                            .get("resolution_status")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        // 该冲突从哪个阶段开始成立（对应 arc_stages[].stage）
                        phase: item
                            .get("phase")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        created_at: now,
                        updated_at: now,
                    });
                }
                crate::repos::character_repo::CharacterConflictRepo::new(self.pool.clone())
                    .replace_by_entity(id, &items)
                    .await?;
            }
        }

        // 阶段弧线（arc_stages）：外部时间线——何时上场、演什么、戏份多大。
        // 与 conflicts / secrets 一样是「整组替换」；人物 / 势力 / 地点共用同一张表。
        sync_arc_stages(&self.pool, id, &profile).await?;

        // 秘密：整组替换，同样接受纯字符串元素。
        if let Some(sv) = profile.get("secrets") {
            if !sv.is_null() {
                let arr = sv
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("secrets 应为数组"))?;
                let mut items = Vec::with_capacity(arr.len());
                for (i, item) in arr.iter().enumerate() {
                    let content = match item {
                        Value::String(s) => s.trim().to_string(),
                        Value::Object(_) => item
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                        _ => String::new(),
                    };
                    if content.is_empty() {
                        anyhow::bail!(
                            "secrets[{}] 缺少内容（元素应为字符串，或含 content 的对象）",
                            i
                        );
                    }
                    let now = Utc::now();
                    items.push(CharacterSecret {
                        // 有 id 时保留，供 secrets_mode=merge 的局部更新维持引用稳定。
                        id: item
                            .get("id")
                            .and_then(|v| v.as_str())
                            .and_then(|s| Uuid::parse_str(s).ok())
                            .unwrap_or_else(Uuid::new_v4),
                        entity_id: id,
                        content,
                        importance: item
                            .get("importance")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32)
                            .unwrap_or(3),
                        reveal_condition: item
                            .get("reveal_condition")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        related_entities: item
                            .get("related_entities")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .filter_map(|s| Uuid::parse_str(s).ok())
                                    .collect()
                            })
                            .unwrap_or_default(),
                        created_at: now,
                        updated_at: now,
                    });
                }
                crate::repos::character_repo::CharacterSecretRepo::new(self.pool.clone())
                    .replace_by_entity(id, &items)
                    .await?;
            }
        }

        let mut out = profile.clone();
        out["entity_id"] = serde_json::json!(id.to_string());
        Ok(out)
    }

    async fn update_character_state(&self, id: Uuid, state: Value, _actor: &str) -> Result<Value> {
        let now = Utc::now();
        let s = |k: &str| state.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        // flags 列是 jsonb，必须绑定 serde_json::Value；
        // 绑定 Vec<String> 会按 text[] 编码，与列类型不匹配而报错。
        let flags: Value = state
            .get("flags")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let extra: Value = state
            .get("extra")
            .cloned()
            .unwrap_or(serde_json::json!(null));

        // 同档案：未传的字段保持原值，不能整行覆盖成 NULL
        let existing: Option<(
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Value,
            Value,
        )> = sqlx::query_as(
            "SELECT id, location, physical_state, mental_state, resource_state, social_state, \
             flags, extra FROM character_state WHERE entity_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("check character state")?;
        match existing {
            Some((pid, old_loc, old_phys, old_ment, old_res, old_soc, old_flags, old_extra)) => {
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                sqlx::query("UPDATE character_state SET location=$1, physical_state=$2, mental_state=$3, resource_state=$4, social_state=$5, flags=$6, extra=$7, updated_at=$8 WHERE id=$9")
                    .bind(kept(s("location"), old_loc))
                    .bind(kept(s("physical_state"), old_phys))
                    .bind(kept(s("mental_state"), old_ment))
                    .bind(kept(s("resource_state"), old_res))
                    .bind(kept(s("social_state"), old_soc))
                    .bind(if state.get("flags").is_some() { flags } else { old_flags })
                    .bind(if state.get("extra").is_some() { extra } else { old_extra })
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
        let arc_stages = load_arc_stages(&self.pool, id).await?;
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
        let Some(r) = row else {
            // 可能只写了阶段弧线、还没写静态档案：这时也要能读到阶段
            return Ok(if arc_stages.is_empty() {
                None
            } else {
                Some(serde_json::json!({
                    "entity_id": id.to_string(),
                    "arc_stages": arc_stages
                }))
            });
        };
        Ok(Some(serde_json::json!({
            "geography": r.0, "appearance": r.1, "population": r.2, "economy": r.3,
            "rules": r.4, "history": r.5, "narrative_usage": r.6,
            "location_type": r.7, "size": r.8, "climate": r.9, "era": r.10, "accessibility": r.11,
            "arc_stages": arc_stages
        })))
    }

    async fn upsert_location_profile(&self, id: Uuid, profile: Value, _actor: &str) -> Result<Value> {
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

        // 取出 uuid 本身（**不要** `id::text`）：若拿字符串回填到 `WHERE id = $n`，
        // Postgres 会因 text 与 uuid 类型不符而报错，表现为更新已有档案时 500。
        //
        // 同时把旧值一起读出来：agent 工具契约是「只传需要设置的字段，未传的保持原值」，
        // 整行覆盖会让模型只改一个字段就把其余字段（含另一张表里的）清空。
        let lp: Option<(
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, geography, appearance, population, economy, rules, history, \
             narrative_usage FROM location_profile WHERE entity_id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await.context("chk loc profile")?;
        match lp {
            Some((pid, g0, a0, p0, e0, r0, h0, n0)) => {
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                sqlx::query("UPDATE location_profile SET geography=$1, appearance=$2, population=$3, economy=$4, rules=$5, history=$6, narrative_usage=$7, updated_at=NOW() WHERE id=$8")
                    .bind(kept(geography, g0)).bind(kept(appearance, a0)).bind(kept(population, p0))
                    .bind(kept(economy, e0)).bind(kept(rules, r0)).bind(kept(history, h0))
                    .bind(kept(narrative_usage, n0)).bind(pid)
                    .execute(&self.pool).await.context("upd loc profile")?;
            }
            None => {
                sqlx::query("INSERT INTO location_profile (id, entity_id, geography, appearance, population, economy, rules, history, narrative_usage) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8)")
                    .bind(id).bind(&geography).bind(&appearance).bind(&population).bind(&economy).bind(&rules).bind(&history).bind(&narrative_usage)
                    .execute(&self.pool).await.context("ins loc profile")?;
            }
        }
        // 地点身份信息同样「未传保持原值」
        let li: Option<(
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, location_type, size, climate, era, accessibility \
             FROM location_identity WHERE entity_id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await.context("chk loc identity")?;
        match li {
            Some((pid, lt0, sz0, cl0, er0, ac0)) => {
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                sqlx::query("UPDATE location_identity SET location_type=$1, size=$2, climate=$3, era=$4, accessibility=$5, updated_at=NOW() WHERE id=$6")
                    .bind(kept(location_type, lt0)).bind(kept(size, sz0)).bind(kept(climate, cl0))
                    .bind(kept(era, er0)).bind(kept(accessibility, ac0)).bind(pid)
                    .execute(&self.pool).await.context("upd loc identity")?;
            }
            None => {
                sqlx::query("INSERT INTO location_identity (id, entity_id, location_type, size, climate, era, accessibility) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6)")
                    .bind(id).bind(&location_type).bind(&size).bind(&climate).bind(&era).bind(&accessibility)
                    .execute(&self.pool).await.context("ins loc identity")?;
            }
        }
        // 阶段弧线：人物 / 势力 / 地点共用同一张表
        sync_arc_stages(&self.pool, id, &profile).await?;
        let mut out = profile.clone();
        out["entity_id"] = serde_json::json!(id.to_string());
        Ok(out)
    }

    async fn get_faction_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let arc_stages = load_arc_stages(&self.pool, id).await?;
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
        let Some(r) = row else {
            // 可能只写了阶段弧线、还没写静态档案：这时也要能读到阶段
            return Ok(if arc_stages.is_empty() {
                None
            } else {
                Some(serde_json::json!({
                    "entity_id": id.to_string(),
                    "arc_stages": arc_stages
                }))
            });
        };
        Ok(Some(serde_json::json!({
            "goals": r.0, "leader": r.1, "values": r.2, "resources": r.3,
            "territory": r.4, "members": r.5, "enemies": r.6, "allies": r.7,
            "internal_conflicts": r.8, "secrets": r.9, "modus_operandi": r.10,
            "arc_stages": arc_stages
        })))
    }

    async fn upsert_faction_profile(&self, id: Uuid, profile: Value, _actor: &str) -> Result<Value> {
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
        // 未传的字段保持原值（agent 工具契约，见 update_faction_profile 的描述）
        let existing: Option<(
            Uuid, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, goals, leader, \"values\", resources, territory, members, enemies, \
             allies, internal_conflicts, secrets, modus_operandi \
             FROM faction_profile WHERE entity_id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await.context("chk faction profile")?;
        match existing {
            Some((pid, g0, l0, v0, r0, t0, m0, e0, a0, ic0, s0, mo0)) => {
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                sqlx::query("UPDATE faction_profile SET goals=$1, leader=$2, \"values\"=$3, resources=$4, territory=$5, members=$6, enemies=$7, allies=$8, internal_conflicts=$9, secrets=$10, modus_operandi=$11, updated_at=NOW() WHERE id=$12")
                    .bind(kept(goals, g0)).bind(kept(leader, l0)).bind(kept(values, v0))
                    .bind(kept(resources, r0)).bind(kept(territory, t0)).bind(kept(members, m0))
                    .bind(kept(enemies, e0)).bind(kept(allies, a0)).bind(kept(internal_conflicts, ic0))
                    .bind(kept(secrets, s0)).bind(kept(modus_operandi, mo0)).bind(pid)
                    .execute(&self.pool).await.context("upd faction profile")?;
            }
            None => {
                sqlx::query("INSERT INTO faction_profile (id, entity_id, goals, leader, \"values\", resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
                    .bind(id).bind(&goals).bind(&leader).bind(&values).bind(&resources).bind(&territory).bind(&members).bind(&enemies).bind(&allies).bind(&internal_conflicts).bind(&secrets).bind(&modus_operandi)
                    .execute(&self.pool).await.context("ins faction profile")?;
            }
        }
        // 阶段弧线：人物 / 势力 / 地点共用同一张表
        sync_arc_stages(&self.pool, id, &profile).await?;
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

    async fn snapshot_profile(&self, id: Uuid, kind: &str, before: &Value, actor: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO entity_profile_snapshot (entity_id, project_id, kind, payload, actor) \
             SELECT id, project_id, $2, $3, $4 FROM entity WHERE id = $1",
        )
        .bind(id)
        .bind(kind)
        .bind(before)
        .bind(actor)
        .execute(&self.pool)
        .await
        .context("Failed to snapshot entity profile")?;
        Ok(())
    }

    async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<Value>> {
        // 双向查：关系是**相互**的，但原先只按 `source_entity_id = $1` 过滤，
        // 于是「王久财 → 周浩」这条在周浩的详情页里根本看不到——
        // 用户会以为这条关系丢了。
        //
        // 同时统一字段名：原先返回 `target` / `target_id`，而前端按
        // `source_name` / `target_name` 取名字，取不到就显示「未知对象」。
        // 现在直接给「对方」的 id 与名字，并由 `outgoing` 标明方向。
        let rows: Vec<(String, String, String, String, Option<String>, bool)> = sqlx::query_as(
            "SELECT r.id::text, r.relation_type, other.id::text, other.name, r.description, \
                    (r.source_entity_id = $1) AS outgoing \
             FROM relation r \
             JOIN entity other ON other.id = CASE \
                 WHEN r.source_entity_id = $1 THEN r.target_entity_id \
                 ELSE r.source_entity_id END \
             WHERE (r.source_entity_id = $1 OR r.target_entity_id = $1) \
               AND other.status != 'Deleted' \
             ORDER BY other.name",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get character relationships")?;
        Ok(rows
            .into_iter()
            .map(|(rel_id, rtype, other_id, other_name, desc, outgoing)| {
                serde_json::json!({
                    "id": rel_id,
                    "relation_type": rtype,
                    "other_id": other_id,
                    "other_name": other_name,
                    "description": desc,
                    // true = 当前实体指向对方
                    "outgoing": outgoing,
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

// ============================================================
// 阶段弧线（entity_arc_stage）：人物 / 势力 / 地点共用
// ============================================================

/// 解析 `profile["arc_stages"]` 为结构化的阶段列表。
///
/// - 元素可写字符串（只给阶段名），也可写成对象
/// - `screen_weight` 接受 Light/Medium/Heavy 或中文「轻/中/重」
/// - `order` 缺省用数组下标，保证"没写 order"也能按给出顺序展示
///
/// 返回 `Ok(None)` 表示入参里**没有**这个字段（调用方应保持原值）。
fn parse_arc_stages(entity_id: Uuid, raw: Option<&Value>) -> Result<Option<Vec<EntityArcStage>>> {
    let Some(v) = raw else { return Ok(None) };
    if v.is_null() {
        return Ok(None);
    }
    let arr = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("arc_stages 应为数组"))?;
    let mut items = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let (stage, obj) = match item {
            Value::String(s) => (s.trim().to_string(), None),
            Value::Object(_) => {
                let stage = item
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                (stage, Some(item))
            }
            _ => (String::new(), None),
        };
        if stage.is_empty() {
            anyhow::bail!(
                "arc_stages[{}] 缺少阶段名（元素应为字符串，或含 stage 的对象）",
                i
            );
        }
        let field = |k: &str| -> Option<String> {
            obj.and_then(|o| o.get(k))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let order = obj
            .and_then(|o| o.get("order"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(i as i32);
        let screen_weight = match field("screen_weight") {
            Some(raw) => Some(ScreenWeight::parse(&raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "arc_stages[{}].screen_weight 无法识别：{}（可传 Light / Medium / Heavy，或中文「轻」「中」「重」）",
                    i,
                    raw
                )
            })?),
            None => None,
        };
        let now = Utc::now();
        items.push(EntityArcStage {
            id: obj
                .and_then(|o| o.get("id"))
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4),
            entity_id,
            stage,
            order,
            role: field("role"),
            screen_weight,
            goal: field("goal"),
            function: field("function"),
            entry_trigger: field("entry_trigger"),
            status: field("status"),
            created_at: now,
            updated_at: now,
        });
    }
    Ok(Some(items))
}

/// 有 `arc_stages` 字段就整组替换落库（人物 / 势力 / 地点共用）。
async fn sync_arc_stages(pool: &PgPool, entity_id: Uuid, profile: &Value) -> Result<()> {
    if let Some(items) = parse_arc_stages(entity_id, profile.get("arc_stages"))? {
        crate::repos::arc_stage_repo::EntityArcStageRepo::new(pool.clone())
            .replace_by_entity(entity_id, &items)
            .await?;
    }
    Ok(())
}

/// 读取某实体的阶段弧线（人物 / 势力 / 地点共用）。
async fn load_arc_stages(pool: &PgPool, entity_id: Uuid) -> Result<Vec<EntityArcStage>> {
    crate::repos::arc_stage_repo::EntityArcStageRepo::new(pool.clone())
        .list_by_entity(entity_id)
        .await
}

