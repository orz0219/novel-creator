//! EntityAlias + IdentityTimeline + TestCase Repos

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::identity::*;
use sqlx::PgPool;
use uuid::Uuid;

// ============= EntityAliasRepo =============

pub struct EntityAliasRepo {
    pool: PgPool,
}

impl EntityAliasRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        entity_id: Uuid,
        alias_type: AliasType,
        alias: &str,
    ) -> Result<EntityAlias> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO entity_alias (id, entity_id, alias_type, alias, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(entity_id)
        .bind(alias_type.as_str())
        .bind(alias)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert entity_alias")?;

        Ok(EntityAlias {
            id,
            entity_id,
            alias_type,
            alias: alias.to_string(),
            valid_from_scene_id: None,
            valid_until_scene_id: None,
            created_at: now,
        })
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<EntityAlias>> {
        let rows = sqlx::query_as::<_, EntityAliasRow>(
            "SELECT id, entity_id, alias_type, alias, valid_from_scene_id, valid_until_scene_id, created_at \
             FROM entity_alias WHERE entity_id = $1",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query entity aliases")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn find_by_alias(&self, alias: &str) -> Result<Vec<EntityAlias>> {
        let pattern = format!("%{}%", alias);
        let rows = sqlx::query_as::<_, EntityAliasRow>(
            "SELECT id, entity_id, alias_type, alias, valid_from_scene_id, valid_until_scene_id, created_at \
             FROM entity_alias WHERE alias LIKE $1",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query entity aliases")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct EntityAliasRow {
    id: Uuid,
    entity_id: Uuid,
    alias_type: String,
    alias: String,
    valid_from_scene_id: Option<Uuid>,
    valid_until_scene_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl From<EntityAliasRow> for EntityAlias {
    fn from(r: EntityAliasRow) -> Self {
        EntityAlias {
            id: r.id,
            entity_id: r.entity_id,
            alias_type: AliasType::from_str(&r.alias_type),
            alias: r.alias,
            valid_from_scene_id: r.valid_from_scene_id,
            valid_until_scene_id: r.valid_until_scene_id,
            created_at: r.created_at,
        }
    }
}

// ============= IdentityTimelineRepo =============

pub struct IdentityTimelineRepo {
    pool: PgPool,
}

impl IdentityTimelineRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        entity_id: Uuid,
        identity: &str,
        start_scene_id: Uuid,
        change_reason: Option<&str>,
    ) -> Result<IdentityTimeline> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO identity_timeline (id, entity_id, identity, start_scene_id, change_reason, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(entity_id)
        .bind(identity)
        .bind(start_scene_id)
        .bind(change_reason.unwrap_or(""))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert identity_timeline")?;

        Ok(IdentityTimeline {
            id,
            entity_id,
            identity: identity.to_string(),
            start_scene_id,
            end_scene_id: None,
            change_reason: change_reason.map(|s| s.to_string()),
            created_at: now,
        })
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<IdentityTimeline>> {
        let rows = sqlx::query_as::<_, IdentityTimelineRow>(
            "SELECT id, entity_id, identity, start_scene_id, end_scene_id, change_reason, created_at \
             FROM identity_timeline WHERE entity_id = $1 ORDER BY created_at",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query identity timelines")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct IdentityTimelineRow {
    id: Uuid,
    entity_id: Uuid,
    identity: String,
    start_scene_id: Uuid,
    end_scene_id: Option<Uuid>,
    change_reason: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<IdentityTimelineRow> for IdentityTimeline {
    fn from(r: IdentityTimelineRow) -> Self {
        IdentityTimeline {
            id: r.id,
            entity_id: r.entity_id,
            identity: r.identity,
            start_scene_id: r.start_scene_id,
            end_scene_id: r.end_scene_id,
            change_reason: r.change_reason,
            created_at: r.created_at,
        }
    }
}

// ============= TestCaseRepo =============

pub struct TestCaseRepo {
    pool: PgPool,
}

impl TestCaseRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        test_type: TestType,
        preconditions: serde_json::Value,
        expected_result: &str,
    ) -> Result<TestCase> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO test_case (id, project_id, name, description, test_type, preconditions, expected_result, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'Pending', $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(test_type.as_str())
        .bind(&preconditions)
        .bind(expected_result)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert test_case")?;

        Ok(TestCase {
            id,
            project_id,
            name: name.to_string(),
            description: description.to_string(),
            test_type,
            preconditions,
            expected_result: expected_result.to_string(),
            status: TestStatus::Pending,
            created_at: now,
        })
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<TestCase>> {
        let rows = sqlx::query_as::<_, TestCaseRow>(
            "SELECT id, project_id, name, description, test_type, preconditions, expected_result, status, created_at \
             FROM test_case WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query test cases")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct TestCaseRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: String,
    test_type: String,
    preconditions: Option<serde_json::Value>,
    expected_result: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl From<TestCaseRow> for TestCase {
    fn from(r: TestCaseRow) -> Self {
        TestCase {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            test_type: TestType::from_str(&r.test_type),
            preconditions: r.preconditions.unwrap_or(serde_json::json!({})),
            expected_result: r.expected_result,
            status: TestStatus::from_str(&r.status),
            created_at: r.created_at,
        }
    }
}
