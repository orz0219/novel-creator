//! Integration Tests - 数据库事务和跨项目隔离
//!
//! 需要 PostgreSQL 环境运行
//! 设置 DATABASE_URL 环境变量

#[cfg(test)]
mod integration_tests {
    use anyhow::Result;
    use chrono::Utc;
    use domain::*;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn test_pool() -> Result<PgPool> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string());
        let pool = sqlx::PgPool::connect(&database_url).await?;
        Ok(pool)
    }

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

    async fn create_test_entity(pool: &PgPool, project_id: Uuid) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let world_id = Uuid::new_v4();
        let entity_type_id = Uuid::new_v4();

        sqlx::query("INSERT INTO entity_type (id, name, created_at, updated_at) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING")
            .bind(entity_type_id)
            .bind("Character")
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(pool)
            .await?;

        sqlx::query("INSERT INTO world (id, project_id, name, is_main, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING")
            .bind(world_id)
            .bind(project_id)
            .bind("Test World")
            .bind(true)
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
    // P2-10: Transaction Atomicity Test
    // ============================================================
    #[tokio::test]
    async fn test_commit_atomicity() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;

        // 创建初始状态
        let state_repo = db::repos::state_repo::StateRepo::new(pool.clone());
        state_repo.upsert_state(project_id, entity_id, "hp", serde_json::json!(100), None).await?;

        // 创建两个 Approved 的 ProposedChange
        let val_repo = db::repos::validation_repo::ValidationRepo::new(pool.clone());

        let change1 = val_repo.create_proposed_change(
            project_id,
            Uuid::new_v4(),
            ProposedChangeType::StateChange,
            entity_id,
            "Take damage",
            serde_json::json!({"state_key": "hp", "new_value": 80}),
        ).await?;

        let change2 = val_repo.create_proposed_change(
            project_id,
            Uuid::new_v4(),
            ProposedChangeType::StateChange,
            entity_id,
            "Heal",
            serde_json::json!({"state_key": "hp", "new_value": 90}),
        ).await?;

        // 更新状态为 Approved
        sqlx::query("UPDATE proposed_change SET status = 'Approved' WHERE id IN ($1, $2)")
            .bind(change1.id)
            .bind(change2.id)
            .execute(&pool)
            .await?;

        // 执行 commit
        let state_committer = runtime::state_committer::DbStateCommitter::new(pool.clone());
        let result = state_committer.commit(project_id, &[change1.id, change2.id]).await;

        // 验证结果
        assert!(result.is_ok(), "Commit should succeed");

        let response = result.unwrap();
        assert_eq!(response.results.len(), 2, "Should have 2 results");
        assert_eq!(response.events.len(), 2, "Should have 2 events");

        // 验证最终状态
        let final_state = state_repo.get_current_state(project_id, entity_id, "hp").await?;
        assert!(final_state.is_some());
        assert_eq!(final_state.unwrap().state_value, serde_json::json!(90));

        // 验证 proposal 状态
        let pc1 = val_repo.get_proposed_change_by_id(change1.id).await?;
        assert_eq!(pc1.unwrap().status, ProposedChangeStatus::Applied);

        let pc2 = val_repo.get_proposed_change_by_id(change2.id).await?;
        assert_eq!(pc2.unwrap().status, ProposedChangeStatus::Applied);

        // 清理
        sqlx::query("DELETE FROM state_change WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM current_state WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM proposed_change WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM system_event WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM entity WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM world WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM project WHERE id = $1").bind(project_id).execute(&pool).await?;

        Ok(())
    }

    // ============================================================
    // P2-11: Cross-Project Isolation Test
    // ============================================================
    #[tokio::test]
    async fn test_cross_project_isolation() -> Result<()> {
        let pool = test_pool().await?;
        let project_a = create_test_project(&pool).await?;
        let project_b = create_test_project(&pool).await?;
        let entity_a = create_test_entity(&pool, project_a).await?;
        let entity_b = create_test_entity(&pool, project_b).await?;

        let state_repo = db::repos::state_repo::StateRepo::new(pool.clone());

        // 在 Project A 中设置状态
        state_repo.upsert_state(project_a, entity_a, "hp", serde_json::json!(100), None).await?;

        // 在 Project B 中设置状态
        state_repo.upsert_state(project_b, entity_b, "hp", serde_json::json!(200), None).await?;

        // 验证 Project A 只能看到自己的状态
        let state_a = state_repo.get_current_state(project_a, entity_a, "hp").await?;
        assert!(state_a.is_some());
        assert_eq!(state_a.unwrap().state_value, serde_json::json!(100));

        // 验证 Project B 只能看到自己的状态
        let state_b = state_repo.get_current_state(project_b, entity_b, "hp").await?;
        assert!(state_b.is_some());
        assert_eq!(state_b.unwrap().state_value, serde_json::json!(200));

        // 验证 Project A 不能访问 Project B 的实体
        let cross_state = state_repo.get_current_state(project_a, entity_b, "hp").await?;
        assert!(cross_state.is_none(), "Cross-project access should return None");

        // 验证 Project B 不能访问 Project A 的实体
        let cross_state2 = state_repo.get_current_state(project_b, entity_a, "hp").await?;
        assert!(cross_state2.is_none(), "Cross-project access should return None");

        // 验证 EntityRepo 的 project isolation
        let entity_repo = db::repos::entity_repo::EntityRepo::new(pool.clone());

        let entity = entity_repo.get_by_id_with_project(project_a, entity_a).await?;
        assert!(entity.is_some(), "Should find entity in correct project");

        let entity_cross = entity_repo.get_by_id_with_project(project_a, entity_b).await?;
        assert!(entity_cross.is_none(), "Should not find entity in wrong project");

        // 清理
        sqlx::query("DELETE FROM current_state WHERE project_id IN ($1, $2)").bind(project_a).bind(project_b).execute(&pool).await?;
        sqlx::query("DELETE FROM entity WHERE project_id IN ($1, $2)").bind(project_a).bind(project_b).execute(&pool).await?;
        sqlx::query("DELETE FROM world WHERE project_id IN ($1, $2)").bind(project_a).bind(project_b).execute(&pool).await?;
        sqlx::query("DELETE FROM project WHERE id IN ($1, $2)").bind(project_a).bind(project_b).execute(&pool).await?;

        Ok(())
    }

    // ============================================================
    // CAS Conflict Test
    // ============================================================
    #[tokio::test]
    async fn test_cas_conflict_rollback() -> Result<()> {
        let pool = test_pool().await?;
        let project_id = create_test_project(&pool).await?;
        let entity_id = create_test_entity(&pool, project_id).await?;

        let state_repo = db::repos::state_repo::StateRepo::new(pool.clone());

        // 创建初始状态 (version 1)
        state_repo.upsert_state(project_id, entity_id, "hp", serde_json::json!(100), None).await?;

        // 创建两个 Approved 的 ProposedChange
        let val_repo = db::repos::validation_repo::ValidationRepo::new(pool.clone());

        let change1 = val_repo.create_proposed_change(
            project_id,
            Uuid::new_v4(),
            ProposedChangeType::StateChange,
            entity_id,
            "Change 1",
            serde_json::json!({"state_key": "hp", "new_value": 80}),
        ).await?;

        let change2 = val_repo.create_proposed_change(
            project_id,
            Uuid::new_v4(),
            ProposedChangeType::StateChange,
            entity_id,
            "Change 2",
            serde_json::json!({"state_key": "hp", "new_value": 90}),
        ).await?;

        // 更新状态为 Approved
        sqlx::query("UPDATE proposed_change SET status = 'Approved' WHERE id IN ($1, $2)")
            .bind(change1.id)
            .bind(change2.id)
            .execute(&pool)
            .await?;

        // 模拟并发：手动修改状态版本
        // change1 会成功，change2 应该因为 CAS 冲突而失败
        let state_committer = runtime::state_committer::DbStateCommitter::new(pool.clone());
        let result = state_committer.commit(project_id, &[change1.id, change2.id]).await;

        // 由于两个 change 都修改同一个 state_key，第二个应该因为 CAS 冲突失败
        // 整个事务应该回滚
        assert!(result.is_err(), "Should fail due to CAS conflict");

        // 验证状态没有改变（事务回滚）
        let final_state = state_repo.get_current_state(project_id, entity_id, "hp").await?;
        assert!(final_state.is_some());
        assert_eq!(final_state.unwrap().state_value, serde_json::json!(100));

        // 验证 proposal 状态没有改变
        let pc1 = val_repo.get_proposed_change_by_id(change1.id).await?;
        assert_eq!(pc1.unwrap().status, ProposedChangeStatus::Approved);

        // 清理
        sqlx::query("DELETE FROM proposed_change WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM current_state WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM entity WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM world WHERE project_id = $1").bind(project_id).execute(&pool).await?;
        sqlx::query("DELETE FROM project WHERE id = $1").bind(project_id).execute(&pool).await?;

        Ok(())
    }
}