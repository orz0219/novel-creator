//! Character Profile/State/Goal/Trait Repository

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

// ============= CharacterProfileRepo =============

pub struct CharacterProfileRepo {
    pool: PgPool,
}

impl CharacterProfileRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, entity_id: Uuid, profile: &CharacterProfile) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_profile (id, entity_id, real_name, nickname, age, gender, identity, appearance, background, social_status, core_personality, values_desc, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(profile.id)
        .bind(entity_id)
        .bind(profile.real_name.as_deref().unwrap_or(""))
        .bind(profile.nickname.as_deref().unwrap_or(""))
        .bind(profile.age.as_deref().unwrap_or(""))
        .bind(profile.gender.as_deref().unwrap_or(""))
        .bind(profile.identity.as_deref().unwrap_or(""))
        .bind(profile.appearance.as_deref().unwrap_or(""))
        .bind(profile.background.as_deref().unwrap_or(""))
        .bind(profile.social_status.as_deref().unwrap_or(""))
        .bind(profile.core_personality.as_deref().unwrap_or(""))
        .bind(profile.values.as_deref().unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character_profile")?;
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterProfile>> {
        let row = sqlx::query_as::<_, CharacterProfileRow>(
            "SELECT id, entity_id, real_name, nickname, age, gender, identity, appearance, background, social_status, core_personality, values_desc, created_at, updated_at \
             FROM character_profile WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_profile")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn update(&self, profile: &CharacterProfile) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE character_profile SET real_name=$1, nickname=$2, age=$3, gender=$4, identity=$5, appearance=$6, background=$7, social_status=$8, core_personality=$9, values_desc=$10, updated_at=$11 WHERE entity_id=$12",
        )
        .bind(profile.real_name.as_deref().unwrap_or(""))
        .bind(profile.nickname.as_deref().unwrap_or(""))
        .bind(profile.age.as_deref().unwrap_or(""))
        .bind(profile.gender.as_deref().unwrap_or(""))
        .bind(profile.identity.as_deref().unwrap_or(""))
        .bind(profile.appearance.as_deref().unwrap_or(""))
        .bind(profile.background.as_deref().unwrap_or(""))
        .bind(profile.social_status.as_deref().unwrap_or(""))
        .bind(profile.core_personality.as_deref().unwrap_or(""))
        .bind(profile.values.as_deref().unwrap_or(""))
        .bind(now)
        .bind(profile.entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_profile")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct CharacterProfileRow {
    id: Uuid,
    entity_id: Uuid,
    real_name: Option<String>,
    nickname: Option<String>,
    age: Option<String>,
    gender: Option<String>,
    identity: Option<String>,
    appearance: Option<String>,
    background: Option<String>,
    social_status: Option<String>,
    core_personality: Option<String>,
    values_desc: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterProfileRow> for CharacterProfile {
    fn from(r: CharacterProfileRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
        CharacterProfile {
            id: r.id,
            entity_id: r.entity_id,
            real_name: filter_empty(r.real_name),
            nickname: filter_empty(r.nickname),
            age: filter_empty(r.age),
            gender: filter_empty(r.gender),
            identity: filter_empty(r.identity),
            appearance: filter_empty(r.appearance),
            background: filter_empty(r.background),
            social_status: filter_empty(r.social_status),
            core_personality: filter_empty(r.core_personality),
            values: filter_empty(r.values_desc),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterStateRepo =============

pub struct CharacterStateRepo {
    pool: PgPool,
}

impl CharacterStateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, entity_id: Uuid, state: &CharacterState) -> Result<()> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE character_state SET location=$1, health=$2, cultivation=$3, money=$4, wanted=$5, extra=$6, updated_at=$7 WHERE entity_id=$8",
        )
        .bind(state.location.as_deref().unwrap_or(""))
        .bind(state.health.as_deref().unwrap_or(""))
        .bind(state.cultivation.as_deref().unwrap_or(""))
        .bind(state.money.as_deref().unwrap_or(""))
        .bind(state.wanted)
        .bind(&state.extra)
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_state")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_state (id, entity_id, location, health, cultivation, money, wanted, extra, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(state.location.as_deref().unwrap_or(""))
            .bind(state.health.as_deref().unwrap_or(""))
            .bind(state.cultivation.as_deref().unwrap_or(""))
            .bind(state.money.as_deref().unwrap_or(""))
            .bind(state.wanted)
            .bind(&state.extra)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert character_state")?;
        }
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterState>> {
        let row = sqlx::query_as::<_, CharacterStateRow>(
            "SELECT id, entity_id, location, health, cultivation, money, wanted, extra, created_at, updated_at \
             FROM character_state WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_state")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct CharacterStateRow {
    id: Uuid,
    entity_id: Uuid,
    location: Option<String>,
    health: Option<String>,
    cultivation: Option<String>,
    money: Option<String>,
    wanted: bool,
    extra: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterStateRow> for CharacterState {
    fn from(r: CharacterStateRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
        CharacterState {
            id: r.id,
            entity_id: r.entity_id,
            location: filter_empty(r.location),
            health: filter_empty(r.health),
            cultivation: filter_empty(r.cultivation),
            money: filter_empty(r.money),
            wanted: r.wanted,
            extra: r.extra.unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterGoalRepo =============

pub struct CharacterGoalRepo {
    pool: PgPool,
}

impl CharacterGoalRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, entity_id: Uuid, goal: &CharacterGoal) -> Result<()> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE character_goal SET long_term=$1, current_goal=$2, immediate=$3, updated_at=$4 WHERE entity_id=$5",
        )
        .bind(goal.long_term.as_deref().unwrap_or(""))
        .bind(goal.current.as_deref().unwrap_or(""))
        .bind(goal.immediate.as_deref().unwrap_or(""))
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_goal")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_goal (id, entity_id, long_term, current_goal, immediate, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(goal.long_term.as_deref().unwrap_or(""))
            .bind(goal.current.as_deref().unwrap_or(""))
            .bind(goal.immediate.as_deref().unwrap_or(""))
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert character_goal")?;
        }
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterGoal>> {
        let row = sqlx::query_as::<_, CharacterGoalRow>(
            "SELECT id, entity_id, long_term, current_goal, immediate, created_at, updated_at \
             FROM character_goal WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_goal")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct CharacterGoalRow {
    id: Uuid,
    entity_id: Uuid,
    long_term: Option<String>,
    current_goal: Option<String>,
    immediate: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterGoalRow> for CharacterGoal {
    fn from(r: CharacterGoalRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
        CharacterGoal {
            id: r.id,
            entity_id: r.entity_id,
            long_term: filter_empty(r.long_term),
            current: filter_empty(r.current_goal),
            immediate: filter_empty(r.immediate),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterTraitRepo =============

pub struct CharacterTraitRepo {
    pool: PgPool,
}

impl CharacterTraitRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, entity_id: Uuid, trait_item: &CharacterTrait) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_trait (id, entity_id, trait_type, name, description, parent_trait_id, intensity, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(trait_item.id)
        .bind(entity_id)
        .bind(serde_json::to_value(&trait_item.trait_type).unwrap_or_default())
        .bind(&trait_item.name)
        .bind(trait_item.description.as_deref().unwrap_or(""))
        .bind(trait_item.parent_trait_id)
        .bind(trait_item.intensity.unwrap_or(5))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character_trait")?;
        Ok(())
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<CharacterTrait>> {
        let rows = sqlx::query_as::<_, CharacterTraitRow>(
            "SELECT id, entity_id, trait_type, name, description, parent_trait_id, intensity, created_at, updated_at \
             FROM character_trait WHERE entity_id = $1 ORDER BY created_at",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query character traits")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct CharacterTraitRow {
    id: Uuid,
    entity_id: Uuid,
    trait_type: Option<serde_json::Value>,
    name: String,
    description: Option<String>,
    parent_trait_id: Option<Uuid>,
    intensity: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterTraitRow> for CharacterTrait {
    fn from(r: CharacterTraitRow) -> Self {
        CharacterTrait {
            id: r.id,
            entity_id: r.entity_id,
            trait_type: r.trait_type
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or(TraitType::Personality),
            name: r.name,
            description: r.description.filter(|s| !s.is_empty()),
            parent_trait_id: r.parent_trait_id,
            intensity: r.intensity,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
