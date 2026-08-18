//! ContextSnapshot Repository - 上下文快照持久化

use anyhow::{Context, Result};
use chrono::Utc;
use domain::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct ContextSnapshotRepo<'a> {
    db: &'a Database,
}

impl<'a> ContextSnapshotRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 保存上下文快照
    pub fn save(&self, package: &ContextPackage) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO context_snapshot (id, project_id, scene_id, token_budget, l0_essential, l1_scene_relevant, l2_recent_history, l3_narrative_context, l4_character_knowledge, l5_world_background, l6_optional_supplement, actual_tokens, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                package.id.to_string(),
                package.project_id.to_string(),
                package.scene_id.to_string(),
                package.token_budget.to_string(),
                serde_json::to_string(&package.l0_essential).unwrap_or_default(),
                serde_json::to_string(&package.l1_scene_relevant).unwrap_or_default(),
                serde_json::to_string(&package.l2_recent_history).unwrap_or_default(),
                serde_json::to_string(&package.l3_narrative_context).unwrap_or_default(),
                serde_json::to_string(&package.l4_character_knowledge).unwrap_or_default(),
                serde_json::to_string(&package.l5_world_background).unwrap_or_default(),
                serde_json::to_string(&package.l6_optional_supplement).unwrap_or_default(),
                package.actual_tokens.to_string(),
                package.created_at.to_string(),
            ],
        ).context("Failed to save context snapshot")?;
        Ok(())
    }

    /// 按 ID 获取快照
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<ContextPackage>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, scene_id, token_budget, l0_essential, l1_scene_relevant, l2_recent_history, l3_narrative_context, l4_character_knowledge, l5_world_background, l6_optional_supplement, actual_tokens, created_at FROM context_snapshot WHERE id = ?",
            [id.to_string()],
            |row| {
                let parse_layer = |s: Option<String>| -> ContextLayer {
                    s.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(ContextLayer { content: String::new(), token_estimate: 0, included: false })
                };
                Ok(ContextPackage {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    scene_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    token_budget: row.get(3)?,
                    l0_essential: parse_layer(row.get(4)?),
                    l1_scene_relevant: parse_layer(row.get(5)?),
                    l2_recent_history: parse_layer(row.get(6)?),
                    l3_narrative_context: parse_layer(row.get(7)?),
                    l4_character_knowledge: parse_layer(row.get(8)?),
                    l5_world_background: parse_layer(row.get(9)?),
                    l6_optional_supplement: parse_layer(row.get(10)?),
                    actual_tokens: row.get(11)?,
                    created_at: get_timestamp(row, 12),
                })
            },
        ).ok();
        Ok(result)
    }

    /// 按场景获取快照列表
    pub fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<ContextPackage>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id FROM context_snapshot WHERE scene_id = ? ORDER BY created_at DESC",
        ).context("Failed to prepare")?;
        let ids: Vec<Uuid> = stmt.query_map([scene_id.to_string()], |row| {
            Ok(Uuid::parse_str(&row.get::<_, String>(0)?).unwrap())
        }).context("Failed to query")?.filter_map(|r| r.ok()).collect();

        let mut result = Vec::new();
        for id in ids {
            if let Some(snapshot) = self.get_by_id(id)? {
                result.push(snapshot);
            }
        }
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
    fn test_save_and_get_snapshot() {
        let db = setup_db();
        let repo = ContextSnapshotRepo::new(&db);

        let package = ContextPackage {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            scene_id: Uuid::new_v4(),
            token_budget: 12000,
            l0_essential: ContextLayer { content: "Scene objective".into(), token_estimate: 100, included: true },
            l1_scene_relevant: ContextLayer { content: "Character states".into(), token_estimate: 200, included: true },
            l2_recent_history: ContextLayer { content: String::new(), token_estimate: 0, included: false },
            l3_narrative_context: ContextLayer { content: String::new(), token_estimate: 0, included: false },
            l4_character_knowledge: ContextLayer { content: String::new(), token_estimate: 0, included: false },
            l5_world_background: ContextLayer { content: String::new(), token_estimate: 0, included: false },
            l6_optional_supplement: ContextLayer { content: String::new(), token_estimate: 0, included: false },
            actual_tokens: 300,
            created_at: Utc::now(),
        };

        repo.save(&package).unwrap();
        let fetched = repo.get_by_id(package.id).unwrap().unwrap();
        assert_eq!(fetched.id, package.id);
        assert_eq!(fetched.actual_tokens, 300);
        assert!(fetched.l0_essential.content.contains("Scene objective"));
    }
}
