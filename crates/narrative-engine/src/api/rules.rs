//! Rules API handlers (canon_rule table)
//!
//! 通过 application::rule_service::RuleService（依赖 RuleRepositoryPort）。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::rule_service::RuleService;
use db::application_ports::DbRuleRepositoryPort;
use std::sync::Arc;

fn service(state: &AppState) -> RuleService {
    RuleService::new(Arc::new(DbRuleRepositoryPort::new(state.pool.clone())))
}

#[derive(Deserialize)]
pub struct CreateRuleInput {
    pub rule_content: String,
    pub rule_level: Option<String>,
    pub affected_scope: Option<String>,
    pub enforcement: Option<String>,
    #[allow(dead_code)]
    pub severity: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRuleInput {
    pub rule_content: Option<String>,
    pub rule_level: Option<String>,
    pub severity: Option<String>,
}

pub async fn list_rules(State(state): State<AppState>, Path(world_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let rules = service(&state).list_rules(world_id).await?;
    Ok(Json(serde_json::json!(rules)))
}

pub async fn create_rule(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateRuleInput>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let rule = service(&state)
        .create_rule(
            world_id,
            &input.rule_content,
            input.rule_level.as_deref(),
            input.affected_scope.as_deref(),
            input.enforcement.as_deref(),
        )
        .await?;
    Ok(Json(rule))
}

pub async fn get_rule(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid rule ID")))?;
    let rule = service(&state)
        .get_rule(id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Rule not found")))?;
    Ok(Json(rule))
}

pub async fn update_rule(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateRuleInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid rule ID")))?;
    // severity 字段在 schema 中不存在，此前误绑到 enforcement；这里彻底忽略，避免数据污染。
    if input.severity.is_some() {
        tracing::warn!("update_rule received deprecated 'severity' field for rule {}; ignored", id);
    }
    let rule = service(&state)
        .update_rule(id, input.rule_content.as_deref(), input.rule_level.as_deref())
        .await?;
    Ok(Json(rule))
}

pub async fn delete_rule(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid rule ID")))?;
    service(&state).delete_rule(id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
