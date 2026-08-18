//! EntityAlias + IdentityTimeline + TestCase Repos

use anyhow::{Context, Result};
use chrono::Utc;
use domain::identity::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct EntityAliasRepo<'a> {
    db: &'a Database,
}

impl<'a> EntityAliasRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, entity_id: Uuid, alias_type: AliasType, alias: &str) -> Result<EntityAlias> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO entity_alias (id, entity_id, alias_type, alias, created_at) VALUES (?, ?, ?, ?, ?)",
            [id.to_string(), entity_id.to_string(), alias_type.as_str().to_string(), alias.to_string(), now.to_rfc3339()],
        ).context("Failed to insert entity_alias")?;
        Ok(EntityAlias { id, entity_id, alias_type, alias: alias.to_string(), valid_from_scene_id: None, valid_until_scene_id: None, created_at: now })
    }

    pub fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<EntityAlias>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, entity_id, alias_type, alias, valid_from_scene_id, valid_until_scene_id, created_at FROM entity_alias WHERE entity_id = ?").context("Failed to prepare")?;
        let rows = stmt.query_map([entity_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let entity_id: String = row.get(1)?;
            let alias_type: String = row.get(2)?;
            let alias: String = row.get(3)?;
            let valid_from: Option<String> = row.get(4)?;
            let valid_until: Option<String> = row.get(5)?;
            Ok(EntityAlias {
                id: Uuid::parse_str(&id).unwrap(),
                entity_id: Uuid::parse_str(&entity_id).unwrap(),
                alias_type: AliasType::from_str(&alias_type),
                alias,
                valid_from_scene_id: valid_from.and_then(|s| Uuid::parse_str(&s).ok()),
                valid_until_scene_id: valid_until.and_then(|s| Uuid::parse_str(&s).ok()),
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_by_alias(&self, alias: &str) -> Result<Vec<EntityAlias>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, entity_id, alias_type, alias, valid_from_scene_id, valid_until_scene_id, created_at FROM entity_alias WHERE alias LIKE ?").context("Failed to prepare")?;
        let rows = stmt.query_map([format!("%{}%", alias)], |row| {
            let id: String = row.get(0)?;
            let entity_id: String = row.get(1)?;
            let alias_type: String = row.get(2)?;
            let alias: String = row.get(3)?;
            let valid_from: Option<String> = row.get(4)?;
            let valid_until: Option<String> = row.get(5)?;
            Ok(EntityAlias {
                id: Uuid::parse_str(&id).unwrap(),
                entity_id: Uuid::parse_str(&entity_id).unwrap(),
                alias_type: AliasType::from_str(&alias_type),
                alias,
                valid_from_scene_id: valid_from.and_then(|s| Uuid::parse_str(&s).ok()),
                valid_until_scene_id: valid_until.and_then(|s| Uuid::parse_str(&s).ok()),
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct IdentityTimelineRepo<'a> {
    db: &'a Database,
}

impl<'a> IdentityTimelineRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, entity_id: Uuid, identity: &str, start_scene_id: Uuid, change_reason: Option<&str>) -> Result<IdentityTimeline> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO identity_timeline (id, entity_id, identity, start_scene_id, change_reason, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            [id.to_string(), entity_id.to_string(), identity.to_string(), start_scene_id.to_string(), change_reason.unwrap_or("").to_string(), now.to_rfc3339()],
        ).context("Failed to insert identity_timeline")?;
        Ok(IdentityTimeline { id, entity_id, identity: identity.to_string(), start_scene_id, end_scene_id: None, change_reason: change_reason.map(|s| s.to_string()), created_at: now })
    }

    pub fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<IdentityTimeline>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, entity_id, identity, start_scene_id, end_scene_id, change_reason, created_at FROM identity_timeline WHERE entity_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([entity_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let entity_id: String = row.get(1)?;
            let identity: String = row.get(2)?;
            let start_scene_id: String = row.get(3)?;
            let end_scene_id: Option<String> = row.get(4)?;
            let change_reason: Option<String> = row.get(5)?;
            Ok(IdentityTimeline {
                id: Uuid::parse_str(&id).unwrap(),
                entity_id: Uuid::parse_str(&entity_id).unwrap(),
                identity,
                start_scene_id: Uuid::parse_str(&start_scene_id).unwrap(),
                end_scene_id: end_scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                change_reason,
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct TestCaseRepo<'a> {
    db: &'a Database,
}

impl<'a> TestCaseRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, name: &str, description: &str, test_type: TestType, preconditions: serde_json::Value, expected_result: &str) -> Result<TestCase> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO test_case (id, project_id, name, description, test_type, preconditions, expected_result, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), name.to_string(), description.to_string(), test_type.as_str().to_string(), preconditions.to_string(), expected_result.to_string(), "Pending".to_string(), now.to_rfc3339()],
        ).context("Failed to insert test_case")?;
        Ok(TestCase { id, project_id, name: name.to_string(), description: description.to_string(), test_type, preconditions, expected_result: expected_result.to_string(), status: TestStatus::Pending, created_at: now })
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<TestCase>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, name, description, test_type, preconditions, expected_result, status, created_at FROM test_case WHERE project_id = ?").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let name: String = row.get(2)?;
            let description: String = row.get(3)?;
            let test_type: String = row.get(4)?;
            let preconditions_str: String = row.get(5)?;
            let expected_result: String = row.get(6)?;
            let status: String = row.get(7)?;
            Ok(TestCase {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                name,
                description,
                test_type: TestType::from_str(&test_type),
                preconditions: serde_json::from_str(&preconditions_str).unwrap_or(serde_json::json!({})),
                expected_result,
                status: TestStatus::from_str(&status),
                created_at: get_timestamp(row, 8),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;

    #[test]
    fn test_alias_and_identity() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();

        let entity_id = Uuid::new_v4();
        let alias_repo = EntityAliasRepo::new(&db);
        let identity_repo = IdentityTimelineRepo::new(&db);

        // 创建别名
        let a1 = alias_repo.create(entity_id, AliasType::Canonical, "林凡").unwrap();
        let a2 = alias_repo.create(entity_id, AliasType::Alias, "黑袍人").unwrap();
        let a3 = alias_repo.create(entity_id, AliasType::Title, "玄天尊者").unwrap();

        let aliases = alias_repo.list_by_entity(entity_id).unwrap();
        assert_eq!(aliases.len(), 3);

        // 搜索别名
        let found = alias_repo.find_by_alias("林凡").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].entity_id, entity_id);

        // 创建身份时间线
        let start_scene = Uuid::new_v4();
        let t1 = identity_repo.create(entity_id, "普通散修", start_scene, None).unwrap();
        assert_eq!(t1.identity, "普通散修");

        let timelines = identity_repo.list_by_entity(entity_id).unwrap();
        assert_eq!(timelines.len(), 1);
    }

    #[test]
    fn test_evaluation_dataset() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();

        let project_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let repo = TestCaseRepo::new(&db);
        let tc = repo.create(project_id, "主角不知道幕后黑手", "验证主角在特定场景下不知道幕后黑手身份", TestType::KnowledgeBoundary, serde_json::json!({"chapter": 1, "character": "林凡"}), "主角不应该知道幕后黑手是A").unwrap();
        assert_eq!(tc.test_type, TestType::KnowledgeBoundary);
        assert_eq!(tc.status, TestStatus::Pending);

        let cases = repo.list_by_project(project_id).unwrap();
        assert_eq!(cases.len(), 1);
    }
}
