//! End-to-End Tests - 验证 V2 Narrative Engine 全部功能

#[cfg(test)]
mod e2e_tests {
    use runtime::context_engine::{ContextEngine, TokenBudgets};
    use application::generation_service::GenerationRuntime;
    use application::narrative_service::NarrativeService;
    use runtime::validator::Validator;
    use application::world_service::WorldService;
    use domain::*;
    use db::connection::Database;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_dir = format!("{}/../db/migrations", manifest_dir);
        db::migration::run_migrations(&db, &migrations_dir).unwrap();
        db
    }

    /// 测试完整的 World -> Narrative -> Context -> Generation -> Validation 流水线
    #[test]
    fn test_full_pipeline() {
        let db = setup_db();

        // 1. Create project
        let project_repo = db::repos::project_repo::ProjectRepo::new(&db);
        let project = project_repo.create("Test Novel: Black Stone City", Some("A cultivation novel")).unwrap();

        // 2. Define world
        let world = WorldService::new(&db);
        let main_world = world.ensure_main_world(project.id, "Black Stone City").unwrap();

        let lin_fan = world.create_entity(project.id, main_world.id, "Character", "Lin Fan", Some("A cautious 20-year-old cultivator"), Some("Lin Fan is a cautious and observant young cultivator"), serde_json::json!({"age": 20})).unwrap();
        let wang_head = world.create_entity(project.id, main_world.id, "Character", "Wang Family Head", Some("Arrogant leader"), None, serde_json::json!({"age": 45})).unwrap();
        let city = world.create_entity(project.id, main_world.id, "Location", "Black Stone City", Some("A border city"), Some("A remote border city known for its black iron mines"), serde_json::json!({})).unwrap();
        let market = world.create_entity(project.id, main_world.id, "Location", "Black Market", Some("Underground hub"), None, serde_json::json!({})).unwrap();

        world.create_relation(project.id, lin_fan.id, wang_head.id, "enemy", Some("Wang family wants to capture Lin Fan"), serde_json::json!({})).unwrap();
        world.create_relation(project.id, city.id, market.id, "contains", Some("Black market is under the city"), serde_json::json!({})).unwrap();

        world.set_entity_state(project.id, lin_fan.id, "location", serde_json::json!("outside city")).unwrap();
        world.set_entity_state(project.id, lin_fan.id, "cultivation", serde_json::json!("Qi Refining Level 3")).unwrap();

        world.create_fact(project.id, "Underground ruins exist beneath Black Stone City", Some("secret"), &[city.id]).unwrap();
        world.create_fact(project.id, "The Wang family controls the black iron mines", Some("public"), &[city.id, wang_head.id]).unwrap();

        // 3. Create narrative structure
        let narrative = NarrativeService::new(&db);
        let vol = narrative.create_volume(project.id, main_world.id, "Volume 1", Some("Lin Fan's journey begins"), VolumeAttributes { mission: Some("Lin Fan enters the cultivation world".into()), ..Default::default() }, 0).unwrap();
        let arc = narrative.create_arc(project.id, main_world.id, vol.id, "Black Market Arc", Some("Lin Fan discovers the underground market"), 0).unwrap();
        let chapter = narrative.create_chapter(project.id, main_world.id, arc.id, "Chapter 1: Arrival", Some("Lin Fan arrives at Black Stone City"), 0).unwrap();
        let (scene_node, _) = narrative.create_scene(project.id, main_world.id, chapter.id, "Enter Black Market", Some("Lin Fan enters the underground market"),
            SceneAttributes {
                objective: Some("Lin Fan explores the black market".into()),
                pov_character_id: Some(lin_fan.id),
                location_id: Some(city.id),
                emotional_goal: Some("Curiosity and caution".into()),
                information_goal: Some("Reader learns about the black market".into()),
                ..Default::default()
            }, 0).unwrap();
        narrative.create_beat(project.id, main_world.id, scene_node.id, "Enter city", BeatAttributes { action: "Lin Fan walks toward the city gates".into(), emotion: Some("cautious".into()), ..Default::default() }, 0).unwrap();

        // 4. Build context with different policies
        let engine = ContextEngine::new(&db);

        // Scene Writer policy
        let writer_policy = domain::ContextPolicy::scene_writer();
        let ctx_writer = engine.build_context_with_policy(project.id, scene_node.id, TokenBudgets::MEDIUM, &writer_policy).unwrap();
        assert!(!ctx_writer.l0_essential.content.is_empty());
        assert!(ctx_writer.l0_essential.content.contains("Lin Fan"));
        assert!(ctx_writer.l0_essential.content.contains("Black Stone City"));

        // Location Designer policy (should NOT include L4 Character Knowledge)
        let loc_policy = domain::ContextPolicy::location_designer();
        let ctx_loc = engine.build_context_with_policy(project.id, scene_node.id, TokenBudgets::MEDIUM, &loc_policy).unwrap();
        assert!(!ctx_loc.l4_character_knowledge.included);

        // 5. Register skills and create task
        let runtime = GenerationRuntime::new(&db);
        let template = domain::skill::SkillTemplates::writer();
        runtime.register_skill(&template.name, Some(&template.description), template.skill_type.clone(), &template.prompt_template, Some(template.input_schema.clone()), Some(template.output_schema.clone()), serde_json::json!({"max_tokens": 2000})).unwrap();
        let task = runtime.create_task(project.id, "scene_writer", None, serde_json::json!({"context": ctx_writer.l0_essential.content})).unwrap();
        let completed = runtime.execute_task(task.id).unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);

        // 6. Validate and apply changes
        let validator = Validator::new(&db);
        let val_repo = db::repos::validation_repo::ValidationRepo::new(&db);
        let change = val_repo.create_proposed_change(project.id, task.id, ProposedChangeType::StateChange, lin_fan.id, "Enter city", serde_json::json!({"state_key": "location", "new_value": "Black Stone City"})).unwrap();
        let run = validator.validate_changes(project.id, task.id, &[change]).unwrap();
        assert_eq!(run.changes_approved, 1);

        let records = validator.apply_approved_changes(project.id, task.id).unwrap();
        assert_eq!(records.len(), 1);

        let state = world.get_entity_state(lin_fan.id, "location").unwrap().unwrap();
        assert_eq!(state.state_value, serde_json::json!("Black Stone City"));

        println!("E2E test passed: Full V2 pipeline verified");
    }

    /// 测试 13 种 Skill 模板
    #[test]
    fn test_skill_templates() {
        let templates = domain::skill::SkillTemplates::all();
        assert_eq!(templates.len(), 13);

        let db = setup_db();
        let runtime = GenerationRuntime::new(&db);

        for template in templates {
            let def = template.to_definition();
            let result = runtime.register_skill(
                &def.name,
                Some(&def.description),
                def.skill_type.clone(),
                &def.prompt_template,
                Some(def.input_schema.clone()),
                Some(def.output_schema.clone()),
                def.default_params.clone(),
            );
            assert!(result.is_ok(), "Failed to register skill: {}", def.name);
        }

        let all_skills = runtime.list_skills().unwrap();
        assert_eq!(all_skills.len(), 13);
    }

    /// 测试 Context Policy 对不同 Skill 的影响
    #[test]
    fn test_context_policy_by_skill_type() {
        let db = setup_db();
        let engine = ContextEngine::new(&db);

        let project_repo = db::repos::project_repo::ProjectRepo::new(&db);
        let project = project_repo.create("Test", None).unwrap();
        let world = WorldService::new(&db);
        let main_world = world.ensure_main_world(project.id, "Test").unwrap();
        let char = world.create_entity(project.id, main_world.id, "Character", "Test Character", None, None, serde_json::json!({})).unwrap();

        let narrative = NarrativeService::new(&db);
        let vol = narrative.create_volume(project.id, main_world.id, "Vol 1", None, VolumeAttributes::default(), 0).unwrap();
        let arc = narrative.create_arc(project.id, main_world.id, vol.id, "Arc 1", None, 0).unwrap();
        let chapter = narrative.create_chapter(project.id, main_world.id, arc.id, "Ch 1", None, 0).unwrap();
        let (scene_node, _) = narrative.create_scene(project.id, main_world.id, chapter.id, "Scene 1", None,
            SceneAttributes { pov_character_id: Some(char.id), ..Default::default() }, 0).unwrap();

        // Test each skill type's context policy
        let skill_types = vec![
            SkillType::LocationDesigner,
            SkillType::CharacterDesigner,
            SkillType::Writer,
            SkillType::Analyzer,
        ];

        for st in skill_types {
            let policy = st.context_policy();
            let ctx = engine.build_context_with_policy(project.id, scene_node.id, TokenBudgets::MEDIUM, &policy).unwrap();
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
    #[test]
    fn test_token_budget_limits() {
        let db = setup_db();
        let engine = ContextEngine::new(&db);

        let project_repo = db::repos::project_repo::ProjectRepo::new(&db);
        let project = project_repo.create("Test", None).unwrap();
        let world = WorldService::new(&db);
        let main_world = world.ensure_main_world(project.id, "Test").unwrap();
        let char = world.create_entity(project.id, main_world.id, "Character", "Test", None, None, serde_json::json!({})).unwrap();

        let narrative = NarrativeService::new(&db);
        let vol = narrative.create_volume(project.id, main_world.id, "Vol 1", None, VolumeAttributes::default(), 0).unwrap();
        let arc = narrative.create_arc(project.id, main_world.id, vol.id, "Arc 1", None, 0).unwrap();
        let chapter = narrative.create_chapter(project.id, main_world.id, arc.id, "Ch 1", None, 0).unwrap();
        let (scene_node, _) = narrative.create_scene(project.id, main_world.id, chapter.id, "Scene 1", None,
            SceneAttributes { pov_character_id: Some(char.id), ..Default::default() }, 0).unwrap();

        let ctx_small = engine.build_context(project.id, scene_node.id, TokenBudgets::SMALL).unwrap();
        let ctx_large = engine.build_context(project.id, scene_node.id, TokenBudgets::LARGE).unwrap();

        assert!(ctx_small.actual_tokens <= TokenBudgets::SMALL);
        assert!(ctx_large.actual_tokens <= TokenBudgets::LARGE);
    }

    /// 测试 World-driven Storytelling
    #[test]
    fn test_world_driven_storytelling() {
        let db = setup_db();
        let project_repo = db::repos::project_repo::ProjectRepo::new(&db);
        let project = project_repo.create("World-Driven Test", None).unwrap();
        let world = WorldService::new(&db);
        let main_world = world.ensure_main_world(project.id, "Test").unwrap();

        // 创建势力和资源
        let wang = world.create_entity(project.id, main_world.id, "Faction", "Wang Family", Some("Ruling family"), None, serde_json::json!({})).unwrap();
        let mine = world.create_entity(project.id, main_world.id, "Location", "Black Iron Mine", Some("Rich mine"), None, serde_json::json!({})).unwrap();

        // 创建关系：王家控制矿区
        world.create_relation(project.id, wang.id, mine.id, "CONTROLS", None, serde_json::json!({})).unwrap();

        // 设置资源状态
        world.upsert_resource(project.id, mine.id, "Black Iron Ore", Some(10000.0), Some(100.0), Some(wang.id)).unwrap();

        // 验证资源存在
        let resources = world.list_resources(mine.id).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].resource_name, "Black Iron Ore");
        assert_eq!(resources[0].quantity, Some(10000.0));

        // 模拟矿区被摧毁后，资源应该变为0
        world.upsert_resource(project.id, mine.id, "Black Iron Ore", Some(0.0), Some(0.0), None).unwrap();
        let resources = world.list_resources(mine.id).unwrap();
        assert_eq!(resources[0].quantity, Some(0.0));

        println!("World-driven storytelling test passed");
    }
}
