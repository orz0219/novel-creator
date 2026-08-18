//! Entity API handlers

use axum::extract::{Path, State, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct EntityTypeFilter { pub r#type: Option<String> }

#[derive(Deserialize)]
pub struct CreateEntityInput {
    pub name: String, pub summary: Option<String>, pub description: Option<String>,
    pub attributes: Option<serde_json::Value>,
}

fn parse_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({}))
}

pub async fn list_entities(State(state): State<AppState>, Path(world_id): Path<String>, Query(q): Query<EntityTypeFilter>) -> Result<Json<serde_json::Value>, AppError> {
    let sql = if q.r#type.is_some() {
        "SELECT e.id, e.name, et.name, e.summary, e.description, e.attributes::text, e.version, e.created_at::text, e.updated_at::text FROM entity e JOIN entity_type et ON e.entity_type_id = et.id WHERE e.world_id = $1 AND et.name = $2 ORDER BY e.name"
    } else {
        "SELECT e.id, e.name, et.name, e.summary, e.description, e.attributes::text, e.version, e.created_at::text, e.updated_at::text FROM entity e JOIN entity_type et ON e.entity_type_id = et.id WHERE e.world_id = $1 ORDER BY e.name"
    };

    let rows: Vec<(String, String, String, Option<String>, Option<String>, String, i32, String, String)> = if let Some(t) = &q.r#type {
        sqlx::query_as(sql).bind(&world_id).bind(t).fetch_all(&state.pool).await?
    } else {
        sqlx::query_as(sql).bind(&world_id).fetch_all(&state.pool).await?
    };

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, name, etype, summary, desc, attrs, ver, created, updated)| {
        serde_json::json!({"id": id, "world_id": world_id, "entity_type_id": etype, "name": name, "summary": summary, "description": desc, "attributes": parse_json(&attrs), "version": ver, "created_by": "user", "created_at": created, "updated_at": updated})
    }).collect::<Vec<_>>())))
}

pub async fn get_entity(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row: Option<(String, String, String, String, Option<String>, Option<String>, String, i32, String, String, String)> = sqlx::query_as(
        "SELECT e.id, e.project_id, e.world_id, e.name, e.summary, e.description, e.attributes::text, e.version, e.created_by, e.created_at::text, e.updated_at::text FROM entity e WHERE e.id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;

    match row {
        Some((id, pid, wid, name, summary, desc, attrs, ver, created_by, created, updated)) => {
            Ok(Json(serde_json::json!({"id": id, "project_id": pid, "world_id": wid, "name": name, "summary": summary, "description": desc, "attributes": parse_json(&attrs), "version": ver, "created_by": created_by, "created_at": created, "updated_at": updated})))
        }
        None => Err(AppError(anyhow::anyhow!("Entity not found")))
    }
}

pub async fn create_entity(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let world: (String,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1").bind(&world_id).fetch_one(&state.pool).await?;
    let etype: (String,) = sqlx::query_as("SELECT id FROM entity_type WHERE name = 'Item'").fetch_one(&state.pool).await?;
    sqlx::query("INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'user')")
        .bind(&id).bind(&world.0).bind(&world_id).bind(&etype.0).bind(&input.name).bind(&input.summary).bind(&input.description).bind(serde_json::json!({}))
        .execute(&state.pool).await?;
    get_entity(State(state), Path(id)).await
}

pub async fn update_entity(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE entity SET name = $1, summary = $2, description = $3, version = version + 1, updated_at = NOW() WHERE id = $4")
        .bind(&input.name).bind(&input.summary).bind(&input.description).bind(&id).execute(&state.pool).await?;
    get_entity(State(state), Path(id)).await
}

pub async fn delete_entity(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM entity WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}

pub async fn list_characters(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { list_entities(s, p, Query(EntityTypeFilter { r#type: Some("Character".to_string()) })).await }
pub async fn get_character(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { get_entity(s, p).await }

pub async fn create_character(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let world: (String,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1").bind(&world_id).fetch_one(&state.pool).await?;
    let etype: (String,) = sqlx::query_as("SELECT id FROM entity_type WHERE name = 'Character'").fetch_one(&state.pool).await?;
    sqlx::query("INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'user')")
        .bind(&id).bind(&world.0).bind(&world_id).bind(&etype.0).bind(&input.name).bind(&input.summary).bind(&input.description).bind(serde_json::json!({}))
        .execute(&state.pool).await?;
    get_entity(State(state), Path(id)).await
}

pub async fn get_character_profile(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row: Option<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, entity_id, real_name, nickname, age, gender, identity, appearance, background, social_status, core_personality FROM character_profile WHERE entity_id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;
    match row {
        Some(r) => Ok(Json(serde_json::json!({"id": r.0, "entity_id": r.1, "real_name": r.2, "nickname": r.3, "age": r.4, "gender": r.5, "identity": r.6, "appearance": r.7, "background": r.8, "social_status": r.9, "core_personality": r.10}))),
        None => Ok(Json(serde_json::json!(null)))
    }
}

pub async fn get_character_state(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row: Option<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, entity_id, location, health, cultivation, resources, current_status, emotion FROM character_state WHERE entity_id = $1 ORDER BY updated_at DESC LIMIT 1"
    ).bind(&id).fetch_optional(&state.pool).await?;
    match row {
        Some(r) => Ok(Json(serde_json::json!({"id": r.0, "entity_id": r.1, "location": r.2, "health": r.3, "cultivation": r.4, "resources": r.5, "current_status": r.6, "emotion": r.7}))),
        None => Ok(Json(serde_json::json!(null)))
    }
}

pub async fn get_character_knowledge(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT ks.id, ks.knowledge_level, COALESCE(ks.source, ''), f.content FROM knowledge_state ks JOIN fact f ON ks.fact_id = f.id WHERE ks.subject_id = $1"
    ).bind(&id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, level, source, content)| serde_json::json!({"id": id, "fact": content, "level": level, "source": source})).collect::<Vec<_>>())))
}

pub async fn get_character_relationships(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT r.id, r.relation_type, e2.name, e2.id, r.description FROM relation r JOIN entity e2 ON r.target_entity_id = e2.id WHERE r.source_entity_id = $1"
    ).bind(&id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, rtype, name, tid, desc)| serde_json::json!({"id": id, "type": rtype, "target": name, "target_id": tid, "description": desc})).collect::<Vec<_>>())))
}

pub async fn list_locations(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { list_entities(s, p, Query(EntityTypeFilter { r#type: Some("Location".to_string()) })).await }
pub async fn get_location(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { get_entity(s, p).await }

pub async fn create_location(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let world: (String,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1").bind(&world_id).fetch_one(&state.pool).await?;
    let etype: (String,) = sqlx::query_as("SELECT id FROM entity_type WHERE name = 'Location'").fetch_one(&state.pool).await?;
    sqlx::query("INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'user')")
        .bind(&id).bind(&world.0).bind(&world_id).bind(&etype.0).bind(&input.name).bind(&input.summary).bind(&input.description).bind(serde_json::json!({}))
        .execute(&state.pool).await?;
    get_entity(State(state), Path(id)).await
}

pub async fn get_location_entities(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([])))
}

pub async fn get_location_events(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([])))
}

pub async fn list_factions(s: State<AppState>, p: Path<String>) -> Result<Json<serde_json::Value>, AppError> { list_entities(s, p, Query(EntityTypeFilter { r#type: Some("Faction".to_string()) })).await }

pub async fn create_faction(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateEntityInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let world: (String,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1").bind(&world_id).fetch_one(&state.pool).await?;
    let etype: (String,) = sqlx::query_as("SELECT id FROM entity_type WHERE name = 'Faction'").fetch_one(&state.pool).await?;
    sqlx::query("INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'user')")
        .bind(&id).bind(&world.0).bind(&world_id).bind(&etype.0).bind(&input.name).bind(&input.summary).bind(&input.description).bind(serde_json::json!({}))
        .execute(&state.pool).await?;
    get_entity(State(state), Path(id)).await
}

pub async fn list_relations(State(state): State<AppState>, Path(world_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, String, String, Option<String>, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.source_entity_id, r.target_entity_id, r.relation_type, r.description, r.attributes::text, r.created_at::text, r.updated_at::text FROM relation r JOIN entity e ON r.source_entity_id = e.id WHERE e.world_id = $1"
    ).bind(&world_id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, src, tgt, rtype, desc, attrs, created, updated)| {
        serde_json::json!({"id": id, "source_entity_id": src, "target_entity_id": tgt, "relation_type": rtype, "description": desc, "attributes": parse_json(&attrs), "created_at": created, "updated_at": updated})
    }).collect::<Vec<_>>())))
}

pub async fn create_relation(State(state): State<AppState>, Path(_world_id): Path<String>, Json(input): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let source = input.get("source_entity_id").and_then(|v| v.as_str()).unwrap_or("");
    let target = input.get("target_entity_id").and_then(|v| v.as_str()).unwrap_or("");
    let rtype = input.get("relation_type").and_then(|v| v.as_str()).unwrap_or("related");
    let desc = input.get("description").and_then(|v| v.as_str());
    sqlx::query("INSERT INTO relation (id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes) SELECT $1, e.project_id, $2, $3, $4, $5, '{}'::jsonb FROM entity e WHERE e.id = $2 LIMIT 1")
        .bind(&id).bind(source).bind(target).bind(rtype).bind(desc).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "source_entity_id": source, "target_entity_id": target, "relation_type": rtype})))
}

pub async fn delete_relation(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM relation WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
