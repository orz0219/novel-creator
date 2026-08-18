//! Context Engine - 动态上下文组装
//!
//! 核心职责：根据当前 Scene，从数据库中查询相关信息，
//! 按 L0~L6 分层组织，根据 Token Budget 动态选择，
//! 生成最小充分上下文给 Writer。

use anyhow::{Context, Result};
use db::connection::Database;
use db::repos::{entity_repo, knowledge_repo, narrative_repo, state_repo};
use domain::*;
use uuid::Uuid;

/// Token 预算预设
pub struct TokenBudgets;

impl TokenBudgets {
    pub const SMALL: i32 = 8000;
    pub const MEDIUM: i32 = 12000;
    pub const LARGE: i32 = 20000;
}

/// Context Engine - 信息调度器
pub struct ContextEngine<'a> {
    db: &'a Database,
}

impl<'a> ContextEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 为指定 Scene 组装上下文包
    pub fn build_context(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
    ) -> Result<ContextPackage> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.db);
        let scene_repo = narrative_repo::SceneRepo::new(self.db);
        let entity_repo = entity_repo::EntityRepo::new(self.db);
        let state_repo = state_repo::StateRepo::new(self.db);
        let knowledge_repo = knowledge_repo::KnowledgeRepo::new(self.db);

        // 1. 获取 Scene 节点
        let scene_node = narrative_repo.get_node_by_id(scene_node_id)?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found: {}", scene_node_id))?;

        let scene_attrs: SceneAttributes = serde_json::from_value(scene_node.attributes.clone())
            .unwrap_or_default();

        // 2. 获取 Scene 详情
        let scene = scene_repo.get_by_narrative_node(scene_node_id)?;

        // 3. 获取父节点（Chapter）信息
        let chapter_summary = if let Some(parent_id) = scene_node.parent_id {
            narrative_repo.get_node_by_id(parent_id)?
                .map(|n| format!("Chapter: {} - {}", n.title, n.description.unwrap_or_default()))
        } else {
            None
        };

        // 4. 获取 Volume 信息（向上遍历）
        let volume_summary = self.get_volume_summary(project_id)?;

        // 5. 获取 Arc 信息
        let arc_summary = if let Some(parent_id) = scene_node.parent_id {
            self.get_arc_summary(parent_id)?
        } else {
            None
        };

        // 6. 获取相关角色信息
        let characters = self.get_relevant_characters(
            &scene_attrs,
            &scene,
            &entity_repo,
            &state_repo,
        )?;

        // 7. 获取相关地点信息
        let location = self.get_relevant_location(
            &scene_attrs,
            &entity_repo,
            &state_repo,
        )?;

        // 8. 获取前一个 Scene 的摘要
        let prev_scene_summary = self.get_previous_scene_summary(
            project_id,
            &scene_node,
            &narrative_repo,
        )?;

        // 9. 获取角色知识（POV 角色知道什么）
        let character_knowledge = if let Some(pov_id) = scene_attrs.pov_character_id {
            self.get_character_knowledge(project_id, pov_id, &knowledge_repo)?
        } else {
            String::new()
        };

        // 10. 获取世界规则摘要
        let world_rules = self.get_world_rules_summary(project_id)?;

        // 组装各层
        let l0 = self.build_l0(&scene_node, &scene_attrs, &characters, &location);
        let l1 = self.build_l1(&characters, &location, &state_repo);
        let l2 = self.build_l2(&prev_scene_summary, &chapter_summary);
        let l3 = self.build_l3(&volume_summary, &arc_summary);
        let l4 = ContextLayer {
            content: character_knowledge,
            token_estimate: 0,
            included: true,
        };
        let l5 = ContextLayer {
            content: world_rules,
            token_estimate: 0,
            included: true,
        };
        let l6 = ContextLayer {
            content: String::new(),
            token_estimate: 0,
            included: false,
        };

        // 估算各层 token 数
        let l0_tokens = self.estimate_tokens(&l0.content);
        let l1_tokens = self.estimate_tokens(&l1.content);
        let l2_tokens = self.estimate_tokens(&l2.content);
        let l3_tokens = self.estimate_tokens(&l3.content);
        let l4_tokens = self.estimate_tokens(&l4.content);
        let l5_tokens = self.estimate_tokens(&l5.content);

        let mut used_tokens = 0;
        let mut included_indices = Vec::new();
        
        // L0 始终包含
        used_tokens += l0_tokens;
        included_indices.push(0);
        
        // L1 始终包含
        if used_tokens + l1_tokens <= token_budget {
            used_tokens += l1_tokens;
            included_indices.push(1);
        }
        
        // L2-L5 按优先级添加
        let remaining = [
            (2, l2_tokens, &l2),
            (3, l3_tokens, &l3),
            (4, l4_tokens, &l4),
            (5, l5_tokens, &l5),
        ];
        
        for (idx, tokens, _layer) in &remaining {
            if used_tokens + tokens <= token_budget {
                used_tokens += tokens;
                included_indices.push(*idx);
            }
        }

        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        Ok(ContextPackage {
            id,
            project_id,
            scene_id: scene_node_id,
            token_budget,
            l0_essential: ContextLayer { included: included_indices.contains(&0), token_estimate: l0_tokens, ..l0 },
            l1_scene_relevant: ContextLayer { included: included_indices.contains(&1), token_estimate: l1_tokens, ..l1 },
            l2_recent_history: ContextLayer { included: included_indices.contains(&2), token_estimate: l2_tokens, ..l2 },
            l3_narrative_context: ContextLayer { included: included_indices.contains(&3), token_estimate: l3_tokens, ..l3 },
            l4_character_knowledge: ContextLayer { included: included_indices.contains(&4), token_estimate: l4_tokens, ..l4 },
            l5_world_background: ContextLayer { included: included_indices.contains(&5), token_estimate: l5_tokens, ..l5 },
            l6_optional_supplement: l6,
            actual_tokens: used_tokens,
            created_at: now,
        })
    }

    /// 获取 Volume 摘要
    fn get_volume_summary(&self, project_id: Uuid) -> Result<Option<String>> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        let all = repo.list_nodes_by_project(project_id)?;
        let volumes: Vec<&NarrativeNode> = all.iter()
            .filter(|n| n.node_type == NarrativeNodeType::Volume)
            .collect();

        if let Some(vol) = volumes.first() {
            let attrs: VolumeAttributes = serde_json::from_value(vol.attributes.clone())
                .unwrap_or_default();
            let mut summary = format!("Current Volume: {}", vol.title);
            if let Some(mission) = &attrs.mission {
                summary.push_str(&format!("\nMission: {}", mission));
            }
            if let Some(theme) = &attrs.theme {
                summary.push_str(&format!("\nTheme: {}", theme));
            }
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }

    /// 获取 Arc 摘要
    fn get_arc_summary(&self, chapter_id: Uuid) -> Result<Option<String>> {
        let repo = narrative_repo::NarrativeRepo::new(self.db);
        // chapter -> parent is arc
        if let Some(chapter) = repo.get_node_by_id(chapter_id)? {
            if let Some(arc_id) = chapter.parent_id {
                if let Some(arc) = repo.get_node_by_id(arc_id)? {
                    return Ok(Some(format!("Current Arc: {} - {}", arc.title, arc.description.unwrap_or_default())));
                }
            }
        }
        Ok(None)
    }

    /// 获取相关角色
    fn get_relevant_characters(
        &self,
        scene_attrs: &SceneAttributes,
        scene: &Option<Scene>,
        entity_repo: &entity_repo::EntityRepo,
        state_repo: &state_repo::StateRepo,
    ) -> Result<Vec<(Entity, Vec<CurrentState>)>> {
        let mut character_ids = scene_attrs.characters_present.clone();

        // 添加 POV 角色
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !character_ids.contains(&pov_id) {
                character_ids.push(pov_id);
            }
        }

        // 添加场景中涉及的角色
        if let Some(scene) = scene {
            if let Some(pov) = scene.pov_character_id {
                if !character_ids.contains(&pov) {
                    character_ids.push(pov);
                }
            }
        }

        let mut result = Vec::new();
        for cid in character_ids {
            if let Some(entity) = entity_repo.get_by_id(cid)? {
                let states = state_repo.list_current_states(cid)?;
                result.push((entity, states));
            }
        }

        Ok(result)
    }

    /// 获取相关地点
    fn get_relevant_location(
        &self,
        scene_attrs: &SceneAttributes,
        entity_repo: &entity_repo::EntityRepo,
        state_repo: &state_repo::StateRepo,
    ) -> Result<Option<(Entity, Vec<CurrentState>)>> {
        if let Some(loc_id) = scene_attrs.location_id {
            if let Some(entity) = entity_repo.get_by_id(loc_id)? {
                let states = state_repo.list_current_states(loc_id)?;
                return Ok(Some((entity, states)));
            }
        }
        Ok(None)
    }

    /// 获取前一个 Scene 的摘要
    fn get_previous_scene_summary(
        &self,
        project_id: Uuid,
        current_scene: &NarrativeNode,
        narrative_repo: &narrative_repo::NarrativeRepo,
    ) -> Result<Option<String>> {
        // 找到同级的前一个 Scene
        if let Some(parent_id) = current_scene.parent_id {
            let siblings = narrative_repo.list_children(parent_id)?;
            let scenes: Vec<&NarrativeNode> = siblings.iter()
                .filter(|n| n.node_type == NarrativeNodeType::Scene)
                .collect();

            if let Some(idx) = scenes.iter().position(|s| s.id == current_scene.id) {
                if idx > 0 {
                    let prev = scenes[idx - 1];
                    return Ok(Some(format!("Previous Scene: {} - {}", prev.title, prev.description.as_deref().unwrap_or_default())));
                }
            }
        }
        Ok(None)
    }

    /// 获取角色知识
    fn get_character_knowledge(
        &self,
        _project_id: Uuid,
        _character_id: Uuid,
        _knowledge_repo: &knowledge_repo::KnowledgeRepo,
    ) -> Result<String> {
        // TODO: 当 KnowledgeModel 实现后，这里查询角色已知的事实
        Ok("Character knowledge will be loaded from KnowledgeModel.".to_string())
    }

    /// 获取世界规则摘要
    fn get_world_rules_summary(&self, _project_id: Uuid) -> Result<String> {
        // TODO: 从 Project.world_setting 中提取核心规则
        Ok("World rules summary would be loaded from project settings.".to_string())
    }

    // ============================================================
    // Layer 组装
    // ============================================================

    fn build_l0(
        &self,
        scene_node: &NarrativeNode,
        scene_attrs: &SceneAttributes,
        characters: &[(Entity, Vec<CurrentState>)],
        location: &Option<(Entity, Vec<CurrentState>)>,
    ) -> ContextLayer {
        let mut content = String::new();
        content.push_str(&format!("## Scene Objective\n{}\n", scene_attrs.objective.as_deref().unwrap_or(&scene_node.title)));
        if let Some(conflict) = &scene_attrs.conflict {
            content.push_str(&format!("## Conflict\n{}\n", conflict));
        }
        if let Some(pov_id) = scene_attrs.pov_character_id {
            content.push_str(&format!("## POV Character ID: {}\n", pov_id));
        }
        content.push_str(&format!("## Characters\n"));
        for (entity, _) in characters {
            content.push_str(&format!("- {} ({})\n", entity.name, entity.summary.as_deref().unwrap_or("")));
        }
        if let Some((loc, _)) = location {
            content.push_str(&format!("## Location: {}\n", loc.name));
        }

        ContextLayer { content, token_estimate: 0, included: true }
    }

    fn build_l1(
        &self,
        characters: &[(Entity, Vec<CurrentState>)],
        location: &Option<(Entity, Vec<CurrentState>)>,
        _state_repo: &state_repo::StateRepo,
    ) -> ContextLayer {
        let mut content = String::new();
        content.push_str("## Character States\n");
        for (entity, states) in characters {
            content.push_str(&format!("\n### {}:\n", entity.name));
            for state in states {
                content.push_str(&format!("  {}: {}\n", state.state_key, state.state_value));
            }
        }
        if let Some((loc, states)) = location {
            content.push_str(&format!("\n## Location State: {}\n", loc.name));
            for state in states {
                content.push_str(&format!("  {}: {}\n", state.state_key, state.state_value));
            }
        }

        ContextLayer { content, token_estimate: 0, included: true }
    }

    fn build_l2(
        &self,
        prev_scene_summary: &Option<String>,
        chapter_summary: &Option<String>,
    ) -> ContextLayer {
        let mut content = String::new();
        if let Some(chapter) = chapter_summary {
            content.push_str(&format!("## Current Chapter\n{}\n", chapter));
        }
        if let Some(prev) = prev_scene_summary {
            content.push_str(&format!("## Previous Scene\n{}\n", prev));
        }

        ContextLayer { content, token_estimate: 0, included: true }
    }

    fn build_l3(
        &self,
        volume_summary: &Option<String>,
        arc_summary: &Option<String>,
    ) -> ContextLayer {
        let mut content = String::new();
        if let Some(vol) = volume_summary {
            content.push_str(&format!("## Volume Context\n{}\n", vol));
        }
        if let Some(arc) = arc_summary {
            content.push_str(&format!("## Arc Context\n{}\n", arc));
        }

        ContextLayer { content, token_estimate: 0, included: true }
    }

    /// 简单的 token 估算（1 个中文字符约 2 tokens，1 个英文单词约 1.3 tokens）
    fn estimate_tokens(&self, text: &str) -> i32 {
        let chars = text.chars().count();
        let words = text.split_whitespace().count();
        // 混合估算
        ((chars as f64 * 0.5 + words as f64 * 0.8) as i32).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_dir = format!("{}/../db/migrations", manifest_dir);
        db::migration::run_migrations(&db, &migrations_dir).unwrap();
        db
    }

    fn setup_with_data() -> (Database, Uuid, Uuid) {
        let db = setup_db();
        let project_repo = db::repos::project_repo::ProjectRepo::new(&db);
        let project = project_repo.create("Test Novel", None).unwrap();

        let world_service = super::super::world_service::WorldService::new(&db);
        let narrative_service = super::super::narrative_service::NarrativeService::new(&db);

        // 确保有主世界
        let world = world_service.ensure_main_world(project.id, "Test Novel").unwrap();

        // 创建角色
        let char = world_service.create_entity(
            project.id, world.id, "Character", "Lin Fan",
            Some("A young cultivator"), None,
            serde_json::json!({"age": 20}),
        ).unwrap();

        // 创建地点
        let loc = world_service.create_entity(
            project.id, world.id, "Location", "Black Stone City",
            Some("A border city"), None,
            serde_json::json!({}),
        ).unwrap();

        // 设置角色状态
        world_service.set_entity_state(project.id, char.id, "location", serde_json::json!("outside the city")).unwrap();
        world_service.set_entity_state(project.id, char.id, "cultivation", serde_json::json!("Qi Refining Level 3")).unwrap();

        // 创建叙事结构
        let vol = narrative_service.create_volume(project.id, world.id, "Volume 1", None, VolumeAttributes::default(), 0).unwrap();
        let arc = narrative_service.create_arc(project.id, world.id, vol.id, "Black Stone Arc", None, 0).unwrap();
        let chapter = narrative_service.create_chapter(project.id, world.id, arc.id, "Chapter 1", None, 0).unwrap();

        let (scene_node, _scene) = narrative_service.create_scene(
            project.id, world.id, chapter.id,
            "Enter Black Market",
            Some("Lin Fan enters the underground market"),
            SceneAttributes {
                objective: Some("Lin Fan explores the black market".into()),
                pov_character_id: Some(char.id),
                location_id: Some(loc.id),
                ..Default::default()
            },
            0,
        ).unwrap();

        (db, project.id, scene_node.id)
    }

    #[test]
    fn test_build_context() {
        let (db, project_id, scene_id) = setup_with_data();
        let engine = ContextEngine::new(&db);

        let ctx = engine.build_context(project_id, scene_id, TokenBudgets::MEDIUM).unwrap();

        assert!(!ctx.l0_essential.content.is_empty());
        assert!(ctx.l0_essential.content.contains("Lin Fan"));
        assert!(ctx.l0_essential.content.contains("Black Stone City"));
        assert!(ctx.l0_essential.content.contains("Lin Fan explores"));

        println!("=== Context Package ===");
        println!("L0: {}", ctx.l0_essential.content);
        println!("L1: {}", ctx.l1_scene_relevant.content);
        println!("L2: {}", ctx.l2_recent_history.content);
        println!("L3: {}", ctx.l3_narrative_context.content);
        println!("L4: {}", ctx.l4_character_knowledge.content);
        println!("Actual tokens: {}", ctx.actual_tokens);
    }

    #[test]
    fn test_token_budget_limits() {
        let (db, project_id, scene_id) = setup_with_data();
        let engine = ContextEngine::new(&db);

        // 小预算应该跳过一些层
        let ctx_small = engine.build_context(project_id, scene_id, TokenBudgets::SMALL).unwrap();
        let ctx_large = engine.build_context(project_id, scene_id, TokenBudgets::LARGE).unwrap();

        assert!(ctx_small.actual_tokens <= TokenBudgets::SMALL);
        assert!(ctx_large.actual_tokens <= TokenBudgets::LARGE);
    }
}
