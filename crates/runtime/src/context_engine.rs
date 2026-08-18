//! Context Engine - 动态上下文组装
//!
//! 核心职责：根据当前 Scene，从数据库中查询相关信息，
//! 按 L0~L6 分层组织，根据 Token Budget 动态选择，
//! 生成最小充分上下文给 Skill。
//!
//! V2 增强：
//! - ContextRequest: 正式输入类型
//! - ContextPolicy: 每个 Skill 不同上下文策略
//! - ContextRanking: 5维评分 (relevance/importance/recency/explicitness/visibility)
//! - 7步检索流程
//! - ContextSnapshot 持久化

use anyhow::{Context, Result};
use db::connection::Database;
use db::repos::{entity_repo, knowledge_repo, narrative_repo, state_repo};
use domain::*;
use domain::skill::{ContextPolicy, ContextLayerType};
use uuid::Uuid;

// ============================================================
// ContextRequest - 正式输入类型
// ============================================================

/// 上下文请求 - Context Engine 的正式输入
#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub project_id: Uuid,
    pub world_id: Uuid,
    pub scene_node_id: Uuid,
    pub skill_type: SkillType,
    pub token_budget: i32,
    /// 额外的上下文要求
    pub extra_requirements: Vec<String>,
}

// ============================================================
// ContextRanking - 5维评分系统
// ============================================================

/// 上下文项评分
#[derive(Debug, Clone)]
pub struct ContextScore {
    /// 相关性 (0.0-1.0)
    pub relevance: f64,
    /// 重要性 (0.0-1.0)
    pub importance: f64,
    /// 时效性 (0.0-1.0, 越近越高)
    pub recency: f64,
    /// 显式程度 (0.0-1.0, 场景中直接提及为1.0)
    pub explicitness: f64,
    /// 知识可见性 (0.0-1.0, 角色知道为1.0)
    pub visibility: f64,
}

impl ContextScore {
    /// 计算综合评分
    pub fn total_score(&self) -> f64 {
        self.relevance * self.importance * self.visibility * self.recency
    }

    /// 默认评分
    pub fn default_score() -> Self {
        Self {
            relevance: 0.5,
            importance: 0.5,
            recency: 0.5,
            explicitness: 0.5,
            visibility: 1.0,
        }
    }
}

// ============================================================
// Token 预算预设
// ============================================================

pub struct TokenBudgets;

impl TokenBudgets {
    pub const SMALL: i32 = 8000;
    pub const MEDIUM: i32 = 12000;
    pub const LARGE: i32 = 20000;
}

// ============================================================
// Context Engine - 信息调度器
// ============================================================

pub struct ContextEngine<'a> {
    db: &'a Database,
}

impl<'a> ContextEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 为指定 Scene 组装上下文包（使用默认策略）
    pub fn build_context(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
    ) -> Result<ContextPackage> {
        self.build_context_with_policy(
            project_id,
            scene_node_id,
            token_budget,
            &ContextPolicy::scene_writer(),
        )
    }

    /// 为指定 Scene 组装上下文包（使用指定策略）
    pub fn build_context_with_policy(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
        policy: &ContextPolicy,
    ) -> Result<ContextPackage> {
        // 获取 World ID
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.db);
        let scene_node = narrative_repo.get_node_by_id(scene_node_id)?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found: {}", scene_node_id))?;
        let world_id = scene_node.world_id;

        // 7步检索流程
        let retrieval_result = self.retrieve_context(
            project_id,
            world_id,
            scene_node_id,
            policy,
        )?;

        // 按策略过滤和排序
        let filtered = self.filter_by_policy(retrieval_result, policy)?;

        // Token Budget 分配
        let context_package = self.allocate_budget(
            project_id,
            scene_node_id,
            token_budget,
            filtered,
            policy,
        )?;

        // 持久化 ContextSnapshot
        let snapshot_repo = db::repos::context_snapshot_repo::ContextSnapshotRepo::new(self.db);
        if let Err(e) = snapshot_repo.save(&context_package) {
            tracing::warn!("Failed to save context snapshot: {}", e);
        }

        Ok(context_package)
    }

    /// 7步检索流程
    fn retrieve_context(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        scene_node_id: Uuid,
        _policy: &ContextPolicy,
    ) -> Result<RetrievalResult> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.db);
        let entity_repo = entity_repo::EntityRepo::new(self.db);
        let state_repo = state_repo::StateRepo::new(self.db);
        let knowledge_repo = knowledge_repo::KnowledgeRepo::new(self.db);

        // 第1步: Scene Explicit Entities
        let scene_node = narrative_repo.get_node_by_id(scene_node_id)?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found"))?;
        let scene_attrs: SceneAttributes = serde_json::from_value(scene_node.attributes.clone())
            .unwrap_or_default();

        // 第2步: Current State
        let characters = self.get_relevant_characters(&scene_attrs, &entity_repo, &state_repo)?;
        let location = self.get_relevant_location(&scene_attrs, &entity_repo, &state_repo)?;

        // 第3步: Relations
        let relations = self.get_relevant_relations(project_id, &scene_attrs, &entity_repo)?;

        // 第4步: Recent Events
        let recent_events = self.get_recent_events(project_id, &scene_attrs)?;

        // 第5步: Knowledge
        let knowledge = if let Some(pov_id) = scene_attrs.pov_character_id {
            self.get_character_knowledge(project_id, pov_id, &knowledge_repo)?
        } else {
            String::new()
        };

        // 第6步: Narrative Context
        let chapter_summary = self.get_chapter_summary(&scene_node, &narrative_repo)?;
        let volume_summary = self.get_volume_summary(project_id)?;
        let arc_summary = self.get_arc_summary(&scene_node, &narrative_repo)?;
        let prev_scene_summary = self.get_previous_scene_summary(project_id, &scene_node, &narrative_repo)?;

        // 第7步: World Rules
        let world_rules = self.get_world_rules_summary(project_id)?;

        Ok(RetrievalResult {
            scene_node,
            scene_attrs,
            characters,
            location,
            relations,
            recent_events,
            knowledge,
            chapter_summary,
            volume_summary,
            arc_summary,
            prev_scene_summary,
            world_rules,
        })
    }

    /// 按策略过滤和排序
    fn filter_by_policy(
        &self,
        result: RetrievalResult,
        policy: &ContextPolicy,
    ) -> Result<FilteredContext> {
        let mut layers = Vec::new();

        // L0: Essential
        if policy.required_layers.contains(&ContextLayerType::L0Essential) ||
           policy.optional_layers.contains(&ContextLayerType::L0Essential) {
            let l0 = self.build_l0(&result.scene_node, &result.scene_attrs, &result.characters, &result.location);
            layers.push((ContextLayerType::L0Essential, l0, ContextScore { relevance: 1.0, importance: 1.0, recency: 1.0, explicitness: 1.0, visibility: 1.0 }));
        }

        // L1: Scene Relevant
        if policy.required_layers.contains(&ContextLayerType::L1SceneRelevant) ||
           policy.optional_layers.contains(&ContextLayerType::L1SceneRelevant) {
            let l1 = self.build_l1(&result.characters, &result.location, &result.relations);
            layers.push((ContextLayerType::L1SceneRelevant, l1, ContextScore { relevance: 0.9, importance: 0.8, recency: 0.9, explicitness: 0.8, visibility: 1.0 }));
        }

        // L2: Recent History
        if policy.required_layers.contains(&ContextLayerType::L2RecentHistory) ||
           policy.optional_layers.contains(&ContextLayerType::L2RecentHistory) {
            let l2 = self.build_l2(&result.prev_scene_summary, &result.chapter_summary, &result.recent_events);
            layers.push((ContextLayerType::L2RecentHistory, l2, ContextScore { relevance: 0.6, importance: 0.5, recency: 0.8, explicitness: 0.5, visibility: 1.0 }));
        }

        // L3: Narrative Context
        if policy.required_layers.contains(&ContextLayerType::L3NarrativeContext) ||
           policy.optional_layers.contains(&ContextLayerType::L3NarrativeContext) {
            let l3 = self.build_l3(&result.volume_summary, &result.arc_summary);
            layers.push((ContextLayerType::L3NarrativeContext, l3, ContextScore { relevance: 0.5, importance: 0.6, recency: 0.5, explicitness: 0.3, visibility: 1.0 }));
        }

        // L4: Character Knowledge
        if policy.required_layers.contains(&ContextLayerType::L4CharacterKnowledge) ||
           policy.optional_layers.contains(&ContextLayerType::L4CharacterKnowledge) {
            let l4 = ContextLayer {
                content: result.knowledge,
                token_estimate: 0,
                included: true,
            };
            layers.push((ContextLayerType::L4CharacterKnowledge, l4, ContextScore { relevance: 0.7, importance: 0.7, recency: 0.6, explicitness: 0.4, visibility: 0.8 }));
        }

        // L5: World Background
        if policy.required_layers.contains(&ContextLayerType::L5WorldBackground) ||
           policy.optional_layers.contains(&ContextLayerType::L5WorldBackground) {
            let l5 = ContextLayer {
                content: result.world_rules,
                token_estimate: 0,
                included: true,
            };
            layers.push((ContextLayerType::L5WorldBackground, l5, ContextScore { relevance: 0.4, importance: 0.5, recency: 0.3, explicitness: 0.2, visibility: 1.0 }));
        }

        // 按评分排序
        layers.sort_by(|a, b| b.2.total_score().partial_cmp(&a.2.total_score()).unwrap_or(std::cmp::Ordering::Equal));

        Ok(FilteredContext { layers })
    }

    /// Token Budget 分配
    fn allocate_budget(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
        filtered: FilteredContext,
        policy: &ContextPolicy,
    ) -> Result<ContextPackage> {
        let max_tokens = (token_budget as f64 * policy.max_budget_ratio) as i32;
        let mut used_tokens = 0;
        let mut included_layers = Vec::new();

        for (layer_type, mut layer, score) in filtered.layers {
            let estimated_tokens = self.estimate_tokens(&layer.content);
            layer.token_estimate = estimated_tokens;

            if used_tokens + estimated_tokens <= max_tokens {
                layer.included = true;
                used_tokens += estimated_tokens;
                included_layers.push((layer_type, layer, score));
            } else {
                layer.included = false;
            }
        }

        // 按层级排序
        included_layers.sort_by_key(|(lt, _, _)| match lt {
            ContextLayerType::L0Essential => 0,
            ContextLayerType::L1SceneRelevant => 1,
            ContextLayerType::L2RecentHistory => 2,
            ContextLayerType::L3NarrativeContext => 3,
            ContextLayerType::L4CharacterKnowledge => 4,
            ContextLayerType::L5WorldBackground => 5,
            ContextLayerType::L6OptionalSupplement => 6,
        });

        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let empty_layer = ContextLayer { content: String::new(), token_estimate: 0, included: false };

        Ok(ContextPackage {
            id,
            project_id,
            scene_id: scene_node_id,
            token_budget,
            l0_essential: included_layers.iter().find(|(lt, _, _)| *lt == ContextLayerType::L0Essential).map(|(_, l, _)| l.clone()).unwrap_or_else(|| empty_layer.clone()),
            l1_scene_relevant: included_layers.iter().find(|(lt, _, _)| *lt == ContextLayerType::L1SceneRelevant).map(|(_, l, _)| l.clone()).unwrap_or_else(|| empty_layer.clone()),
            l2_recent_history: included_layers.iter().find(|(lt, _, _)| *lt == ContextLayerType::L2RecentHistory).map(|(_, l, _)| l.clone()).unwrap_or_else(|| empty_layer.clone()),
            l3_narrative_context: included_layers.iter().find(|(lt, _, _)| *lt == ContextLayerType::L3NarrativeContext).map(|(_, l, _)| l.clone()).unwrap_or_else(|| empty_layer.clone()),
            l4_character_knowledge: included_layers.iter().find(|(lt, _, _)| *lt == ContextLayerType::L4CharacterKnowledge).map(|(_, l, _)| l.clone()).unwrap_or_else(|| empty_layer.clone()),
            l5_world_background: included_layers.iter().find(|(lt, _, _)| *lt == ContextLayerType::L5WorldBackground).map(|(_, l, _)| l.clone()).unwrap_or_else(|| empty_layer.clone()),
            l6_optional_supplement: empty_layer,
            actual_tokens: used_tokens,
            created_at: now,
        })
    }

    // ============================================================
    // 辅助方法
    // ============================================================

    fn get_chapter_summary(&self, scene_node: &NarrativeNode, narrative_repo: &narrative_repo::NarrativeRepo) -> Result<Option<String>> {
        if let Some(parent_id) = scene_node.parent_id {
            if let Some(chapter) = narrative_repo.get_node_by_id(parent_id)? {
                return Ok(Some(format!("Chapter: {} - {}", chapter.title, chapter.description.unwrap_or_default())));
            }
        }
        Ok(None)
    }

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
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }

    fn get_arc_summary(&self, scene_node: &NarrativeNode, narrative_repo: &narrative_repo::NarrativeRepo) -> Result<Option<String>> {
        if let Some(chapter_id) = scene_node.parent_id {
            if let Some(chapter) = narrative_repo.get_node_by_id(chapter_id)? {
                if let Some(arc_id) = chapter.parent_id {
                    if let Some(arc) = narrative_repo.get_node_by_id(arc_id)? {
                        return Ok(Some(format!("Current Arc: {} - {}", arc.title, arc.description.unwrap_or_default())));
                    }
                }
            }
        }
        Ok(None)
    }

    fn get_relevant_characters(
        &self,
        scene_attrs: &SceneAttributes,
        entity_repo: &entity_repo::EntityRepo,
        state_repo: &state_repo::StateRepo,
    ) -> Result<Vec<(Entity, Vec<CurrentState>)>> {
        let mut character_ids = scene_attrs.characters_present.clone();
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !character_ids.contains(&pov_id) {
                character_ids.push(pov_id);
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

    fn get_relevant_relations(
        &self,
        project_id: Uuid,
        scene_attrs: &SceneAttributes,
        entity_repo: &entity_repo::EntityRepo,
    ) -> Result<Vec<Relation>> {
        // Collect all entity IDs mentioned in the scene
        let mut entity_ids: Vec<Uuid> = scene_attrs.characters_present.clone();
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !entity_ids.contains(&pov_id) {
                entity_ids.push(pov_id);
            }
        }
        if let Some(loc_id) = scene_attrs.location_id {
            if !entity_ids.contains(&loc_id) {
                entity_ids.push(loc_id);
            }
        }

        let conn = self.db.conn();
        let mut all_relations = Vec::new();

        // Query relations where source or target is in the scene entities
        for entity_id in &entity_ids {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, valid_from, valid_until, created_at, updated_at FROM relation WHERE project_id = ? AND (source_entity_id = ? OR target_entity_id = ?) ORDER BY created_at DESC LIMIT 20"
            ).context("Failed to prepare relation query")?;

            let rows = stmt.query_map(
                [project_id.to_string(), entity_id.to_string(), entity_id.to_string()],
                |row| {
                    Ok(Relation {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                        project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        source_entity_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                        target_entity_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                        relation_type: row.get(4)?,
                        description: row.get(5)?,
                        attributes: serde_json::from_str(&row.get::<_, String>(6).unwrap_or_default()).unwrap_or_default(),
                        valid_from: row.get(7)?,
                        valid_until: row.get(8)?,
                        created_at: db::time_utils::get_timestamp(row, 9),
                        updated_at: db::time_utils::get_timestamp(row, 10),
                    })
                },
            ).context("Failed to query relations")?;

            for row in rows {
                if let Ok(rel) = row {
                    // Avoid duplicates
                    if !all_relations.iter().any(|r: &Relation| r.id == rel.id) {
                        all_relations.push(rel);
                    }
                }
            }
        }

        Ok(all_relations)
    }

    fn get_character_knowledge(
        &self,
        project_id: Uuid,
        character_id: Uuid,
        knowledge_repo: &knowledge_repo::KnowledgeRepo,
    ) -> Result<String> {
        let states = knowledge_repo.get_character_knowledge(character_id, project_id)?;
        if states.is_empty() {
            return Ok(String::new());
        }

        let mut content = String::from("## Character Knowledge\n");
        for state in &states {
            let level_str = match state.knowledge_level {
                KnowledgeLevel::Unknown => "unknown",
                KnowledgeLevel::Hearsay => "hearsay",
                KnowledgeLevel::Partial => "partial",
                KnowledgeLevel::Complete => "complete",
                KnowledgeLevel::Misunderstood => "misunderstood",
            };
            let knows_str = if state.knows { "knows" } else { "does not know" };
            content.push_str(&format!("- {} ({}): fact_id={}, source={}\n",
                knows_str, level_str,
                state.fact_id,
                state.source.as_deref().unwrap_or("unknown")
            ));
        }
        Ok(content)
    }

    fn get_world_rules_summary(&self, project_id: Uuid) -> Result<String> {
        let conn = self.db.conn();

        // Get world rules from canon_rule table
        let mut content = String::new();
        let stmt_result = conn.prepare(
            "SELECT rule_level, rule_content, affected_scope FROM canon_rule WHERE project_id = ? ORDER BY rule_level"
        );

        if let Ok(mut stmt) = stmt_result {
            if let Ok(rows) = stmt.query_map([project_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, String>(2).unwrap_or_default(),
                ))
            }) {
                let mut has_rules = false;
                for row in rows {
                    if let Ok((level, rule_content, scope)) = row {
                        if !has_rules {
                            content.push_str("## World Rules (Canon Constitution)\n");
                            has_rules = true;
                        }
                        content.push_str(&format!("- [{}] {}: {}\n", level, scope, rule_content));
                    }
                }
            }
        }

        // Also get world description/rules from world table
        let mut stmt_result2 = conn.prepare(
            "SELECT world_rules FROM world WHERE project_id = ? AND is_main = TRUE"
        );
        if let Ok(mut stmt2) = stmt_result2 {
            if let Ok(mut rows) = stmt2.query_map([project_id.to_string()], |row| {
                row.get::<_, Option<String>>(0)
            }) {
                if let Some(Ok(Some(rules))) = rows.next() {
                    if !rules.is_empty() {
                        if content.is_empty() {
                            content.push_str("## World Rules\n");
                        }
                        content.push_str(&format!("{}\n", rules));
                    }
                }
            }
        }

        Ok(content)
    }

    fn get_previous_scene_summary(
        &self,
        _project_id: Uuid,
        current_scene: &NarrativeNode,
        narrative_repo: &narrative_repo::NarrativeRepo,
    ) -> Result<Option<String>> {
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

    /// Get recent events relevant to the scene entities
    fn get_recent_events(
        &self,
        project_id: Uuid,
        scene_attrs: &SceneAttributes,
    ) -> Result<Vec<Event>> {
        let mut entity_ids: Vec<Uuid> = scene_attrs.characters_present.clone();
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !entity_ids.contains(&pov_id) {
                entity_ids.push(pov_id);
            }
        }
        if let Some(loc_id) = scene_attrs.location_id {
            if !entity_ids.contains(&loc_id) {
                entity_ids.push(loc_id);
            }
        }

        let conn = self.db.conn();
        let mut all_events = Vec::new();

        // Query events involving these entities via event_entity junction
        for entity_id in &entity_ids {
            let mut stmt = match conn.prepare(
                "SELECT e.id, e.project_id, e.name, e.description, e.event_type, e.event_time, e.duration, e.created_at, e.updated_at FROM event e INNER JOIN event_entity ee ON e.id = ee.event_id WHERE e.project_id = ? AND ee.entity_id = ? ORDER BY e.created_at DESC LIMIT 5"
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let rows = stmt.query_map(
                [project_id.to_string(), entity_id.to_string()],
                |row| {
                    Ok(Event {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                        project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        name: row.get(2)?,
                        description: row.get(3)?,
                        event_type: row.get(4)?,
                        timestamp: None,
                        event_time: row.get(5)?,
                        duration: row.get(6)?,
                        involved_entity_ids: Vec::new(),
                        state_changes: Vec::new(),
                        created_at: db::time_utils::get_timestamp(row, 7),
                        updated_at: db::time_utils::get_timestamp(row, 8),
                    })
                },
            );

            if let Ok(rows) = rows {
                for row in rows {
                    if let Ok(event) = row {
                        if !all_events.iter().any(|e: &Event| e.id == event.id) {
                            all_events.push(event);
                        }
                    }
                }
            }
        }

        // Sort by created_at descending and limit
        all_events.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all_events.truncate(10);

        Ok(all_events)
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
        content.push_str("## Characters\n");
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
        relations: &[Relation],
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
        if !relations.is_empty() {
            content.push_str("\n## Relevant Relations\n");
            for rel in relations {
                content.push_str(&format!("- {} --[{}]--> {}\n",
                    rel.source_entity_id, rel.relation_type, rel.target_entity_id));
            }
        }

        ContextLayer { content, token_estimate: 0, included: true }
    }

    fn build_l2(
        &self,
        prev_scene_summary: &Option<String>,
        chapter_summary: &Option<String>,
        recent_events: &[Event],
    ) -> ContextLayer {
        let mut content = String::new();
        if let Some(chapter) = chapter_summary {
            content.push_str(&format!("## Current Chapter\n{}\n", chapter));
        }
        if let Some(prev) = prev_scene_summary {
            content.push_str(&format!("## Previous Scene\n{}\n", prev));
        }
        if !recent_events.is_empty() {
            content.push_str("## Recent Events\n");
            for event in recent_events.iter().take(5) {
                content.push_str(&format!("- {}: {}\n", event.name, event.description));
            }
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

    fn estimate_tokens(&self, text: &str) -> i32 {
        let chars = text.chars().count();
        let words = text.split_whitespace().count();
        ((chars as f64 * 0.5 + words as f64 * 0.8) as i32).max(0)
    }
}

// ============================================================
// 辅助类型
// ============================================================

/// 检索结果
struct RetrievalResult {
    scene_node: NarrativeNode,
    scene_attrs: SceneAttributes,
    characters: Vec<(Entity, Vec<CurrentState>)>,
    location: Option<(Entity, Vec<CurrentState>)>,
    relations: Vec<Relation>,
    recent_events: Vec<Event>,
    knowledge: String,
    chapter_summary: Option<String>,
    volume_summary: Option<String>,
    arc_summary: Option<String>,
    prev_scene_summary: Option<String>,
    world_rules: String,
}

/// 过滤后的上下文
struct FilteredContext {
    layers: Vec<(ContextLayerType, ContextLayer, ContextScore)>,
}

// ============================================================
// 测试
// ============================================================

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

        let world_service = application::world_service::WorldService::new(&db);
        let narrative_service = application::narrative_service::NarrativeService::new(&db);

        let world = world_service.ensure_main_world(project.id, "Test Novel").unwrap();

        let char = world_service.create_entity(
            project.id, world.id, "Character", "Lin Fan",
            Some("A young cultivator"), None,
            serde_json::json!({"age": 20}),
        ).unwrap();

        let loc = world_service.create_entity(
            project.id, world.id, "Location", "Black Stone City",
            Some("A border city"), None,
            serde_json::json!({}),
        ).unwrap();

        world_service.set_entity_state(project.id, char.id, "location", serde_json::json!("outside the city")).unwrap();
        world_service.set_entity_state(project.id, char.id, "cultivation", serde_json::json!("Qi Refining Level 3")).unwrap();

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
    }

    #[test]
    fn test_context_policy() {
        let (db, project_id, scene_id) = setup_with_data();
        let engine = ContextEngine::new(&db);

        // Location Designer 策略：不需要 L4 (Character Knowledge)
        let policy = ContextPolicy::location_designer();
        let ctx = engine.build_context_with_policy(project_id, scene_id, TokenBudgets::MEDIUM, &policy).unwrap();
        assert!(!ctx.l4_character_knowledge.included);

        // Scene Writer 策略：需要 L4
        let policy = ContextPolicy::scene_writer();
        let ctx = engine.build_context_with_policy(project_id, scene_id, TokenBudgets::MEDIUM, &policy).unwrap();
        assert!(ctx.l0_essential.included);
    }

    #[test]
    fn test_context_ranking() {
        let score = ContextScore {
            relevance: 1.0,
            importance: 1.0,
            recency: 1.0,
            explicitness: 1.0,
            visibility: 1.0,
        };
        assert_eq!(score.total_score(), 1.0);

        let score_low = ContextScore {
            relevance: 0.1,
            importance: 0.1,
            recency: 0.1,
            explicitness: 0.1,
            visibility: 0.1,
        };
        assert!(score_low.total_score() < 0.01);
    }

    #[test]
    fn test_token_budget_limits() {
        let (db, project_id, scene_id) = setup_with_data();
        let engine = ContextEngine::new(&db);

        let ctx_small = engine.build_context(project_id, scene_id, TokenBudgets::SMALL).unwrap();
        let ctx_large = engine.build_context(project_id, scene_id, TokenBudgets::LARGE).unwrap();

        assert!(ctx_small.actual_tokens <= TokenBudgets::SMALL);
        assert!(ctx_large.actual_tokens <= TokenBudgets::LARGE);
    }
}
