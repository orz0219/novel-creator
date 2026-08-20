//! Mutation 校验：第一层（schema）结构完整性。
//!
//! 第二层（domain 强约束，如「角色不能同时死亡又存活」「关系双方必须存在」
//! 「时间线不能倒退」等）将在后续阶段（提案 二十五）接入 `ValidationService`。

use domain::mutation::*;

pub fn validate_mutation(cmd: &MutationCommand) -> Result<(), MutationError> {
    match &cmd.payload {
        MutationPayload::CreateEntity { name, .. } => {
            if name.trim().is_empty() {
                return Err(MutationError::Validation("entity name is required".into()));
            }
        }
        MutationPayload::UpdateEntity {
            name,
            summary,
            description,
            attributes,
        } => {
            if name.is_none() && summary.is_none() && description.is_none() && attributes.is_none() {
                return Err(MutationError::Validation(
                    "UpdateEntity requires at least one field".into(),
                ));
            }
        }
        MutationPayload::CreateRelation { relation_type, .. } => {
            if relation_type.trim().is_empty() {
                return Err(MutationError::Validation("relation_type is required".into()));
            }
        }
        MutationPayload::SetEntityState { state_key, .. } => {
            if state_key.trim().is_empty() {
                return Err(MutationError::Validation("state_key is required".into()));
            }
        }
        MutationPayload::CreateEvent { name, .. } => {
            if name.trim().is_empty() {
                return Err(MutationError::Validation("event name is required".into()));
            }
        }
        MutationPayload::DeleteEntity | MutationPayload::EndRelation { .. } => {
            if cmd.expected_version.is_none() {
                return Err(MutationError::Validation(format!(
                    "{:?} requires expected_version for optimistic locking",
                    cmd.payload
                )));
            }
        }
        _ => {}
    }
    Ok(())
}
