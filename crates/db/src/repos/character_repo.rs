//! Character module repositories (R2 redesign).

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
            "INSERT INTO character_profile (id, entity_id, name, aliases, age_range, gender, identity, appearance, background_origin, social_position, core_personality, \"values\", role_in_story, narrative_necessity, extra, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
        )
        .bind(profile.id)
        .bind(entity_id)
        .bind(&profile.name)
        .bind(serde_json::to_value(&profile.aliases).unwrap_or(serde_json::Value::Null))
        .bind(profile.age.map(|a| a.as_str()))
        .bind(profile.gender.map(|g| g.as_str()))
        .bind(&profile.identity)
        .bind(&profile.appearance)
        .bind(&profile.background_origin)
        .bind(serde_json::to_value(&profile.social_position).unwrap_or(serde_json::Value::Null))
        .bind(&profile.core_personality)
        .bind(&profile.values)
        .bind(profile.role_in_story.map(|r| r.as_str()))
        .bind(serde_json::to_value(&profile.narrative_necessity).unwrap_or(serde_json::Value::Null))
        .bind(serde_json::Value::Null)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character_profile")?;
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterProfile>> {
        let row = sqlx::query_as::<_, CharacterProfileRow>(
            "SELECT id, entity_id, name, aliases, age_range, gender, identity, appearance, background_origin, social_position, core_personality, \"values\", role_in_story, narrative_necessity, extra, created_at, updated_at \
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
            "UPDATE character_profile SET name=$1, aliases=$2, age_range=$3, gender=$4, identity=$5, appearance=$6, background_origin=$7, social_position=$8, core_personality=$9, \"values\"=$10, role_in_story=$11, narrative_necessity=$12, extra=$13, updated_at=$14 WHERE entity_id=$15",
        )
        .bind(&profile.name)
        .bind(serde_json::to_value(&profile.aliases).unwrap_or(serde_json::Value::Null))
        .bind(profile.age.map(|a| a.as_str()))
        .bind(profile.gender.map(|g| g.as_str()))
        .bind(&profile.identity)
        .bind(&profile.appearance)
        .bind(&profile.background_origin)
        .bind(serde_json::to_value(&profile.social_position).unwrap_or(serde_json::Value::Null))
        .bind(&profile.core_personality)
        .bind(&profile.values)
        .bind(profile.role_in_story.map(|r| r.as_str()))
        .bind(serde_json::to_value(&profile.narrative_necessity).unwrap_or(serde_json::Value::Null))
        .bind(serde_json::Value::Null)
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
    name: Option<String>,
    aliases: Option<serde_json::Value>,
    age_range: Option<String>,
    gender: Option<String>,
    identity: Option<String>,
    appearance: Option<String>,
    background_origin: Option<String>,
    social_position: Option<serde_json::Value>,
    core_personality: Option<String>,
    values: Option<String>,
    role_in_story: Option<String>,
    narrative_necessity: Option<serde_json::Value>,
    extra: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterProfileRow> for CharacterProfile {
    fn from(r: CharacterProfileRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
        CharacterProfile {
            id: r.id,
            entity_id: r.entity_id,
            name: filter_empty(r.name),
            aliases: r
                .aliases
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            age: r.age_range.as_deref().map(AgeRange::from_str),
            gender: r.gender.as_deref().map(Gender::from_str),
            identity: filter_empty(r.identity),
            appearance: filter_empty(r.appearance),
            background_origin: filter_empty(r.background_origin),
            social_position: r
                .social_position
                .and_then(|v| serde_json::from_value(v).ok()),
            core_personality: filter_empty(r.core_personality),
            values: filter_empty(r.values),
            role_in_story: r.role_in_story.as_deref().map(StoryRole::from_str),
            narrative_necessity: r
                .narrative_necessity
                .and_then(|v| serde_json::from_value(v).ok()),
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
            "UPDATE character_state SET location=$1, physical_state=$2, mental_state=$3, resource_state=$4, social_state=$5, flags=$6, extra=$7, updated_at=$8 WHERE entity_id=$9",
        )
        .bind(&state.location)
        .bind(&state.physical_state)
        .bind(&state.mental_state)
        .bind(&state.resource_state)
        .bind(&state.social_state)
        .bind(serde_json::to_value(&state.flags).unwrap_or(serde_json::Value::Null))
        .bind(&state.extra)
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_state")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_state (id, entity_id, location, physical_state, mental_state, resource_state, social_state, flags, extra, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(&state.location)
            .bind(&state.physical_state)
            .bind(&state.mental_state)
            .bind(&state.resource_state)
            .bind(&state.social_state)
            .bind(&state.flags)
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
            "SELECT id, entity_id, location, physical_state, mental_state, resource_state, social_state, flags, extra, created_at, updated_at \
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
    physical_state: Option<String>,
    mental_state: Option<String>,
    resource_state: Option<String>,
    social_state: Option<String>,
    flags: Option<serde_json::Value>,
    extra: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterStateRow> for CharacterState {
    fn from(r: CharacterStateRow) -> Self {
        CharacterState {
            id: r.id,
            entity_id: r.entity_id,
            location: r.location.filter(|s| !s.is_empty()),
            physical_state: r.physical_state.filter(|s| !s.is_empty()),
            mental_state: r.mental_state.filter(|s| !s.is_empty()),
            resource_state: r.resource_state.filter(|s| !s.is_empty()),
            social_state: r.social_state.filter(|s| !s.is_empty()),
            flags: r
                .flags
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            extra: r.extra.unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterDriveRepo =============

pub struct CharacterDriveRepo {
    pool: PgPool,
}

impl CharacterDriveRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, entity_id: Uuid, drive: &CharacterDrive) -> Result<()> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE character_drive SET primary_goal=$1, motivation=$2, urgency=$3, long_term=$4, current_goal=$5, immediate=$6, hidden_goal=$7, fear=$8, weakness=$9, desire=$10, contradiction=$11, updated_at=$12 WHERE entity_id=$13",
        )
        .bind(&drive.primary_goal)
        .bind(&drive.motivation)
        .bind(drive.urgency)
        .bind(&drive.long_term)
        .bind(&drive.current)
        .bind(&drive.immediate)
        .bind(&drive.hidden_goal)
        .bind(&drive.fear)
        .bind(&drive.weakness)
        .bind(&drive.desire)
        .bind(&drive.contradiction)
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_drive")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_drive (id, entity_id, primary_goal, motivation, urgency, long_term, current_goal, immediate, hidden_goal, fear, weakness, desire, contradiction, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(&drive.primary_goal)
            .bind(&drive.motivation)
            .bind(drive.urgency)
            .bind(&drive.long_term)
            .bind(&drive.current)
            .bind(&drive.immediate)
            .bind(&drive.hidden_goal)
            .bind(&drive.fear)
            .bind(&drive.weakness)
            .bind(&drive.desire)
            .bind(&drive.contradiction)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert character_drive")?;
        }
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterDrive>> {
        let row = sqlx::query_as::<_, CharacterDriveRow>(
            "SELECT id, entity_id, primary_goal, motivation, urgency, long_term, current_goal, immediate, hidden_goal, fear, weakness, desire, contradiction, created_at, updated_at \
             FROM character_drive WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_drive")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct CharacterDriveRow {
    id: Uuid,
    entity_id: Uuid,
    primary_goal: Option<String>,
    motivation: Option<String>,
    urgency: i32,
    long_term: Option<String>,
    current_goal: Option<String>,
    immediate: Option<String>,
    hidden_goal: Option<String>,
    fear: Option<String>,
    weakness: Option<String>,
    desire: Option<String>,
    contradiction: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterDriveRow> for CharacterDrive {
    fn from(r: CharacterDriveRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
        CharacterDrive {
            primary_goal: filter_empty(r.primary_goal),
            motivation: filter_empty(r.motivation),
            urgency: r.urgency,
            long_term: filter_empty(r.long_term),
            current: filter_empty(r.current_goal),
            immediate: filter_empty(r.immediate),
            hidden_goal: filter_empty(r.hidden_goal),
            fear: filter_empty(r.fear),
            weakness: filter_empty(r.weakness),
            desire: filter_empty(r.desire),
            contradiction: filter_empty(r.contradiction),
        }
    }
}

// ============= CharacterConflictRepo =============

pub struct CharacterConflictRepo {
    pool: PgPool,
}

impl CharacterConflictRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, entity_id: Uuid, conflict: &CharacterConflict) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_conflict (id, entity_id, conflict_type, description, target_entity_id, resolution_status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(conflict.id)
        .bind(entity_id)
        .bind(conflict.conflict_type.as_str())
        .bind(&conflict.description)
        .bind(conflict.target_entity_id)
        .bind(&conflict.resolution_status)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character_conflict")?;
        Ok(())
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<CharacterConflict>> {
        let rows = sqlx::query_as::<_, CharacterConflictRow>(
            "SELECT id, entity_id, conflict_type, description, target_entity_id, resolution_status, created_at, updated_at \
             FROM character_conflict WHERE entity_id = $1 ORDER BY created_at",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query character conflicts")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct CharacterConflictRow {
    id: Uuid,
    entity_id: Uuid,
    conflict_type: Option<String>,
    description: String,
    target_entity_id: Option<Uuid>,
    resolution_status: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterConflictRow> for CharacterConflict {
    fn from(r: CharacterConflictRow) -> Self {
        CharacterConflict {
            id: r.id,
            entity_id: r.entity_id,
            conflict_type: r
                .conflict_type
                .as_deref()
                .map(ConflictType::from_str)
                .unwrap_or(ConflictType::Internal),
            description: r.description,
            target_entity_id: r.target_entity_id,
            resolution_status: r.resolution_status.filter(|s| !s.is_empty()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterRelationshipRepo =============

pub struct CharacterRelationshipRepo {
    pool: PgPool,
}

impl CharacterRelationshipRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        entity_id: Uuid,
        relationship: &CharacterRelationship,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_relationship (id, entity_id, target_entity_id, relationship_type, attitude, trust_level, secret_knowledge, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(relationship.id)
        .bind(entity_id)
        .bind(relationship.target_entity_id)
        .bind(&relationship.relationship_type)
        .bind(&relationship.attitude)
        .bind(relationship.trust_level)
        .bind(&relationship.secret_knowledge)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character_relationship")?;
        Ok(())
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<CharacterRelationship>> {
        let rows = sqlx::query_as::<_, CharacterRelationshipRow>(
            "SELECT id, entity_id, target_entity_id, relationship_type, attitude, trust_level, secret_knowledge, created_at, updated_at \
             FROM character_relationship WHERE entity_id = $1 ORDER BY created_at",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query character relationships")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct CharacterRelationshipRow {
    id: Uuid,
    entity_id: Uuid,
    target_entity_id: Uuid,
    relationship_type: String,
    attitude: String,
    trust_level: i32,
    secret_knowledge: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterRelationshipRow> for CharacterRelationship {
    fn from(r: CharacterRelationshipRow) -> Self {
        CharacterRelationship {
            id: r.id,
            entity_id: r.entity_id,
            target_entity_id: r.target_entity_id,
            relationship_type: r.relationship_type,
            attitude: r.attitude,
            trust_level: r.trust_level,
            secret_knowledge: r.secret_knowledge.filter(|s| !s.is_empty()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterSecretRepo =============

pub struct CharacterSecretRepo {
    pool: PgPool,
}

impl CharacterSecretRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, entity_id: Uuid, secret: &CharacterSecret) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_secret (id, entity_id, content, importance, reveal_condition, related_entities, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(secret.id)
        .bind(entity_id)
        .bind(&secret.content)
        .bind(secret.importance)
        .bind(&secret.reveal_condition)
        .bind(serde_json::to_value(&secret.related_entities).unwrap_or(serde_json::Value::Null))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create character_secret")?;
        Ok(())
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<CharacterSecret>> {
        let rows = sqlx::query_as::<_, CharacterSecretRow>(
            "SELECT id, entity_id, content, importance, reveal_condition, related_entities, created_at, updated_at \
             FROM character_secret WHERE entity_id = $1 ORDER BY created_at",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query character secrets")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct CharacterSecretRow {
    id: Uuid,
    entity_id: Uuid,
    content: String,
    importance: i32,
    reveal_condition: Option<String>,
    related_entities: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterSecretRow> for CharacterSecret {
    fn from(r: CharacterSecretRow) -> Self {
        CharacterSecret {
            id: r.id,
            entity_id: r.entity_id,
            content: r.content,
            importance: r.importance,
            reveal_condition: r.reveal_condition.filter(|s| !s.is_empty()),
            related_entities: r
                .related_entities
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= CharacterCapabilityRepo =============

pub struct CharacterCapabilityRepo {
    pool: PgPool,
}

impl CharacterCapabilityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, entity_id: Uuid, capability: &CharacterCapability) -> Result<()> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE character_capability SET skills=$1, limitations=$2, updated_at=$3 WHERE entity_id=$4",
        )
        .bind(serde_json::to_value(&capability.skills).unwrap_or(serde_json::Value::Null))
        .bind(serde_json::to_value(&capability.limitations).unwrap_or(serde_json::Value::Null))
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_capability")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_capability (id, entity_id, skills, limitations, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(serde_json::to_value(&capability.skills).unwrap_or(serde_json::Value::Null))
            .bind(serde_json::to_value(&capability.limitations).unwrap_or(serde_json::Value::Null))
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert character_capability")?;
        }
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterCapability>> {
        let row = sqlx::query_as::<_, CharacterCapabilityRow>(
            "SELECT id, entity_id, skills, limitations, created_at, updated_at \
             FROM character_capability WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_capability")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct CharacterCapabilityRow {
    id: Uuid,
    entity_id: Uuid,
    skills: Option<serde_json::Value>,
    limitations: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterCapabilityRow> for CharacterCapability {
    fn from(r: CharacterCapabilityRow) -> Self {
        CharacterCapability {
            skills: r
                .skills
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            limitations: r
                .limitations
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
        }
    }
}

// ============= CharacterArcRepo =============

pub struct CharacterArcRepo {
    pool: PgPool,
}

impl CharacterArcRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, entity_id: Uuid, arc: &CharacterArcPotential) -> Result<()> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE character_arc_potential SET starting_state=$1, possible_change=$2, resistance=$3, updated_at=$4 WHERE entity_id=$5",
        )
        .bind(&arc.starting_state)
        .bind(&arc.possible_change)
        .bind(&arc.resistance)
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_arc_potential")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_arc_potential (id, entity_id, starting_state, possible_change, resistance, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(&arc.starting_state)
            .bind(&arc.possible_change)
            .bind(&arc.resistance)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert character_arc_potential")?;
        }
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterArcPotential>> {
        let row = sqlx::query_as::<_, CharacterArcRow>(
            "SELECT id, entity_id, starting_state, possible_change, resistance, created_at, updated_at \
             FROM character_arc_potential WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_arc_potential")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct CharacterArcRow {
    id: Uuid,
    entity_id: Uuid,
    starting_state: Option<String>,
    possible_change: Option<String>,
    resistance: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterArcRow> for CharacterArcPotential {
    fn from(r: CharacterArcRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
        CharacterArcPotential {
            starting_state: filter_empty(r.starting_state),
            possible_change: filter_empty(r.possible_change),
            resistance: filter_empty(r.resistance),
        }
    }
}

// ============= CharacterExtensionRepo =============

pub struct CharacterExtensionRepo {
    pool: PgPool,
}

impl CharacterExtensionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, entity_id: Uuid, extension: &CharacterExtension) -> Result<()> {
        let now = Utc::now();
        let (ext_type, data) = match extension {
            CharacterExtension::Fantasy(f) => ("Fantasy", serde_json::to_value(f)?),
            CharacterExtension::Modern(m) => ("Modern", serde_json::to_value(m)?),
            CharacterExtension::SciFi(s) => ("SciFi", serde_json::to_value(s)?),
            CharacterExtension::Custom(v) => ("Custom", v.clone()),
        };

        let result = sqlx::query(
            "UPDATE character_extension SET extension_type=$1, data=$2, updated_at=$3 WHERE entity_id=$4",
        )
        .bind(ext_type)
        .bind(&data)
        .bind(now)
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .context("Failed to update character_extension")?;

        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO character_extension (id, entity_id, extension_type, data, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(entity_id)
            .bind(ext_type)
            .bind(&data)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert character_extension")?;
        }
        Ok(())
    }

    pub async fn get_by_entity(&self, entity_id: Uuid) -> Result<Option<CharacterExtension>> {
        let row = sqlx::query_as::<_, CharacterExtensionRow>(
            "SELECT id, entity_id, extension_type, data, created_at, updated_at \
             FROM character_extension WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query character_extension")?;

        Ok(row.and_then(|r| {
            let data = r.data.unwrap_or(serde_json::Value::Null);
            match r.extension_type.as_deref() {
                Some("Fantasy") => serde_json::from_value(data).ok().map(CharacterExtension::Fantasy),
                Some("Modern") => serde_json::from_value(data).ok().map(CharacterExtension::Modern),
                Some("SciFi") => serde_json::from_value(data).ok().map(CharacterExtension::SciFi),
                _ => Some(CharacterExtension::Custom(data)),
            }
        }))
    }
}

#[derive(sqlx::FromRow)]
struct CharacterExtensionRow {
    id: Uuid,
    entity_id: Uuid,
    extension_type: Option<String>,
    data: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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
            "INSERT INTO character_trait (id, entity_id, trait_type, name, description, intensity, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(trait_item.id)
        .bind(entity_id)
        .bind(trait_type_to_str(trait_item.trait_type.clone()))
        .bind(&trait_item.name)
        .bind(trait_item.description.as_deref().unwrap_or(""))
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
            "SELECT id, entity_id, trait_type, name, description, intensity, created_at, updated_at \
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
    trait_type: Option<String>,
    name: String,
    description: Option<String>,
    intensity: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterTraitRow> for CharacterTrait {
    fn from(r: CharacterTraitRow) -> Self {
        CharacterTrait {
            id: r.id,
            entity_id: r.entity_id,
            trait_type: r
                .trait_type
                .as_deref()
                .map(trait_type_from_str)
                .unwrap_or(TraitType::Personality),
            name: r.name,
            description: r.description.filter(|s| !s.is_empty()),
            intensity: r.intensity,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

fn trait_type_to_str(t: TraitType) -> &'static str {
    match t {
        TraitType::Personality => "Personality",
        TraitType::Behavior => "Behavior",
        TraitType::Value => "Value",
        TraitType::Fear => "Fear",
        TraitType::Habit => "Habit",
        TraitType::Strength => "Strength",
        TraitType::Weakness => "Weakness",
    }
}

fn trait_type_from_str(s: &str) -> TraitType {
    match s {
        "Behavior" => TraitType::Behavior,
        "Value" => TraitType::Value,
        "Fear" => TraitType::Fear,
        "Habit" => TraitType::Habit,
        "Strength" => TraitType::Strength,
        "Weakness" => TraitType::Weakness,
        _ => TraitType::Personality,
    }
}
