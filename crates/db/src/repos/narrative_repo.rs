//! Narrative Repository - CRUD operations for NarrativeNode, Scene

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{NarrativeNode, Scene};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ser;

pub struct NarrativeRepo {
    pool: PgPool,
}

impl NarrativeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // 注：这里原先还有一个 8 参数的 create_node —— 它的 INSERT 列数（13）与
    // VALUES 占位符（12）不匹配，本身就是坏的，而且没有任何调用点。
    // 建节点的唯一入口是端口层的 `NarrativeRepositoryPort::create_node_full`
    // （带父节点归属校验、显式序号让位、挂在同一事务里），不再保留第二份实现。
    pub async fn get_node_by_id(&self, id: Uuid) -> Result<Option<NarrativeNode>> {
        let row = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, storyline_id, arc_stage, stage_refs, participant_entity_ids, location_id, item_ids, estimated_chapters, estimated_words, story_time, created_at, updated_at \
             FROM narrative_node WHERE id = $1 AND status != 'Deleted'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query narrative node")?;

        row.map(NarrativeNode::try_from).transpose()
    }

    /// Project-scoped narrative node query. Ensures node belongs to the specified project.
    ///
    /// Use this method instead of get_node_by_id when you need project isolation.
    /// Returns None if node doesn't exist OR doesn't belong to the project.
    pub async fn get_node_by_id_with_project(&self, project_id: Uuid, id: Uuid) -> Result<Option<NarrativeNode>> {
        let row = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, storyline_id, arc_stage, stage_refs, participant_entity_ids, location_id, item_ids, estimated_chapters, estimated_words, story_time, created_at, updated_at              FROM narrative_node WHERE id = $1 AND project_id = $2 AND status != 'Deleted'",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query narrative node")?;

        row.map(NarrativeNode::try_from).transpose()
    }

    pub async fn list_nodes_by_project(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let rows = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, storyline_id, arc_stage, stage_refs, participant_entity_ids, location_id, item_ids, estimated_chapters, estimated_words, story_time, created_at, updated_at \
             FROM narrative_node WHERE project_id = $1 AND status != 'Deleted' ORDER BY sort_order",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query narrative nodes")?;

        rows.into_iter().map(NarrativeNode::try_from).collect::<Result<Vec<_>>>()
    }

    pub async fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let rows = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, storyline_id, arc_stage, stage_refs, participant_entity_ids, location_id, item_ids, estimated_chapters, estimated_words, story_time, created_at, updated_at \
             FROM narrative_node WHERE parent_id = $1 AND status != 'Deleted' ORDER BY sort_order",
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query children")?;

        rows.into_iter().map(NarrativeNode::try_from).collect::<Result<Vec<_>>>()
    }

    pub async fn update_node(&self, node: &NarrativeNode) -> Result<()> {
        let status_str = ser::narrative_node_status_str(&node.status);
        sqlx::query(
            "UPDATE narrative_node SET title = $1, description = $2, content = $3, attributes = $4, sort_order = $5, status = $6, \
             node_type = $7, parent_id = $8, storyline_id = $9, arc_stage = $10, stage_refs = $11, participant_entity_ids = $12, \
             location_id = $13, item_ids = $14, estimated_chapters = $15, estimated_words = $16, story_time = $17, updated_at = $18 \
             WHERE id = $19",
        )
        .bind(&node.title)
        .bind(&node.description)
        .bind(&node.content)
        .bind(&node.attributes)
        .bind(node.sort_order)
        .bind(&status_str)
        .bind(ser::narrative_node_type_str(&node.node_type))
        .bind(node.parent_id)
        .bind(node.storyline_id)
        .bind(&node.arc_stage)
        .bind(serde_json::to_value(&node.stage_refs).context("stage_refs 序列化失败")?)
        .bind(serde_json::to_value(&node.participant_entity_ids).context("participant_entity_ids 序列化失败")?)
        .bind(node.location_id)
        .bind(serde_json::to_value(&node.item_ids).context("item_ids 序列化失败")?)
        .bind(node.estimated_chapters)
        .bind(node.estimated_words)
        .bind(&node.story_time)
        .bind(Utc::now())
        .bind(node.id)
        .execute(&self.pool)
        .await
        .context("Failed to update narrative node")?;
        Ok(())
    }

    // 注：原先这里有一个**物理递归 DELETE** 的 delete_node，与项目明文约定
    // 「创作数据绝不物理 DELETE，统一软删除 status='Deleted'」相反，且无调用者。
    // 软删除入口是 `soft_delete_node_tx`（WITH RECURSIVE 置 status='Deleted'）。
    pub async fn get_node_by_id_with_project_tx(
        executor: &mut sqlx::PgConnection,
        project_id: Uuid,
        id: Uuid,
    ) -> Result<Option<NarrativeNode>> {
        let row = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, storyline_id, arc_stage, stage_refs, participant_entity_ids, location_id, item_ids, estimated_chapters, estimated_words, story_time, created_at, updated_at \
             FROM narrative_node WHERE id = $1 AND project_id = $2 AND status != 'Deleted'",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(executor)
        .await
        .context("Failed to query narrative node")?;
        row.map(NarrativeNode::try_from).transpose()
    }

    /// CAS 更新叙事节点（提案 四 / 六）：乐观锁 version + 软更新，不物理 DELETE。
    ///
    /// 结构与挂载列（node_type / parent_id / storyline_id / arc_stage / stage_refs /
    /// participant_entity_ids / location_id / item_ids / 元数据）一并写回：
    /// 细纲的「移动、重排、挂载」都走这条 CAS 路径，漏一列就等于改不动。
    pub async fn update_node_tx(
        executor: &mut sqlx::PgConnection,
        node: &NarrativeNode,
        expected_version: i32,
    ) -> Result<bool> {
        let status_str = ser::narrative_node_status_str(&node.status);
        let node_type_str = ser::narrative_node_type_str(&node.node_type);
        let result = sqlx::query(
            "UPDATE narrative_node SET title=$1, description=$2, content=$3, attributes=$4, sort_order=$5, status=$6, \
             node_type=$7, parent_id=$8, storyline_id=$9, arc_stage=$10, stage_refs=$11, participant_entity_ids=$12, \
             location_id=$13, item_ids=$14, estimated_chapters=$15, estimated_words=$16, story_time=$17, \
             version=version+1, updated_at=NOW() \
             WHERE id=$18 AND project_id=$19 AND version=$20",
        )
        .bind(&node.title)
        .bind(&node.description)
        .bind(&node.content)
        .bind(&node.attributes)
        .bind(node.sort_order)
        .bind(&status_str)
        .bind(&node_type_str)
        .bind(node.parent_id)
        .bind(node.storyline_id)
        .bind(&node.arc_stage)
        .bind(serde_json::to_value(&node.stage_refs).context("stage_refs 序列化失败")?)
        .bind(serde_json::to_value(&node.participant_entity_ids).context("participant_entity_ids 序列化失败")?)
        .bind(node.location_id)
        .bind(serde_json::to_value(&node.item_ids).context("item_ids 序列化失败")?)
        .bind(node.estimated_chapters)
        .bind(node.estimated_words)
        .bind(&node.story_time)
        .bind(node.id)
        .bind(node.project_id)
        .bind(expected_version)
        .execute(executor)
        .await
        .context("Failed to update narrative node (CAS)")?;
        Ok(result.rows_affected() > 0)
    }

    /// 软删除叙事节点（含递归子节点），绝不物理 DELETE（提案 二十二）。
    pub async fn soft_delete_node_tx(
        executor: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<u64> {
        let result = sqlx::query(
            "WITH RECURSIVE sub(id) AS ( \
                SELECT id FROM narrative_node WHERE id = $1 \
                UNION ALL \
                SELECT n.id FROM narrative_node n JOIN sub s ON n.parent_id = s.id \
             ) UPDATE narrative_node SET status = 'Deleted', updated_at = NOW() WHERE id IN (SELECT id FROM sub)",
        )
        .bind(id)
        .execute(executor)
        .await
        .context("Failed to soft-delete narrative node")?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct NarrativeNodeRow {
    id: Uuid,
    project_id: Uuid,
    world_id: Uuid,
    node_type: String,
    parent_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    content: Option<String>,
    attributes: Option<serde_json::Value>,
    sort_order: i32,
    status: String,
    storyline_id: Option<Uuid>,
    arc_stage: Option<String>,
    stage_refs: Option<serde_json::Value>,
    participant_entity_ids: Option<serde_json::Value>,
    location_id: Option<Uuid>,
    item_ids: Option<serde_json::Value>,
    estimated_chapters: Option<i32>,
    estimated_words: Option<i32>,
    story_time: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// JSONB 列 -> Vec<Uuid> / Vec<NarrativeStageRef>。
///
/// 解析失败**直接报错**：这些列是结构化挂载（在场角色 / 道具 / 线阶段引用），
/// 静默当成空数组等于「挂载信息悄悄消失」，比报错更难查。
fn parse_uuid_array(value: Option<serde_json::Value>, column: &str, id: Uuid) -> Result<Vec<Uuid>> {
    let raw = value.unwrap_or_else(|| serde_json::json!([]));
    serde_json::from_value::<Vec<Uuid>>(raw)
        .with_context(|| format!("叙事节点 {} 的 {} 不是合法 UUID 数组", id, column))
}

fn parse_stage_refs(value: Option<serde_json::Value>, id: Uuid) -> Result<Vec<domain::NarrativeStageRef>> {
    let raw = value.unwrap_or_else(|| serde_json::json!([]));
    serde_json::from_value::<Vec<domain::NarrativeStageRef>>(raw)
        .with_context(|| format!("叙事节点 {} 的 stage_refs 结构非法", id))
}

impl TryFrom<NarrativeNodeRow> for NarrativeNode {
    type Error = anyhow::Error;

    fn try_from(r: NarrativeNodeRow) -> Result<Self> {
        let node_type = ser::parse_narrative_node_type(&r.node_type)
            .with_context(|| format!("叙事节点 {} 的 node_type 非法", r.id))?;
        let status = ser::parse_narrative_node_status(&r.status)
            .with_context(|| format!("叙事节点 {} 的 status 非法", r.id))?;
        let participant_entity_ids =
            parse_uuid_array(r.participant_entity_ids, "participant_entity_ids", r.id)?;
        let item_ids = parse_uuid_array(r.item_ids, "item_ids", r.id)?;
        let stage_refs = parse_stage_refs(r.stage_refs, r.id)?;
        Ok(NarrativeNode {
            id: r.id,
            project_id: r.project_id,
            world_id: r.world_id,
            node_type,
            parent_id: r.parent_id,
            title: r.title,
            description: r.description,
            content: r.content,
            attributes: r.attributes.unwrap_or_default(),
            sort_order: r.sort_order,
            status,
            storyline_id: r.storyline_id,
            arc_stage: r.arc_stage,
            stage_refs,
            participant_entity_ids,
            location_id: r.location_id,
            item_ids,
            estimated_chapters: r.estimated_chapters,
            estimated_words: r.estimated_words,
            story_time: r.story_time,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

// ============= SceneRepo =============

pub struct SceneRepo {
    pool: PgPool,
}

impl SceneRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        narrative_node_id: Uuid,
        objective: Option<&str>,
        conflict: Option<&str>,
        pov_character_id: Option<Uuid>,
        location_id: Option<Uuid>,
        time: Option<&str>,
    ) -> Result<Scene> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO scene (id, narrative_node_id, objective, conflict, pov_character_id, location_id, time, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(narrative_node_id)
        .bind(objective.unwrap_or(""))
        .bind(conflict.unwrap_or(""))
        .bind(pov_character_id)
        .bind(location_id)
        .bind(time.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create scene")?;

        Ok(Scene {
            id,
            narrative_node_id,
            objective: objective.map(|s| s.to_string()),
            conflict: conflict.map(|s| s.to_string()),
            pov_character_id,
            location_id,
            time: time.map(|s| s.to_string()),
            scene_start_time: None,
            scene_end_time: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_narrative_node(&self, node_id: Uuid) -> Result<Option<Scene>> {
        let row = sqlx::query_as::<_, SceneRow>(
            "SELECT id, narrative_node_id, objective, conflict, pov_character_id, location_id, time, scene_start_time, scene_end_time, created_at, updated_at \
             FROM scene WHERE narrative_node_id = $1",
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query scene")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct SceneRow {
    id: Uuid,
    narrative_node_id: Uuid,
    objective: Option<String>,
    conflict: Option<String>,
    pov_character_id: Option<Uuid>,
    location_id: Option<Uuid>,
    time: Option<String>,
    scene_start_time: Option<String>,
    scene_end_time: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SceneRow> for Scene {
    fn from(r: SceneRow) -> Self {
        Scene {
            id: r.id,
            narrative_node_id: r.narrative_node_id,
            objective: r.objective,
            conflict: r.conflict,
            pov_character_id: r.pov_character_id,
            location_id: r.location_id,
            time: r.time,
            scene_start_time: r.scene_start_time,
            scene_end_time: r.scene_end_time,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
