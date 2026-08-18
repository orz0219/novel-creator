//! CanonRule Repository - 世界规则 CRUD

use anyhow::{Context, Result};
use chrono::Utc;
use domain::canon::{CanonRule, RuleLevel, EnforcementAction};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct CanonRuleRepo<'a> {
    db: &'a Database,
}

impl<'a> CanonRuleRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建规则
    pub fn create(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        rule_level: RuleLevel,
        rule_content: &str,
        affected_scope: &str,
        enforcement: EnforcementAction,
        constraints: serde_json::Value,
        source: Option<&str>,
    ) -> Result<CanonRule> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO canon_rule (id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                id.to_string(),
                project_id.to_string(),
                world_id.to_string(),
                rule_level.as_str().to_string(),
                rule_content.to_string(),
                affected_scope.to_string(),
                enforcement.as_str().to_string(),
                constraints.to_string(),
                source.unwrap_or("").to_string(),
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        ).context("Failed to insert canon_rule")?;

        Ok(CanonRule {
            id,
            project_id,
            world_id,
            rule_level,
            rule_content: rule_content.to_string(),
            affected_scope: affected_scope.to_string(),
            enforcement,
            constraints,
            source: source.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    /// 查询项目的所有规则
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CanonRule>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at FROM canon_rule WHERE project_id = ? ORDER BY rule_level, created_at",
        ).context("Failed to prepare canon_rule query")?;

        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let world_id: String = row.get(2)?;
            let rule_level: String = row.get(3)?;
            let rule_content: String = row.get(4)?;
            let affected_scope: String = row.get(5)?;
            let enforcement: String = row.get(6)?;
            let constraints_str: String = row.get(7)?;
            let source: Option<String> = row.get(8)?;

            Ok(CanonRule {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                world_id: Uuid::parse_str(&world_id).unwrap(),
                rule_level: RuleLevel::from_str(&rule_level),
                rule_content,
                affected_scope,
                enforcement: EnforcementAction::from_str(&enforcement),
                constraints: serde_json::from_str(&constraints_str).unwrap_or(serde_json::json!({})),
                source,
                created_at: get_timestamp(row, 9),
                updated_at: get_timestamp(row, 10),
            })
        })?.collect::<Result<Vec<_>, _>>().context("Failed to collect canon_rules")?;

        Ok(rows)
    }

    /// 查询指定世界的所有规则
    pub fn list_by_world(&self, world_id: Uuid) -> Result<Vec<CanonRule>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at FROM canon_rule WHERE world_id = ? ORDER BY rule_level, created_at",
        ).context("Failed to prepare canon_rule query")?;

        let rows = stmt.query_map([world_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let world_id: String = row.get(2)?;
            let rule_level: String = row.get(3)?;
            let rule_content: String = row.get(4)?;
            let affected_scope: String = row.get(5)?;
            let enforcement: String = row.get(6)?;
            let constraints_str: String = row.get(7)?;
            let source: Option<String> = row.get(8)?;

            Ok(CanonRule {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                world_id: Uuid::parse_str(&world_id).unwrap(),
                rule_level: RuleLevel::from_str(&rule_level),
                rule_content,
                affected_scope,
                enforcement: EnforcementAction::from_str(&enforcement),
                constraints: serde_json::from_str(&constraints_str).unwrap_or(serde_json::json!({})),
                source,
                created_at: get_timestamp(row, 9),
                updated_at: get_timestamp(row, 10),
            })
        })?.collect::<Result<Vec<_>, _>>().context("Failed to collect canon_rules")?;

        Ok(rows)
    }

    /// 删除规则
    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "DELETE FROM canon_rule WHERE id = ?",
            [id.to_string()],
        ).context("Failed to delete canon_rule")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;

    #[test]
    fn test_canon_rule_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = CanonRuleRepo::new(&db);

        // 先创建 project
        let project_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test Project".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        // 创建
        let rule = repo.create(
            project_id,
            world_id,
            RuleLevel::Rule0,
            "死者不能复活",
            "life_death",
            EnforcementAction::Reject,
            serde_json::json!({"condition": "death", "action": "no_revive"}),
            Some("author_defined"),
        ).unwrap();

        assert_eq!(rule.rule_level, RuleLevel::Rule0);
        assert_eq!(rule.rule_content, "死者不能复活");
        assert_eq!(rule.enforcement, EnforcementAction::Reject);

        // 查询
        let rules = repo.list_by_project(project_id).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule_content, "死者不能复活");

        // 删除
        repo.delete(rule.id).unwrap();
        let rules = repo.list_by_project(project_id).unwrap();
        assert!(rules.is_empty());
    }
}
