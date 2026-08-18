//! Story Contract - Volume/Arc Level Completion Requirements
//!
//! Defines what must happen for a Volume/Arc to be considered complete.
//! Writer uses this to know "what's still missing for this volume's mission".

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Story Contract - Defines completion requirements for Volume/Arc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryContract {
    pub id: Uuid,
    pub project_id: Uuid,
    /// Associated narrative node ID (Volume or Arc)
    pub narrative_node_id: Uuid,
    /// Mission description
    pub mission: Option<String>,
    /// List of objectives
    pub objectives: Vec<String>,
    /// Required events that must occur
    pub required_events: Vec<String>,
    /// Required revelations
    pub required_revelations: Vec<String>,
    /// Required character changes
    pub required_character_changes: Vec<String>,
    /// Required world changes
    pub required_world_changes: Vec<String>,
    /// Forbidden events
    pub forbidden_events: Vec<String>,
    /// Exit conditions
    pub exit_conditions: Vec<String>,
    /// Completion progress (0.0 - 1.0)
    pub completion_progress: f64,
    /// Completed events
    pub completed_events: Vec<String>,
    /// Completed character changes
    pub completed_character_changes: Vec<String>,
    /// Completed world changes
    pub completed_world_changes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Story Contract creation request
#[derive(Debug, Clone)]
pub struct CreateStoryContractRequest {
    pub project_id: Uuid,
    pub narrative_node_id: Uuid,
    pub mission: Option<String>,
    pub objectives: Vec<String>,
    pub required_events: Vec<String>,
    pub required_revelations: Vec<String>,
    pub required_character_changes: Vec<String>,
    pub required_world_changes: Vec<String>,
    pub forbidden_events: Vec<String>,
    pub exit_conditions: Vec<String>,
}

impl StoryContract {
    /// Create a new story contract
    pub fn new(request: CreateStoryContractRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id: request.project_id,
            narrative_node_id: request.narrative_node_id,
            mission: request.mission,
            objectives: request.objectives,
            required_events: request.required_events,
            required_revelations: request.required_revelations,
            required_character_changes: request.required_character_changes,
            required_world_changes: request.required_world_changes,
            forbidden_events: request.forbidden_events,
            exit_conditions: request.exit_conditions,
            completion_progress: 0.0,
            completed_events: Vec::new(),
            completed_character_changes: Vec::new(),
            completed_world_changes: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Mark event as completed
    pub fn mark_event_completed(&mut self, event: &str) {
        if !self.completed_events.contains(&event.to_string()) {
            self.completed_events.push(event.to_string());
            self.update_progress();
        }
    }

    /// Mark character change as completed
    pub fn mark_character_change_completed(&mut self, change: &str) {
        if !self.completed_character_changes.contains(&change.to_string()) {
            self.completed_character_changes.push(change.to_string());
            self.update_progress();
        }
    }

    /// Mark world change as completed
    pub fn mark_world_change_completed(&mut self, change: &str) {
        if !self.completed_world_changes.contains(&change.to_string()) {
            self.completed_world_changes.push(change.to_string());
            self.update_progress();
        }
    }

    /// Update completion progress
    fn update_progress(&mut self) {
        let total = self.required_events.len() + self.required_character_changes.len() + self.required_world_changes.len();
        let completed = self.completed_events.len() + self.completed_character_changes.len() + self.completed_world_changes.len();
        
        self.completion_progress = if total > 0 {
            completed as f64 / total as f64
        } else {
            1.0
        };
        self.updated_at = Utc::now();
    }

    /// Check exit conditions
    pub fn check_exit_conditions(&self) -> Vec<String> {
        let mut unmet = Vec::new();
        
        for event in &self.required_events {
            if !self.completed_events.contains(event) {
                unmet.push(format!("Required event not completed: {}", event));
            }
        }
        
        for change in &self.required_character_changes {
            if !self.completed_character_changes.contains(change) {
                unmet.push(format!("Required character change not completed: {}", change));
            }
        }
        
        for change in &self.required_world_changes {
            if !self.completed_world_changes.contains(change) {
                unmet.push(format!("Required world change not completed: {}", change));
            }
        }
        
        unmet
    }

    /// Check if fully complete
    pub fn is_complete(&self) -> bool {
        self.check_exit_conditions().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_story_contract_creation() {
        let contract = StoryContract::new(CreateStoryContractRequest {
            project_id: Uuid::new_v4(),
            narrative_node_id: Uuid::new_v4(),
            mission: Some("Let the protagonist leave hometown".to_string()),
            objectives: vec!["Get golden finger".to_string(), "Encounter enemy".to_string()],
            required_events: vec!["Get golden finger".to_string()],
            required_revelations: vec![],
            required_character_changes: vec!["Build core relationship".to_string()],
            required_world_changes: vec!["Leave Greenstone Village".to_string()],
            forbidden_events: vec![],
            exit_conditions: vec![],
        });
        
        assert_eq!(contract.completion_progress, 0.0);
        assert!(!contract.is_complete());
    }

    #[test]
    fn test_story_contract_completion() {
        let mut contract = StoryContract::new(CreateStoryContractRequest {
            project_id: Uuid::new_v4(),
            narrative_node_id: Uuid::new_v4(),
            mission: None,
            objectives: vec![],
            required_events: vec!["event1".to_string()],
            required_revelations: vec![],
            required_character_changes: vec!["change1".to_string()],
            required_world_changes: vec![],
            forbidden_events: vec![],
            exit_conditions: vec![],
        });
        
        contract.mark_event_completed("event1");
        contract.mark_character_change_completed("change1");
        
        assert!(contract.is_complete());
        assert_eq!(contract.completion_progress, 1.0);
    }
}
