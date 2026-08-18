//! DuckDB implementation of Narrative Repository

use anyhow::Result;
use domain::narrative::{NarrativeNode, NarrativeNodeType};
use duckdb::Connection;
use uuid::Uuid;

/// DuckDB implementation of narrative repository
pub struct DuckDbNarrativeRepository {
    conn: Connection,
}

impl DuckDbNarrativeRepository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get narrative node by ID
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<NarrativeNode>> {
        let result = self.conn.query_row(
            "SELECT id, project_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at FROM narrative_node WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(NarrativeNode {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    node_type: match row.get::<_, String>(2)?.as_str() {
                        "Volume" => NarrativeNodeType::Volume,
                        "Arc" => NarrativeNodeType::Arc,
                        "Sequence" => NarrativeNodeType::Sequence,
                        "Chapter" => NarrativeNodeType::Chapter,
                        "Scene" => NarrativeNodeType::Scene,
                        "Beat" => NarrativeNodeType::Beat,
                        _ => NarrativeNodeType::Special,
                    },
                    parent_id: row.get::<_, Option<String>>(3)?.and_then(|s| Uuid::parse_str(&s).ok()),
                    title: row.get::<_, String>(4)?,
                    description: row.get::<_, Option<String>>(5)?,
                    attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    sort_order: row.get::<_, i32>(7)?,
                    status: row.get::<_, String>(8)?,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            },
        );

        match result {
            Ok(node) => Ok(Some(node)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a new narrative node
    pub fn create(&self, node: &NarrativeNode) -> Result<NarrativeNode> {
        self.conn.execute(
            "INSERT INTO narrative_node (id, project_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                node.id.to_string(),
                node.project_id.to_string(),
                format!("{:?}", node.node_type),
                node.parent_id.map(|id| id.to_string()).unwrap_or_default(),
                node.title.clone(),
                node.description.clone().unwrap_or_default(),
                serde_json::to_string(&node.attributes)?,
                node.sort_order.to_string(),
                node.status.clone(),
                node.created_at.to_rfc3339(),
                node.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(node.clone())
    }

    /// Update a narrative node
    pub fn update(&self, node: &NarrativeNode) -> Result<NarrativeNode> {
        self.conn.execute(
            "UPDATE narrative_node SET title = ?, description = ?, attributes = ?, sort_order = ?, status = ?, updated_at = ? WHERE id = ?",
            [
                node.title.clone(),
                node.description.clone().unwrap_or_default(),
                serde_json::to_string(&node.attributes)?,
                node.sort_order.to_string(),
                node.status.clone(),
                chrono::Utc::now().to_rfc3339(),
                node.id.to_string(),
            ],
        )?;
        Ok(node.clone())
    }

    /// Delete a narrative node
    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM narrative_node WHERE id = ?",
            [id.to_string()],
        )?;
        Ok(affected > 0)
    }

    /// List all narrative nodes in a project
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at FROM narrative_node WHERE project_id = ? ORDER BY sort_order",
        )?;

        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(NarrativeNode {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                node_type: match row.get::<_, String>(2)?.as_str() {
                    "Volume" => NarrativeNodeType::Volume,
                    "Arc" => NarrativeNodeType::Arc,
                    "Sequence" => NarrativeNodeType::Sequence,
                    "Chapter" => NarrativeNodeType::Chapter,
                    "Scene" => NarrativeNodeType::Scene,
                    "Beat" => NarrativeNodeType::Beat,
                    _ => NarrativeNodeType::Special,
                },
                parent_id: row.get::<_, Option<String>>(3)?.and_then(|s| Uuid::parse_str(&s).ok()),
                title: row.get::<_, String>(4)?,
                description: row.get::<_, Option<String>>(5)?,
                attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                sort_order: row.get::<_, i32>(7)?,
                status: row.get::<_, String>(8)?,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }

        Ok(nodes)
    }

    /// Get child nodes of a parent
    pub fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, node_type, parent_id, title, description, attributes, sort_order, status, created_at, updated_at FROM narrative_node WHERE parent_id = ? ORDER BY sort_order",
        )?;

        let rows = stmt.query_map([parent_id.to_string()], |row| {
            Ok(NarrativeNode {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                node_type: match row.get::<_, String>(2)?.as_str() {
                    "Volume" => NarrativeNodeType::Volume,
                    "Arc" => NarrativeNodeType::Arc,
                    "Sequence" => NarrativeNodeType::Sequence,
                    "Chapter" => NarrativeNodeType::Chapter,
                    "Scene" => NarrativeNodeType::Scene,
                    "Beat" => NarrativeNodeType::Beat,
                    _ => NarrativeNodeType::Special,
                },
                parent_id: row.get::<_, Option<String>>(3)?.and_then(|s| Uuid::parse_str(&s).ok()),
                title: row.get::<_, String>(4)?,
                description: row.get::<_, Option<String>>(5)?,
                attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                sort_order: row.get::<_, i32>(7)?,
                status: row.get::<_, String>(8)?,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }

        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_narrative_node_crud() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        // Create table
        conn.execute_batch(
            "CREATE TABLE narrative_node (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                node_type VARCHAR NOT NULL,
                parent_id VARCHAR,
                title VARCHAR NOT NULL,
                description TEXT,
                attributes JSON,
                sort_order INTEGER DEFAULT 0,
                status VARCHAR NOT NULL DEFAULT 'Draft',
                created_at TIMESTAMP,
                updated_at TIMESTAMP
            )"
        ).unwrap();

        let repo = DuckDbNarrativeRepository::new(conn);

        // Create volume
        let volume = NarrativeNode {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            node_type: NarrativeNodeType::Volume,
            parent_id: None,
            title: "Volume 1".to_string(),
            description: Some("First volume".to_string()),
            attributes: serde_json::json!({"goal": "Introduction"}),
            sort_order: 1,
            status: "Draft".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created = repo.create(&volume).unwrap();
        assert_eq!(created.title, "Volume 1");

        // Create arc under volume
        let arc = NarrativeNode {
            id: Uuid::new_v4(),
            project_id: created.project_id,
            node_type: NarrativeNodeType::Arc,
            parent_id: Some(created.id),
            title: "Arc 1".to_string(),
            description: Some("First arc".to_string()),
            attributes: serde_json::json!({"conflict": "Internal"}),
            sort_order: 1,
            status: "Draft".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created_arc = repo.create(&arc).unwrap();
        assert_eq!(created_arc.title, "Arc 1");

        // Get children
        let children = repo.list_children(created.id).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].title, "Arc 1");

        // Delete arc
        let deleted = repo.delete(created_arc.id).unwrap();
        assert!(deleted);

        // Verify deletion
        let children = repo.list_children(created.id).unwrap();
        assert_eq!(children.len(), 0);
    }
}
