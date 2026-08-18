//! Narrative Service - 叙事结构管理的业务逻辑层
//!
//! 负责 Volume/Arc/Sequence/Chapter/Scene/Beat 的创建、查询、遍历。

use anyhow::{Context, Result};
use chrono::Utc;
use db::connection::Database;
use db::repos::narrative_repo;
use domain::*;
use uuid::Uuid;

/// Narrative Service - 叙事管理服务
pub struct NarrativeService<'a> {
    db: &'a Database,
}

impl<'a> NarrativeService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // ============================================================
    // Volume 相关
    // ============================================================

    /// 创建卷
    pub fn create_volume(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        title: &str,
        description: Option<&str>,
        attributes: VolumeAttributes,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let attrs = serde_json::to_value(&attributes).unwrap_or_default();
        let node = repo.create_node(project_id, world_id, NarrativeNodeType::Volume, None, title, description, attrs, sort_order)?;
        tracing::info!("Created volume: {}", title);
        Ok(node)
    }

    /// 列出项目中的所有卷
    pub fn list_volumes(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let all = repo.list_nodes_by_project(project_id)?;
        Ok(all.into_iter().filter(|n| n.node_type == NarrativeNodeType::Volume).collect())
    }

    // ============================================================
    // Arc 相关
    // ============================================================

    /// 创建故事弧线
    pub fn create_arc(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        volume_id: Uuid,
        title: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let node = repo.create_node(project_id, world_id, NarrativeNodeType::Arc, Some(volume_id), title, description, serde_json::json!({}), sort_order)?;
        tracing::info!("Created arc: {} under volume {}", title, volume_id);
        Ok(node)
    }

    /// 列出卷下的所有弧线
    pub fn list_arcs(&self, volume_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        repo.list_children(volume_id)
    }

    // ============================================================
    // Sequence 相关
    // ============================================================

    /// 创建序列
    pub fn create_sequence(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        arc_id: Uuid,
        title: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let node = repo.create_node(project_id, world_id, NarrativeNodeType::Sequence, Some(arc_id), title, description, serde_json::json!({}), sort_order)?;
        Ok(node)
    }

    // ============================================================
    // Chapter 相关
    // ============================================================

    /// 创建章节
    pub fn create_chapter(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        parent_id: Uuid,
        title: &str,
        description: Option<&str>,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let node = repo.create_node(project_id, world_id, NarrativeNodeType::Chapter, Some(parent_id), title, description, serde_json::json!({}), sort_order)?;
        Ok(node)
    }

    // ============================================================
    // Scene 相关
    // ============================================================

    /// 创建场景
    pub fn create_scene(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        chapter_id: Uuid,
        title: &str,
        description: Option<&str>,
        scene_attributes: SceneAttributes,
        sort_order: i32,
    ) -> Result<(NarrativeNode, Scene)> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.db);
        let attrs = serde_json::to_value(&scene_attributes).unwrap_or_default();
        let node = narrative_repo.create_node(project_id, world_id, NarrativeNodeType::Scene, Some(chapter_id), title, description, attrs, sort_order)?;

        let scene_repo = narrative_repo::SceneRepo::new(self.db);
        let scene = scene_repo.create(
            node.id,
            scene_attributes.objective.as_deref(),
            scene_attributes.conflict.as_deref(),
            scene_attributes.pov_character_id,
            scene_attributes.location_id,
            None,
        )?;

        tracing::info!("Created scene: {} in chapter {}", title, chapter_id);
        Ok((node, scene))
    }

    /// 获取场景详情
    pub fn get_scene(&self, scene_node_id: Uuid) -> Result<Option<(NarrativeNode, Option<Scene>)>> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.db);
        let node = narrative_repo.get_node_by_id(scene_node_id)?;

        if let Some(node) = node {
            let scene_repo = narrative_repo::SceneRepo::new(self.db);
            let scene = scene_repo.get_by_narrative_node(scene_node_id)?;
            Ok(Some((node, scene)))
        } else {
            Ok(None)
        }
    }

    // ============================================================
    // Beat 相关
    // ============================================================

    /// 创建节拍
    pub fn create_beat(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        scene_id: Uuid,
        title: &str,
        beat_attributes: BeatAttributes,
        sort_order: i32,
    ) -> Result<NarrativeNode> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let attrs = serde_json::to_value(&beat_attributes).unwrap_or_default();
        let node = repo.create_node(project_id, world_id, NarrativeNodeType::Beat, Some(scene_id), title, None, attrs, sort_order)?;
        Ok(node)
    }

    // ============================================================
    // 通用操作
    // ============================================================

    /// 获取节点及其完整子树
    pub fn get_tree(&self, node_id: Uuid) -> Result<Option<NarrativeNodeTree>> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let node = repo.get_node_by_id(node_id)?;

        if let Some(node) = node {
            let children = repo.list_children(node_id)?;
            let mut child_trees = Vec::new();
            for child in children {
                if let Some(tree) = self.get_tree(child.id)? {
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
    pub fn list_all_nodes(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        repo.list_nodes_by_project(project_id)
    }

    /// 更新节点状态
    pub fn update_status(&self, node_id: Uuid, status: NarrativeNodeStatus) -> Result<()> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        if let Some(mut node) = repo.get_node_by_id(node_id)? {
            node.status = status;
            repo.update_node(&node)?;
        }
        Ok(())
    }

    /// 创建角色弧线
    pub fn create_character_arc(
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
        let conn = self.db.conn();

        conn.execute(
            "INSERT INTO character_arc (id, project_id, character_id, volume_id, arc_type, start_state, mid_state, end_state, key_moments, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                id.to_string(), project_id.to_string(), character_id.to_string(),
                volume_id.map(|v| v.to_string()).unwrap_or_default(),
                arc_type.to_string(),
                start_state.unwrap_or("").to_string(),
                mid_state.unwrap_or("").to_string(),
                end_state.unwrap_or("").to_string(),
                serde_json::to_string(&key_moments).unwrap_or_default(),
                now.to_string(), now.to_string(),
            ],
        ).context("Failed to create character arc")?;

        Ok(CharacterArc {
            id, project_id, character_id, volume_id,
            arc_type: arc_type.to_string(),
            start_state: start_state.map(|s| s.to_string()),
            mid_state: mid_state.map(|s| s.to_string()),
            end_state: end_state.map(|s| s.to_string()),
            key_moments,
            created_at: now, updated_at: now,
        })
    }
}

/// 叙事节点树
#[derive(Debug, Clone)]
pub struct NarrativeNodeTree {
    pub node: NarrativeNode,
    pub children: Vec<NarrativeNodeTree>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_dir = format!("{}/../db/migrations", manifest_dir);
        db::migration::run_migrations(&db, &migrations_dir).unwrap();
        db
    }

    fn create_test_project(db: &Database) -> Uuid {
        let repo = db::repos::project_repo::ProjectRepo::new(db);
        repo.create("Test Novel", None).unwrap().id
    }

    fn create_test_world(db: &Database, project_id: Uuid) -> Uuid {
        let ws = super::super::world_service::WorldService::new(db);
        ws.ensure_main_world(project_id, "Test Novel").unwrap().id
    }

    #[test]
    fn test_create_volume_and_arcs() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = NarrativeService::new(&db);

        let vol = service.create_volume(project_id, world_id, "Volume 1", None, VolumeAttributes { mission: Some("Enter cultivation world".into()), ..Default::default() }, 0).unwrap();
        assert_eq!(vol.node_type, NarrativeNodeType::Volume);

        let arc1 = service.create_arc(project_id, world_id, vol.id, "Black Stone City Arc", None, 0).unwrap();
        let arc2 = service.create_arc(project_id, world_id, vol.id, "Sect Arc", None, 1).unwrap();

        let arcs = service.list_arcs(vol.id).unwrap();
        assert_eq!(arcs.len(), 2);
        assert_eq!(arcs[0].id, arc1.id);
        assert_eq!(arcs[1].id, arc2.id);
    }

    #[test]
    fn test_create_scene_with_beats() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = NarrativeService::new(&db);

        let vol = service.create_volume(project_id, world_id, "Vol 1", None, VolumeAttributes::default(), 0).unwrap();
        let arc = service.create_arc(project_id, world_id, vol.id, "Arc 1", None, 0).unwrap();
        let chapter = service.create_chapter(project_id, world_id, arc.id, "Chapter 1", None, 0).unwrap();

        let (scene_node, _scene) = service.create_scene(project_id, world_id, chapter.id, "Enter Black Market", None, SceneAttributes::default(), 0).unwrap();

        let beat1 = service.create_beat(project_id, world_id, scene_node.id, "Enter market", BeatAttributes { action: "Lin Fan walks in".into(), emotion: None, dialogue_needed: false, word_count_target: Some(500) }, 0).unwrap();
        let beat2 = service.create_beat(project_id, world_id, scene_node.id, "Discover anomaly", BeatAttributes { action: "Notices something wrong".into(), emotion: Some("Suspicious".into()), dialogue_needed: false, word_count_target: Some(300) }, 1).unwrap();

        let tree = service.get_tree(scene_node.id).unwrap().unwrap();
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].node.id, beat1.id);
        assert_eq!(tree.children[1].node.id, beat2.id);
    }

    #[test]
    fn test_full_tree() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = NarrativeService::new(&db);

        let vol = service.create_volume(project_id, world_id, "Vol 1", None, VolumeAttributes::default(), 0).unwrap();
        let arc = service.create_arc(project_id, world_id, vol.id, "Arc 1", None, 0).unwrap();
        let chapter = service.create_chapter(project_id, world_id, arc.id, "Ch 1", None, 0).unwrap();
        service.create_scene(project_id, world_id, chapter.id, "Scene 1", None, SceneAttributes::default(), 0).unwrap();

        let tree = service.get_tree(vol.id).unwrap().unwrap();
        assert_eq!(tree.children.len(), 1); // 1 arc
        assert_eq!(tree.children[0].children.len(), 1); // 1 chapter
        assert_eq!(tree.children[0].children[0].children.len(), 1); // 1 scene
    }

    #[test]
    fn test_update_status() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = NarrativeService::new(&db);

        let vol = service.create_volume(project_id, world_id, "Vol 1", None, VolumeAttributes::default(), 0).unwrap();
        assert_eq!(vol.status, NarrativeNodeStatus::Draft);

        service.update_status(vol.id, NarrativeNodeStatus::InProgress).unwrap();

        let repo = narrative_repo::NarrativeRepo::new(&db);
        let updated = repo.get_node_by_id(vol.id).unwrap().unwrap();
        assert_eq!(updated.status, NarrativeNodeStatus::InProgress);
    }
}
