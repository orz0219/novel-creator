//! Domain Event Log - system event tracking

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain Event - system internal events (different from World Events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: Uuid,
    pub event_type: DomainEventType,
    pub project_id: Uuid,
    pub entity_id: Option<Uuid>,
    pub data: serde_json::Value,
    pub metadata: EventMetadata,
    pub created_at: DateTime<Utc>,
}

/// Domain Event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DomainEventType {
    // Entity events
    EntityCreated,
    EntityUpdated,
    EntityDeleted,

    // World events
    WorldCreated,
    WorldUpdated,

    // Narrative events
    NarrativeNodeCreated,
    NarrativeNodeUpdated,
    NarrativeNodeDeleted,
    SceneCommitted,

    // Knowledge events
    KnowledgeChanged,
    RevelationCreated,

    // Proposal events
    ProposalCreated,
    ProposalApproved,
    ProposalRejected,
    ProposalCommitted,

    // Validation events
    ValidationStarted,
    ValidationCompleted,
    ValidationFailed,

    // Job events
    JobCreated,
    JobStarted,
    JobCompleted,
    JobFailed,
    JobCancelled,

    // Generation events
    GenerationStarted,
    GenerationCompleted,
    GenerationFailed,

    // Context events
    ContextSnapshotCreated,

    // Custom events
    Custom(String),
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub source: String,
    pub user_id: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub tags: Vec<String>,
}

impl DomainEvent {
    /// Create a new domain event
    pub fn new(
        event_type: DomainEventType,
        project_id: Uuid,
        entity_id: Option<Uuid>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            project_id,
            entity_id,
            data,
            metadata: EventMetadata {
                source: "system".to_string(),
                user_id: None,
                correlation_id: None,
                tags: Vec::new(),
            },
            created_at: Utc::now(),
        }
    }

    /// Create with metadata
    pub fn with_metadata(
        event_type: DomainEventType,
        project_id: Uuid,
        entity_id: Option<Uuid>,
        data: serde_json::Value,
        metadata: EventMetadata,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            project_id,
            entity_id,
            data,
            metadata,
            created_at: Utc::now(),
        }
    }
}

/// Event Dispatcher - routes events to subscribers
pub struct EventDispatcher {
    subscribers: Vec<Box<dyn EventSubscriber>>,
}

/// Event Subscriber trait
pub trait EventSubscriber: Send + Sync {
    fn handle_event(&self, event: &DomainEvent) -> Result<()>;
    fn event_types(&self) -> Vec<DomainEventType>;
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    /// Add a subscriber
    pub fn add_subscriber(&mut self, subscriber: Box<dyn EventSubscriber>) {
        self.subscribers.push(subscriber);
    }

    /// Dispatch an event to all matching subscribers
    ///
    /// 不会因为单个 subscriber 失败而停止整个分发。
    /// 所有错误会被收集并返回。
    pub fn dispatch(&self, event: &DomainEvent) -> Result<()> {
        let mut errors = Vec::new();

        for subscriber in &self.subscribers {
            if subscriber.event_types().is_empty() || subscriber.event_types().contains(&event.event_type) {
                if let Err(e) = subscriber.handle_event(event) {
                    tracing::warn!("Event subscriber failed for event {:?}: {}", event.event_type, e);
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("{} event subscribers failed", errors.len()))
        }
    }
}

/// Audit Log - in-memory event storage (NOT persistent)
///
/// 注意：这是进程内的内存存储，不是持久化审计日志。
/// 对于持久化事件，请使用 system_events 表。
pub struct InMemoryAuditLog {
    events: Vec<DomainEvent>,
}

impl InMemoryAuditLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    /// Record an event (in-memory only)
    pub fn record(&mut self, event: DomainEvent) {
        self.events.push(event);
    }

    /// Get events by project
    pub fn get_by_project(&self, project_id: Uuid) -> Vec<&DomainEvent> {
        self.events.iter().filter(|e| e.project_id == project_id).collect()
    }

    /// Get events by type
    pub fn get_by_type(&self, event_type: &DomainEventType) -> Vec<&DomainEvent> {
        self.events.iter().filter(|e| e.event_type == *event_type).collect()
    }

    /// Get events by entity
    pub fn get_by_entity(&self, entity_id: Uuid) -> Vec<&DomainEvent> {
        self.events.iter().filter(|e| e.entity_id == Some(entity_id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event_creation() {
        let event = DomainEvent::new(
            DomainEventType::EntityCreated,
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            serde_json::json!({"name": "Test Entity"}),
        );

        assert_eq!(event.event_type, DomainEventType::EntityCreated);
        assert!(event.entity_id.is_some());
    }

    #[test]
    fn test_event_dispatcher() {
        struct TestSubscriber;

        impl EventSubscriber for TestSubscriber {
            fn handle_event(&self, _event: &DomainEvent) -> Result<()> {
                Ok(())
            }

            fn event_types(&self) -> Vec<DomainEventType> {
                vec![DomainEventType::EntityCreated]
            }
        }

        let mut dispatcher = EventDispatcher::new();
        dispatcher.add_subscriber(Box::new(TestSubscriber));

        let event = DomainEvent::new(
            DomainEventType::EntityCreated,
            Uuid::new_v4(),
            None,
            serde_json::json!({}),
        );

        assert!(dispatcher.dispatch(&event).is_ok());
    }

    #[test]
    fn test_audit_log() {
        let mut log = InMemoryAuditLog::new();
        let project_id = Uuid::new_v4();

        log.record(DomainEvent::new(
            DomainEventType::EntityCreated,
            project_id,
            None,
            serde_json::json!({}),
        ));

        log.record(DomainEvent::new(
            DomainEventType::EntityUpdated,
            project_id,
            None,
            serde_json::json!({}),
        ));

        let events = log.get_by_project(project_id);
        assert_eq!(events.len(), 2);
    }
}
