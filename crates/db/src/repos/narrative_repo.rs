//! Narrative Repository - CRUD operations for NarrativeNode, Scene

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{NarrativeNode, NarrativeNodeType, NarrativeNodeStatus, Scene};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct NarrativeRepo<'a> {
    db: &'a Database,
}

impl<'a> NarrativeRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create_node(&self, project_id: Uuid, world_id: Uuid, node_type: NarrativeNodeType, parent_id: Option<Uuid>, title: &str, description: Option<&str>, attributes: serde_json::Value, sort_order: i32) -> Result<NarrativeNode> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let type_str = crate::ser::narrative_node_type_str(&node_type);
        let status_str = crate::ser::narrative_node_status_str(&NarrativeNodeStatus::Draft);
        let conn = self.db.conn();
        
        match parent_id {
            Some(pid) => {
                conn.execute(
                    "INSERT INTO narrative_node (id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    [id.to_string(), project_id.to_string(), world_id.to_string(), type_str, pid.to_string(), title.to_string(), description.unwrap_or("").to_string(), attributes.to_string(), sort_order.to_string(), status_str, now.to_string(), now.to_string()],
                ).context("Failed to create narrative node")?;
            }
            None => {
                conn.execute(
                    "INSERT INTO narrative_node (id, project_id, world_id, node_type, title, description, attributes, sort_order, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    [id.to_string(), project_id.to_string(), world_id.to_string(), type_str, title.to_string(), description.unwrap_or("").to_string(), attributes.to_string(), sort_order.to_string(), status_str, now.to_string(), now.to_string()],
                ).context("Failed to create narrative node")?;
            }
        }
        Ok(NarrativeNode { id, project_id, world_id, node_type, parent_id, title: title.to_string(), description: description.map(|s| s.to_string()), attributes, sort_order, status: NarrativeNodeStatus::Draft, created_at: now, updated_at: now })
    }

    pub fn get_node_by_id(&self, id: Uuid) -> Result<Option<NarrativeNode>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at FROM narrative_node WHERE id = ?",
            [id.to_string()],
            |row| {
                let parent: Option<String> = row.get(4)?;
                Ok(NarrativeNode {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    node_type: crate::ser::parse_narrative_node_type(&row.get::<_, String>(3)?),
                    parent_id: parent.and_then(|p| Uuid::parse_str(&p).ok()),
                    title: row.get(5)?,
                    description: row.get(6)?,
                    attributes: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    sort_order: row.get(8)?,
                    status: crate::ser::parse_narrative_node_status(&row.get::<_, String>(9)?),
                    created_at: get_timestamp(row, 10),
                    updated_at: get_timestamp(row, 11),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_nodes_by_project(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at FROM narrative_node WHERE project_id = ? ORDER BY sort_order",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let parent: Option<String> = row.get(4)?;
            Ok(NarrativeNode {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                node_type: crate::ser::parse_narrative_node_type(&row.get::<_, String>(3)?),
                parent_id: parent.and_then(|p| Uuid::parse_str(&p).ok()),
                title: row.get(5)?,
                description: row.get(6)?,
                attributes: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                sort_order: row.get(8)?,
                status: crate::ser::parse_narrative_node_status(&row.get::<_, String>(9)?),
                created_at: get_timestamp(row, 10),
                updated_at: get_timestamp(row, 11),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at FROM narrative_node WHERE parent_id = ? ORDER BY sort_order",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([parent_id.to_string()], |row| {
            let parent: Option<String> = row.get(4)?;
            Ok(NarrativeNode {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                node_type: crate::ser::parse_narrative_node_type(&row.get::<_, String>(3)?),
                parent_id: parent.and_then(|p| Uuid::parse_str(&p).ok()),
                title: row.get(5)?,
                description: row.get(6)?,
                attributes: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                sort_order: row.get(8)?,
                status: crate::ser::parse_narrative_node_status(&row.get::<_, String>(9)?),
                created_at: get_timestamp(row, 10),
                updated_at: get_timestamp(row, 11),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update_node(&self, node: &NarrativeNode) -> Result<()> {
        let conn = self.db.conn();
        let status_str = crate::ser::narrative_node_status_str(&node.status);
        conn.execute(
            "UPDATE narrative_node SET title = ?, description = ?, attributes = ?, sort_order = ?, status = ?, updated_at = ? WHERE id = ?",
            [node.title.clone(), node.description.clone().unwrap_or_default(), node.attributes.to_string(), node.sort_order.to_string(), status_str, Utc::now().to_string(), node.id.to_string()],
        ).context("Failed to update")?;
        Ok(())
    }

    pub fn delete_node(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        let children = self.list_children(id)?;
        for child in children {
            self.delete_node(child.id)?;
        }
        conn.execute("DELETE FROM narrative_node WHERE id = ?", [id.to_string()]).context("Failed to delete")?;
        Ok(())
    }
}

pub struct SceneRepo<'a> {
    db: &'a Database,
}

impl<'a> SceneRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, narrative_node_id: Uuid, objective: Option<&str>, conflict: Option<&str>, pov_character_id: Option<Uuid>, location_id: Option<Uuid>, time: Option<&str>) -> Result<Scene> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        
        // Build SQL dynamically to handle optional UUID fields properly
        let mut columns = vec!["id", "narrative_node_id", "objective", "conflict", "time", "created_at", "updated_at"];
        let mut placeholders = vec!["?", "?", "?", "?", "?", "?", "?"];
        let mut params: Vec<Box<dyn duckdb::types::ToSql>> = vec![
            Box::new(id.to_string()), Box::new(narrative_node_id.to_string()),
            Box::new(objective.unwrap_or("").to_string()), Box::new(conflict.unwrap_or("").to_string()),
            Box::new(time.unwrap_or("").to_string()), Box::new(now.to_string()), Box::new(now.to_string()),
        ];
        
        if let Some(pov) = pov_character_id {
            columns.insert(4, "pov_character_id");
            placeholders.insert(4, "?");
            params.insert(4, Box::new(pov.to_string()));
        }
        if let Some(loc) = location_id {
            let insert_idx = if pov_character_id.is_some() { 5 } else { 4 };
            columns.insert(insert_idx, "location_id");
            placeholders.insert(insert_idx, "?");
            params.insert(insert_idx, Box::new(loc.to_string()));
        }
        
        let sql = format!(
            "INSERT INTO scene ({}) VALUES ({})",
            columns.join(", "),
            placeholders.join(", ")
        );
        
        let param_refs: Vec<&dyn duckdb::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).context("Failed to create scene")?;
        
        Ok(Scene { id, narrative_node_id, objective: objective.map(|s| s.to_string()), conflict: conflict.map(|s| s.to_string()), pov_character_id, location_id, time: time.map(|s| s.to_string()), scene_start_time: None, scene_end_time: None, created_at: now, updated_at: now })
    }

    pub fn get_by_narrative_node(&self, node_id: Uuid) -> Result<Option<Scene>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, narrative_node_id, objective, conflict, pov_character_id, location_id, time, scene_start_time, scene_end_time, created_at, updated_at FROM scene WHERE narrative_node_id = ?",
            [node_id.to_string()],
            |row| {
                let pov: Option<String> = row.get(4)?;
                let loc: Option<String> = row.get(5)?;
                Ok(Scene {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    narrative_node_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    objective: row.get(2)?,
                    conflict: row.get(3)?,
                    pov_character_id: pov.and_then(|p| Uuid::parse_str(&p).ok()),
                    location_id: loc.and_then(|l| Uuid::parse_str(&l).ok()),
                    time: row.get(6)?,
                    scene_start_time: row.get(7)?,
                    scene_end_time: row.get(8)?,
                    created_at: get_timestamp(row, 9),
                    updated_at: get_timestamp(row, 10),
                })
            },
        ).ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        db
    }

    #[test]
    fn test_narrative_node_crud() {
        let db = setup_db();
        let project_id = super::super::project_repo::ProjectRepo::new(&db).create("Test", None).unwrap().id;
        let world_id = super::super::world_repo::WorldRepo::new(&db).ensure_main_world(project_id, "Test").unwrap().id;
        let repo = NarrativeRepo::new(&db);
        let vol = repo.create_node(project_id, world_id, NarrativeNodeType::Volume, None, "Vol 1", None, serde_json::json!({}), 0).unwrap();
        let arc = repo.create_node(project_id, world_id, NarrativeNodeType::Arc, Some(vol.id), "Arc 1", None, serde_json::json!({}), 0).unwrap();
        let children = repo.list_children(vol.id).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, arc.id);
    }

    #[test]
    fn test_scene_crud() {
        let db = setup_db();
        let project_id = super::super::project_repo::ProjectRepo::new(&db).create("Test", None).unwrap().id;
        let world_id = super::super::world_repo::WorldRepo::new(&db).ensure_main_world(project_id, "Test").unwrap().id;
        let node = NarrativeRepo::new(&db).create_node(project_id, world_id, NarrativeNodeType::Scene, None, "Scene 1", None, serde_json::json!({}), 0).unwrap();
        let scene = SceneRepo::new(&db).create(node.id, Some("objective"), None, None, None, None).unwrap();
        let fetched = SceneRepo::new(&db).get_by_narrative_node(node.id).unwrap().unwrap();
        assert_eq!(fetched.id, scene.id);
    }
}
