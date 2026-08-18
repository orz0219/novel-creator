//! DuckDB implementation of Entity Repository

use anyhow::Result;
use domain::entity::Entity;
use domain::project::ProjectId;
use duckdb::Connection;
use uuid::Uuid;

/// DuckDB implementation of entity repository
pub struct DuckDbEntityRepository {
    conn: Connection,
}

impl DuckDbEntityRepository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get entity by ID
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Entity>> {
        let result = self.conn.query_row(
            "SELECT id, project_id, world_id, entity_type_id, name, description, attributes, created_at, updated_at FROM entity WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Entity {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    entity_type_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                    name: row.get::<_, String>(4)?,
                    description: row.get::<_, Option<String>>(5)?,
                    attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            },
        );

        match result {
            Ok(entity) => Ok(Some(entity)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a new entity
    pub fn create(&self, entity: &Entity) -> Result<Entity> {
        self.conn.execute(
            "INSERT INTO entity (id, project_id, world_id, entity_type_id, name, description, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                entity.id.to_string(),
                entity.project_id.to_string(),
                entity.world_id.to_string(),
                entity.entity_type_id.to_string(),
                entity.name.clone(),
                entity.description.clone().unwrap_or_default(),
                serde_json::to_string(&entity.attributes)?,
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(entity.clone())
    }

    /// Update an existing entity
    pub fn update(&self, entity: &Entity) -> Result<Entity> {
        self.conn.execute(
            "UPDATE entity SET name = ?, description = ?, attributes = ?, updated_at = ? WHERE id = ?",
            [
                entity.name.clone(),
                entity.description.clone().unwrap_or_default(),
                serde_json::to_string(&entity.attributes)?,
                chrono::Utc::now().to_rfc3339(),
                entity.id.to_string(),
            ],
        )?;
        Ok(entity.clone())
    }

    /// Delete an entity
    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM entity WHERE id = ?",
            [id.to_string()],
        )?;
        Ok(affected > 0)
    }

    /// List all entities in a project
    pub fn list_by_project(&self, project_id: ProjectId) -> Result<Vec<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, world_id, entity_type_id, name, description, attributes, created_at, updated_at FROM entity WHERE project_id = ?",
        )?;

        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(Entity {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_type_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                name: row.get::<_, String>(4)?,
                description: row.get::<_, Option<String>>(5)?,
                attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }

        Ok(entities)
    }

    /// Search entities by name
    pub fn search_by_name(&self, project_id: ProjectId, name: &str) -> Result<Vec<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, world_id, entity_type_id, name, description, attributes, created_at, updated_at FROM entity WHERE project_id = ? AND name LIKE ?",
        )?;

        let search_pattern = format!("%{}%", name);
        let rows = stmt.query_map([project_id.to_string(), search_pattern], |row| {
            Ok(Entity {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_type_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                name: row.get::<_, String>(4)?,
                description: row.get::<_, Option<String>>(5)?,
                attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }

        Ok(entities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_entity_crud() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        // Create table
        conn.execute_batch(
            "CREATE TABLE entity (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                world_id VARCHAR NOT NULL,
                entity_type_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description TEXT,
                attributes JSON,
                created_at TIMESTAMP,
                updated_at TIMESTAMP
            )"
        ).unwrap();

        let repo = DuckDbEntityRepository::new(conn);

        // Create entity
        let entity = Entity {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            world_id: Uuid::new_v4(),
            entity_type_id: Uuid::new_v4(),
            name: "Test Character".to_string(),
            description: Some("A test character".to_string()),
            attributes: serde_json::json!({"level": 10}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created = repo.create(&entity).unwrap();
        assert_eq!(created.name, "Test Character");

        // Get entity
        let retrieved = repo.get_by_id(created.id).unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Character");

        // Update entity
        let mut updated = retrieved.clone();
        updated.name = "Updated Character".to_string();
        let updated_entity = repo.update(&updated).unwrap();
        assert_eq!(updated_entity.name, "Updated Character");

        // Delete entity
        let deleted = repo.delete(updated_entity.id).unwrap();
        assert!(deleted);

        // Verify deletion
        let not_found = repo.get_by_id(updated_entity.id).unwrap();
        assert!(not_found.is_none());
    }
}
