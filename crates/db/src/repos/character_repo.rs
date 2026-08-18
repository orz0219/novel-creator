//! Character Profile/State/Goal/Trait Repository
//!
//! 对应 migration 007 创建的人物子结构表。

use anyhow::{Context, Result};
use chrono::Utc;
use domain::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils;

/// CharacterProfile Repository
pub struct CharacterProfileRepo<'a> {
    db: &'a Database,
}

impl<'a> CharacterProfileRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, entity_id: Uuid, profile: &CharacterProfile) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO character_profile (id, entity_id, real_name, nickname, age, gender, identity, appearance, background, social_status, core_personality, values_desc, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                profile.id.to_string(), entity_id.to_string(),
                profile.real_name.clone().unwrap_or_default(),
                profile.nickname.clone().unwrap_or_default(),
                profile.age.clone().unwrap_or_default(),
                profile.gender.clone().unwrap_or_default(),
                profile.identity.clone().unwrap_or_default(),
                profile.appearance.clone().unwrap_or_default(),
                profile.background.clone().unwrap_or_default(),
                profile.social_status.clone().unwrap_or_default(),
                profile.core_personality.clone().unwrap_or_default(),
                profile.values.clone().unwrap_or_default(),
                now.to_string(), now.to_string(),
            ],
        ).context("Failed to create character_profile")?;
        Ok(())
    }

    pub fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterProfile>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, entity_id, real_name, nickname, age, gender, identity, appearance, background, social_status, core_personality, values_desc, created_at, updated_at FROM character_profile WHERE entity_id = ?",
            [entity_id.to_string()],
            |row| {
                Ok(CharacterProfile {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    entity_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    real_name: Some(row.get::<_, String>(2)?).filter(|s| !s.is_empty()),
                    nickname: Some(row.get::<_, String>(3)?).filter(|s| !s.is_empty()),
                    age: Some(row.get::<_, String>(4)?).filter(|s| !s.is_empty()),
                    gender: Some(row.get::<_, String>(5)?).filter(|s| !s.is_empty()),
                    identity: Some(row.get::<_, String>(6)?).filter(|s| !s.is_empty()),
                    appearance: Some(row.get::<_, String>(7)?).filter(|s| !s.is_empty()),
                    background: Some(row.get::<_, String>(8)?).filter(|s| !s.is_empty()),
                    social_status: Some(row.get::<_, String>(9)?).filter(|s| !s.is_empty()),
                    core_personality: Some(row.get::<_, String>(10)?).filter(|s| !s.is_empty()),
                    values: Some(row.get::<_, String>(11)?).filter(|s| !s.is_empty()),
                    created_at: time_utils::get_timestamp(row, 12),
                    updated_at: time_utils::get_timestamp(row, 13),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn update(&self, profile: &CharacterProfile) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        conn.execute(
            "UPDATE character_profile SET real_name=?, nickname=?, age=?, gender=?, identity=?, appearance=?, background=?, social_status=?, core_personality=?, values_desc=?, updated_at=? WHERE entity_id=?",
            [
                profile.real_name.clone().unwrap_or_default(),
                profile.nickname.clone().unwrap_or_default(),
                profile.age.clone().unwrap_or_default(),
                profile.gender.clone().unwrap_or_default(),
                profile.identity.clone().unwrap_or_default(),
                profile.appearance.clone().unwrap_or_default(),
                profile.background.clone().unwrap_or_default(),
                profile.social_status.clone().unwrap_or_default(),
                profile.core_personality.clone().unwrap_or_default(),
                profile.values.clone().unwrap_or_default(),
                now.to_string(),
                profile.entity_id.to_string(),
            ],
        ).context("Failed to update character_profile")?;
        Ok(())
    }
}

/// CharacterState Repository
pub struct CharacterStateRepo<'a> {
    db: &'a Database,
}

impl<'a> CharacterStateRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, entity_id: Uuid, state: &CharacterState) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        // Try update first, if no rows affected, insert
        let updated = conn.execute(
            "UPDATE character_state SET location=?, health=?, cultivation=?, money=?, wanted=?, extra=?, updated_at=? WHERE entity_id=?",
            [
                state.location.clone().unwrap_or_default(),
                state.health.clone().unwrap_or_default(),
                state.cultivation.clone().unwrap_or_default(),
                state.money.clone().unwrap_or_default(),
                state.wanted.to_string(),
                state.extra.to_string(),
                now.to_string(),
                entity_id.to_string(),
            ],
        ).context("Failed to update character_state")?;

        if updated == 0 {
            conn.execute(
                "INSERT INTO character_state (id, entity_id, location, health, cultivation, money, wanted, extra, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    Uuid::new_v4().to_string(), entity_id.to_string(),
                    state.location.clone().unwrap_or_default(),
                    state.health.clone().unwrap_or_default(),
                    state.cultivation.clone().unwrap_or_default(),
                    state.money.clone().unwrap_or_default(),
                    state.wanted.to_string(),
                    state.extra.to_string(),
                    now.to_string(), now.to_string(),
                ],
            ).context("Failed to insert character_state")?;
        }
        Ok(())
    }

    pub fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterState>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, entity_id, location, health, cultivation, money, wanted, extra, created_at, updated_at FROM character_state WHERE entity_id = ?",
            [entity_id.to_string()],
            |row| {
                let extra_str: String = row.get(7)?;
                Ok(CharacterState {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    entity_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    location: Some(row.get::<_, String>(2)?).filter(|s| !s.is_empty()),
                    health: Some(row.get::<_, String>(3)?).filter(|s| !s.is_empty()),
                    cultivation: Some(row.get::<_, String>(4)?).filter(|s| !s.is_empty()),
                    money: Some(row.get::<_, String>(5)?).filter(|s| !s.is_empty()),
                    wanted: row.get(6)?,
                    extra: serde_json::from_str(&extra_str).unwrap_or_default(),
                    created_at: time_utils::get_timestamp(row, 8),
                    updated_at: time_utils::get_timestamp(row, 9),
                })
            },
        ).ok();
        Ok(result)
    }
}

/// CharacterGoal Repository
pub struct CharacterGoalRepo<'a> {
    db: &'a Database,
}

impl<'a> CharacterGoalRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, entity_id: Uuid, goal: &CharacterGoal) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        let updated = conn.execute(
            "UPDATE character_goal SET long_term=?, current_goal=?, immediate=?, updated_at=? WHERE entity_id=?",
            [
                goal.long_term.clone().unwrap_or_default(),
                goal.current.clone().unwrap_or_default(),
                goal.immediate.clone().unwrap_or_default(),
                now.to_string(),
                entity_id.to_string(),
            ],
        ).context("Failed to update character_goal")?;

        if updated == 0 {
            conn.execute(
                "INSERT INTO character_goal (id, entity_id, long_term, current_goal, immediate, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                [
                    Uuid::new_v4().to_string(), entity_id.to_string(),
                    goal.long_term.clone().unwrap_or_default(),
                    goal.current.clone().unwrap_or_default(),
                    goal.immediate.clone().unwrap_or_default(),
                    now.to_string(), now.to_string(),
                ],
            ).context("Failed to insert character_goal")?;
        }
        Ok(())
    }

    pub fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterGoal>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, entity_id, long_term, current_goal, immediate, created_at, updated_at FROM character_goal WHERE entity_id = ?",
            [entity_id.to_string()],
            |row| {
                Ok(CharacterGoal {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    entity_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    long_term: Some(row.get::<_, String>(2)?).filter(|s| !s.is_empty()),
                    current: Some(row.get::<_, String>(3)?).filter(|s| !s.is_empty()),
                    immediate: Some(row.get::<_, String>(4)?).filter(|s| !s.is_empty()),
                    created_at: time_utils::get_timestamp(row, 5),
                    updated_at: time_utils::get_timestamp(row, 6),
                })
            },
        ).ok();
        Ok(result)
    }
}

/// CharacterTrait Repository
pub struct CharacterTraitRepo<'a> {
    db: &'a Database,
}

impl<'a> CharacterTraitRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, entity_id: Uuid, trait_item: &CharacterTrait) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO character_trait (id, entity_id, trait_type, name, description, parent_trait_id, intensity, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                trait_item.id.to_string(), entity_id.to_string(),
                serde_json::to_string(&trait_item.trait_type).unwrap_or_default().trim_matches('"').to_string(),
                trait_item.name.clone(),
                trait_item.description.clone().unwrap_or_default(),
                trait_item.parent_trait_id.map(|u| u.to_string()).unwrap_or_default(),
                trait_item.intensity.unwrap_or(5).to_string(),
                now.to_string(), now.to_string(),
            ],
        ).context("Failed to create character_trait")?;
        Ok(())
    }

    pub fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<CharacterTrait>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, entity_id, trait_type, name, description, parent_trait_id, intensity, created_at, updated_at FROM character_trait WHERE entity_id = ? ORDER BY created_at"
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([entity_id.to_string()], |row| {
            let trait_type_str: String = row.get(2)?;
            let parent_str: Option<String> = row.get::<_, Option<String>>(5)?;
            Ok(CharacterTrait {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                entity_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                trait_type: serde_json::from_str(&format!("\"{}\"", trait_type_str)).unwrap_or(TraitType::Personality),
                name: row.get(3)?,
                description: row.get::<_, Option<String>>(4)?.filter(|s| !s.is_empty()),
                parent_trait_id: parent_str.and_then(|s| Uuid::parse_str(&s).ok()),
                intensity: row.get::<_, Option<i32>>(6)?,
                created_at: time_utils::get_timestamp(row, 7),
                updated_at: time_utils::get_timestamp(row, 8),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
