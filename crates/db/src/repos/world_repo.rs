//! World Repository - CRUD operations for World

use anyhow::{Context, Result};
use chrono::Utc;
use domain::World;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct WorldRepo<'a> {
    db: &'a Database,
}

impl<'a> WorldRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建世界
    pub fn create(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO world (id, project_id, name, description, world_rules, config, is_main, created_at, updated_at) VALUES (?, ?, ?, ?, ?, '{}', ?, ?, ?)",
            [
                id.to_string(),
                project_id.to_string(),
                name.to_string(),
                description.unwrap_or("").to_string(),
                world_rules.unwrap_or("").to_string(),
                is_main.to_string(),
                now.to_string(),
                now.to_string(),
            ],
        )
        .context("Failed to create world")?;

        Ok(World {
            id,
            project_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            world_rules: world_rules.map(|s| s.to_string()),
            config: serde_json::json!({}),
            is_main,
            created_at: now,
            updated_at: now,
        })
    }

    /// 按 ID 获取世界
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<World>> {
        let conn = self.db.conn();
        let result = conn
            .query_row(
                "SELECT id, project_id, name, description, world_rules, config, is_main, created_at, updated_at FROM world WHERE id = ?",
                [id.to_string()],
                |row| {
                    Ok(World {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                        project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        name: row.get(2)?,
                        description: row.get(3)?,
                        world_rules: row.get(4)?,
                        config: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                        is_main: row.get(6)?,
                        created_at: get_timestamp(row, 7),
                        updated_at: get_timestamp(row, 8),
                    })
                },
            )
            .ok();
        Ok(result)
    }

    /// 获取项目的主要世界
    pub fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        let conn = self.db.conn();
        let result = conn
            .query_row(
                "SELECT id, project_id, name, description, world_rules, config, is_main, created_at, updated_at FROM world WHERE project_id = ? AND is_main = true LIMIT 1",
                [project_id.to_string()],
                |row| {
                    Ok(World {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                        project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        name: row.get(2)?,
                        description: row.get(3)?,
                        world_rules: row.get(4)?,
                        config: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                        is_main: row.get(6)?,
                        created_at: get_timestamp(row, 7),
                        updated_at: get_timestamp(row, 8),
                    })
                },
            )
            .ok();
        Ok(result)
    }

    /// 列出项目的所有世界
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<World>> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, name, description, world_rules, config, is_main, created_at, updated_at FROM world WHERE project_id = ? ORDER BY is_main DESC, name",
            )
            .context("Failed to prepare")?;

        let rows = stmt
            .query_map([project_id.to_string()], |row| {
                Ok(World {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get(2)?,
                    description: row.get(3)?,
                    world_rules: row.get(4)?,
                    config: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    is_main: row.get(6)?,
                    created_at: get_timestamp(row, 7),
                    updated_at: get_timestamp(row, 8),
                })
            })
            .context("Failed to query")?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 更新世界
    pub fn update(&self, world: &World) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE world SET name = ?, description = ?, world_rules = ?, config = ?, is_main = ?, updated_at = ? WHERE id = ?",
            [
                world.name.clone(),
                world.description.clone().unwrap_or_default(),
                world.world_rules.clone().unwrap_or_default(),
                world.config.to_string(),
                world.is_main.to_string(),
                Utc::now().to_string(),
                world.id.to_string(),
            ],
        )
        .context("Failed to update world")?;
        Ok(())
    }

    /// 删除世界
    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM world WHERE id = ?", [id.to_string()])
            .context("Failed to delete world")?;
        Ok(())
    }

    /// 确保项目有至少一个世界，没有则创建默认世界
    pub fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World> {
        if let Some(world) = self.get_main_world(project_id)? {
            return Ok(world);
        }
        self.create(
            project_id,
            &format!("{} - Main World", project_name),
            None,
            None,
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        migration::run_migrations(&db, migrations_dir).unwrap();
        db
    }

    fn create_test_project(db: &Database) -> Uuid {
        let repo = super::super::project_repo::ProjectRepo::new(db);
        repo.create("Test", None).unwrap().id
    }

    #[test]
    fn test_create_and_get_world() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let repo = WorldRepo::new(&db);

        let world = repo
            .create(project_id, "Main World", Some("A fantasy world"), None, true)
            .unwrap();
        assert_eq!(world.name, "Main World");
        assert!(world.is_main);

        let fetched = repo.get_by_id(world.id).unwrap().unwrap();
        assert_eq!(fetched.id, world.id);
    }

    #[test]
    fn test_get_main_world() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let repo = WorldRepo::new(&db);

        repo.create(project_id, "Alt World", None, None, false).unwrap();
        let main = repo.create(project_id, "Main World", None, None, true).unwrap();

        let found = repo.get_main_world(project_id).unwrap().unwrap();
        assert_eq!(found.id, main.id);
    }

    #[test]
    fn test_ensure_main_world() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let repo = WorldRepo::new(&db);

        let world = repo.ensure_main_world(project_id, "My Novel").unwrap();
        assert!(world.is_main);

        // 第二次调用应该返回同一个世界
        let world2 = repo.ensure_main_world(project_id, "My Novel").unwrap();
        assert_eq!(world.id, world2.id);
    }
}
