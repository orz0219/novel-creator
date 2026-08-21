//! Entity API handlers
//!
//! 所有实体 / 关系 / 角色子数据读写通过 application::entity_service::EntityService
//! （依赖 EntityRepositoryPort），host 层不再直接执行 SQL。

use axum::extract::{Path, State, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::entity_service::EntityService;
use application::mutation::MutationCommitter;
use db::application_ports::DbEntityRepositoryPort;
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use std::sync::Arc;

fn service(state: &AppState) -> EntityService {
    let committer = Arc::new(MutationCommitter::new(Arc::new(
        DbMutationCommitter::new(state.pool.clone()),
    )));
    let resolver = Arc::new(DbProjectResolverPort::new(state.pool.clone()));
    EntityService::new(
        Arc::new(DbEntityRepositoryPort::new(state.pool.clone())),
        committer,
        resolver,
    )
}

#[derive(Deserialize)]
pub struct EntityTypeFilter { pub r#type: Option<String> }

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CreateEntityInput {
    pub name: String, pub summary: Option<String>, pub description: Option<String>,
    pub attributes: Option<serde_json::Value>,
}

pub async fn list_entities(State(state): State<AppState>, Path(world_id): Path<String>, Query(q): Query<EntityTypeFilter>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let entities = service(&state).list_entities(world_id, q.r#type.as_deref()).await?;
    Ok(Json(serde_json::json!(entities)))
}

pub async fn get_entity(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let entity = service(&state)
        .get_entity(id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Entity not found")))?;
    Ok(Json(entity))
}

pub async fn create_entity(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    // host 层默认按 Item 类型创建（与旧实现一致）。
    let entity = service(&state)
        .create_entity(world_id, "Item", &input.name, input.summary.as_deref(), input.description.as_deref())
        .await?;
    Ok(Json(entity))
}

pub async fn update_entity(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let entity = service(&state)
        .update_entity(id, Some(&input.name), input.summary.as_deref(), input.description.as_deref(), input.attributes.as_ref())
        .await?;
    Ok(Json(entity))
}

pub async fn delete_entity(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let result = service(&state).delete_entity(id).await?;
    Ok(Json(result))
}

pub async fn list_characters(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { list_entities(s, p, Query(EntityTypeFilter { r#type: Some("Character".to_string()) })).await }
pub async fn get_character(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { get_entity(s, p).await }

pub async fn create_character(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let entity = service(&state)
        .create_entity(world_id, "Character", &input.name, input.summary.as_deref(), input.description.as_deref())
        .await?;
    Ok(Json(entity))
}

pub async fn get_character_profile(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let profile = service(&state).get_character_profile(id).await?;
    Ok(Json(profile.unwrap_or(serde_json::Value::Null)))
}

pub async fn get_character_state(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let state_val = service(&state).get_character_state(id).await?;
    Ok(Json(state_val.unwrap_or(serde_json::Value::Null)))
}

pub async fn update_character_profile(State(state): State<AppState>, Path(id): Path<String>, Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let profile = service(&state).update_character_profile(id, body).await?;
    Ok(Json(profile))
}

pub async fn update_character_state(State(state): State<AppState>, Path(id): Path<String>, Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let state_val = service(&state).update_character_state(id, body).await?;
    Ok(Json(state_val))
}

pub async fn get_location_profile(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let profile = service(&state).get_location_profile(id).await?;
    Ok(Json(profile.unwrap_or(serde_json::Value::Null)))
}

pub async fn upsert_location_profile(State(state): State<AppState>, Path(id): Path<String>, Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let profile = service(&state).upsert_location_profile(id, body).await?;
    Ok(Json(profile))
}

pub async fn get_faction_profile(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let profile = service(&state).get_faction_profile(id).await?;
    Ok(Json(profile.unwrap_or(serde_json::Value::Null)))
}

pub async fn upsert_faction_profile(State(state): State<AppState>, Path(id): Path<String>, Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let profile = service(&state).upsert_faction_profile(id, body).await?;
    Ok(Json(profile))
}

pub async fn get_character_knowledge(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let knowledge = service(&state).get_character_knowledge(id).await?;
    Ok(Json(serde_json::json!(knowledge)))
}

pub async fn get_character_relationships(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let relationships = service(&state).get_character_relationships(id).await?;
    Ok(Json(serde_json::json!(relationships)))
}

pub async fn list_locations(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { list_entities(s, p, Query(EntityTypeFilter { r#type: Some("Location".to_string()) })).await }
pub async fn get_location(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { get_entity(s, p).await }

pub async fn create_location(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let entity = service(&state)
        .create_entity(world_id, "Location", &input.name, input.summary.as_deref(), input.description.as_deref())
        .await?;
    Ok(Json(entity))
}

pub async fn get_location_entities(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([])))
}

pub async fn get_location_events(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([])))
}

pub async fn list_factions(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { list_entities(s, p, Query(EntityTypeFilter { r#type: Some("Faction".to_string()) })).await }

pub async fn create_faction(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let entity = service(&state)
        .create_entity(world_id, "Faction", &input.name, input.summary.as_deref(), input.description.as_deref())
        .await?;
    Ok(Json(entity))
}

pub async fn list_relations(State(state): State<AppState>, Path(world_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let world_id = Uuid::parse_str(&world_id).map_err(|_| AppError(anyhow::anyhow!("Invalid world ID")))?;
    let relations = service(&state).list_relations(world_id).await?;
    Ok(Json(serde_json::json!(relations)))
}

pub async fn create_relation(State(state): State<AppState>, Path(_world_id): Path<String>, Json(input): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let source = input.get("source_entity_id").and_then(|v| v.as_str()).unwrap_or("");
    let target = input.get("target_entity_id").and_then(|v| v.as_str()).unwrap_or("");
    let rtype = input.get("relation_type").and_then(|v| v.as_str()).unwrap_or("related");
    let desc = input.get("description").and_then(|v| v.as_str());

    let source_id = Uuid::parse_str(source).map_err(|_| AppError(anyhow::anyhow!("Invalid source_entity_id")))?;
    let target_id = Uuid::parse_str(target).map_err(|_| AppError(anyhow::anyhow!("Invalid target_entity_id")))?;

    let relation = service(&state)
        .create_relation(source_id, target_id, rtype, desc)
        .await?;
    Ok(Json(relation))
}

pub async fn delete_relation(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid relation ID")))?;
    service(&state).delete_relation(id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
