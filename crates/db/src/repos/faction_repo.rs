//! FactionProfile Repository

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::FactionProfile;
use sqlx::PgPool;
use uuid::Uuid;

pub struct FactionProfileRepo {
    pool: PgPool,
}

impl FactionProfileRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, entity_id: Uuid, profile: &FactionProfile) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO faction_profile (id, entity_id, goals, leader, values_desc, resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(profile.id)
        .bind(entity_id)
        .bind(profile.goals.as_deref().unwrap_or(""))
        .bind(profile.leader.as_deref().unwrap_or(""))
        .bind(profile.values.as_deref().unwrap_or(""))
        .bind(profile.resources.as_deref().unwrap_or(""))
        .bind(profile.territory.as_deref().unwrap_or(""))
        .bind(profile.members.as_deref().unwrap_or(""))
        .bind(profile.enemies.as_deref().unwrap_or(""))
        .bind(profile.allies.as_deref().unwrap_or(""))
        .bind(profile.internal_conflicts.as_deref().unwrap_or(""))
        .bind(profile.secrets.as_deref().unwrap_or(""))
        .bind(profile.modus_operandi.as_deref().unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create faction_profile")?;
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<FactionProfile>> {
        let row = sqlx::query_as::<_, FactionProfileRow>(
            "SELECT id, entity_id, goals, leader, values_desc, resources, territory, members, enemies, allies, internal_conflicts, secrets, modus_operandi, created_at, updated_at \
             FROM faction_profile WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query faction_profile")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn update(&self, profile: &FactionProfile) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE faction_profile SET goals=$1, leader=$2, values_desc=$3, resources=$4, territory=$5, members=$6, enemies=$7, allies=$8, internal_conflicts=$9, secrets=$10, modus_operandi=$11, updated_at=$12 WHERE entity_id=$13",
        )
        .bind(profile.goals.as_deref().unwrap_or(""))
        .bind(profile.leader.as_deref().unwrap_or(""))
        .bind(profile.values.as_deref().unwrap_or(""))
        .bind(profile.resources.as_deref().unwrap_or(""))
        .bind(profile.territory.as_deref().unwrap_or(""))
        .bind(profile.members.as_deref().unwrap_or(""))
        .bind(profile.enemies.as_deref().unwrap_or(""))
        .bind(profile.allies.as_deref().unwrap_or(""))
        .bind(profile.internal_conflicts.as_deref().unwrap_or(""))
        .bind(profile.secrets.as_deref().unwrap_or(""))
        .bind(profile.modus_operandi.as_deref().unwrap_or(""))
        .bind(now)
        .bind(profile.entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update faction_profile")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct FactionProfileRow {
    id: Uuid,
    entity_id: Uuid,
    goals: Option<String>,
    leader: Option<String>,
    values_desc: Option<String>,
    resources: Option<String>,
    territory: Option<String>,
    members: Option<String>,
    enemies: Option<String>,
    allies: Option<String>,
    internal_conflicts: Option<String>,
    secrets: Option<String>,
    modus_operandi: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FactionProfileRow> for FactionProfile {
    fn from(r: FactionProfileRow) -> Self {
        FactionProfile {
            id: r.id,
            entity_id: r.entity_id,
            goals: r.goals.filter(|s| !s.is_empty()),
            leader: r.leader.filter(|s| !s.is_empty()),
            values: r.values_desc.filter(|s| !s.is_empty()),
            resources: r.resources.filter(|s| !s.is_empty()),
            territory: r.territory.filter(|s| !s.is_empty()),
            members: r.members.filter(|s| !s.is_empty()),
            enemies: r.enemies.filter(|s| !s.is_empty()),
            allies: r.allies.filter(|s| !s.is_empty()),
            internal_conflicts: r.internal_conflicts.filter(|s| !s.is_empty()),
            secrets: r.secrets.filter(|s| !s.is_empty()),
            modus_operandi: r.modus_operandi.filter(|s| !s.is_empty()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
