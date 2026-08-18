//! Foreshadowing Repository - CRUD operations for Foreshadowing

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{Foreshadowing, ForeshadowingStatus, ForeshadowingImportance, HintLevel};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct ForeshadowingRepo<'a> {
    db: &'a Database,
}

impl<'a> ForeshadowingRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建伏笔
    pub fn create(&self, project_id: Uuid, name: &str, description: Option<&str>, importance: ForeshadowingImportance, hint_level: HintLevel) -> Result<Foreshadowing> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status_str = "Planned";
        let imp_str = match importance {
            ForeshadowingImportance::Core => "Core",
            ForeshadowingImportance::Important => "Important",
            ForeshadowingImportance::Normal => "Normal",
            ForeshadowingImportance::Minor => "Minor",
        };
        let hl_str = match hint_level {
            HintLevel::Explicit => "Explicit",
            HintLevel::Direct => "Direct",
            HintLevel::Subtle => "Subtle",
            HintLevel::Hidden => "Hidden",
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO foreshadowing (id, project_id, name, description, status, importance, hint_level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), name.to_string(), description.unwrap_or("").to_string(), status_str.to_string(), imp_str.to_string(), hl_str.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create foreshadowing")?;
        Ok(Foreshadowing { id, project_id, storyline_id: None, name: name.to_string(), description: description.map(|s| s.to_string()), status: ForeshadowingStatus::Planned, importance, hint_level, introduced_at: None, expected_reveal_at: None, actual_reveal_at: None, created_at: now, updated_at: now })
    }

    /// 更新伏笔状态
    pub fn update_status(&self, id: Uuid, status: ForeshadowingStatus) -> Result<()> {
        let status_str = match status {
            ForeshadowingStatus::Planned => "Planned",
            ForeshadowingStatus::Introduced => "Introduced",
            ForeshadowingStatus::Active => "Active",
            ForeshadowingStatus::Revealed => "Revealed",
            ForeshadowingStatus::Abandoned => "Abandoned",
        };
        let conn = self.db.conn();
        conn.execute(
            "UPDATE foreshadowing SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            [status_str.to_string(), id.to_string()],
        ).context("Failed to update foreshadowing status")?;
        Ok(())
    }

    /// 列出项目中的所有伏笔
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Foreshadowing>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, storyline_id, name, description, status, importance, hint_level, introduced_at, expected_reveal_at, actual_reveal_at, created_at, updated_at FROM foreshadowing WHERE project_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(Foreshadowing {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                storyline_id: row.get::<_, Option<String>>(2)?.and_then(|s| Uuid::parse_str(&s).ok()),
                name: row.get(3)?,
                description: row.get::<_, Option<String>>(4)?,
                status: match row.get::<_, String>(5)?.as_str() {
                    "Planned" => ForeshadowingStatus::Planned,
                    "Introduced" => ForeshadowingStatus::Introduced,
                    "Active" => ForeshadowingStatus::Active,
                    "Revealed" => ForeshadowingStatus::Revealed,
                    "Abandoned" => ForeshadowingStatus::Abandoned,
                    _ => ForeshadowingStatus::Planned,
                },
                importance: match row.get::<_, String>(6)?.as_str() {
                    "Core" => ForeshadowingImportance::Core,
                    "Important" => ForeshadowingImportance::Important,
                    "Normal" => ForeshadowingImportance::Normal,
                    "Minor" => ForeshadowingImportance::Minor,
                    _ => ForeshadowingImportance::Normal,
                },
                hint_level: match row.get::<_, String>(7)?.as_str() {
                    "Explicit" => HintLevel::Explicit,
                    "Direct" => HintLevel::Direct,
                    "Subtle" => HintLevel::Subtle,
                    "Hidden" => HintLevel::Hidden,
                    _ => HintLevel::Subtle,
                },
                introduced_at: row.get::<_, Option<String>>(8)?,
                expected_reveal_at: row.get::<_, Option<String>>(9)?,
                actual_reveal_at: row.get::<_, Option<String>>(10)?,
                created_at: get_timestamp(row, 11),
                updated_at: get_timestamp(row, 12),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> (Database, Uuid) {
        let db = Database::open_in_memory().unwrap();
        migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let project_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test Project".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }
        (db, project_id)
    }

    #[test]
    fn test_create_foreshadowing() {
        let (db, project_id) = setup_db();
        let repo = ForeshadowingRepo::new(&db);
        let f = repo.create(project_id, "地下遗迹存在", Some("贯穿多卷的核心悬念"), ForeshadowingImportance::Core, HintLevel::Subtle).unwrap();
        assert_eq!(f.name, "地下遗迹存在");
        assert_eq!(f.status, ForeshadowingStatus::Planned);
    }

    #[test]
    fn test_update_status() {
        let (db, project_id) = setup_db();
        let repo = ForeshadowingRepo::new(&db);
        let f = repo.create(project_id, "奇怪石碑", None, ForeshadowingImportance::Important, HintLevel::Hidden).unwrap();
        repo.update_status(f.id, ForeshadowingStatus::Introduced).unwrap();
        let list = repo.list_by_project(project_id).unwrap();
        assert_eq!(list[0].status, ForeshadowingStatus::Introduced);
    }
}
