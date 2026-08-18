//! StateCommitter - 事务化提交状态变更
//!
//! 将 ProposedChange 列表事务化提交到世界状态。

use anyhow::Result;
use db::connection::Database;
use db::repos::{state_repo, validation_repo};
use domain::*;
use uuid::Uuid;

/// StateCommitter 实现 - 事务化提交状态变更
pub struct DbStateCommitter<'a> {
    db: &'a Database,
}

impl<'a> DbStateCommitter<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> domain::extractor::StateCommitter for DbStateCommitter<'a> {
    fn commit(
        &self,
        project_id: Uuid,
        changes: &[ProposedChange],
    ) -> Result<Vec<StateChangeRecord>> {
        let val_repo = validation_repo::ValidationRepo::new(self.db);
        let state_repo = state_repo::StateRepo::new(self.db);
        let mut records = Vec::new();

        for change in changes {
            match &change.change_type {
                ProposedChangeType::StateChange => {
                    let state_key = change.payload.get("state_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let new_value = change.payload.get("new_value")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let old_state = state_repo.get_current_state(change.target_entity_id, state_key)?;
                    let old_value = old_state.map(|s| s.state_value);

                    let record = state_repo.record_change(
                        project_id, None, "STATE_CHANGE",
                        change.target_entity_id, state_key,
                        old_value, new_value.clone(),
                        Some("committer"),
                    )?;

                    state_repo.upsert_state(
                        project_id, change.target_entity_id,
                        state_key, new_value,
                    )?;

                    records.push(record);
                    val_repo.update_status(change.id, ProposedChangeStatus::Applied)?;
                }
                _ => {
                    tracing::warn!("Unsupported change type: {:?}", change.change_type);
                }
            }
        }

        tracing::info!("Committed {} changes", records.len());
        Ok(records)
    }
}
