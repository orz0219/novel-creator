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

    // Ensure a generation_task exists for the given task_id.
    // proposed_change.task_id FKs to generation_task(id); tests mint random task_ids.
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

        // find-or-create entity_type "Character" (name is UNIQUE)
        let entity_type_id = match sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM entity_type WHERE name = $1",
        )
        .bind("Character")
        .fetch_optional(pool)
        .await?
        {
            Some(eid) => eid,
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

}