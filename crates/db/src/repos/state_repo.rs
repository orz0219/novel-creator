//! State Repository
//!
//! Provides both pool-based and transaction-based methods.
//! Transaction-aware methods (suffixed _tx) accept a &mut PgConnection
//! and should be used when multiple operations must be atomic.
//!
//! All state queries require project_id for project isolation.
//! upsert_state_tx uses version-based optimistic concurrency control (CAS).

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{CurrentState, ResourceState, StateChangeRecord};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// 并发修改错误
#[derive(Debug, thiserror::Error)]
#[error("Concurrent modification detected for entity {entity_id}, state_key {state_key}, expected version {expected_version}")]
pub struct ConcurrentModificationError {
    pub entity_id: Uuid,
    pub state_key: String,
    pub expected_version: i32,
}

pub struct StateRepo {
    pool: PgPool,
}

impl StateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ============================================================
    // Project-scoped queries
    // ============================================================

    pub async fn get_current_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        Self::get_current_state_tx(
            &mut *self.pool.acquire().await.context("Failed to acquire connection")?,
            project_id, entity_id, state_key,
        ).await
    }

    /// Transaction-aware get_current_state. Requires project_id for isolation.
    pub async fn get_current_state_tx(
        conn: &mut PgConnection,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        let row = sqlx::query_as::<_, CurrentStateRow>(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, version, created_at, updated_at \
             FROM current_state WHERE project_id = $1 AND entity_id = $2 AND state_key = $3 AND effective_to IS NULL",
        )
        .bind(project_id)
        .bind(entity_id)
        .bind(state_key)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to query current state")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_current_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>> {
        let rows = sqlx::query_as::<_, CurrentStateRow>(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, version, created_at, updated_at \
             FROM current_state WHERE project_id = $1 AND entity_id = $2 AND effective_to IS NULL ORDER BY state_key",
        )
        .bind(project_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query current states")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Batch query: requires project_id for isolation.
    pub async fn list_current_states_batch(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<CurrentState>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, CurrentStateRow>(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, version, created_at, updated_at \
             FROM current_state WHERE project_id = $1 AND entity_id = ANY($2) AND effective_to IS NULL ORDER BY entity_id, state_key",
        )
        .bind(project_id)
        .bind(entity_ids)
        .fetch_all(&self.pool)
        .await
        .context("Failed to batch query current states")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // ============================================================
    // Upsert with Optimistic Concurrency Control (CAS)
    // ============================================================

    /// Upsert current state with version CAS.
    ///
    /// **WARNING**: This method should only be used for:
    /// - Testing
    /// - Initial state setup
    /// - Migration scripts
    ///
    /// For production state mutations, use StateCommitter which ensures:
    /// - StateChangeRecord is created (audit trail)
    /// - All operations are atomic (single transaction)
    /// - Project isolation is enforced
    /// - No canonical state mutation can occur without a StateChangeRecord
    ///
    /// Violation of this invariant will be caught in code review.
    pub async fn upsert_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
        expected_version: Option<i32>,
    ) -> Result<CurrentState> {
        tracing::warn!(
            "Direct upsert_state called outside StateCommitter.              This should only be used for testing/initial setup.              For production, use StateCommitter to ensure audit trail.",
        );
        Self::upsert_state_tx(
            &mut *self.pool.acquire().await.context("Failed to acquire connection")?,
            project_id, entity_id, state_key, state_value, expected_version,
        ).await
    }

    /// Transaction-aware upsert_state with version CAS.
    ///
    /// If expected_version is Some(v), performs compare-and-swap:
    ///   UPDATE ... WHERE version = v AND effective_to IS NULL
    /// Returns ConcurrentModificationError if rows_affected == 0.
    ///
    /// If expected_version is None, skips CAS check (for initial state setup).
    ///
    /// **WARNING**: This method should ONLY be called from StateCommitter.
    /// For production state mutations, use StateCommitter which ensures:
    /// - StateChangeRecord is created (audit trail)
    /// - All operations are atomic (single transaction)
    /// - Project isolation is enforced
    /// - No canonical state mutation can occur without a StateChangeRecord
    ///
    /// Direct calls to this method from business logic are FORBIDDEN.
    /// This invariant will be enforced in code review.
    pub async fn upsert_state_tx(
        conn: &mut PgConnection,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
        expected_version: Option<i32>,
    ) -> Result<CurrentState> {
        let now = Utc::now();
        let new_version = expected_version.map(|v| v + 1).unwrap_or(1);

        // Step 1: Invalidate existing state (with CAS if expected_version provided)
        let invalidated = if let Some(expected_ver) = expected_version {
            let result = sqlx::query(
                "UPDATE current_state SET effective_to = $1 \
                 WHERE project_id = $2 AND entity_id = $3 AND state_key = $4 \
                 AND version = $5 AND effective_to IS NULL",
            )
            .bind(now)
            .bind(project_id)
            .bind(entity_id)
            .bind(state_key)
            .bind(expected_ver)
            .execute(&mut *conn)
            .await
            .context("Failed to invalidate")?;
            result.rows_affected()
        } else {
            // No CAS check - just invalidate
            let result = sqlx::query(
                "UPDATE current_state SET effective_to = $1 \
                 WHERE project_id = $2 AND entity_id = $3 AND state_key = $4 AND effective_to IS NULL",
            )
            .bind(now)
            .bind(project_id)
            .bind(entity_id)
            .bind(state_key)
            .execute(&mut *conn)
            .await
            .context("Failed to invalidate")?;
            result.rows_affected()
        };

        // Step 2: If CAS expected but no rows invalidated, concurrent modification
        if expected_version.is_some() && invalidated == 0 {
            return Err(anyhow::anyhow!(ConcurrentModificationError {
                entity_id,
                state_key: state_key.to_string(),
                expected_version: expected_version.unwrap(),
            }));
        }

        // Step 3: Insert new state with incremented version
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO current_state (id, project_id, entity_id, state_key, state_value, effective_from, version, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(project_id)
        .bind(entity_id)
        .bind(state_key)
        .bind(&state_value)
        .bind(now)
        .bind(new_version)
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
            version: new_version,
            created_at: now,
            updated_at: now,
        })
    }

    // ============================================================
    // P1-4: Atomic state mutation - commit_state_change_tx
    // ============================================================

    /// 原子化状态变更 - 在单一事务中完成 read + record + write
    ///
    /// P1-4: 将 read + record + write 封装成单一方法，确保：
    /// 1. 读取当前状态
    /// 2. 记录变更历史
    /// 3. 更新当前状态
    /// 所有操作在同一事务中完成，避免逻辑耦合风险。
    ///
    /// 返回 (StateChangeRecord, new_version)
    pub async fn commit_state_change_tx(
        conn: &mut PgConnection,
        project_id: Uuid,
        event_id: Option<Uuid>,
        change_type: &str,
        target_entity_id: Uuid,
        state_key: &str,
        new_value: serde_json::Value,
        committed_by: Option<&str>,
    ) -> Result<(StateChangeRecord, i32)> {
        // Step 1: 读取当前状态
        let old_state = Self::get_current_state_tx(
            conn, project_id, target_entity_id, state_key,
        ).await?;
        let old_value = old_state.as_ref().map(|s| s.state_value.clone());
        let expected_version = old_state.as_ref().map(|s| s.version);

        // Step 2: 记录变更历史
        let record = Self::record_change_tx(
            conn,
            project_id,
            event_id,
            change_type,
            target_entity_id,
            state_key,
            old_value,
            new_value.clone(),
            committed_by,
        ).await?;

        // Step 3: 更新当前状态 (CAS)
        Self::upsert_state_tx(
            conn,
            project_id,
            target_entity_id,
            state_key,
            new_value,
            expected_version,
        ).await?;

        let new_version = expected_version.map(|v| v + 1).unwrap_or(1);
        Ok((record, new_version))
    }

    // ============================================================
    // State change recording
    // ============================================================

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

    // ============================================================
    // Resource state (unchanged)
    // ============================================================

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

// ============================================================
// Row types
// ============================================================

#[derive(sqlx::FromRow)]
struct CurrentStateRow {
    id: Uuid,
    project_id: Uuid,
    entity_id: Uuid,
    state_key: String,
    state_value: serde_json::Value,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
    version: i32,
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
            version: r.version,
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
