//! Character Mind Repos - Belief, Memory, EmotionState, Goal, Fear CRUD

use anyhow::{Context, Result};
use chrono::Utc;
use domain::character_mind::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct BeliefRepo<'a> {
    db: &'a Database,
}

impl<'a> BeliefRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, character_id: Uuid, belief_content: &str, confidence: f64, source: Option<&str>) -> Result<Belief> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO belief (id, project_id, character_id, belief_content, confidence, source, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), character_id.to_string(), belief_content.to_string(), confidence.to_string(), source.unwrap_or("").to_string(), true.to_string(), now.to_rfc3339(), now.to_rfc3339()],
        ).context("Failed to insert belief")?;
        Ok(Belief { id, project_id, character_id, belief_content: belief_content.to_string(), confidence, source: source.map(|s| s.to_string()), source_scene_id: None, is_active: true, created_at: now, updated_at: now })
    }

    pub fn list_by_character(&self, character_id: Uuid) -> Result<Vec<Belief>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, character_id, belief_content, confidence, source, source_scene_id, is_active, created_at, updated_at FROM belief WHERE character_id = ? AND is_active = 1").context("Failed to prepare")?;
        let rows = stmt.query_map([character_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let character_id: String = row.get(2)?;
            let belief_content: String = row.get(3)?;
            let confidence: f64 = row.get(4)?;
            let source: Option<String> = row.get(5)?;
            let source_scene_id: Option<String> = row.get(6)?;
            let is_active: bool = row.get(7)?;
            Ok(Belief {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                character_id: Uuid::parse_str(&character_id).unwrap(),
                belief_content,
                confidence,
                source,
                source_scene_id: source_scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                is_active,
                created_at: get_timestamp(row, 8),
                updated_at: get_timestamp(row, 9),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct MemoryRepo<'a> {
    db: &'a Database,
}

impl<'a> MemoryRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, character_id: Uuid, memory_content: &str, emotional_impact: Option<&str>, importance: i32) -> Result<Memory> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO character_memory (id, project_id, character_id, memory_content, emotional_impact, importance, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), character_id.to_string(), memory_content.to_string(), emotional_impact.unwrap_or("").to_string(), importance.to_string(), true.to_string(), now.to_rfc3339(), now.to_rfc3339()],
        ).context("Failed to insert memory")?;
        Ok(Memory { id, project_id, character_id, memory_content: memory_content.to_string(), emotional_impact: emotional_impact.map(|s| s.to_string()), scene_id: None, importance, is_active: true, created_at: now, updated_at: now })
    }

    pub fn list_by_character(&self, character_id: Uuid) -> Result<Vec<Memory>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, character_id, memory_content, emotional_impact, scene_id, importance, is_active, created_at, updated_at FROM character_memory WHERE character_id = ? AND is_active = 1 ORDER BY importance DESC").context("Failed to prepare")?;
        let rows = stmt.query_map([character_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let character_id: String = row.get(2)?;
            let memory_content: String = row.get(3)?;
            let emotional_impact: Option<String> = row.get(4)?;
            let scene_id: Option<String> = row.get(5)?;
            let importance: i32 = row.get(6)?;
            let is_active: bool = row.get(7)?;
            Ok(Memory {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                character_id: Uuid::parse_str(&character_id).unwrap(),
                memory_content,
                emotional_impact,
                scene_id: scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                importance,
                is_active,
                created_at: get_timestamp(row, 8),
                updated_at: get_timestamp(row, 9),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct EmotionRepo<'a> {
    db: &'a Database,
}

impl<'a> EmotionRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, character_id: Uuid, emotion_type: &str, intensity: i32, decay_rate: f64, trigger_description: Option<&str>) -> Result<EmotionState> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO emotion_state (id, project_id, character_id, emotion_type, intensity, decay_rate, trigger_description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), character_id.to_string(), emotion_type.to_string(), intensity.to_string(), decay_rate.to_string(), trigger_description.unwrap_or("").to_string(), now.to_rfc3339(), now.to_rfc3339()],
        ).context("Failed to insert emotion")?;
        Ok(EmotionState { id, project_id, character_id, emotion_type: emotion_type.to_string(), intensity, decay_rate, trigger_scene_id: None, trigger_description: trigger_description.map(|s| s.to_string()), created_at: now, updated_at: now })
    }

    /// 更新情绪强度（考虑衰减）
    pub fn update_intensity(&self, id: Uuid, new_intensity: i32) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE emotion_state SET intensity = ?, updated_at = ? WHERE id = ?",
            [new_intensity.to_string(), Utc::now().to_rfc3339(), id.to_string()],
        ).context("Failed to update emotion intensity")?;
        Ok(())
    }

    /// 应用衰减：每个 Scene 结束后调用
    pub fn apply_decay(&self, project_id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE emotion_state SET intensity = CASE WHEN intensity - CAST(intensity * decay_rate AS INTEGER) < 0 THEN 0 ELSE intensity - CAST(intensity * decay_rate AS INTEGER) END, updated_at = ? WHERE project_id = ? AND intensity > 0",
            [Utc::now().to_rfc3339(), project_id.to_string()],
        ).context("Failed to apply emotion decay")?;
        Ok(())
    }

    pub fn list_by_character(&self, character_id: Uuid) -> Result<Vec<EmotionState>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, character_id, emotion_type, intensity, decay_rate, trigger_scene_id, trigger_description, created_at, updated_at FROM emotion_state WHERE character_id = ? AND intensity > 0 ORDER BY intensity DESC").context("Failed to prepare")?;
        let rows = stmt.query_map([character_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let character_id: String = row.get(2)?;
            let emotion_type: String = row.get(3)?;
            let intensity: i32 = row.get(4)?;
            let decay_rate: f64 = row.get(5)?;
            let trigger_scene_id: Option<String> = row.get(6)?;
            let trigger_description: Option<String> = row.get(7)?;
            Ok(EmotionState {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                character_id: Uuid::parse_str(&character_id).unwrap(),
                emotion_type,
                intensity,
                decay_rate,
                trigger_scene_id: trigger_scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                trigger_description,
                created_at: get_timestamp(row, 8),
                updated_at: get_timestamp(row, 9),
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
    fn test_belief_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = BeliefRepo::new(&db);

        // 先创建 project
        let project_id = Uuid::new_v4();
        let character_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test Project".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let belief = repo.create(project_id, character_id, "王家家主是一个谨慎的人", 0.8, Some("personal_observation")).unwrap();
        assert_eq!(belief.confidence, 0.8);

        let beliefs = repo.list_by_character(character_id).unwrap();
        assert_eq!(beliefs.len(), 1);
    }

    #[test]
    fn test_emotion_decay() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = EmotionRepo::new(&db);

        // 先创建 project
        let project_id = Uuid::new_v4();
        let character_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test Project".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let emotion = repo.create(project_id, character_id, "fear", 80, 0.2, Some("被王家追杀")).unwrap();
        assert_eq!(emotion.intensity, 80);

        // 应用衰减
        repo.apply_decay(project_id).unwrap();

        let emotions = repo.list_by_character(character_id).unwrap();
        assert_eq!(emotions.len(), 1);
        assert!(emotions[0].intensity < 80); // 衰减后强度降低
    }
}
