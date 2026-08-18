//! HTTP API Layer - Axum routes and handlers
//!
//! Bridges frontend API calls to backend application services.

pub mod project;
pub mod world;
pub mod entity;
pub mod narrative;
pub mod context;
pub mod generation;
pub mod proposal;
pub mod validation;
pub mod history;
pub mod error;

use axum::{Router, routing::{get, post, put, delete}};
use tower_http::cors::{CorsLayer, Any};
use crate::state::AppState;

/// Build the API router with all routes
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Projects
        .route("/api/v1/projects", get(project::list_projects).post(project::create_project))
        .route("/api/v1/projects/{id}", get(project::get_project).put(project::update_project).delete(project::delete_project))
        // World
        .route("/api/v1/projects/{id}/world", get(world::get_world).put(world::update_world))
        // Entities
        .route("/api/v1/worlds/{id}/entities", get(entity::list_entities).post(entity::create_entity))
        .route("/api/v1/entities/{id}", get(entity::get_entity).put(entity::update_entity).delete(entity::delete_entity))
        // Characters (specialized entity)
        .route("/api/v1/worlds/{id}/characters", get(entity::list_characters).post(entity::create_character))
        .route("/api/v1/characters/{id}", get(entity::get_character).put(entity::update_entity).delete(entity::delete_entity))
        .route("/api/v1/characters/{id}/profile", get(entity::get_character_profile))
        .route("/api/v1/characters/{id}/state", get(entity::get_character_state))
        .route("/api/v1/characters/{id}/knowledge", get(entity::get_character_knowledge))
        .route("/api/v1/characters/{id}/relationships", get(entity::get_character_relationships))
        // Locations (specialized entity)
        .route("/api/v1/worlds/{id}/locations", get(entity::list_locations).post(entity::create_location))
        .route("/api/v1/locations/{id}", get(entity::get_location).put(entity::update_entity).delete(entity::delete_entity))
        .route("/api/v1/locations/{id}/entities", get(entity::get_location_entities))
        .route("/api/v1/locations/{id}/events", get(entity::get_location_events))
        // Factions (specialized entity)
        .route("/api/v1/worlds/{id}/factions", get(entity::list_factions).post(entity::create_faction))
        // Relations
        .route("/api/v1/worlds/{id}/relations", get(entity::list_relations).post(entity::create_relation))
        .route("/api/v1/relations/{id}", delete(entity::delete_relation))
        // Events
        .route("/api/v1/projects/{id}/events", get(history::list_events).post(history::create_event))
        // Facts
        .route("/api/v1/projects/{id}/facts", get(history::list_facts).post(history::create_fact))
        // Narrative
        .route("/api/v1/projects/{id}/narrative", get(narrative::list_nodes).post(narrative::create_node))
        .route("/api/v1/narrative/{id}", get(narrative::get_node).put(narrative::update_node).delete(narrative::delete_node))
        // Storylines
        .route("/api/v1/projects/{id}/storylines", get(narrative::list_storylines).post(narrative::create_storyline))
        .route("/api/v1/storylines/{id}", put(narrative::update_storyline))
        // Foreshadows
        .route("/api/v1/projects/{id}/foreshadows", get(narrative::list_foreshadows).post(narrative::create_foreshadow))
        .route("/api/v1/foreshadows/{id}", put(narrative::update_foreshadow))
        // Context
        .route("/api/v1/scenes/{id}/context", get(context::get_context))
        .route("/api/v1/scenes/{id}/context/build", post(context::build_context))
        .route("/api/v1/scenes/{id}/context/pin/{entity_id}", post(context::pin_entity).delete(context::unpin_entity))
        .route("/api/v1/scenes/{id}/context/exclude/{entity_id}", post(context::exclude_entity).delete(context::unexclude_entity))
        // Generation
        .route("/api/v1/projects/{id}/generations", get(generation::list_tasks).post(generation::create_task))
        .route("/api/v1/generations/{id}", get(generation::get_task))
        .route("/api/v1/generations/{id}/cancel", post(generation::cancel_task))
        // Proposals
        .route("/api/v1/projects/{id}/proposals", get(proposal::list_proposals))
        .route("/api/v1/proposals/{id}", get(proposal::get_proposal))
        .route("/api/v1/proposals/{id}/accept", post(proposal::accept_proposal))
        .route("/api/v1/proposals/{id}/reject", post(proposal::reject_proposal))
        .route("/api/v1/proposals/{id}/changes/{change_id}/accept", post(proposal::accept_change))
        .route("/api/v1/proposals/{id}/changes/{change_id}/reject", post(proposal::reject_change))
        // Validation
        .route("/api/v1/scenes/{id}/validate", post(validation::validate_scene))
        .route("/api/v1/proposals/{id}/validate", post(validation::validate_proposal))
        .route("/api/v1/worlds/{id}/validate", post(validation::validate_world))
        // Versions / History
        .route("/api/v1/entities/{id}/versions", get(history::list_versions))
        .route("/api/v1/entities/{id}/versions/{version}", get(history::get_version))
        .route("/api/v1/entities/{id}/versions/compare", get(history::compare_versions))
        // Health
        .route("/api/v1/health", get(health_check))
        .with_state(state)
        .layer(cors)
}

async fn health_check() -> &'static str {
    "OK"
}
