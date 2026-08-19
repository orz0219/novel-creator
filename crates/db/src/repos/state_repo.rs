//! State Repository
//!
//! Provides both pool-based and transaction-based methods.
//! Transaction-aware methods (suffixed _tx) accept a &mut PgConnection
//! and should be used when multiple operations must be atomic.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{CurrentState, ResourceState, StateChangeRecord};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub struct StateRepo {
    pool: PgPool,
}

impl StateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState> {
        Self::upsert_state_tx(&mut *self.pool.acquire().await.context("Failed to acquire connection")?, project_id, entity_id, state_key, state_value).await
    }

    /// Transaction-aware upsert_state. Use inside a transaction block.
    pub async fn upsert_state_tx(
        conn: &mut PgConnection,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Invalidate existing state
        sqlx::query(
            "UPDATE current_state SET effective_to = $1 WHERE entity_id = $2 AND state_key = $3 AND effective_to IS NULL",
        )
        .bind(now)
        .bind(entity_id)
        .bind(state_key)
        .execute(&mut *conn)
        .await
        .context("Failed to invalidate")?;

        // Insert new state
        sqlx::query(
            "INSERT INTO current_state (id, project_id, entity_id, state_key, state_value, effective_from, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(entity_id)
        .bind(state_key)
        .bind(&state_value)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&mut *conn)
        .await
        .context("Failed to insert")?;

        Ok(CurrentState {
            id,
            project_id,
            entity_id,
            state_key: state_key.to_string(),
            state_value,
            effective_from: now,
            effective_to: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_current_state(
        &self,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        Self::get_current_state_tx(&mut *self.pool.acquire().await.context("Failed to acquire connection")?, entity_id, state_key).await
    }

    /// Transaction-aware get_current_state.
    pub async fn get_current_state_tx(
        conn: &mut PgConnection,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        let row = sqlx::query_as::<_, CurrentStateRow>(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, created_at, updated_at \
             FROM current_state WHERE entity_id = $1 AND state_key = $2 AND effective_to IS NULL",
        )
        .bind(entity_id)
        .bind(state_key)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to query current state")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_current_states(&self, entity_id: Uuid) -> Result<Vec<CurrentState>> {
        let rows = sqlx::query_as::<_, CurrentStateRow>(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, created_at, updated_at \
             FROM current_state WHERE entity_id = $1 AND effective_to IS NULL ORDER BY state_key",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query current states")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 批量获取多个 entity 的当前状态，避免 N+1 query。
    pub async fn list_current_states_batch(&self, entity_ids: &[Uuid]) -> Result<Vec<CurrentState>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, CurrentStateRow>(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, created_at, updated_at \
             FROM current_state WHERE entity_id = ANY($1) AND effective_to IS NULL ORDER BY entity_id, state_key",
        )
        .bind(entity_ids)
        .fetch_all(&self.pool)
        .await
        .context("Failed to batch query current states")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn record_change(
        &self,
        project_id: Uuid,
        event_id: Option<Uuid>,
        change_type: &str,
        target_entity_id: Uuid,
        state_key: &str,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
        committed_by: Option<&str>,
    ) -> Result<StateChangeRecord> {
        Self::record_change_tx(
            &mut *self.pool.acquire().await.context("Failed to acquire connection")?,
            project_id, event_id, change_type, target_entity_id, state_key, old_value, new_value, committed_by,
        ).await
    }

    /// Transaction-aware record_change. Use inside a transaction block.
    pub async fn record_change_tx(
        conn: &mut PgConnection,
        project_id: Uuid,
        event_id: Option<Uuid>,
        change_type: &str,
        target_entity_id: Uuid,
        state_key: &str,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
        committed_by: Option<&str>,
    ) -> Result<StateChangeRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO state_change (id, project_id, event_id, change_type, target_entity_id, state_key, old_value, new_value, committed_at, committed_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(project_id)
        .bind(event_id)
        .bind(change_type)
        .bind(target_entity_id)
        .bind(state_key)
        .bind(old_value.as_ref())
        .bind(&new_value)
        .bind(now)
        .bind(committed_by.unwrap_or(""))
        .execute(&mut *conn)
        .await
        .context("Failed to record change")?;

        Ok(StateChangeRecord {
            id,
            project_id,
            event_id,
            change_type: change_type.to_string(),
            target_entity_id,
            state_key: state_key.to_string(),
            old_value,
            new_value,
            committed_at: now,
            committed_by: committed_by.map(|s| s.to_string()),
        })
    }

    pub async fn upsert_resource(
        &self,
        project_id: Uuid,
        location_id: Uuid,
        resource_name: &str,
        quantity: Option<f64>,
        production_rate: Option<f64>,
        controlled_by: Option<Uuid>,
    ) -> Result<ResourceState> {
        let now = Utc::now();

        // Try update first
        let result = sqlx::query(
            "UPDATE resource_state SET quantity = $1, production_rate = $2, controlled_by_entity_id = $3, updated_at = $4 \
             WHERE project_id = $5 AND location_id = $6 AND resource_name = $7",
        )
        .bind(quantity)
        .bind(production_rate)
        .bind(controlled_by)
        .bind(now)
        .bind(project_id)
        .bind(location_id)
        .bind(resource_name)
        .execute(&self.pool)
        .await
        .context("Failed to update resource")?;

        if result.rows_affected() == 0 {
            // Insert new record
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO resource_state (id, project_id, location_id, resource_name, quantity, production_rate, controlled_by_entity_id, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(id)
            .bind(project_id)
            .bind(location_id)
            .bind(resource_name)
            .bind(quantity)
            .bind(production_rate)
            .bind(controlled_by)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to insert resource")?;

            Ok(ResourceState {
                id,
                project_id,
                location_id,
                resource_name: resource_name.to_string(),
                quantity,
                production_rate,
                controlled_by_entity_id: controlled_by,
                created_at: now,
                updated_at: now,
            })
        } else {
            // Return updated record
            let row = sqlx::query_as::<_, ResourceStateRow>(
                "SELECT id, project_id, location_id, resource_name, quantity, production_rate, controlled_by_entity_id, created_at, updated_at \
                 FROM resource_state WHERE project_id = $1 AND location_id = $2 AND resource_name = $3",
            )
            .bind(project_id)
            .bind(location_id)
            .bind(resource_name)
            .fetch_one(&self.pool)
            .await
            .context("Failed to read updated resource")?;

            Ok(row.into())
        }
    }

    pub async fn list_resources_by_location(&self, location_id: Uuid) -> Result<Vec<ResourceState>> {
        let rows = sqlx::query_as::<_, ResourceStateRow>(
            "SELECT id, project_id, location_id, resource_name, quantity, production_rate, controlled_by_entity_id, created_at, updated_at \
             FROM resource_state WHERE location_id = $1 ORDER BY resource_name",
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query resources")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct CurrentStateRow {
    id: Uuid,
    project_id: Uuid,
    entity_id: Uuid,
    state_key: String,
    state_value: serde_json::Value,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CurrentStateRow> for CurrentState {
    fn from(r: CurrentStateRow) -> Self {
        CurrentState {
            id: r.id,
            project_id: r.project_id,
            entity_id: r.entity_id,
            state_key: r.state_key,
            state_value: r.state_value,
            effective_from: r.effective_from,
            effective_to: r.effective_to,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ResourceStateRow {
    id: Uuid,
    project_id: Uuid,
    location_id: Uuid,
    resource_name: String,
    quantity: Option<f64>,
    production_rate: Option<f64>,
    controlled_by_entity_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ResourceStateRow> for ResourceState {
    fn from(r: ResourceStateRow) -> Self {
        ResourceState {
            id: r.id,
            project_id: r.project_id,
            location_id: r.location_id,
            resource_name: r.resource_name,
            quantity: r.quantity,
            production_rate: r.production_rate,
            controlled_by_entity_id: r.controlled_by_entity_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
