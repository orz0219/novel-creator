//! DuckDB implementation of World Repository

use anyhow::Result;
use domain::world::World;
use duckdb::Connection;
use uuid::Uuid;

/// DuckDB implementation of world repository
pub struct DuckDbWorldRepository {
    conn: Connection,
}

impl DuckDbWorldRepository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get world by ID
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<World>> {
        let result = self.conn.query_row(
            "SELECT id, project_id, name, description, rules, created_at, updated_at FROM world WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(World {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get::<_, String>(2)?,
                    description: row.get::<_, Option<String>>(3)?,
                    rules: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            },
        );

        match result {
            Ok(world) => Ok(Some(world)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a new world
    pub fn create(&self, world: &World) -> Result<World> {
        self.conn.execute(
            "INSERT INTO world (id, project_id, name, description, rules, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                world.id.to_string(),
                world.project_id.to_string(),
                world.name.clone(),
                world.description.clone().unwrap_or_default(),
                serde_json::to_string(&world.rules)?,
                world.created_at.to_rfc3339(),
                world.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(world.clone())
    }

    /// Update an existing world
    pub fn update(&self, world: &World) -> Result<World> {
        self.conn.execute(
            "UPDATE world SET name = ?, description = ?, rules = ?, updated_at = ? WHERE id = ?",
            [
                world.name.clone(),
                world.description.clone().unwrap_or_default(),
                serde_json::to_string(&world.rules)?,
                chrono::Utc::now().to_rfc3339(),
                world.id.to_string(),
            ],
        )?;
        Ok(world.clone())
    }

    /// Delete a world
    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM world WHERE id = ?",
            [id.to_string()],
        )?;
        Ok(affected > 0)
    }

    /// List all worlds in a project
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<World>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, name, description, rules, created_at, updated_at FROM world WHERE project_id = ?",
        )?;

        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(World {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                name: row.get::<_, String>(2)?,
                description: row.get::<_, Option<String>>(3)?,
                rules: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut worlds = Vec::new();
        for row in rows {
            worlds.push(row?);
        }

        Ok(worlds)
    }

    /// Get main world for a project
    pub fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        let result = self.conn.query_row(
            "SELECT id, project_id, name, description, rules, created_at, updated_at FROM world WHERE project_id = ? AND is_main = 1",
            [project_id.to_string()],
            |row| {
                Ok(World {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get::<_, String>(2)?,
                    description: row.get::<_, Option<String>>(3)?,
                    rules: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            },
        );

        match result {
            Ok(world) => Ok(Some(world)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_world_crud() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        // Create table
        conn.execute_batch(
            "CREATE TABLE world (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description TEXT,
                rules JSON,
                is_main BOOLEAN DEFAULT 0,
                created_at TIMESTAMP,
                updated_at TIMESTAMP
            )"
        ).unwrap();

        let repo = DuckDbWorldRepository::new(conn);

        // Create world
        let world = World {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            name: "Main World".to_string(),
            description: Some("The main world of the novel".to_string()),
            rules: serde_json::json!({"physics": "normal"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created = repo.create(&world).unwrap();
        assert_eq!(created.name, "Main World");

        // Get world
        let retrieved = repo.get_by_id(created.id).unwrap().unwrap();
        assert_eq!(retrieved.name, "Main World");

        // Delete world
        let deleted = repo.delete(created.id).unwrap();
        assert!(deleted);
    }
}
