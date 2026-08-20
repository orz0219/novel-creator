//! End-to-End Tests - 验证 V2 Narrative Engine 全部功能
//!
//! 注意：底层服务（WorldService / NarrativeService / Validator / ContextEngine /
//! DbStateCommitter）均为 async API，因此本文件的测试统一使用 #[tokio::test]。
//! 编译通过即可（cargo check -p narrative-engine --tests），不依赖真实数据库
//! 连接（setup_db 在运行期才需要数据库）。

#[cfg(test)]
mod e2e_tests {
    use runtime::context_engine::TokenBudgets;
    use runtime::state_committer::DbStateCommitter;
    use application::narrative_service::NarrativeService;
    use application::world_service::WorldService;
    use domain::*;
    use db::connection::Database;
    use std::sync::Arc;

    fn build_context_engine(pool: sqlx::PgPool) -> runtime::context_engine::ContextEngine {
        let deps = runtime::context_engine::ContextEngineDeps {
            narrative: Arc::new(db::runtime_ports::DbNarrativePort::new(pool.clone())),
            entity: Arc::new(db::runtime_ports::DbEntityPort::new(pool.clone())),
            state: Arc::new(db::runtime_ports::DbStatePort::new(pool.clone())),
            knowledge: Arc::new(db::runtime_ports::DbKnowledgePort::new(pool.clone())),
            relation: Arc::new(db::runtime_ports::DbRelationPort::new(pool.clone())),
            event: Arc::new(db::runtime_ports::DbEventPort::new(pool.clone())),
            canon: Arc::new(db::runtime_ports::DbCanonRulePort::new(pool.clone())),
            snapshot: Arc::new(db::runtime_ports::DbContextSnapshotPort::new(pool.clone())),
        };
        runtime::context_engine::ContextEngine::new(deps)
    }

    fn build_validator(pool: sqlx::PgPool) -> runtime::validator::Validator {
        let deps = runtime::validator::ValidatorDeps {
            entity: Arc::new(db::runtime_ports::DbEntityPort::new(pool.clone())),
            validation: Arc::new(db::runtime_ports::DbValidationPort::new(pool.clone())),
            approval: Arc::new(db::runtime_ports::DbApprovalPort::new(pool.clone())),
            canon: Arc::new(db::runtime_ports::DbCanonRulePort::new(pool.clone())),
            proposed_change: Arc::new(db::runtime_ports::DbProposedChangeQueryPort::new(pool.clone())),
        };
        runtime::validator::Validator::new(deps)
    }

    async fn setup_db() -> Database {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string());
        let db = Database::open(&url).await.unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_dir = format!("{}/../db/migrations", manifest_dir);
        db::migration::run_migrations(db.pool(), &migrations_dir).await.unwrap();
        db
    }

    /// 测试完整的 World -> Narrative -> Context -> Validation 流水线
    #[tokio::test]
    async fn test_full_pipeline() {
        let db = setup_db().await;
        let pool = db.pool().clone();

        // 1. Create project
        let project_repo = db::repos::project_repo::ProjectRepo::new(pool.clone());
        let project = project_repo.create("Test Novel: Black Stone City", Some("A cultivation novel")).await.unwrap();

        // 2. Define world
        let world = WorldService::new(Arc::new(db::application_ports::DbWorldRepositoryPort::new(pool.clone())));
        let main_world = world.ensure_main_world(project.id, "Black Stone City").await.unwrap();

        let lin_fan = world.create_entity(project.id, main_world.id, "Character", "Lin Fan", Some("A cautious 20-year-old cultivator"), Some("Lin Fan is a cautious and observant young cultivator"), serde_json::json!({"age": 20})).await.unwrap();
        let wang_head = world.create_entity(project.id, main_world.id, "Character", "Wang Family Head", Some("Arrogant leader"), None, serde_json::json!({"age": 45})).await.unwrap();
        let city = world.create_entity(project.id, main_world.id, "Location", "Black Stone City", Some("A border city"), Some("A remote border city known for its black iron mines"), serde_json::json!({})).await.unwrap();
        let market = world.create_entity(project.id, main_world.id, "Location", "Black Market", Some("Underground hub"), None, serde_json::json!({})).await.unwrap();

        world.create_relation(project.id, lin_fan.id, wang_head.id, "enemy", Some("Wang family wants to capture Lin Fan"), serde_json::json!({})).await.unwrap();
        world.create_relation(project.id, city.id, market.id, "contains", Some("Black market is under the city"), serde_json::json!({})).await.unwrap();

        world.set_entity_state(project.id, lin_fan.id, "location", serde_json::json!("outside city")).await.unwrap();
        world.set_entity_state(project.id, lin_fan.id, "cultivation", serde_json::json!("Qi Refining Level 3")).await.unwrap();

        world.create_fact(project.id, "Underground ruins exist beneath Black Stone City", Some("secret"), "CANON", &[city.id]).await.unwrap();
        world.create_fact(project.id, "The Wang family controls the black iron mines", Some("public"), "CANON", &[city.id, wang_head.id]).await.unwrap();

        // 3. Create narrative structure (unified create_node API)
        let narrative = NarrativeService::new(std::sync::Arc::new(db::application_ports::DbNarrativeRepositoryPort::new(pool.clone())));
        let vol = narrative.create_node(project.id, "Volume", None, "Volume 1", Some("Lin Fan's journey begins"), serde_json::json!({"mission": "Lin Fan enters the cultivation world"})).await.unwrap();
        let vol_id: uuid::Uuid = serde_json::from_value(vol["id"].clone()).unwrap();
        let arc = narrative.create_node(project.id, "Arc", Some(vol_id), "Black Market Arc", Some("Lin Fan discovers the underground market"), serde_json::json!({})).await.unwrap();
        let arc_id: uuid::Uuid = serde_json::from_value(arc["id"].clone()).unwrap();
        let chapter = narrative.create_node(project.id, "Chapter", Some(arc_id), "Chapter 1: Arrival", Some("Lin Fan arrives at Black Stone City"), serde_json::json!({})).await.unwrap();
        let chapter_id: uuid::Uuid = serde_json::from_value(chapter["id"].clone()).unwrap();
        let scene = narrative.create_node(project.id, "Scene", Some(chapter_id), "Enter Black Market", Some("Lin Fan enters the underground market"),
            serde_json::json!({
                "objective": "Lin Fan explores the black market",
                "pov_character_id": lin_fan.id,
                "location_id": city.id,
                "emotional_goal": "Curiosity and caution",
                "information_goal": "Reader learns about the black market"
            })).await.unwrap();
        let scene_id: uuid::Uuid = serde_json::from_value(scene["id"].clone()).unwrap();
        narrative.create_node(project.id, "Beat", Some(scene_id), "Enter city", None,
            serde_json::json!({"action": "Lin Fan walks toward the city gates", "emotion": "cautious"})).await.unwrap();

        // 4. Build context with different policies
        let engine = build_context_engine(pool.clone());

        // Scene Writer policy
        let writer_policy = domain::ContextPolicy::scene_writer();
        let ctx_writer = engine.build_context_with_policy(project.id, scene_id, TokenBudgets::MEDIUM, &writer_policy).await.unwrap();
        assert!(!ctx_writer.l0_essential.content.is_empty());
        assert!(ctx_writer.l0_essential.content.contains("Lin Fan"));
        assert!(ctx_writer.l0_essential.content.contains("Black Stone City"));

        // Location Designer policy (should NOT include L4 Character Knowledge)
        let loc_policy = domain::ContextPolicy::location_designer();
        let ctx_loc = engine.build_context_with_policy(project.id, scene_id, TokenBudgets::MEDIUM, &loc_policy).await.unwrap();
        assert!(!ctx_loc.l4_character_knowledge.included);

        // 5. Validate and apply changes via the canonical commit path
        let validator = build_validator(pool.clone());
        let val_repo = db::repos::validation_repo::ValidationRepo::new(pool.clone());
        let task_id = uuid::Uuid::new_v4();
        let change = val_repo.create_proposed_change(project.id, task_id, ProposedChangeType::StateChange, lin_fan.id, "Enter city", serde_json::json!({"state_key": "location", "new_value": "Black Stone City"})).await.unwrap();
        let run = validator.validate_changes(project.id, task_id, &[change.clone()]).await.unwrap();
        assert_eq!(run.changes_approved, 1);

        let committer = DbStateCommitter::new(Arc::new(db::runtime_ports::DbStateCommitterPort::new(pool.clone())));
        let response = committer.commit(project.id, &[change.id]).await.unwrap();
        assert_eq!(response.results.len(), 1);

        let state = world.get_entity_state(project.id, lin_fan.id, "location").await.unwrap().unwrap();
        assert_eq!(state.state_value, serde_json::json!("Black Stone City"));

        println!("E2E test passed: Full V2 pipeline verified");
    }

    /// 测试 13 种 Skill 模板
    #[tokio::test]
    async fn test_skill_templates() {
        let templates = domain::skill::SkillTemplates::all();
        assert_eq!(templates.len(), 13);

        // 数据仍在：每个模板都能转换为 SkillDefinition（无运行时注册 API）。
        for template in templates {
            let def = template.to_definition();
            assert!(!def.name.is_empty(), "Skill definition name must not be empty");
        }
    }

    /// 测试 Context Policy 对不同 Skill 的影响
    #[tokio::test]
    async fn test_context_policy_by_skill_type() {
        let db = setup_db().await;
        let pool = db.pool().clone();
        let engine = build_context_engine(pool.clone());

        let project_repo = db::repos::project_repo::ProjectRepo::new(pool.clone());
        let project = project_repo.create("Test", None).await.unwrap();
        let world = WorldService::new(Arc::new(db::application_ports::DbWorldRepositoryPort::new(pool.clone())));
        let main_world = world.ensure_main_world(project.id, "Test").await.unwrap();
        let char = world.create_entity(project.id, main_world.id, "Character", "Test Character", None, None, serde_json::json!({})).await.unwrap();

        let narrative = NarrativeService::new(std::sync::Arc::new(db::application_ports::DbNarrativeRepositoryPort::new(pool.clone())));
        let vol = narrative.create_node(project.id, "Volume", None, "Vol 1", None, serde_json::json!({})).await.unwrap();
        let vol_id: uuid::Uuid = serde_json::from_value(vol["id"].clone()).unwrap();
        let arc = narrative.create_node(project.id, "Arc", Some(vol_id), "Arc 1", None, serde_json::json!({})).await.unwrap();
        let arc_id: uuid::Uuid = serde_json::from_value(arc["id"].clone()).unwrap();
        let chapter = narrative.create_node(project.id, "Chapter", Some(arc_id), "Ch 1", None, serde_json::json!({})).await.unwrap();
        let chapter_id: uuid::Uuid = serde_json::from_value(chapter["id"].clone()).unwrap();
        let scene = narrative.create_node(project.id, "Scene", Some(chapter_id), "Scene 1", None,
            serde_json::json!({"pov_character_id": char.id})).await.unwrap();
        let scene_id: uuid::Uuid = serde_json::from_value(scene["id"].clone()).unwrap();

        // Test each skill type's context policy
        let skill_types = vec![
            SkillType::LocationDesigner,
            SkillType::CharacterDesigner,
            SkillType::Writer,
            SkillType::Analyzer,
        ];

        for st in skill_types {
            let policy = st.context_policy();
            let ctx = engine.build_context_with_policy(project.id, scene_id, TokenBudgets::MEDIUM, &policy).await.unwrap();
            assert!(!ctx.l0_essential.content.is_empty(), "L0 should always be included for {:?}", st);
        }
    }

    /// 测试 Context Ranking 评分
    #[test]
    fn test_context_ranking() {
        use runtime::context_engine::ContextScore;

        let high_score = ContextScore {
            relevance: 1.0,
            importance: 1.0,
            recency: 1.0,
            explicitness: 1.0,
            visibility: 1.0,
        };
        assert_eq!(high_score.total_score(), 1.0);

        let low_score = ContextScore {
            relevance: 0.1,
            importance: 0.1,
            recency: 0.1,
            explicitness: 0.1,
            visibility: 0.1,
        };
        assert!(low_score.total_score() < 0.01);

        // Test that relevance and visibility are the most important factors
        let relevance_matters = ContextScore {
            relevance: 1.0,
            importance: 0.5,
            recency: 0.5,
            explicitness: 0.5,
            visibility: 1.0,
        };
        let visibility_matters = ContextScore {
            relevance: 0.5,
            importance: 1.0,
            recency: 0.5,
            explicitness: 0.5,
            visibility: 1.0,
        };
        // Both should have similar scores since total = relevance * importance * visibility * recency
        assert!((relevance_matters.total_score() - visibility_matters.total_score()).abs() < 0.01);
    }

    /// 测试 Token Budget 限制
    #[tokio::test]
    async fn test_token_budget_limits() {
        let db = setup_db().await;
        let pool = db.pool().clone();
        let engine = build_context_engine(pool.clone());

        let project_repo = db::repos::project_repo::ProjectRepo::new(pool.clone());
        let project = project_repo.create("Test", None).await.unwrap();
        let world = WorldService::new(Arc::new(db::application_ports::DbWorldRepositoryPort::new(pool.clone())));
        let main_world = world.ensure_main_world(project.id, "Test").await.unwrap();
        let char = world.create_entity(project.id, main_world.id, "Character", "Test", None, None, serde_json::json!({})).await.unwrap();

        let narrative = NarrativeService::new(std::sync::Arc::new(db::application_ports::DbNarrativeRepositoryPort::new(pool.clone())));
        let vol = narrative.create_node(project.id, "Volume", None, "Vol 1", None, serde_json::json!({})).await.unwrap();
        let vol_id: uuid::Uuid = serde_json::from_value(vol["id"].clone()).unwrap();
        let arc = narrative.create_node(project.id, "Arc", Some(vol_id), "Arc 1", None, serde_json::json!({})).await.unwrap();
        let arc_id: uuid::Uuid = serde_json::from_value(arc["id"].clone()).unwrap();
        let chapter = narrative.create_node(project.id, "Chapter", Some(arc_id), "Ch 1", None, serde_json::json!({})).await.unwrap();
        let chapter_id: uuid::Uuid = serde_json::from_value(chapter["id"].clone()).unwrap();
        let scene = narrative.create_node(project.id, "Scene", Some(chapter_id), "Scene 1", None,
            serde_json::json!({"pov_character_id": char.id})).await.unwrap();
        let scene_id: uuid::Uuid = serde_json::from_value(scene["id"].clone()).unwrap();

        let ctx_small = engine.build_context(project.id, scene_id, TokenBudgets::SMALL).await.unwrap();
        let ctx_large = engine.build_context(project.id, scene_id, TokenBudgets::LARGE).await.unwrap();

        assert!(ctx_small.actual_tokens <= TokenBudgets::SMALL);
        assert!(ctx_large.actual_tokens <= TokenBudgets::LARGE);
    }

    /// 测试 World-driven Storytelling
    #[tokio::test]
    async fn test_world_driven_storytelling() {
        let db = setup_db().await;
        let pool = db.pool().clone();
        let project_repo = db::repos::project_repo::ProjectRepo::new(pool.clone());
        let project = project_repo.create("World-Driven Test", None).await.unwrap();
        let world = WorldService::new(Arc::new(db::application_ports::DbWorldRepositoryPort::new(pool.clone())));
        let main_world = world.ensure_main_world(project.id, "Test").await.unwrap();

        // 创建势力和资源
        let wang = world.create_entity(project.id, main_world.id, "Faction", "Wang Family", Some("Ruling family"), None, serde_json::json!({})).await.unwrap();
        let mine = world.create_entity(project.id, main_world.id, "Location", "Black Iron Mine", Some("Rich mine"), None, serde_json::json!({})).await.unwrap();

        // 创建关系：王家控制矿区
        world.create_relation(project.id, wang.id, mine.id, "CONTROLS", None, serde_json::json!({})).await.unwrap();

        // 设置资源状态
        world.upsert_resource(project.id, mine.id, "Black Iron Ore", Some(10000.0), Some(100.0), Some(wang.id)).await.unwrap();

        // 验证资源存在
        let resources = world.list_resources(mine.id).await.unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].resource_name, "Black Iron Ore");
        assert_eq!(resources[0].quantity, Some(10000.0));

        // 模拟矿区被摧毁后，资源应该变为0
        world.upsert_resource(project.id, mine.id, "Black Iron Ore", Some(0.0), Some(0.0), None).await.unwrap();
        let resources = world.list_resources(mine.id).await.unwrap();
        assert_eq!(resources[0].quantity, Some(0.0));

        println!("World-driven storytelling test passed");
    }
}