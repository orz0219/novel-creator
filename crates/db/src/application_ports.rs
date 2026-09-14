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
        let rows: Vec<NarrativeNodeJsonRow> = sqlx::query_as(
            "SELECT n.id, n.project_id, n.world_id, n.node_type, n.parent_id, n.title, n.description, n.content, n.attributes, n.sort_order, n.status, n.storyline_id, n.arc_stage, n.stage_refs, n.participant_entity_ids, n.location_id, n.item_ids, n.estimated_chapters, n.estimated_words, n.story_time, n.created_at::text AS created_at, n.updated_at::text AS updated_at, s.name AS storyline_name, (SELECT COUNT(*) FROM narrative_node c WHERE c.parent_id = n.id AND c.status != 'Deleted') AS child_count FROM narrative_node n LEFT JOIN storyline s ON s.id = n.storyline_id WHERE n.project_id = $1 AND n.status != 'Deleted' ORDER BY n.sort_order, n.created_at"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list narrative nodes")?;

        rows.into_iter().map(node_row_to_json).collect()
    }

    async fn get_node(&self, id: Uuid) -> Result<Option<Value>> {
        // 详情里带上故事线名字与子节点数：模型与前端都不必再为一行节点多查两次。
        let row: Option<NarrativeNodeJsonRow> = sqlx::query_as(
            "SELECT n.id, n.project_id, n.world_id, n.node_type, n.parent_id, n.title, n.description, n.content, n.attributes, n.sort_order, n.status, n.storyline_id, n.arc_stage, n.stage_refs, n.participant_entity_ids, n.location_id, n.item_ids, n.estimated_chapters, n.estimated_words, n.story_time, n.created_at::text AS created_at, n.updated_at::text AS updated_at, s.name AS storyline_name, (SELECT COUNT(*) FROM narrative_node c WHERE c.parent_id = n.id AND c.status != 'Deleted') AS child_count FROM narrative_node n LEFT JOIN storyline s ON s.id = n.storyline_id WHERE n.id = $1 AND n.status != 'Deleted'"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get narrative node")?;

        row.map(node_row_to_json).transpose()
    }

    async fn create_node_full(&self, input: domain::narrative::NewNarrativeNode) -> Result<Value> {
        // 词表校验（严格：拼错就报错，不静默造出新的节点类型）
        let node_type = NarrativeNodeType::parse_strict(&input.node_type)?;
        let status = match input.status.as_deref() {
            Some(s) => NarrativeNodeStatus::parse_strict(s)?,
            None => NarrativeNodeStatus::Draft,
        };
        if input.title.trim().is_empty() {
            anyhow::bail!("title 不能为空");
        }
        if let Some(o) = input.sort_order {
            if o < 1 {
                anyhow::bail!("sort_order 从 1 起（同父下的序号），收到 {}", o);
            }
        }
        if input.arc_stage.is_some() && input.storyline_id.is_none() {
            anyhow::bail!("arc_stage 需要同时给 storyline_id：阶段属于某条故事线");
        }

        let world_id: (Uuid,) = sqlx::query_as("SELECT id FROM world WHERE project_id = $1 LIMIT 1")
            .bind(input.project_id)
            .fetch_one(&self.pool)
            .await
            .context("Failed to get world for project")?;

        // 让位 + 插入必须在同一事务里：兄弟序号的唯一约束是 DEFERRABLE，
        // 只有事务内才允许「先让位、后落位」这种瞬时重复。
        let mut tx = self.pool.begin().await.context("Failed to begin node tx")?;

        if let Some(p) = input.parent_id {
            if p == input.project_id {
                anyhow::bail!("parent_id 不是项目 id");
            }
            let parent_project: Option<Uuid> = sqlx::query_scalar(
                "SELECT project_id FROM narrative_node WHERE id=$1 AND status != 'Deleted'",
            )
            .bind(p)
            .fetch_optional(&mut *tx)
            .await
            .context("Failed to read parent narrative node")?;
            match parent_project {
                None => anyhow::bail!("父节点不存在或已被删除：{}", p),
                Some(pid) if pid != input.project_id => {
                    anyhow::bail!("父节点 {} 不属于本项目", p)
                }
                Some(_) => {}
            }
        }

        if let Some(sid) = input.storyline_id {
            crate::narrative_checks::ensure_storyline_stage(
                &mut tx,
                input.project_id,
                sid,
                input.arc_stage.as_deref(),
            )
            .await?;
        }
        for r in &input.stage_refs {
            crate::narrative_checks::ensure_storyline_stage(
                &mut tx,
                input.project_id,
                r.storyline_id,
                r.arc_stage.as_deref(),
            )
            .await?;
        }
        crate::narrative_checks::ensure_entities_in_project(
            &mut tx,
            input.project_id,
            &input.participant_entity_ids,
            "participant_entity_ids",
        )
        .await?;
        if let Some(lid) = input.location_id {
            crate::narrative_checks::ensure_entities_in_project(
                &mut tx,
                input.project_id,
                &[lid],
                "location_id",
            )
            .await?;
        }
        crate::narrative_checks::ensure_entities_in_project(
            &mut tx,
            input.project_id,
            &input.item_ids,
            "item_ids",
        )
        .await?;

        // 序号：显式给了就先给同父的其他节点让位，否则追加到末尾。
        let sort_order: i32 = match input.sort_order {
            Some(o) => {
                sqlx::query(
                    "UPDATE narrative_node SET sort_order = sort_order + 1, updated_at = NOW() WHERE project_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND sort_order >= $3",
                )
                .bind(input.project_id)
                .bind(input.parent_id)
                .bind(o)
                .execute(&mut *tx)
                .await
                .context("Failed to shift sibling sort_order")?;
                o
            }
            None => {
                let max: (i32,) = sqlx::query_as(
                    "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM narrative_node WHERE project_id = $1 AND parent_id IS NOT DISTINCT FROM $2",
                )
                .bind(input.project_id)
                .bind(input.parent_id)
                .fetch_one(&mut *tx)
                .await
                .context("Failed to get sort order")?;
                max.0
            }
        };

        let id = Uuid::new_v4();
        let status_str = status.as_db_str();
        sqlx::query(
            "INSERT INTO narrative_node (id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, storyline_id, arc_stage, stage_refs, participant_entity_ids, location_id, item_ids, estimated_chapters, estimated_words, story_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
        )
        .bind(&id)
        .bind(input.project_id)
        .bind(&world_id.0)
        .bind(node_type.as_db_str())
        .bind(input.parent_id)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.content)
        .bind(&input.attributes)
        .bind(sort_order)
        .bind(&status_str)
        .bind(input.storyline_id)
        .bind(&input.arc_stage)
        .bind(serde_json::to_value(&input.stage_refs).context("stage_refs 序列化失败")?)
        .bind(serde_json::to_value(&input.participant_entity_ids).context("participant_entity_ids 序列化失败")?)
        .bind(input.location_id)
        .bind(serde_json::to_value(&input.item_ids).context("item_ids 序列化失败")?)
        .bind(input.estimated_chapters)
        .bind(input.estimated_words)
        .bind(&input.story_time)
        .execute(&mut *tx)
        .await
        .context("Failed to create narrative node")?;

        tx.commit().await.context("Failed to commit node tx")?;

        self.get_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("新建的叙事节点读不回来（id={}）", id))
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
                return Err(anyhow::anyhow!("叙事节点已逻辑删除，不能再修改（id={}）", id));
            }
            None => {
                return Err(anyhow::anyhow!("叙事节点不存在（id={}）", id));
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
            let parsed = NarrativeNodeStatus::parse_strict(s)?;
            sqlx::query("UPDATE narrative_node SET status=$1, updated_at=NOW() WHERE id=$2 AND project_id = (SELECT project_id FROM narrative_node WHERE id = $3)")
                .bind(parsed.as_db_str()).bind(id).bind(id).execute(&self.pool).await?;
        }

        self.get_node(id).await?
            .ok_or_else(|| anyhow::anyhow!("叙事节点更新后读不回来（id={}）", id))
    }

    async fn list_nodes_page(
        &self,
        project_id: Uuid,
        filter: &domain::narrative::NarrativeNodeFilter,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Value>, usize)> {
        if filter.parent_id.is_some() && filter.roots_only {
            anyhow::bail!("parent_id 与 roots_only 不能同时给：一个是下钻某个父节点，一个是只看顶层");
        }
        // 节点类型过滤同样走严格词表：拼错的类型应该报错，而不是静默返回空列表
        let node_type: Option<String> = match filter.node_type.as_deref() {
            Some(t) => Some(NarrativeNodeType::parse_strict(t)?.as_db_str()),
            None => None,
        };
        // 过滤条件写进同一条 SQL 的可选参数里（不拼接字符串），LIMIT/OFFSET 下推到数据库
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM narrative_node n WHERE n.project_id = $1 AND n.status != 'Deleted' AND ($2::uuid IS NULL OR n.parent_id = $2) AND ($3::varchar IS NULL OR n.node_type = $3) AND ($4::uuid IS NULL OR n.storyline_id = $4) AND (NOT $5::bool OR n.parent_id IS NULL)",
        )
        .bind(project_id)
        .bind(filter.parent_id)
        .bind(&node_type)
        .bind(filter.storyline_id)
        .bind(filter.roots_only)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count narrative nodes")?;

        let rows: Vec<NarrativeNodeJsonRow> = sqlx::query_as(
            "SELECT n.id, n.project_id, n.world_id, n.node_type, n.parent_id, n.title, n.description, n.content, n.attributes, n.sort_order, n.status, n.storyline_id, n.arc_stage, n.stage_refs, n.participant_entity_ids, n.location_id, n.item_ids, n.estimated_chapters, n.estimated_words, n.story_time, n.created_at::text AS created_at, n.updated_at::text AS updated_at, s.name AS storyline_name, (SELECT COUNT(*) FROM narrative_node c WHERE c.parent_id = n.id AND c.status != 'Deleted') AS child_count FROM narrative_node n LEFT JOIN storyline s ON s.id = n.storyline_id WHERE n.project_id = $1 AND n.status != 'Deleted' AND ($2::uuid IS NULL OR n.parent_id = $2) AND ($3::varchar IS NULL OR n.node_type = $3) AND ($4::uuid IS NULL OR n.storyline_id = $4) AND (NOT $5::bool OR n.parent_id IS NULL) ORDER BY n.sort_order, n.created_at LIMIT $6 OFFSET $7",
        )
        .bind(project_id)
        .bind(filter.parent_id)
        .bind(&node_type)
        .bind(filter.storyline_id)
        .bind(filter.roots_only)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list narrative nodes page")?;

        let items = rows
            .into_iter()
            .map(node_row_to_json)
            .collect::<Result<Vec<_>>>()?;
        Ok((items, total.0 as usize))
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
            return Err(anyhow::anyhow!("叙事节点不存在或已逻辑删除（id={}）", id));
        }

        Ok(())
    }
}

/// 叙事节点的行投影（含故事线名与直接子节点数）。
///
/// 为什么不再用位置元组：加了细纲的九列之后元组有 22 个元素，
/// 而 sqlx 只为最多 16 元的元组实现 FromRow——继续用元组会直接编译不过；
/// 而且位置元组加一列就要改所有解构点，很容易错位。
#[derive(sqlx::FromRow)]
struct NarrativeNodeJsonRow {
    id: Uuid,
    project_id: Uuid,
    world_id: Uuid,
    node_type: String,
    parent_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    content: Option<String>,
    attributes: Option<Value>,
    sort_order: i32,
    status: String,
    storyline_id: Option<Uuid>,
    arc_stage: Option<String>,
    stage_refs: Option<Value>,
    participant_entity_ids: Option<Value>,
    location_id: Option<Uuid>,
    item_ids: Option<Value>,
    estimated_chapters: Option<i32>,
    estimated_words: Option<i32>,
    story_time: Option<String>,
    created_at: String,
    updated_at: String,
    storyline_name: Option<String>,
    child_count: i64,
}

/// 行 -> JSON。JSONB 列缺省即空（NULL 与 '[]' 同义），不做静默改写。
fn node_row_to_json(r: NarrativeNodeJsonRow) -> Result<Value> {
    let empty_array = Value::Array(Vec::new());
    Ok(serde_json::json!({
        "id": r.id,
        "project_id": r.project_id,
        "world_id": r.world_id,
        "node_type": r.node_type,
        "parent_id": r.parent_id,
        "title": r.title,
        "description": r.description,
        "content": r.content,
        "attributes": r.attributes.unwrap_or_else(|| serde_json::json!({})),
        "sort_order": r.sort_order,
        "status": r.status,
        "storyline_id": r.storyline_id,
        "storyline_name": r.storyline_name,
        "arc_stage": r.arc_stage,
        "stage_refs": r.stage_refs.unwrap_or_else(|| empty_array.clone()),
        "participant_entity_ids": r.participant_entity_ids.unwrap_or_else(|| empty_array.clone()),
        "location_id": r.location_id,
        "item_ids": r.item_ids.unwrap_or_else(|| empty_array.clone()),
        "estimated_chapters": r.estimated_chapters,
        "estimated_words": r.estimated_words,
        "story_time": r.story_time,
        "child_count": r.child_count,
        "created_at": r.created_at,
        "updated_at": r.updated_at,
    }))
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

/// 按 id 读回一条剧情线：`update_storyline` 用它回显修改后的**完整对象**。
///
/// 原先只回 `{"updated":true}`——调用方（AI）看不到写进去的是什么，只能再 list
/// 一遍；本项目的 storyline 全文近万字，重复拉取代价很大（上下文也是钱）。
async fn fetch_storyline_row_opt(pool: &sqlx::PgPool, id: Uuid) -> Result<Option<Value>> {
    let row: Option<(
        String, String, String, Option<String>, String, String, String, String, String, String, Value,
    )> = sqlx::query_as(
        "SELECT id::text, project_id::text, name, description, status, importance, \
                COALESCE(tone,'light'), COALESCE(visibility,'visible'), \
                created_at::text, updated_at::text, arc_stages \
         FROM storyline WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to read back storyline")?;
    Ok(row.map(
        |(id, project_id, name, desc, st, imp, tone, vis, cr, up, arc_stages)| {
            serde_json::json!({
                "id": id, "project_id": project_id, "name": name,
                "description": desc, "status": st, "importance": imp,
                "tone": tone, "visibility": vis,
                "arc_stages": arc_stages,
                "created_at": cr, "updated_at": up
            })
        },
    ))
}

/// 读回一条剧情线，不存在则报错。
async fn fetch_storyline_row(pool: &sqlx::PgPool, id: Uuid) -> Result<Value> {
    fetch_storyline_row_opt(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("剧情线不存在: {}", id))
}

#[async_trait]
impl StorylineRepositoryPort for DbStorylineRepositoryPort {
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Storyline>> {
        crate::repos::storyline_repo::StorylineRepo::new(self.pool.clone())
            .list_by_project(project_id)
            .await
    }

    async fn list_storylines(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, Option<String>, String, String, String, String, String, String, Value)> =
            sqlx::query_as(
                "SELECT id::text, name, description, status, importance, COALESCE(tone,'light'), COALESCE(visibility,'visible'), created_at::text, updated_at::text, arc_stages FROM storyline WHERE project_id=$1 ORDER BY importance, name",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list storylines")?;
        Ok(rows
            .into_iter()
            .map(|(id, name, desc, st, imp, tone, vis, cr, up, arc_stages)| {
                serde_json::json!({
                    "id": id, "project_id": project_id.to_string(), "name": name,
                    "description": desc, "status": st, "importance": imp,
                    "tone": tone, "visibility": vis,
                    "arc_stages": arc_stages,
                    "created_at": cr, "updated_at": up
                })
            })
            .collect())
    }

    async fn get_storyline(&self, id: Uuid) -> Result<Option<Value>> {
        fetch_storyline_row_opt(&self.pool, id).await
    }

    async fn create_storyline(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        status: &str,
        importance: &str,
        tone: &str,
        visibility: &str,
        parent_id: Option<Uuid>,
        arc_stages: Option<&Value>,
    ) -> Result<Value> {
        // 业务规则：
        //   - Main 主线必须 parent_id=None
        //   - Normal 副线 parent_id 可选（None 表示独立于任何 story line）
        //   - DB 唯一约束：UNIQUE(parent_id, child_id) 在 storyline_relation 表上
        if importance == "Main" && parent_id.is_some() {
            anyhow::bail!("Main 主线不能挂到其他 story line 下");
        }
        let id = Uuid::new_v4();
        // 未提供阶段弧线就是空列表（列本身有 DEFAULT '[]'，这里显式给值以免落 NULL）
        let arc_stages = arc_stages.cloned().unwrap_or_else(|| serde_json::json!([]));
        sqlx::query(
            "INSERT INTO storyline (id, project_id, name, description, status, importance, tone, visibility, arc_stages) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(status)
        .bind(importance)
        .bind(tone)
        .bind(visibility)
        .bind(&arc_stages)
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

        // 回显**完整对象**（含刚写入的 arc_stages 与 parent_id），
        // 与 update 的回显保持一致：调用方建完就能看到实际落库的内容。
        let mut created = fetch_storyline_row(&self.pool, id).await?;
        if let Some(pid) = parent_id {
            created["parent_id"] = serde_json::json!(pid.to_string());
        }
        Ok(created)
    }

    async fn update_storyline(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        tone: Option<&str>,
        visibility: Option<&str>,
        arc_stages: Option<&Value>,
    ) -> Result<Value> {
        // 动态 SET 拼接：未传的字段保持原值。
        // 占位符编号必须与绑定顺序严格对应，因此这里显式记录每个字段用到的编号。
        // name / description 走 COALESCE：None（不传）保持原值。
        // 原先是无条件写入 —— 「只改名字」会把描述一起清空（实测语义陷阱）。
        let mut sql = String::from(
            "UPDATE storyline SET name=COALESCE($1, name), \
             description=COALESCE($2, description), updated_at=NOW()",
        );
        let mut next_index = 5;
        let mut bind_status = false;
        let mut bind_tone = false;
        let mut bind_visibility = false;
        let mut bind_arc_stages = false;
        if status.is_some() {
            sql.push_str(&format!(", status=${}", next_index));
            next_index += 1;
            bind_status = true;
        }
        if tone.is_some() {
            sql.push_str(&format!(", tone=${}", next_index));
            next_index += 1;
            bind_tone = true;
        }
        if visibility.is_some() {
            sql.push_str(&format!(", visibility=${}", next_index));
            bind_visibility = true;
        }
        if arc_stages.is_some() {
            sql.push_str(&format!(", arc_stages=${}", next_index));
            bind_arc_stages = true;
        }
        sql.push_str(" WHERE id=$3 AND project_id = (SELECT project_id FROM storyline WHERE id = $4)");

        let mut q = sqlx::query(&sql)
            .bind(name)
            .bind(description)
            .bind(id)
            .bind(id);
        if bind_status {
            q = q.bind(status);
        }
        if bind_tone {
            q = q.bind(tone);
        }
        if bind_visibility {
            q = q.bind(visibility);
        }
        if bind_arc_stages {
            q = q.bind(arc_stages);
        }
        q.execute(&self.pool).await.context("Failed to update storyline")?;
        // 回显修改后的完整对象（原先只回 {"updated":true}，调用方只能再 list 全文）
        fetch_storyline_row(&self.pool, id).await
    }

    async fn delete_storyline(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM storyline WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete storyline")?;
        Ok(())
    }

    async fn relate_storylines(
        &self,
        project_id: Uuid,
        from_storyline_id: Uuid,
        to_storyline_id: Uuid,
        relation_type: &str,
    ) -> Result<Value> {
        if from_storyline_id == to_storyline_id {
            anyhow::bail!("不能把一条剧情线连到它自己");
        }
        // 幂等：同 (from, to) 已存在就更新类型（表上有 UNIQUE(parent_id, child_id)）
        let (id, relation_type): (String, String) = sqlx::query_as(
            "INSERT INTO storyline_relation (id, project_id, parent_id, child_id, relation_type) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4) \
             ON CONFLICT (parent_id, child_id) DO UPDATE SET relation_type = EXCLUDED.relation_type \
             RETURNING id::text, relation_type",
        )
        .bind(project_id)
        .bind(from_storyline_id)
        .bind(to_storyline_id)
        .bind(relation_type)
        .fetch_one(&self.pool)
        .await
        .context("Failed to relate storylines")?;
        Ok(serde_json::json!({
            "id": id,
            "project_id": project_id.to_string(),
            "parent_id": from_storyline_id.to_string(),
            "child_id": to_storyline_id.to_string(),
            "relation_type": relation_type,
        }))
    }

    async fn unrelate_storylines(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM storyline_relation WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete storyline relation")?;
        if result.rows_affected() == 0 {
            anyhow::bail!("剧情线关系不存在: {}", id);
        }
        Ok(())
    }

    async fn list_storyline_relations(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<Value>> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id::text, parent_id::text, child_id::text, relation_type, created_at::text \
             FROM storyline_relation WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list storyline_relations")?;
        Ok(rows
            .into_iter()
            .map(|(id, parent, child, relation_type, cr)| {
                serde_json::json!({
                    "id": id, "project_id": project_id.to_string(),
                    "parent_id": parent, "child_id": child,
                    "relation_type": relation_type,
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


/// 伏笔行（含所属剧情线名字）：`list_foreshadows` 与「更新后回显」共用同一份，
/// 避免两处字段清单不一致——历史上 hint_level 的语义就在两处漂移过。
#[derive(sqlx::FromRow)]
struct ForeshadowRow {
    id: String,
    project_id: String,
    name: String,
    description: Option<String>,
    status: String,
    importance: String,
    hint_level: String,
    hint_note: Option<String>,
    introduced_at: Option<String>,
    expected_reveal_at: Option<String>,
    actual_reveal_at: Option<String>,
    planted_node_id: Option<String>,
    payoff_node_id: Option<String>,
    parent_foreshadow_id: Option<String>,
    created_at: String,
    updated_at: String,
    storyline_id: Option<String>,
    storyline_name: Option<String>,
}

impl ForeshadowRow {
    fn into_json(self) -> Value {
        serde_json::json!({
            "id": self.id,
            "project_id": self.project_id,
            "name": self.name,
            "description": self.description,
            "status": self.status,
            "importance": self.importance,
            "hint_level": self.hint_level,
            "hint_note": self.hint_note,
            "introduced_at": self.introduced_at,
            "expected_reveal_at": self.expected_reveal_at,
            "actual_reveal_at": self.actual_reveal_at,
            "planted_node_id": self.planted_node_id,
            "payoff_node_id": self.payoff_node_id,
            "parent_foreshadow_id": self.parent_foreshadow_id,
            "storyline_id": self.storyline_id,
            "storyline_name": self.storyline_name,
            "related_entity_ids": [],
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        })
    }
}

/// 伏笔行的统一查询（列清单只有一份，`where_clause` 由调用方给）。
fn foreshadow_select(where_clause: &str) -> String {
    format!(
        "SELECT f.id::text AS id, f.project_id::text AS project_id, f.name AS name, \
                f.description AS description, f.status AS status, \
                f.importance AS importance, f.hint_level AS hint_level, \
                f.hint_note AS hint_note, f.introduced_at AS introduced_at, \
                f.expected_reveal_at AS expected_reveal_at, \
                f.actual_reveal_at AS actual_reveal_at, \
                f.planted_node_id::text AS planted_node_id, \
                f.payoff_node_id::text AS payoff_node_id, \
                f.parent_foreshadow_id::text AS parent_foreshadow_id, \
                f.created_at::text AS created_at, f.updated_at::text AS updated_at, \
                f.storyline_id::text AS storyline_id, s.name AS storyline_name \
         FROM foreshadowing f LEFT JOIN storyline s ON s.id = f.storyline_id {where_clause}"
    )
}

/// 按 id 读回一条伏笔：`update_foreshadow` 用它回显（省掉调用方再 list 一遍全文），
/// `create_foreshadow` 也用它返回完整对象。
///
/// 顺带修掉一个隐患：原先 update 不校验 id 是否存在，改一个不存在的 id 也会回
/// `{"updated":true}`；现在读回失败会明确报错。
///
/// 刻意**不**放进 `ForeshadowRepositoryPort`：它是实现细节，端口只描述业务能力。
async fn fetch_foreshadow_row(pool: &sqlx::PgPool, id: Uuid) -> Result<Value> {
    let row: ForeshadowRow = sqlx::query_as(&foreshadow_select("WHERE f.id=$1"))
        .bind(id)
        .fetch_one(pool)
        .await
        .context("Failed to read back foreshadow")?;
    Ok(row.into_json())
}

#[async_trait]
impl ForeshadowRepositoryPort for DbForeshadowRepositoryPort {
    async fn list_foreshadows(&self, project_id: Uuid) -> Result<Vec<Value>> {
        // 顺带把所属剧情线的**名字**查出来：只有 uuid 的话，调用方还得再查一次才知道
        // 这条伏笔挂在哪条线上，而无主的伏笔（storyline_id IS NULL）必须一眼能看出来。
        let rows: Vec<ForeshadowRow> =
            sqlx::query_as(&foreshadow_select("WHERE f.project_id=$1 ORDER BY f.created_at"))
                .bind(project_id)
                .fetch_all(&self.pool)
                .await
                .context("Failed to list foreshadows")?;
        Ok(rows.into_iter().map(|r| r.into_json()).collect())
    }

    async fn create_foreshadow(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        status: &str,
        importance: &str,
        hint_level: &str,
        hint_note: Option<&str>,
        introduced_at: Option<&str>,
        expected_reveal_at: Option<&str>,
        planted_node_id: Option<Uuid>,
        payoff_node_id: Option<Uuid>,
        parent_foreshadow_id: Option<Uuid>,
        storyline_id: Option<Uuid>,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        // 写入前归一：这两个字段此前被 HTTP 路径直接透传，库里混进了
        // '重要' / 'Main' / 'Major' / '低（前期只透风，不揭示）' 这类自由文本，
        // 前端无法排序过滤（见迁移 034）。归一放在仓储层=最后一道闸：
        // 无论从 HTTP、工具还是以后新增的入口进来，都只可能落枚举值。
        let importance = domain::foreshadowing::ForeshadowingImportance::parse(importance)
            .ok_or_else(|| anyhow::anyhow!(
                "importance 无法识别：{}（合法取值：{}，也可写中文）",
                importance,
                domain::foreshadowing::FORESHADOWING_IMPORTANCES.join(" / ")
            ))?
            .as_str();
        let hint_level = domain::foreshadowing::HintLevel::parse(hint_level)
            .ok_or_else(|| anyhow::anyhow!(
                "hint_level 无法识别：{}（合法取值：{}，也可写中文）",
                hint_level,
                domain::foreshadowing::HINT_LEVELS.join(" / ")
            ))?
            .as_str();
        sqlx::query(
            "INSERT INTO foreshadowing (id, project_id, name, description, status, importance, \
             hint_level, hint_note, introduced_at, expected_reveal_at, \
             planted_node_id, payoff_node_id, parent_foreshadow_id, storyline_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(status)
        .bind(importance)
        .bind(hint_level)
        .bind(hint_note)
        .bind(introduced_at)
        .bind(expected_reveal_at)
        .bind(planted_node_id)
        .bind(payoff_node_id)
        .bind(parent_foreshadow_id)
        .bind(storyline_id)
        .execute(&self.pool)
        .await
        .context("Failed to create foreshadow")?;
        // 回显完整对象：创建后立即可见埋点 / 备注，不必再 list 一遍
        fetch_foreshadow_row(&self.pool, id).await
    }

    async fn update_foreshadow(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        hint_note: Option<&str>,
        introduced_at: Option<&str>,
        expected_reveal_at: Option<&str>,
        actual_reveal_at: Option<&str>,
        planted_node_id: Option<Uuid>,
        payoff_node_id: Option<Uuid>,
        parent_foreshadow_id: Option<Uuid>,
        storyline_id: Option<Option<Uuid>>,
    ) -> Result<Value> {
        // 状态单独一条语句：只有明确传了才动（与归属同理）
        if let Some(status) = status {
            sqlx::query(
                "UPDATE foreshadowing SET status=$1, updated_at=NOW() WHERE id=$2",
            )
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update foreshadow status")?;
        }
        // 归属单独一条语句：只有调用方明确要求时才动，
        // 免得「改个名字」顺手把伏笔从暗线上摘下来。
        if let Some(target) = storyline_id {
            sqlx::query("UPDATE foreshadowing SET storyline_id=$1, updated_at=NOW() WHERE id=$2")
                .bind(target)
                .bind(id)
                .execute(&self.pool)
                .await
                .context("Failed to update foreshadow storyline")?;
        }
        // name / description 走 COALESCE：None（不传）保持原值，
        // 不会「只想改归属却把名字和描述一起清空」。
        // 全部走 COALESCE：None（不传）保持原值。备注与三个时间锚同理。
        sqlx::query(
            "UPDATE foreshadowing SET name=COALESCE($1, name), \
             description=COALESCE($2, description), \
             hint_note=COALESCE($3, hint_note), \
             introduced_at=COALESCE($4, introduced_at), \
             expected_reveal_at=COALESCE($5, expected_reveal_at), \
             actual_reveal_at=COALESCE($6, actual_reveal_at), \
             planted_node_id=COALESCE($7, planted_node_id), \
             payoff_node_id=COALESCE($8, payoff_node_id), \
             parent_foreshadow_id=COALESCE($9, parent_foreshadow_id), \
             updated_at=NOW() WHERE id=$10",
        )
        .bind(name)
        .bind(description)
        .bind(hint_note)
        .bind(introduced_at)
        .bind(expected_reveal_at)
        .bind(actual_reveal_at)
        .bind(planted_node_id)
        .bind(payoff_node_id)
        .bind(parent_foreshadow_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update foreshadow")?;
        // 回显修改后的完整对象（原先只回 {"updated":true}，调用方只能再 list 全文）
        fetch_foreshadow_row(&self.pool, id).await
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

/// 事件要挂到的叙事节点必须存在、属于本项目、且未被逻辑删除。
///
/// 不写悬空引用：挂错节点的事后表现是「事件列表里那一章是空的」，
/// 查起来比当场报错贵得多。
async fn ensure_event_node_in_project(
    pool: &sqlx::PgPool,
    project_id: Uuid,
    node_id: Uuid,
) -> Result<()> {
    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT project_id FROM narrative_node WHERE id = $1 AND status != 'Deleted'")
            .bind(node_id)
            .fetch_optional(pool)
            .await
            .context("校验叙事节点是否存在失败")?;
    match found {
        None => anyhow::bail!(
            "叙事节点 {} 不存在或已删除：事件要挂的节点必须真实存在（先用 list_nodes 查）",
            node_id
        ),
        Some(pid) if pid != project_id => {
            anyhow::bail!("叙事节点 {} 不属于本项目", node_id)
        }
        Some(_) => Ok(()),
    }
}

/// 按 id 读回一条事件：`update_event` 用它回显修改后的完整对象。
///
/// 原先只回 `{"updated":true}`，调用方（AI）改完看不到写进去的是什么。
async fn fetch_event_row(pool: &sqlx::PgPool, id: Uuid) -> Result<Value> {
    let row: (
        String, String, String, Option<String>, Option<String>,
        Option<String>, Option<String>, Value, String, Option<i32>,
        Option<String>, Option<String>,
    ) = sqlx::query_as(
        "SELECT id::text, name, description, event_type, timestamp, \
                event_time, duration, attributes, created_at::text, era_order, \
                narrative_node_id::text, \
                (SELECT title FROM narrative_node n WHERE n.id = event.narrative_node_id) \
         FROM event WHERE id=$1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .context("Failed to read back event")?;
    let (id, name, desc, etype, ts, event_time, duration, attrs, created, era_order, node_id, node_title) = row;
    Ok(serde_json::json!({
        "id": id, "name": name, "description": desc,
        "event_type": etype, "timestamp": ts,
        "event_time": event_time, "duration": duration, "attributes": attrs,
        "era_order": era_order,
        // 事件挂在哪一章 / 哪一场（连同节点标题一起给，省一次往返）
        "narrative_node_id": node_id,
        "narrative_node_title": node_title,
        "created_at": created
    }))
}

#[async_trait]
impl HistoryRepositoryPort for DbHistoryRepositoryPort {
    async fn list_events(&self, project_id: Uuid, limit: i64, order_by: &str) -> Result<Vec<Value>> {
        // 排序显式可选：中文 when 没法比大小，所以「历史轴顺序」要靠 era_order 这个数值锚；
        // `(era_order IS NULL)` 让未标定的事件排在最后（PG / SQLite 都支持这种写法）。
        let order_clause = match order_by {
            "recent" => "ORDER BY created_at DESC",
            "era" => "ORDER BY (era_order IS NULL), era_order ASC, created_at ASC",
            other => anyhow::bail!(
                "list_events 的 order_by 只支持 \"recent\" / \"era\"，收到：{}",
                other
            ),
        };
        let sql = format!(
            "SELECT id::text, name, description, event_type, timestamp, \
                    event_time, duration, attributes, created_at::text, era_order, \
                    narrative_node_id::text, \
                    (SELECT title FROM narrative_node n WHERE n.id = event.narrative_node_id) \
             FROM event WHERE project_id = $1 AND status != 'Deleted' {order_clause} LIMIT $2"
        );
        let rows = sqlx::query_as::<
            _,
            (
                String, String, String, Option<String>, Option<String>,
                Option<String>, Option<String>, Value, String, Option<i32>,
                Option<String>, Option<String>,
            ),
        >(&sql)
        .bind(project_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list events")?;

        Ok(rows
            .into_iter()
            .map(|(id, name, desc, etype, ts, event_time, duration, attrs, created, era_order, node_id, node_title)| {
                serde_json::json!({
                    "id": id, "name": name, "description": desc,
                    "event_type": etype, "timestamp": ts,
                    // when / where / participants / consequences / reveal_at 都在 attributes 里
                    "event_time": event_time, "duration": duration, "attributes": attrs,
                    "era_order": era_order,
                    "narrative_node_id": node_id,
                    "narrative_node_title": node_title,
                    "created_at": created
                })
            })
            .collect())
    }

    async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        event_time: Option<&str>,
        duration: Option<&str>,
        attributes: &Value,
        era_order: Option<i64>,
        narrative_node_id: Option<Uuid>,
    ) -> Result<Value> {
        let id = Uuid::new_v4();
        if let Some(node_id) = narrative_node_id {
            ensure_event_node_in_project(&self.pool, project_id, node_id).await?;
        }
        sqlx::query(
            "INSERT INTO event (id, project_id, name, description, event_type, event_time, duration, attributes, era_order, narrative_node_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(event_type)
        .bind(event_time)
        .bind(duration)
        .bind(attributes)
        .bind(era_order)
        .bind(narrative_node_id)
        .execute(&self.pool)
        .await
        .context("Failed to create event")?;
        // 回显完整对象（含刚写入的 era_order），与 update 的回显一致
        fetch_event_row(&self.pool, id).await
    }

    async fn update_event(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        event_type: Option<&str>,
        event_time: Option<&str>,
        duration: Option<&str>,
        attributes: Option<&Value>,
        era_order: Option<i64>,
        narrative_node_id: Option<Uuid>,
        clear_narrative_node: bool,
    ) -> Result<Value> {
        // 事件所属项目：既用于节点归属校验，也避免跨项目改到别人的事件
        let project_id: Option<Uuid> = sqlx::query_scalar("SELECT project_id FROM event WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to read event project")?;
        let project_id = project_id.ok_or_else(|| anyhow::anyhow!("事件不存在: {}", id))?;
        if let Some(node_id) = narrative_node_id {
            ensure_event_node_in_project(&self.pool, project_id, node_id).await?;
        }
        // 未传的字段保持原值：拼接时只带上明确给出的列
        let mut sql = String::from("UPDATE event SET updated_at = NOW()");
        let mut next_index = 1;
        let mut bind_name = false;
        let mut bind_description = false;
        let mut bind_event_type = false;
        let mut bind_event_time = false;
        let mut bind_duration = false;
        let mut bind_attributes = false;
        let mut bind_era_order = false;
        let mut bind_narrative_node = false;
        if name.is_some() {
            sql.push_str(&format!(", name=${}", next_index));
            next_index += 1;
            bind_name = true;
        }
        if description.is_some() {
            sql.push_str(&format!(", description=${}", next_index));
            next_index += 1;
            bind_description = true;
        }
        if event_type.is_some() {
            sql.push_str(&format!(", event_type=${}", next_index));
            next_index += 1;
            bind_event_type = true;
        }
        if event_time.is_some() {
            sql.push_str(&format!(", event_time=${}", next_index));
            next_index += 1;
            bind_event_time = true;
        }
        if duration.is_some() {
            sql.push_str(&format!(", duration=${}", next_index));
            next_index += 1;
            bind_duration = true;
        }
        if attributes.is_some() {
            sql.push_str(&format!(", attributes=${}", next_index));
            next_index += 1;
            bind_attributes = true;
        }
        if era_order.is_some() {
            sql.push_str(&format!(", era_order=${}", next_index));
            next_index += 1;
            bind_era_order = true;
        }
        // 挂 / 换 / 解绑叙事节点：解绑与「设成 NULL」都写成显式的 NULL，不靠缺省值猜测
        if narrative_node_id.is_some() || clear_narrative_node {
            sql.push_str(&format!(", narrative_node_id=${}", next_index));
            next_index += 1;
            bind_narrative_node = true;
        }
        sql.push_str(&format!(" WHERE id=${}", next_index));

        let mut q = sqlx::query(&sql);
        if bind_name { q = q.bind(name); }
        if bind_description { q = q.bind(description); }
        if bind_event_type { q = q.bind(event_type); }
        if bind_event_time { q = q.bind(event_time); }
        if bind_duration { q = q.bind(duration); }
        if bind_attributes { q = q.bind(attributes); }
        if bind_era_order { q = q.bind(era_order); }
        if bind_narrative_node { q = q.bind(narrative_node_id); }
        q = q.bind(id);
        q.execute(&self.pool)
            .await
            .context("Failed to update event")?;
        // 回显修改后的完整对象（原先只回 {"updated":true}）
        fetch_event_row(&self.pool, id).await
    }

    async fn delete_event(&self, id: Uuid) -> Result<()> {
        // 语义化结束而非物理删除：与 entity / narrative_node 一致
        let result = sqlx::query(
            "UPDATE event SET status = 'Deleted', updated_at = NOW() WHERE id = $1 AND status != 'Deleted'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to retire event")?;
        if result.rows_affected() == 0 {
            anyhow::bail!("事件不存在或已结束: {}", id);
        }
        Ok(())
    }

    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Value>> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String, String)>(
            "SELECT id::text, content, category, certainty, created_at::text, status \
             FROM fact WHERE project_id = $1 AND status != 'Retired' ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list facts")?;

        Ok(rows
            .into_iter()
            .map(|(id, content, cat, cert, created, status)| {
                serde_json::json!({
                    "id": id,
                    "project_id": project_id.to_string(),
                    "content": content,
                    "status": status,
                    "category": cat,
                    "certainty": cert,
                    "created_at": created,
                    "updated_at": created
                })
            })
            .collect())
    }

    async fn delete_fact(&self, id: Uuid) -> Result<()> {
        // 语义化结束：fact 表本就有 status 列（默认 Active），此前没有任何写入路径
        let result = sqlx::query(
            "UPDATE fact SET status = 'Retired', updated_at = NOW() WHERE id = $1 AND status != 'Retired'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to retire fact")?;
        if result.rows_affected() == 0 {
            anyhow::bail!("事实不存在或已结束: {}", id);
        }
        Ok(())
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
            Value, Option<String>,
        )> = sqlx::query_as(
            "SELECT lp.geography, lp.appearance, lp.population, lp.economy, lp.rules, lp.history, lp.narrative_usage, \
             li.location_type, li.size, li.climate, li.era, li.accessibility, \
             lp.aliases, lp.secrets \
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
            "aliases": r.12, "secrets": r.13,
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
        // 别名：整块替换语义（与角色档案一致：传了才动，没传保持原值）
        let aliases = profile.get("aliases").filter(|v| !v.is_null()).cloned();
        let secrets = s("secrets");

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
            Option<Value>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, geography, appearance, population, economy, rules, history, \
             narrative_usage, aliases, secrets FROM location_profile WHERE entity_id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await.context("chk loc profile")?;
        match lp {
            Some((pid, g0, a0, p0, e0, r0, h0, n0, al0, sec0)) => {
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                // aliases 是 JSONB 列：未传时保留旧值（与其它字段一致），不能落 NULL
                let aliases = match aliases.clone() {
                    Some(v) => v,
                    None => al0.unwrap_or_else(|| Value::Array(vec![])),
                };
                sqlx::query("UPDATE location_profile SET geography=$1, appearance=$2, population=$3, economy=$4, rules=$5, history=$6, narrative_usage=$7, aliases=$8, secrets=$9, updated_at=NOW() WHERE id=$10")
                    .bind(kept(geography, g0)).bind(kept(appearance, a0)).bind(kept(population, p0))
                    .bind(kept(economy, e0)).bind(kept(rules, r0)).bind(kept(history, h0))
                    .bind(kept(narrative_usage, n0)).bind(&aliases).bind(kept(secrets, sec0))
                    .bind(pid)
                    .execute(&self.pool).await.context("upd loc profile")?;
            }
            None => {
                sqlx::query("INSERT INTO location_profile (id, entity_id, geography, appearance, population, economy, rules, history, narrative_usage, aliases, secrets) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
                    .bind(id).bind(&geography).bind(&appearance).bind(&population).bind(&economy).bind(&rules).bind(&history).bind(&narrative_usage)
                    .bind(aliases.clone().unwrap_or_else(|| Value::Array(vec![]))).bind(&secrets)
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
            Option<String>, Option<String>, Option<String>, Value,
        )> = sqlx::query_as(
            "SELECT goals, leader, \"values\", resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi, aliases FROM faction_profile WHERE entity_id = $1",
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
            "aliases": r.11,
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
        // 别名：整块替换语义（传了才动，没传保持原值）
        let aliases = profile.get("aliases").filter(|v| !v.is_null()).cloned();
        // 未传的字段保持原值（agent 工具契约，见 update_faction_profile 的描述）
        let existing: Option<(
            Uuid, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<String>, Option<String>,
            Option<String>, Option<Value>,
        )> = sqlx::query_as(
            "SELECT id, goals, leader, \"values\", resources, territory, members, enemies, \
             allies, internal_conflicts, secrets, modus_operandi, aliases \
             FROM faction_profile WHERE entity_id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await.context("chk faction profile")?;
        match existing {
            Some((pid, g0, l0, v0, r0, t0, m0, e0, a0, ic0, s0, mo0, al0)) => {
                let kept = |new: Option<String>, old: Option<String>| new.or(old);
                let aliases = match aliases.clone() {
                    Some(v) => v,
                    None => al0.unwrap_or_else(|| Value::Array(vec![])),
                };
                sqlx::query("UPDATE faction_profile SET goals=$1, leader=$2, \"values\"=$3, resources=$4, territory=$5, members=$6, enemies=$7, allies=$8, internal_conflicts=$9, secrets=$10, modus_operandi=$11, aliases=$12, updated_at=NOW() WHERE id=$13")
                    .bind(kept(goals, g0)).bind(kept(leader, l0)).bind(kept(values, v0))
                    .bind(kept(resources, r0)).bind(kept(territory, t0)).bind(kept(members, m0))
                    .bind(kept(enemies, e0)).bind(kept(allies, a0)).bind(kept(internal_conflicts, ic0))
                    .bind(kept(secrets, s0)).bind(kept(modus_operandi, mo0)).bind(&aliases).bind(pid)
                    .execute(&self.pool).await.context("upd faction profile")?;
            }
            None => {
                sqlx::query("INSERT INTO faction_profile (id, entity_id, goals, leader, \"values\", resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi, aliases) VALUES (gen_random_uuid(), $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
                    .bind(id).bind(&goals).bind(&leader).bind(&values).bind(&resources).bind(&territory).bind(&members).bind(&enemies).bind(&allies).bind(&internal_conflicts).bind(&secrets).bind(&modus_operandi).bind(aliases.clone().unwrap_or_else(|| Value::Array(vec![])))
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

