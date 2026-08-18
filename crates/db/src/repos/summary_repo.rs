//! Summary Repos - Chapter/Arc/Volume Summary + Global Story State

use anyhow::{Context, Result};
use chrono::Utc;
use domain::state_mgmt::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct ChapterSummaryRepo<'a> {
    db: &'a Database,
}

impl<'a> ChapterSummaryRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, chapter_id: Uuid, summary: &str, key_events: Vec<String>, involved_characters: Vec<Uuid>) -> Result<ChapterSummary> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO chapter_summary (id, project_id, chapter_id, summary, key_events, involved_characters, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), chapter_id.to_string(), summary.to_string(), serde_json::to_string(&key_events).unwrap_or_default(), serde_json::to_string(&involved_characters.iter().map(|u| u.to_string()).collect::<Vec<_>>()).unwrap_or_default(), now.to_rfc3339()],
        ).context("Failed to insert chapter_summary")?;
        Ok(ChapterSummary { id, project_id, chapter_id, summary: summary.to_string(), key_events, involved_characters, created_at: now })
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<ChapterSummary>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, chapter_id, summary, key_events, involved_characters, created_at FROM chapter_summary WHERE project_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let chapter_id: String = row.get(2)?;
            let summary: String = row.get(3)?;
            let key_events_str: String = row.get(4)?;
            Ok(ChapterSummary {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                chapter_id: Uuid::parse_str(&chapter_id).unwrap(),
                summary,
                key_events: serde_json::from_str(&key_events_str).unwrap_or_default(),
                involved_characters: Vec::new(),
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct ArcSummaryRepo<'a> {
    db: &'a Database,
}

impl<'a> ArcSummaryRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, arc_id: Uuid, summary: &str, key_turning_points: Vec<String>, status: &str) -> Result<ArcSummary> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO arc_summary (id, project_id, arc_id, summary, key_turning_points, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), arc_id.to_string(), summary.to_string(), serde_json::to_string(&key_turning_points).unwrap_or_default(), status.to_string(), now.to_rfc3339()],
        ).context("Failed to insert arc_summary")?;
        Ok(ArcSummary { id, project_id, arc_id, summary: summary.to_string(), key_turning_points, status: status.to_string(), created_at: now })
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<ArcSummary>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, arc_id, summary, key_turning_points, status, created_at FROM arc_summary WHERE project_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let arc_id: String = row.get(2)?;
            let summary: String = row.get(3)?;
            let key_turning_points_str: String = row.get(4)?;
            let status: String = row.get(5)?;
            Ok(ArcSummary {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                arc_id: Uuid::parse_str(&arc_id).unwrap(),
                summary,
                key_turning_points: serde_json::from_str(&key_turning_points_str).unwrap_or_default(),
                status,
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct VolumeSummaryRepo<'a> {
    db: &'a Database,
}

impl<'a> VolumeSummaryRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, volume_id: Uuid, summary: &str, character_changes: Vec<String>, world_changes: Vec<String>, foreshadowing_progress: Vec<String>) -> Result<VolumeSummary> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO volume_summary (id, project_id, volume_id, summary, character_changes, world_changes, foreshadowing_progress, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), volume_id.to_string(), summary.to_string(), serde_json::to_string(&character_changes).unwrap_or_default(), serde_json::to_string(&world_changes).unwrap_or_default(), serde_json::to_string(&foreshadowing_progress).unwrap_or_default(), now.to_rfc3339()],
        ).context("Failed to insert volume_summary")?;
        Ok(VolumeSummary { id, project_id, volume_id, summary: summary.to_string(), character_changes, world_changes, foreshadowing_progress, created_at: now })
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<VolumeSummary>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, volume_id, summary, character_changes, world_changes, foreshadowing_progress, created_at FROM volume_summary WHERE project_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let volume_id: String = row.get(2)?;
            let summary: String = row.get(3)?;
            let character_changes_str: String = row.get(4)?;
            let world_changes_str: String = row.get(5)?;
            let foreshadowing_progress_str: String = row.get(6)?;
            Ok(VolumeSummary {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                volume_id: Uuid::parse_str(&volume_id).unwrap(),
                summary,
                character_changes: serde_json::from_str(&character_changes_str).unwrap_or_default(),
                world_changes: serde_json::from_str(&world_changes_str).unwrap_or_default(),
                foreshadowing_progress: serde_json::from_str(&foreshadowing_progress_str).unwrap_or_default(),
                created_at: get_timestamp(row, 7),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct GlobalStoryStateRepo<'a> {
    db: &'a Database,
}

impl<'a> GlobalStoryStateRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn upsert(&self, project_id: Uuid, current_progress: &str, open_foreshadowing: Vec<String>, open_storylines: Vec<String>, world_state_summary: &str, character_state_summary: &str) -> Result<GlobalStoryState> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        // Delete existing
        conn.execute("DELETE FROM global_story_state WHERE project_id = ?", [project_id.to_string()]).ok();
        conn.execute(
            "INSERT INTO global_story_state (id, project_id, current_progress, open_foreshadowing, open_storylines, world_state_summary, character_state_summary, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), current_progress.to_string(), serde_json::to_string(&open_foreshadowing).unwrap_or_default(), serde_json::to_string(&open_storylines).unwrap_or_default(), world_state_summary.to_string(), character_state_summary.to_string(), now.to_rfc3339()],
        ).context("Failed to insert global_story_state")?;
        Ok(GlobalStoryState { id, project_id, current_progress: current_progress.to_string(), open_foreshadowing, open_storylines, world_state_summary: world_state_summary.to_string(), character_state_summary: character_state_summary.to_string(), updated_at: now })
    }

    pub fn get(&self, project_id: Uuid) -> Result<Option<GlobalStoryState>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, current_progress, open_foreshadowing, open_storylines, world_state_summary, character_state_summary, updated_at FROM global_story_state WHERE project_id = ?",
            [project_id.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let project_id: String = row.get(1)?;
                let current_progress: String = row.get(2)?;
                let open_foreshadowing_str: String = row.get(3)?;
                let open_storylines_str: String = row.get(4)?;
                let world_state_summary: String = row.get(5)?;
                let character_state_summary: String = row.get(6)?;
                Ok(GlobalStoryState {
                    id: Uuid::parse_str(&id).unwrap(),
                    project_id: Uuid::parse_str(&project_id).unwrap(),
                    current_progress,
                    open_foreshadowing: serde_json::from_str(&open_foreshadowing_str).unwrap_or_default(),
                    open_storylines: serde_json::from_str(&open_storylines_str).unwrap_or_default(),
                    world_state_summary,
                    character_state_summary,
                    updated_at: get_timestamp(row, 7),
                })
            },
        ).ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;

    #[test]
    fn test_multi_level_memory() {
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

        // Chapter Summary
        let ch_repo = ChapterSummaryRepo::new(&db);
        let ch = ch_repo.create(project_id, Uuid::new_v4(), "林凡进入黑市，获得通行证", vec!["进入黑市".to_string()], vec![]).unwrap();
        assert_eq!(ch.summary, "林凡进入黑市，获得通行证");

        // Arc Summary
        let arc_repo = ArcSummaryRepo::new(&db);
        let arc = arc_repo.create(project_id, Uuid::new_v4(), "黑石城 Arc：林凡在黑石城的成长", vec!["发现黑市".to_string(), "获得通行证".to_string()], "Active").unwrap();
        assert_eq!(arc.status, "Active");

        // Volume Summary
        let vol_repo = VolumeSummaryRepo::new(&db);
        let vol = vol_repo.create(project_id, Uuid::new_v4(), "第一卷：踏上修炼之路", vec!["林凡从普通人变为修士".to_string()], vec!["发现地下遗迹".to_string()], vec!["古井伏笔引入".to_string()]).unwrap();
        assert_eq!(vol.summary, "第一卷：踏上修炼之路");

        // Global Story State
        let global_repo = GlobalStoryStateRepo::new(&db);
        let global = global_repo.upsert(project_id, "第一卷进行中", vec!["古井伏笔".to_string()], vec!["地下遗迹线".to_string()], "林凡已进入黑市", "林凡当前恐惧80").unwrap();
        assert_eq!(global.current_progress, "第一卷进行中");

        // 验证 Global Story State upsert（覆盖旧数据）
        let global2 = global_repo.upsert(project_id, "第一卷已完成", vec![], vec![], "林凡已成为修士", "林凡当前平静").unwrap();
        assert_eq!(global2.current_progress, "第一卷已完成");
    }
}
