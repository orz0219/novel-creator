//! Storyline Repository - CRUD operations for Storyline

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{Storyline, StorylineStatus, StorylineImportance, StorylineScene};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct StorylineRepo<'a> {
    db: &'a Database,
}

impl<'a> StorylineRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建剧情线
    pub fn create(&self, project_id: Uuid, name: &str, description: Option<&str>, importance: StorylineImportance) -> Result<Storyline> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status_str = "Active";
        let imp_str = match importance {
            StorylineImportance::Main => "Main",
            StorylineImportance::Important => "Important",
            StorylineImportance::Normal => "Normal",
            StorylineImportance::Minor => "Minor",
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO storyline (id, project_id, name, description, status, importance, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), name.to_string(), description.unwrap_or("").to_string(), status_str.to_string(), imp_str.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create storyline")?;
        Ok(Storyline { id, project_id, name: name.to_string(), description: description.map(|s| s.to_string()), status: StorylineStatus::Active, importance, created_volume_id: None, resolved_volume_id: None, created_at: now, updated_at: now })
    }

    /// 按 ID 获取剧情线
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Storyline>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, name, description, status, importance, created_volume_id, resolved_volume_id, created_at, updated_at FROM storyline WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Storyline {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get(2)?,
                    description: row.get::<_, Option<String>>(3)?,
                    status: match row.get::<_, String>(4)?.as_str() {
                        "Planned" => StorylineStatus::Planned,
                        "Active" => StorylineStatus::Active,
                        "Resolved" => StorylineStatus::Resolved,
                        "Abandoned" => StorylineStatus::Abandoned,
                        _ => StorylineStatus::Active,
                    },
                    importance: match row.get::<_, String>(5)?.as_str() {
                        "Main" => StorylineImportance::Main,
                        "Important" => StorylineImportance::Important,
                        "Normal" => StorylineImportance::Normal,
                        "Minor" => StorylineImportance::Minor,
                        _ => StorylineImportance::Normal,
                    },
                    created_volume_id: row.get::<_, Option<String>>(6)?.and_then(|s| Uuid::parse_str(&s).ok()),
                    resolved_volume_id: row.get::<_, Option<String>>(7)?.and_then(|s| Uuid::parse_str(&s).ok()),
                    created_at: get_timestamp(row, 8),
                    updated_at: get_timestamp(row, 9),
                })
            },
        ).ok();
        Ok(result)
    }

    /// 列出项目中的所有剧情线
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Storyline>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, status, importance, created_volume_id, resolved_volume_id, created_at, updated_at FROM storyline WHERE project_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(Storyline {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                name: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?,
                status: match row.get::<_, String>(4)?.as_str() {
                    "Planned" => StorylineStatus::Planned,
                    "Active" => StorylineStatus::Active,
                    "Resolved" => StorylineStatus::Resolved,
                    "Abandoned" => StorylineStatus::Abandoned,
                    _ => StorylineStatus::Active,
                },
                importance: match row.get::<_, String>(5)?.as_str() {
                    "Main" => StorylineImportance::Main,
                    "Important" => StorylineImportance::Important,
                    "Normal" => StorylineImportance::Normal,
                    "Minor" => StorylineImportance::Minor,
                    _ => StorylineImportance::Normal,
                },
                created_volume_id: row.get::<_, Option<String>>(6)?.and_then(|s| Uuid::parse_str(&s).ok()),
                resolved_volume_id: row.get::<_, Option<String>>(7)?.and_then(|s| Uuid::parse_str(&s).ok()),
                created_at: get_timestamp(row, 8),
                updated_at: get_timestamp(row, 9),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 关联剧情线和场景
    pub fn link_scene(&self, storyline_id: Uuid, scene_id: Uuid, significance: Option<&str>) -> Result<StorylineScene> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO storyline_scene (id, storyline_id, scene_id, significance, created_at) VALUES (?, ?, ?, ?, ?)",
            [id.to_string(), storyline_id.to_string(), scene_id.to_string(), significance.unwrap_or("").to_string(), now.to_string()],
        ).context("Failed to link storyline scene")?;
        Ok(StorylineScene { id, storyline_id, scene_id, significance: significance.map(|s| s.to_string()), created_at: now })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> (Database, Uuid) {
        let db = Database::open_in_memory().unwrap();
        migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        // 先创建 project
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
    fn test_create_storyline() {
        let (db, project_id) = setup_db();
        let repo = StorylineRepo::new(&db);
        let storyline = repo.create(project_id, "地下遗迹真相", Some("贯穿多卷的核心悬念"), StorylineImportance::Main).unwrap();
        assert_eq!(storyline.name, "地下遗迹真相");
        assert_eq!(storyline.status, StorylineStatus::Active);
    }

    #[test]
    fn test_list_and_link() {
        let (db, project_id) = setup_db();
        let repo = StorylineRepo::new(&db);
        let s1 = repo.create(project_id, "王家线", None, StorylineImportance::Important).unwrap();
        let _s2 = repo.create(project_id, "感情线", None, StorylineImportance::Normal).unwrap();
        let list = repo.list_by_project(project_id).unwrap();
        assert_eq!(list.len(), 2);
        let scene_id = Uuid::new_v4();
        let link = repo.link_scene(s1.id, scene_id, Some("伏笔引入")).unwrap();
        assert_eq!(link.storyline_id, s1.id);
    }
}
