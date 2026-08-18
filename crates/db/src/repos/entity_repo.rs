//! Entity Repository - CRUD operations for Entity, EntityType, Relation, Fact

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{Entity, EntityType, Fact, Relation};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct EntityTypeRepo<'a> {
    db: &'a Database,
}

impl<'a> EntityTypeRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, name: &str, description: Option<&str>) -> Result<EntityType> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO entity_type (id, name, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            [id.to_string(), name.to_string(), description.unwrap_or("").to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create entity type")?;
        Ok(EntityType { id, name: name.to_string(), description: description.map(|s| s.to_string()), schema: None, created_at: now, updated_at: now })
    }

    pub fn get_by_name(&self, name: &str) -> Result<Option<EntityType>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, name, description, created_at, updated_at FROM entity_type WHERE name = ?",
            [name],
            |row| {
                Ok(EntityType {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    schema: None,
                    created_at: get_timestamp(row, 3),
                    updated_at: get_timestamp(row, 4),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_all(&self) -> Result<Vec<EntityType>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, name, description, created_at, updated_at FROM entity_type ORDER BY name").context("Failed to prepare")?;
        let rows = stmt.query_map([], |row| {
            Ok(EntityType {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                description: row.get(2)?,
                schema: None,
                created_at: get_timestamp(row, 3),
                updated_at: get_timestamp(row, 4),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn ensure(&self, name: &str, description: Option<&str>) -> Result<EntityType> {
        if let Some(existing) = self.get_by_name(name)? {
            return Ok(existing);
        }
        self.create(name, description)
    }
}

pub struct EntityRepo<'a> {
    db: &'a Database,
}

impl<'a> EntityRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, project_id: Uuid, world_id: Uuid, entity_type_id: Uuid, name: &str, summary: Option<&str>, description: Option<&str>, attributes: serde_json::Value) -> Result<Entity> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), world_id.to_string(), entity_type_id.to_string(), name.to_string(), summary.unwrap_or("").to_string(), description.unwrap_or("").to_string(), attributes.to_string(), "1".to_string(), "system".to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create entity")?;
        Ok(Entity { id, project_id, world_id, entity_type_id, name: name.to_string(), summary: summary.map(|s| s.to_string()), description: description.map(|s| s.to_string()), attributes, version: 1, created_by: "system".to_string(), updated_by: None, source_generation_id: None, created_at: now, updated_at: now })
    }

    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Entity>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at FROM entity WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Entity {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    entity_type_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                    name: row.get(4)?,
                    summary: row.get(5)?,
                    description: row.get(6)?,
                    attributes: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    version: row.get::<_, i32>(8)?,
                    created_by: row.get(9)?,
                    updated_by: row.get(10)?,
                    source_generation_id: row.get::<_, Option<String>>(11)?.and_then(|s| Uuid::parse_str(&s).ok()),
                    created_at: get_timestamp(row, 12),
                    updated_at: get_timestamp(row, 13),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Entity>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at FROM entity WHERE project_id = ? ORDER BY name",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(Entity {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_type_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                name: row.get(4)?,
                summary: row.get(5)?,
                description: row.get(6)?,
                attributes: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                version: row.get::<_, i32>(8)?,
                created_by: row.get(9)?,
                updated_by: row.get(10)?,
                source_generation_id: row.get::<_, Option<String>>(11)?.and_then(|s| Uuid::parse_str(&s).ok()),
                created_at: get_timestamp(row, 12),
                updated_at: get_timestamp(row, 13),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_by_type(&self, project_id: Uuid, entity_type_id: Uuid) -> Result<Vec<Entity>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at FROM entity WHERE project_id = ? AND entity_type_id = ? ORDER BY name",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string(), entity_type_id.to_string()], |row| {
            Ok(Entity {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                world_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_type_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                name: row.get(4)?,
                summary: row.get(5)?,
                description: row.get(6)?,
                attributes: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                version: row.get::<_, i32>(8)?,
                created_by: row.get(9)?,
                updated_by: row.get(10)?,
                source_generation_id: row.get::<_, Option<String>>(11)?.and_then(|s| Uuid::parse_str(&s).ok()),
                created_at: get_timestamp(row, 12),
                updated_at: get_timestamp(row, 13),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update(&self, entity: &Entity) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE entity SET name = ?, summary = ?, attributes = ?, updated_at = ? WHERE id = ?",
            [entity.name.clone(), entity.summary.clone().unwrap_or_default(), entity.attributes.to_string(), Utc::now().to_string(), entity.id.to_string()],
        ).context("Failed to update")?;
        Ok(())
    }

    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM entity WHERE id = ?", [id.to_string()]).context("Failed to delete")?;
        Ok(())
    }
}

pub struct RelationRepo<'a> {
    db: &'a Database,
}

impl<'a> RelationRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, project_id: Uuid, source_entity_id: Uuid, target_entity_id: Uuid, relation_type: &str, description: Option<&str>, attributes: serde_json::Value) -> Result<Relation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO relation (id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), source_entity_id.to_string(), target_entity_id.to_string(), relation_type.to_string(), description.unwrap_or("").to_string(), attributes.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create relation")?;
        Ok(Relation { id, project_id, source_entity_id, target_entity_id, relation_type: relation_type.to_string(), description: description.map(|s| s.to_string()), attributes, valid_from: None, valid_until: None, created_at: now, updated_at: now })
    }

    pub fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<Relation>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, valid_from, valid_until, created_at, updated_at FROM relation WHERE source_entity_id = ? OR target_entity_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([entity_id.to_string(), entity_id.to_string()], |row| {
            Ok(Relation {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                source_entity_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                target_entity_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                relation_type: row.get(4)?,
                description: row.get(5)?,
                attributes: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                valid_from: row.get::<_, Option<String>>(7)?,
                valid_until: row.get::<_, Option<String>>(8)?,
                created_at: get_timestamp(row, 9),
                updated_at: get_timestamp(row, 10),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM relation WHERE id = ?", [id.to_string()]).context("Failed to delete")?;
        Ok(())
    }
}

pub struct FactRepo<'a> {
    db: &'a Database,
}

impl<'a> FactRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, project_id: Uuid, content: &str, category: Option<&str>, certainty: &str, related_entity_ids: &[Uuid]) -> Result<Fact> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO fact (id, project_id, content, category, certainty, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), content.to_string(), category.unwrap_or("").to_string(), certainty.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create fact")?;
        for entity_id in related_entity_ids {
            conn.execute("INSERT INTO fact_entity (id, fact_id, entity_id) VALUES (?, ?, ?)", [Uuid::new_v4().to_string(), id.to_string(), entity_id.to_string()]).context("Failed to create fact_entity")?;
        }
        Ok(Fact { id, project_id, content: content.to_string(), category: category.map(|s| s.to_string()), certainty: domain::canon::FactCertainty::from_str(certainty), created_at: now, updated_at: now })
    }

    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Fact>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, content, category, COALESCE(certainty, 'CANON'), created_at, updated_at FROM fact WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Fact {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    content: row.get(2)?,
                    category: row.get(3)?,
                    certainty: domain::canon::FactCertainty::from_str(&row.get::<_, String>(4).unwrap_or_default()),
                    created_at: get_timestamp(row, 5),
                    updated_at: get_timestamp(row, 6),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Fact>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, content, category, COALESCE(certainty, 'CANON'), created_at, updated_at FROM fact WHERE project_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(Fact {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                content: row.get(2)?,
                category: row.get(3)?,
                certainty: domain::canon::FactCertainty::from_str(&row.get::<_, String>(4).unwrap_or_default()),
                created_at: get_timestamp(row, 5),
                updated_at: get_timestamp(row, 6),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM fact_entity WHERE fact_id = ?", [id.to_string()]).context("Failed to delete")?;
        conn.execute("DELETE FROM fact WHERE id = ?", [id.to_string()]).context("Failed to delete")?;
        Ok(())
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

    fn create_test_world(db: &Database, project_id: Uuid) -> Uuid {
        let repo = super::super::world_repo::WorldRepo::new(db);
        repo.ensure_main_world(project_id, "Test").unwrap().id
    }

    #[test]
    fn test_entity_type_crud() {
        let db = setup_db();
        let repo = EntityTypeRepo::new(&db);
        let et = repo.create("Character", Some("A person")).unwrap();
        assert_eq!(et.name, "Character");
        let fetched = repo.get_by_name("Character").unwrap().unwrap();
        assert_eq!(fetched.id, et.id);
        let et2 = repo.ensure("Character", None).unwrap();
        assert_eq!(et2.id, et.id);
    }

    #[test]
    fn test_entity_crud() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let type_repo = EntityTypeRepo::new(&db);
        let entity_repo = EntityRepo::new(&db);
        let char_type = type_repo.ensure("Character", None).unwrap();
        let entity = entity_repo.create(project_id, world_id, char_type.id, "Lin Fan", Some("A cultivator"), Some("Test desc"), serde_json::json!({"age": 20})).unwrap();
        let fetched = entity_repo.get_by_id(entity.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Lin Fan");
        assert_eq!(fetched.world_id, world_id);
    }

    #[test]
    fn test_fact_crud() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let repo = FactRepo::new(&db);
        let fact = repo.create(project_id, "Ruins exist", Some("secret"), &[]).unwrap();
        assert!(repo.get_by_id(fact.id).unwrap().is_some());
        repo.delete(fact.id).unwrap();
        assert!(repo.get_by_id(fact.id).unwrap().is_none());
    }
}
