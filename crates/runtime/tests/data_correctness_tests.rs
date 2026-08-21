//! Tests for data correctness and runtime hardening
//!
//! Covers:
//! - Approved-only commit
//! - Project isolation
//! - Transaction atomicity
//! - Optimistic concurrency
//! - Required context failure
//! - Optional context failure
//! - Validator batch query
//! - Canon Rule structured validation

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chrono::Utc;
    use domain::*;
    use sqlx::PgPool;
    use uuid::Uuid;


    fn build_context_engine(pool: sqlx::PgPool) -> runtime::context_engine::ContextEngine {
        let deps = runtime::context_engine::ContextEngineDeps {
            narrative: std::sync::Arc::new(db::runtime_ports::DbNarrativePort::new(pool.clone())),
            entity: std::sync::Arc::new(db::runtime_ports::DbEntityPort::new(pool.clone())),
            state: std::sync::Arc::new(db::runtime_ports::DbStatePort::new(pool.clone())),
            knowledge: std::sync::Arc::new(db::runtime_ports::DbKnowledgePort::new(pool.clone())),
            relation: std::sync::Arc::new(db::runtime_ports::DbRelationPort::new(pool.clone())),
            event: std::sync::Arc::new(db::runtime_ports::DbEventPort::new(pool.clone())),
            canon: std::sync::Arc::new(db::runtime_ports::DbCanonRulePort::new(pool.clone())),
            snapshot: std::sync::Arc::new(db::runtime_ports::DbContextSnapshotPort::new(pool.clone())),
        };
        runtime::context_engine::ContextEngine::new(deps)
    }

    fn build_validator(pool: sqlx::PgPool) -> runtime::validator::Validator {
        let deps = runtime::validator::ValidatorDeps {
            entity: std::sync::Arc::new(db::runtime_ports::DbEntityPort::new(pool.clone())),
            validation: std::sync::Arc::new(db::runtime_ports::DbValidationPort::new(pool.clone())),
            approval: std::sync::Arc::new(db::runtime_ports::DbApprovalPort::new(pool.clone())),
            canon: std::sync::Arc::new(db::runtime_ports::DbCanonRulePort::new(pool.clone())),
            proposed_change: std::sync::Arc::new(db::runtime_ports::DbProposedChangeQueryPort::new(pool.clone())),
        };
        runtime::validator::Validator::new(deps)
    }

    fn build_state_committer(pool: sqlx::PgPool) -> runtime::state_committer::DbStateCommitter {
        runtime::state_committer::DbStateCommitter::new(std::sync::Arc::new(db::runtime_ports::DbStateCommitterPort::new(pool)))
    }

    // Helper to create a test pool (requires DATABASE_URL)
    async fn test_pool() -> Result<PgPool> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string());
        let pool = sqlx::PgPool::connect(&database_url).await?;
        Ok(pool)
    }

    // Helper to create a test project
    async fn create_test_project(pool: &PgPool) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO project (id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(format!("Test Project {}", id))
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(pool)
            .await?;
        Ok(id)
    }

    // Ensure a generation_task (and its required skill) exists for the given task_id.
    // proposed_change.task_id and validation_run.task_id both FK to generation_task(id);
    // the tests mint random task_ids, so we must create the referenced rows or the
    // insert violates the foreign key.
    async fn ensure_task(pool: &PgPool, project_id: Uuid, task_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO generation_task (id, project_id, task_type, status, created_at) \
             VALUES ($1, $2, 'general', 'Pending', NOW()) ON CONFLICT (id) DO NOTHING",
        )
        .bind(task_id)
        .bind(project_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Helper to create a test entity
    async fn create_test_entity(pool: &PgPool, project_id: Uuid) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let world_id = Uuid::new_v4();
        // Ensure entity_type "Character" exists. entity_type.name is UNIQUE,
        // so reuse the existing row when present (find-or-create) instead of
        // blindly INSERTing (which collides under concurrent test execution).
        let entity_type_id = match sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM entity_type WHERE name = $1",
        )
        .bind("Character")
        .fetch_optional(pool)
        .await?
        {
            Some(id) => id,
            None => {
                let new_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO entity_type (id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)",
                )
                .bind(new_id)
                .bind("Character")
                .bind(Utc::now())
                .bind(Utc::now())
                .execute(pool)
                .await?;
                new_id
            }
        };

        // Ensure world exists
        sqlx::query("INSERT INTO world (id, project_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING")
            .bind(world_id)
            .bind(project_id)
            .bind("Test World")
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(pool)
            .await?;

        sqlx::query("INSERT INTO entity (id, project_id, world_id, entity_type_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(id)
            .bind(project_id)
            .bind(world_id)
            .bind(entity_type_id)
            .bind("Test Character")
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(pool)
            .await?;
        Ok(id)
    }

    // ============================================================
    // Test 1: Rejected Change Cannot Commit
    // ============================================================
    #[tokio::test]
    async fn test_rejected_change_cannot_commit() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;

        let state_committer = build_state_committer(pool.clone());
        let val_repo = db::repos::validation_repo::ValidationRepo::new(pool.clone());

        // P1-2: 先在 DB 中创建 ProposedChange，然后直接更新状态为 Rejected
        let task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, task_id).await?;
        let change = val_repo.create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "Test change",
            serde_json::json!({"state_key": "test_key", "new_value": "test_value"}),
        ).await?;

        // 直接更新为 Rejected 状态（跳过状态机验证，因为是测试）
        sqlx::query("UPDATE proposed_change SET status = 'Rejected' WHERE id = $1")
            .bind(change.id)
            .execute(&pool)
            .await?;

        let result = state_committer.commit(project_id, &[change.id]).await;
        assert!(result.is_err(), "Rejected change should fail to commit");
        assert!(result.unwrap_err().to_string().contains("status is Rejected"));

        Ok(())
    }

    // ============================================================
    // Test 2: Pending Change Cannot Commit
    // ============================================================
    #[tokio::test]
    async fn test_pending_change_cannot_commit() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;

        let state_committer = build_state_committer(pool.clone());
        let val_repo = db::repos::validation_repo::ValidationRepo::new(pool.clone());

        // P1-2: 先在 DB 中创建 ProposedChange（状态为 Pending）
        let task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, task_id).await?;
        let change = val_repo.create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "Test change",
            serde_json::json!({"state_key": "test_key", "new_value": "test_value"}),
        ).await?;

        let result = state_committer.commit(project_id, &[change.id]).await;
        assert!(result.is_err(), "Pending change should fail to commit");
        assert!(result.unwrap_err().to_string().contains("status is Draft"));

        Ok(())
    }

    // ============================================================
    // Test 3: Approved Change Commits Atomically
    // ============================================================
    #[tokio::test]
    async fn test_approved_change_commits_atomically() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;

        // Set initial state
        let state_repo = db::repos::state_repo::StateRepo::new(pool.clone());
        state_repo.upsert_state(project_id, entity_id, "location", serde_json::json!("city"), None).await?;

        let state_committer = build_state_committer(pool.clone());
        let val_repo = db::repos::validation_repo::ValidationRepo::new(pool.clone());

        // P1-2: 先在 DB 中创建 ProposedChange，然后更新状态为 Approved
        let task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, task_id).await?;
        let change = val_repo.create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "Move to forest",
            serde_json::json!({"state_key": "location", "new_value": "forest"}),
        ).await?;

        // 更新状态为 Approved（跳过状态机验证，因为是测试）
        sqlx::query("UPDATE proposed_change SET status = 'Approved' WHERE id = $1")
            .bind(change.id)
            .execute(&pool)
            .await?;

        let result = state_committer.commit(project_id, &[change.id]).await;
        assert!(result.is_ok(), "Approved change should commit successfully");

        let response = result.unwrap();
        assert_eq!(response.results.len(), 1, "Should have 1 commit result");
        assert_eq!(response.events.len(), 1, "Should have 1 event");

        // Verify state was updated
        let state = state_repo.get_current_state(project_id, entity_id, "location").await?;
        assert!(state.is_some());
        assert_eq!(state.unwrap().state_value, serde_json::json!("forest"));

        Ok(())
    }

    // ============================================================
    // Test 4: Cross Project State Access
    // ============================================================
    #[tokio::test]
    async fn test_cross_project_state_access() -> Result<()> {
        let pool = test_pool().await?;
        let project_a = create_test_project(&pool).await?;
        let project_b = create_test_project(&pool).await?;
        let entity_a = create_test_entity(&pool, project_a).await?;

        let state_repo = db::repos::state_repo::StateRepo::new(pool.clone());

        // Set state in project A
        state_repo.upsert_state(project_a, entity_a, "test_key", serde_json::json!("value_a"), None).await?;

        // Try to access from project B - should return None
        let state = state_repo.get_current_state(project_b, entity_a, "test_key").await?;
        assert!(state.is_none(), "Cross-project access should return None");

        Ok(())
    }

    // ============================================================
    // Test 5: Optimistic Concurrency
    // ============================================================
    #[tokio::test]
    async fn test_optimistic_concurrency() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;

        let state_repo = db::repos::state_repo::StateRepo::new(pool.clone());

        // Set initial state (version 1)
        state_repo.upsert_state(project_id, entity_id, "test_key", serde_json::json!("initial"), None).await?;

        // Get current state to get version
        let current = state_repo.get_current_state(project_id, entity_id, "test_key").await?.unwrap();
        assert_eq!(current.version, 1);

        // Update with correct version (should succeed)
        state_repo.upsert_state(project_id, entity_id, "test_key", serde_json::json!("updated"), Some(1)).await?;

        // Verify version incremented
        let updated = state_repo.get_current_state(project_id, entity_id, "test_key").await?.unwrap();
        assert_eq!(updated.version, 2);
        assert_eq!(updated.state_value, serde_json::json!("updated"));

        // Try to update with old version (should fail)
        let result = state_repo.upsert_state(project_id, entity_id, "test_key", serde_json::json!("conflict"), Some(1)).await;
        assert!(result.is_err(), "Update with old version should fail");

        // Verify state unchanged
        let final_state = state_repo.get_current_state(project_id, entity_id, "test_key").await?.unwrap();
        assert_eq!(final_state.state_value, serde_json::json!("updated"));
        assert_eq!(final_state.version, 2);

        Ok(())
    }

    // ============================================================
    // Test 6: Required Context Failure
    // ============================================================
    #[tokio::test]
    async fn test_required_context_failure() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;

        // Try to build context for non-existent scene
        let engine = build_context_engine(pool.clone());
        let result = engine.build_context(project_id, Uuid::new_v4(), 10000, None).await;

        // Should fail because scene doesn't exist
        assert!(result.is_err(), "Required context failure should propagate error");

        Ok(())
    }

    // ============================================================
    // Test 7: Optional Context Failure
    // ============================================================
    #[tokio::test]
    async fn test_optional_context_failure() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;

        // Create a minimal scene
        let world_id = Uuid::new_v4();
        sqlx::query("INSERT INTO world (id, project_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)")
            .bind(world_id)
            .bind(project_id)
            .bind("Test World")
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&pool)
            .await?;

        let scene_id = Uuid::new_v4();
        sqlx::query("INSERT INTO narrative_node (id, project_id, world_id, node_type, title, sort_order, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(scene_id)
            .bind(project_id)
            .bind(world_id)
            .bind("Scene")
            .bind("Test Scene")
            .bind(0)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&pool)
            .await?;

        // Build context - optional relations/events should not fail the whole build
        let engine = build_context_engine(pool.clone());
        let result = engine.build_context(project_id, scene_id, 10000, None).await;

        // Should succeed even if optional queries fail
        assert!(result.is_ok(), "Optional context failure should not fail the build");

        Ok(())
    }

    // ============================================================
    // Test 8: Validator Batch Query
    // ============================================================
    #[tokio::test]
    async fn test_validator_batch_query() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;

        // Create multiple entities
        let mut entity_ids = Vec::new();
        for _ in 0..5 {
            let entity_id = create_test_entity(&pool, project_id).await?;
            entity_ids.push(entity_id);
        }

        // Create multiple proposed changes
        let mut changes = Vec::new();
        for entity_id in &entity_ids {
            let task_id = Uuid::new_v4();
            ensure_task(&pool, project_id, task_id).await?;
            changes.push(ProposedChange {
                id: Uuid::new_v4(),
                project_id,
                task_id: Some(task_id),
                change_type: ProposedChangeType::StateChange,
                target_entity_id: *entity_id,
                description: "Test change".to_string(),
                payload: serde_json::json!({"state_key": "test_key", "new_value": "test_value"}),
                status: ProposedChangeStatus::Draft,
                created_at: Utc::now(),
                resolved_at: None,
            });
        }

        // Validate - should use batch queries
        let validator = build_validator(pool.clone());
        let run_task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, run_task_id).await?;
        let run = validator.validate_changes(project_id, run_task_id, &changes).await?;

        // All should be approved (no canon rules, valid entities)
        assert_eq!(run.changes_approved, 5);
        assert_eq!(run.changes_rejected, 0);

        Ok(())
    }

    // ============================================================
    // Test 9: Canon Rule Structured Validation
    // ============================================================
    #[tokio::test]
    async fn test_canon_rule_structured() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;
        let world_id = Uuid::new_v4();

        // Create world
        sqlx::query("INSERT INTO world (id, project_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)")
            .bind(world_id)
            .bind(project_id)
            .bind("Test World")
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&pool)
            .await?;

        // Create canon rule: element NOT_IN ["fire"]
        sqlx::query("INSERT INTO canon_rule (id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)")
            .bind(Uuid::new_v4())
            .bind(project_id)
            .bind(world_id)
            .bind("RULE-0")
            .bind("Fire element is forbidden")
            .bind("element")
            .bind("Reject")
            .bind(serde_json::json!({"state_key": "element", "operator": "NOT_IN", "value": ["fire"]}))
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&pool)
            .await?;

        let validator = build_validator(pool.clone());

        // Test 1: element = fire (should be rejected)
        let fire_task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, fire_task_id).await?;
        let fire_change = ProposedChange {
            id: Uuid::new_v4(),
            project_id,
            task_id: Some(fire_task_id),
            change_type: ProposedChangeType::StateChange,
            target_entity_id: entity_id,
            description: "Set fire element".to_string(),
            payload: serde_json::json!({"state_key": "element", "new_value": "fire"}),
            status: ProposedChangeStatus::Draft,
            created_at: Utc::now(),
            resolved_at: None,
        };

        let fire_run_task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, fire_run_task_id).await?;
        // Persist the in-memory proposed change so validation issues can reference it.
        sqlx::query("INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'Draft',$8) ON CONFLICT (id) DO NOTHING")
            .bind(fire_change.id).bind(project_id).bind(fire_task_id).bind("StateChange")
            .bind(entity_id).bind(&fire_change.description).bind(&fire_change.payload).bind(fire_change.created_at)
            .execute(&pool).await?;
        let run = validator.validate_changes(project_id, fire_run_task_id, &[fire_change]).await?;
        assert_eq!(run.changes_rejected, 1, "Fire element should be rejected");

        // Test 2: element = water (should be approved)
        let water_task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, water_task_id).await?;
        let water_change = ProposedChange {
            id: Uuid::new_v4(),
            project_id,
            task_id: Some(water_task_id),
            change_type: ProposedChangeType::StateChange,
            target_entity_id: entity_id,
            description: "Set water element".to_string(),
            payload: serde_json::json!({"state_key": "element", "new_value": "water"}),
            status: ProposedChangeStatus::Draft,
            created_at: Utc::now(),
            resolved_at: None,
        };

        let water_run_task_id = Uuid::new_v4();
        ensure_task(&pool, project_id, water_run_task_id).await?;
        // Persist the in-memory proposed change so validation issues can reference it.
        sqlx::query("INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'Draft',$8) ON CONFLICT (id) DO NOTHING")
            .bind(water_change.id).bind(project_id).bind(water_task_id).bind("StateChange")
            .bind(entity_id).bind(&water_change.description).bind(&water_change.payload).bind(water_change.created_at)
            .execute(&pool).await?;
        let run = validator.validate_changes(project_id, water_run_task_id, &[water_change]).await?;
        assert_eq!(run.changes_approved, 1, "Water element should be approved");

        Ok(())
    }

    // ============================================================
    // Test 10: State Machine - Valid Transitions
    // ============================================================
    #[test]
    fn test_state_machine_valid_transitions() {
        use ProposedChangeStatus::*;

        // Normal flow
        assert!(Draft.can_transition_to(&Validating));
        assert!(Validating.can_transition_to(&Valid));
        assert!(Valid.can_transition_to(&Approved));
        assert!(Approved.can_transition_to(&Committed));
        assert!(Committed.can_transition_to(&Applied));

        // Pending approval flow
        assert!(Valid.can_transition_to(&PendingApproval));
        assert!(PendingApproval.can_transition_to(&Approved));
        assert!(PendingApproval.can_transition_to(&Rejected));

        // Conflicted transitions
        assert!(Validating.can_transition_to(&Conflicted));
        assert!(Valid.can_transition_to(&Conflicted));
        assert!(Approved.can_transition_to(&Conflicted));
        assert!(PendingApproval.can_transition_to(&Conflicted));

        // Failed transition
        assert!(Committed.can_transition_to(&Failed));

        // Expired transition
        assert!(PendingApproval.can_transition_to(&Expired));
    }

    // ============================================================
    // Test 11: State Machine - Invalid Transitions
    // ============================================================
    #[test]
    fn test_state_machine_invalid_transitions() {
        use ProposedChangeStatus::*;

        // Cannot go backwards
        assert!(!Valid.can_transition_to(&Draft));
        assert!(!Approved.can_transition_to(&Validating));
        assert!(!Committed.can_transition_to(&Valid));
        assert!(!Applied.can_transition_to(&Committed));

        // Cannot skip steps
        assert!(!Draft.can_transition_to(&Approved));
        assert!(!Draft.can_transition_to(&Committed));
        assert!(!Draft.can_transition_to(&Applied));
        assert!(!Validating.can_transition_to(&Committed));
        assert!(!Validating.can_transition_to(&Applied));
        assert!(!Valid.can_transition_to(&Applied));

        // Terminal states cannot transition
        assert!(!Applied.can_transition_to(&Draft));
        assert!(!Applied.can_transition_to(&Validating));
        assert!(!Applied.can_transition_to(&Valid));
        assert!(!Applied.can_transition_to(&Approved));
        assert!(!Applied.can_transition_to(&Committed));
        assert!(!Rejected.can_transition_to(&Draft));
        assert!(!Rejected.can_transition_to(&Approved));
        assert!(!Invalid.can_transition_to(&Draft));
        assert!(!Invalid.can_transition_to(&Valid));
        assert!(!Failed.can_transition_to(&Draft));
        assert!(!Failed.can_transition_to(&Committed));
        assert!(!Expired.can_transition_to(&Draft));
        assert!(!Expired.can_transition_to(&Approved));
        assert!(!Conflicted.can_transition_to(&Draft));
        assert!(!Conflicted.can_transition_to(&Approved));

        // Cannot commit from wrong states
        assert!(!Draft.can_transition_to(&Committed));
        assert!(!Validating.can_transition_to(&Committed));
        assert!(!PendingApproval.can_transition_to(&Committed));
        assert!(!Rejected.can_transition_to(&Committed));
        assert!(!Invalid.can_transition_to(&Committed));
    }

    // ============================================================
    // Test 12: Event Dispatcher Error Handling
    // ============================================================
    #[test]
    fn test_event_dispatcher_error_handling() {
        use domain::events::*;

        struct FailingSubscriber;
        impl EventSubscriber for FailingSubscriber {
            fn handle_event(&self, _event: &DomainEvent) -> anyhow::Result<()> {
                Err(anyhow::anyhow!("subscriber failed"))
            }
            fn event_types(&self) -> Vec<DomainEventType> {
                vec![DomainEventType::EntityCreated]
            }
        }

        struct SuccessSubscriber;
        impl EventSubscriber for SuccessSubscriber {
            fn handle_event(&self, _event: &DomainEvent) -> anyhow::Result<()> {
                Ok(())
            }
            fn event_types(&self) -> Vec<DomainEventType> {
                vec![DomainEventType::EntityCreated]
            }
        }

        let mut dispatcher = EventDispatcher::new();
        dispatcher.add_subscriber(Box::new(FailingSubscriber));
        dispatcher.add_subscriber(Box::new(SuccessSubscriber));

        let event = DomainEvent::new(
            DomainEventType::EntityCreated,
            Uuid::new_v4(),
            None,
            serde_json::json!({}),
        );

        // Should fail because one subscriber failed, but both should be called
        let result = dispatcher.dispatch(&event);
        assert!(result.is_err());
    }

    // ============================================================
    // Test 13: InMemoryAuditLog
    // ============================================================
    #[test]
    fn test_in_memory_audit_log() {
        use domain::events::*;

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