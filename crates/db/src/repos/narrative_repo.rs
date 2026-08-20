//! Narrative Repository - CRUD operations for NarrativeNode, Scene

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{
    NarrativeNode, NarrativeNodeStatus, NarrativeNodeType, Scene,
};
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

    pub async fn create_node(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        node_type: NarrativeNodeType,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let type_str = ser::narrative_node_type_str(&node_type);
        let status_str = ser::narrative_node_status_str(&NarrativeNodeStatus::Draft);

        sqlx::query(
            "INSERT INTO narrative_node (id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(project_id)
        .bind(world_id)
        .bind(&type_str)
        .bind(parent_id)
        .bind(title)
        .bind(description.unwrap_or(""))
        .bind(&attributes)
        .bind(sort_order)
        .bind(&status_str)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create narrative node")?;

        Ok(NarrativeNode {
            id,
            project_id,
            world_id,
            node_type,
            parent_id,
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
            content: None,
            attributes,
            sort_order,
            status: NarrativeNodeStatus::Draft,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_node_by_id(&self, id: Uuid) -> Result<Option<NarrativeNode>> {
        let row = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, created_at, updated_at \
             FROM narrative_node WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query narrative node")?;

        Ok(row.map(|r| r.into()))
    }

    /// Project-scoped narrative node query. Ensures node belongs to the specified project.
    ///
    /// Use this method instead of get_node_by_id when you need project isolation.
    /// Returns None if node doesn't exist OR doesn't belong to the project.
    pub async fn get_node_by_id_with_project(&self, project_id: Uuid, id: Uuid) -> Result<Option<NarrativeNode>> {
        let row = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, created_at, updated_at              FROM narrative_node WHERE id = $1 AND project_id = $2",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query narrative node")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_nodes_by_project(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let rows = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, created_at, updated_at \
             FROM narrative_node WHERE project_id = $1 ORDER BY sort_order",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query narrative nodes")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let rows = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, created_at, updated_at \
             FROM narrative_node WHERE parent_id = $1 ORDER BY sort_order",
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query children")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update_node(&self, node: &NarrativeNode) -> Result<()> {
        let status_str = ser::narrative_node_status_str(&node.status);
        sqlx::query(
            "UPDATE narrative_node SET title = $1, description = $2, content = $3, attributes = $4, sort_order = $5, status = $6, updated_at = $7 WHERE id = $8",
        )
        .bind(&node.title)
        .bind(&node.description)
        .bind(&node.content)
        .bind(&node.attributes)
        .bind(node.sort_order)
        .bind(&status_str)
        .bind(Utc::now())
        .bind(node.id)
        .execute(&self.pool)
        .await
        .context("Failed to update narrative node")?;
        Ok(())
    }

    pub async fn delete_node(&self, id: Uuid) -> Result<()> {
        // Delete children recursively
        let children = self.list_children(id).await?;
        for child in children {
            Box::pin(self.delete_node(child.id)).await?;
        }
        sqlx::query("DELETE FROM narrative_node WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete narrative node")?;
        Ok(())
    }

    pub async fn get_node_by_id_with_project_tx(
        executor: &mut sqlx::PgConnection,
        project_id: Uuid,
        id: Uuid,
    ) -> Result<Option<NarrativeNode>> {
        let row = sqlx::query_as::<_, NarrativeNodeRow>(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, content, attributes, sort_order, status, created_at, updated_at \
             FROM narrative_node WHERE id = $1 AND project_id = $2",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(executor)
        .await
        .context("Failed to query narrative node")?;
        Ok(row.map(|r| r.into()))
    }

    /// CAS 更新叙事节点（提案 四 / 六）：乐观锁 version + 软更新，不物理 DELETE。
    pub async fn update_node_tx(
        executor: &mut sqlx::PgConnection,
        node: &NarrativeNode,
        expected_version: i32,
    ) -> Result<bool> {
        let status_str = ser::narrative_node_status_str(&node.status);
        let result = sqlx::query(
            "UPDATE narrative_node SET title=$1, description=$2, content=$3, attributes=$4, sort_order=$5, status=$6, version=version+1, updated_at=NOW() \
             WHERE id=$7 AND project_id=$8 AND version=$9",
        )
        .bind(&node.title)
        .bind(&node.description)
        .bind(&node.content)
        .bind(&node.attributes)
        .bind(node.sort_order)
        .bind(&status_str)
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
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<NarrativeNodeRow> for NarrativeNode {
    fn from(r: NarrativeNodeRow) -> Self {
        NarrativeNode {
            id: r.id,
            project_id: r.project_id,
            world_id: r.world_id,
            node_type: ser::parse_narrative_node_type(&r.node_type),
            parent_id: r.parent_id,
            title: r.title,
            description: r.description,
            content: r.content,
            attributes: r.attributes.unwrap_or_default(),
            sort_order: r.sort_order,
            status: ser::parse_narrative_node_status(&r.status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
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
