//! Narrative Service - 叙事结构管理的业务逻辑层
//!
//! 负责 Volume/Arc/Sequence/Chapter/Scene/Beat 的创建、查询、遍历。

use anyhow::{Context, Result};
use chrono::Utc;
use db::repos::narrative_repo;
use domain::*;
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

    // ============================================================
    // Volume 相关
    // ============================================================

    /// 创建卷
    pub async fn create_volume(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        title: &str,
        description: Option<&str>,
        attributes: VolumeAttributes,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let attrs = serde_json::to_value(&attributes).unwrap_or_default();
        let node = repo
            .create_node(project_id, world_id, NarrativeNodeType::Volume, None, title, description, attrs, sort_order)
            .await?;
        tracing::info!("Created volume: {}", title);
        Ok(node)
    }

    /// 列出项目中的所有卷
    pub async fn list_volumes(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let all = repo.list_nodes_by_project(project_id).await?;
        Ok(all.into_iter().filter(|n| n.node_type == NarrativeNodeType::Volume).collect())
    }

    // ============================================================
    // Arc 相关
    // ============================================================

    /// 创建故事弧线
    pub async fn create_arc(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        volume_id: Uuid,
        title: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let node = repo
            .create_node(project_id, world_id, NarrativeNodeType::Arc, Some(volume_id), title, description, serde_json::json!({}), sort_order)
            .await?;
        tracing::info!("Created arc: {} under volume {}", title, volume_id);
        Ok(node)
    }

    /// 列出卷下的所有弧线
    pub async fn list_arcs(&self, volume_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        repo.list_children(volume_id).await
    }

    // ============================================================
    // Sequence 相关
    // ============================================================

    /// 创建序列
    pub async fn create_sequence(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        arc_id: Uuid,
        title: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let node = repo
            .create_node(project_id, world_id, NarrativeNodeType::Sequence, Some(arc_id), title, description, serde_json::json!({}), sort_order)
            .await?;
        Ok(node)
    }

    // ============================================================
    // Chapter 相关
    // ============================================================

    /// 创建章节
    pub async fn create_chapter(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        parent_id: Uuid,
        title: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let node = repo
            .create_node(project_id, world_id, NarrativeNodeType::Chapter, Some(parent_id), title, description, serde_json::json!({}), sort_order)
            .await?;
        Ok(node)
    }

    // ============================================================
    // Scene 相关
    // ============================================================

    /// 创建场景
    pub async fn create_scene(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        chapter_id: Uuid,
        title: &str,
        description: Option<&str>,
        scene_attributes: SceneAttributes,
        sort_order: i32,
    ) -> Result<(NarrativeNode, Scene)> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let attrs = serde_json::to_value(&scene_attributes).unwrap_or_default();
        let node = narrative_repo
            .create_node(project_id, world_id, NarrativeNodeType::Scene, Some(chapter_id), title, description, attrs, sort_order)
            .await?;

        let scene_repo = narrative_repo::SceneRepo::new(self.pool.clone());
        let scene = scene_repo
            .create(
                node.id,
                scene_attributes.objective.as_deref(),
                scene_attributes.conflict.as_deref(),
                scene_attributes.pov_character_id,
                scene_attributes.location_id,
                None,
            )
            .await?;

        tracing::info!("Created scene: {} in chapter {}", title, chapter_id);
        Ok((node, scene))
    }

    /// 获取场景详情
    pub async fn get_scene(&self, scene_node_id: Uuid) -> Result<Option<(NarrativeNode, Option<Scene>)>> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let node = narrative_repo.get_node_by_id(scene_node_id).await?;

        if let Some(node) = node {
            let scene_repo = narrative_repo::SceneRepo::new(self.pool.clone());
            let scene = scene_repo.get_by_narrative_node(scene_node_id).await?;
            Ok(Some((node, scene)))
        } else {
            Ok(None)
        }
    }

    // ============================================================
    // Beat 相关
    // ============================================================

    /// 创建节拍
    pub async fn create_beat(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        scene_id: Uuid,
        title: &str,
        beat_attributes: BeatAttributes,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let attrs = serde_json::to_value(&beat_attributes).unwrap_or_default();
        let node = repo
            .create_node(project_id, world_id, NarrativeNodeType::Beat, Some(scene_id), title, None, attrs, sort_order)
            .await?;
        Ok(node)
    }

    // ============================================================
    // 通用操作
    // ============================================================

    /// 获取节点及其完整子树
    pub async fn get_tree(&self, node_id: Uuid) -> Result<Option<NarrativeNodeTree>> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let node = repo.get_node_by_id(node_id).await?;

        if let Some(node) = node {
            let children = repo.list_children(node_id).await?;
            let mut child_trees = Vec::new();
            for child in children {
                if let Some(tree) = Box::pin(self.get_tree(child.id)).await? {
                    child_trees.push(tree);
                }
            }
            Ok(Some(NarrativeNodeTree {
                node,
                children: child_trees,
            }))
        } else {
            Ok(None)
        }
    }

    /// 获取当前项目中所有叙事节点
    pub async fn list_all_nodes(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        repo.list_nodes_by_project(project_id).await
    }

    /// 更新节点状态
    pub async fn update_status(&self, node_id: Uuid, status: NarrativeNodeStatus) -> Result<()> {
        let repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        if let Some(mut node) = repo.get_node_by_id(node_id).await? {
            node.status = status;
            repo.update_node(&node).await?;
        }
        Ok(())
    }

    /// 创建角色弧线
    pub async fn create_character_arc(
        &self,
        project_id: Uuid,
        character_id: Uuid,
        volume_id: Option<Uuid>,
        arc_type: &str,
        start_state: Option<&str>,
        mid_state: Option<&str>,
        end_state: Option<&str>,
        key_moments: Vec<String>,
    ) -> Result<CharacterArc> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_arc (id, project_id, character_id, volume_id, arc_type, start_state, mid_state, end_state, key_moments, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(project_id)
        .bind(character_id)
        .bind(volume_id)
        .bind(arc_type)
        .bind(start_state.unwrap_or(""))
        .bind(mid_state.unwrap_or(""))
        .bind(end_state.unwrap_or(""))
        .bind(serde_json::to_value(&key_moments).unwrap_or_default())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character arc")?;

        Ok(CharacterArc {
            id,
            project_id,
            character_id,
            volume_id,
            arc_type: arc_type.to_string(),
            start_state: start_state.map(|s| s.to_string()),
            mid_state: mid_state.map(|s| s.to_string()),
            end_state: end_state.map(|s| s.to_string()),
            key_moments,
            created_at: now,
            updated_at: now,
        })
    }
}

/// 叙事节点树
#[derive(Debug, Clone)]
pub struct NarrativeNodeTree {
    pub node: NarrativeNode,
    pub children: Vec<NarrativeNodeTree>,
}
