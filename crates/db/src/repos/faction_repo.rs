//! FactionProfile Repository

use anyhow::{Context, Result};
use chrono::Utc;
use domain::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils;

pub struct FactionProfileRepo<'a> {
    db: &'a Database,
}

impl<'a> FactionProfileRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, entity_id: Uuid, profile: &FactionProfile) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO faction_profile (id, entity_id, goals, leader, values_desc, resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                profile.id.to_string(), entity_id.to_string(),
                profile.goals.clone().unwrap_or_default(),
                profile.leader.clone().unwrap_or_default(),
                profile.values.clone().unwrap_or_default(),
                profile.resources.clone().unwrap_or_default(),
                profile.territory.clone().unwrap_or_default(),
                profile.members.clone().unwrap_or_default(),
                profile.enemies.clone().unwrap_or_default(),
                profile.allies.clone().unwrap_or_default(),
                profile.internal_conflicts.clone().unwrap_or_default(),
                profile.secrets.clone().unwrap_or_default(),
                profile.modus_operandi.clone().unwrap_or_default(),
                now.to_string(), now.to_string(),
            ],
        ).context("Failed to create faction_profile")?;
        Ok(())
    }

    pub fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<FactionProfile>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, entity_id, goals, leader, values_desc, resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi, created_at, updated_at FROM faction_profile WHERE entity_id = ?",
            [entity_id.to_string()],
            |row| {
                Ok(FactionProfile {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    entity_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    goals: Some(row.get::<_, String>(2)?).filter(|s| !s.is_empty()),
                    leader: Some(row.get::<_, String>(3)?).filter(|s| !s.is_empty()),
                    values: Some(row.get::<_, String>(4)?).filter(|s| !s.is_empty()),
                    resources: Some(row.get::<_, String>(5)?).filter(|s| !s.is_empty()),
                    territory: Some(row.get::<_, String>(6)?).filter(|s| !s.is_empty()),
                    members: Some(row.get::<_, String>(7)?).filter(|s| !s.is_empty()),
                    enemies: Some(row.get::<_, String>(8)?).filter(|s| !s.is_empty()),
                    allies: Some(row.get::<_, String>(9)?).filter(|s| !s.is_empty()),
                    internal_conflicts: Some(row.get::<_, String>(10)?).filter(|s| !s.is_empty()),
                    secrets: Some(row.get::<_, String>(11)?).filter(|s| !s.is_empty()),
                    modus_operandi: Some(row.get::<_, String>(12)?).filter(|s| !s.is_empty()),
                    created_at: time_utils::get_timestamp(row, 13),
                    updated_at: time_utils::get_timestamp(row, 14),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn update(&self, profile: &FactionProfile) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        conn.execute(
            "UPDATE faction_profile SET goals=?, leader=?, values_desc=?, resources=?, territory=?, members=?, enemies=?, allies=?, internal_conflicts=?, secrets=?, modus_operandi=?, updated_at=? WHERE entity_id=?",
            [
                profile.goals.clone().unwrap_or_default(),
                profile.leader.clone().unwrap_or_default(),
                profile.values.clone().unwrap_or_default(),
                profile.resources.clone().unwrap_or_default(),
                profile.territory.clone().unwrap_or_default(),
                profile.members.clone().unwrap_or_default(),
                profile.enemies.clone().unwrap_or_default(),
                profile.allies.clone().unwrap_or_default(),
                profile.internal_conflicts.clone().unwrap_or_default(),
                profile.secrets.clone().unwrap_or_default(),
                profile.modus_operandi.clone().unwrap_or_default(),
                now.to_string(),
                profile.entity_id.to_string(),
            ],
        ).context("Failed to update faction_profile")?;
        Ok(())
    }
}
