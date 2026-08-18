//! DuckDB implementation of Project Repository

use anyhow::Result;
use domain::project::Project;
use duckdb::Connection;
use uuid::Uuid;

/// DuckDB implementation of project repository
pub struct DuckDbProjectRepository {
    conn: Connection,
}

impl DuckDbProjectRepository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get project by ID
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let result = self.conn.query_row(
            "SELECT id, name, description, status, created_at, updated_at FROM project WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Project {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get::<_, String>(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    status: row.get::<_, String>(3)?,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            },
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a new project
    pub fn create(&self, project: &Project) -> Result<Project> {
        self.conn.execute(
            "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            [
                project.id.to_string(),
                project.name.clone(),
                project.description.clone().unwrap_or_default(),
                project.status.clone(),
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(project.clone())
    }

    /// Update an existing project
    pub fn update(&self, project: &Project) -> Result<Project> {
        self.conn.execute(
            "UPDATE project SET name = ?, description = ?, status = ?, updated_at = ? WHERE id = ?",
            [
                project.name.clone(),
                project.description.clone().unwrap_or_default(),
                project.status.clone(),
                chrono::Utc::now().to_rfc3339(),
                project.id.to_string(),
            ],
        )?;
        Ok(project.clone())
    }

    /// Delete a project
    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM project WHERE id = ?",
            [id.to_string()],
        )?;
        Ok(affected > 0)
    }

    /// List all projects
    pub fn list_all(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, status, created_at, updated_at FROM project ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get::<_, String>(1)?,
                description: row.get::<_, Option<String>>(2)?,
                status: row.get::<_, String>(3)?,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }

        Ok(projects)
    }

    /// Search projects by name
    pub fn search_by_name(&self, name: &str) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, status, created_at, updated_at FROM project WHERE name LIKE ? ORDER BY created_at DESC",
        )?;

        let search_pattern = format!("%{}%", name);
        let rows = stmt.query_map([search_pattern], |row| {
            Ok(Project {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get::<_, String>(1)?,
                description: row.get::<_, Option<String>>(2)?,
                status: row.get::<_, String>(3)?,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }

        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_project_crud() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        // Create table
        conn.execute_batch(
            "CREATE TABLE project (
                id VARCHAR PRIMARY KEY,
                name VARCHAR NOT NULL,
                description TEXT,
                status VARCHAR NOT NULL DEFAULT 'active',
                created_at TIMESTAMP,
                updated_at TIMESTAMP
            )"
        ).unwrap();

        let repo = DuckDbProjectRepository::new(conn);

        // Create project
        let project = Project {
            id: Uuid::new_v4(),
            name: "Test Novel".to_string(),
            description: Some("A test novel project".to_string()),
            status: "active".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created = repo.create(&project).unwrap();
        assert_eq!(created.name, "Test Novel");

        // Get project
        let retrieved = repo.get_by_id(created.id).unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Novel");

        // Update project
        let mut updated = retrieved.clone();
        updated.name = "Updated Novel".to_string();
        let updated_project = repo.update(&updated).unwrap();
        assert_eq!(updated_project.name, "Updated Novel");

        // Delete project
        let deleted = repo.delete(updated_project.id).unwrap();
        assert!(deleted);

        // Verify deletion
        let not_found = repo.get_by_id(updated_project.id).unwrap();
        assert!(not_found.is_none());
    }
}
