//! Context Engine - 动态上下文组装
//!
//! 核心职责：根据当前 Scene，从数据库中查询相关信息，
//! 按 L0~L6 分层组织，根据 Token Budget 动态选择，
//! 生成最小充分上下文给 Skill。

use anyhow::Result;
use chrono::{DateTime, Utc};
use db::repos::{entity_repo, knowledge_repo, narrative_repo, state_repo};
use domain::*;
use domain::skill::{ContextLayerType, ContextPolicy};
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================
// TokenEstimator - Token 估算抽象
// ============================================================

/// Token 估算器 trait
///
/// Context Engine 不直接调用 chars().count()，而是通过此 trait 估算。
/// 默认实现 CharacterTokenEstimator 使用字符数估算。
/// 未来可接入 Qwen/DeepSeek/Claude 等模型的 tokenizer。
pub trait TokenEstimator: Send + Sync {
    fn estimate(&self, text: &str) -> i32;
}

/// 基于字符数的 Token 估算器（默认实现）
///
/// 使用 chars().count() 而非 len()，正确处理中文 UTF-8。
/// 估算公式：(char_count * 2) / 3
pub struct CharacterTokenEstimator;

impl TokenEstimator for CharacterTokenEstimator {
    fn estimate(&self, text: &str) -> i32 {
        let char_count = text.chars().count() as i32;
        (char_count * 2) / 3
    }
}

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
    pub extra_requirements: Vec<String>,
}

// ============================================================
// ContextRanking - 5维评分系统
// ============================================================

/// 上下文项评分
#[derive(Debug, Clone)]
pub struct ContextScore {
    pub relevance: f64,
    pub importance: f64,
    pub recency: f64,
    pub explicitness: f64,
    pub visibility: f64,
}

impl ContextScore {
    pub fn total_score(&self) -> f64 {
        self.relevance * self.importance * self.visibility * self.recency
    }

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

pub struct ContextEngine {
    pool: PgPool,
    token_estimator: Box<dyn TokenEstimator>,
}

impl ContextEngine {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            token_estimator: Box::new(CharacterTokenEstimator),
        }
    }

    pub fn with_token_estimator(pool: PgPool, estimator: Box<dyn TokenEstimator>) -> Self {
        Self {
            pool,
            token_estimator: estimator,
        }
    }

    /// 为指定 Scene 组装上下文包（使用默认策略）
    pub async fn build_context(
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
        .await
    }

    /// 为指定 Scene 组装上下文包（使用指定策略）
    pub async fn build_context_with_policy(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
        policy: &ContextPolicy,
    ) -> Result<ContextPackage> {
        // 获取 World ID
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let scene_node = narrative_repo
            .get_node_by_id(scene_node_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found: {}", scene_node_id))?;
        let world_id = scene_node.world_id;

        // 7步检索流程
        let retrieval_result = self.retrieve_context(project_id, world_id, scene_node_id, policy).await?;

        // 按策略过滤和排序
        let filtered = self.filter_by_policy(retrieval_result, policy)?;

        // Token Budget 分配
        let context_package = self.allocate_budget(project_id, scene_node_id, token_budget, filtered, policy);

        // 持久化 ContextSnapshot
        let snapshot_repo = db::repos::context_snapshot_repo::ContextSnapshotRepo::new(self.pool.clone());
        if let Err(e) = snapshot_repo.save(&context_package).await {
            tracing::warn!("Failed to save context snapshot: {}", e);
        }

        Ok(context_package)
    }

    /// 7步检索流程
    async fn retrieve_context(
        &self,
        project_id: Uuid,
        _world_id: Uuid,
        scene_node_id: Uuid,
        _policy: &ContextPolicy,
    ) -> Result<RetrievalResult> {
        let narrative_repo = narrative_repo::NarrativeRepo::new(self.pool.clone());
        let entity_repo = entity_repo::EntityRepo::new(self.pool.clone());
        let state_repo = state_repo::StateRepo::new(self.pool.clone());
        let knowledge_repo = knowledge_repo::KnowledgeRepo::new(self.pool.clone());

        // 第1步: Scene Explicit Entities
        let scene_node = narrative_repo
            .get_node_by_id(scene_node_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found"))?;
        let scene_attrs: SceneAttributes = serde_json::from_value(scene_node.attributes.clone()).unwrap_or_default();

        // 第2步: Current State
        let characters = self.get_relevant_characters(project_id, &scene_attrs, &entity_repo, &state_repo).await?;
        let location = self.get_relevant_location(project_id, &scene_attrs, &entity_repo, &state_repo).await?;

        // 第3步: Relations
        let relations = self.get_relevant_relations(project_id, &scene_attrs).await?;

        // 第4步: Recent Events
        let recent_events = self.get_recent_events(project_id, &scene_attrs).await?;

        // 第5步: Knowledge
        let knowledge = if let Some(pov_id) = scene_attrs.pov_character_id {
            self.get_character_knowledge(project_id, pov_id, &knowledge_repo).await?
        } else {
            String::new()
        };

        // 第6步: Narrative Context
        let chapter_summary = self.get_chapter_summary(&scene_node, &narrative_repo).await?;
        let volume_summary = self.get_volume_summary(project_id, &scene_node, &narrative_repo).await?;
        let arc_summary = self.get_arc_summary(&scene_node, &narrative_repo).await?;
        let prev_scene_summary = self.get_previous_scene_summary(project_id, &scene_node, &narrative_repo).await?;

        // 第7步: World Rules
        let world_rules = self.get_world_rules_summary(project_id).await?;

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
    fn filter_by_policy(&self, result: RetrievalResult, policy: &ContextPolicy) -> Result<FilteredContext> {
        let mut layers = Vec::new();

        // L0: Essential
        if policy.required_layers.contains(&ContextLayerType::L0Essential)
            || policy.optional_layers.contains(&ContextLayerType::L0Essential)
        {
            let l0 = self.build_l0(&result.scene_node, &result.scene_attrs, &result.characters, &result.location);
            layers.push((
                ContextLayerType::L0Essential,
                l0,
                ContextScore {
                    relevance: 1.0,
                    importance: 1.0,
                    recency: 1.0,
                    explicitness: 1.0,
                    visibility: 1.0,
                },
            ));
        }

        // L1: Scene Relevant
        if policy.required_layers.contains(&ContextLayerType::L1SceneRelevant)
            || policy.optional_layers.contains(&ContextLayerType::L1SceneRelevant)
        {
            let l1 = self.build_l1(&result.characters, &result.location, &result.relations);
            layers.push((
                ContextLayerType::L1SceneRelevant,
                l1,
                ContextScore {
                    relevance: 0.9,
                    importance: 0.8,
                    recency: 0.9,
                    explicitness: 0.8,
                    visibility: 1.0,
                },
            ));
        }

        // L2: Recent History
        if policy.required_layers.contains(&ContextLayerType::L2RecentHistory)
            || policy.optional_layers.contains(&ContextLayerType::L2RecentHistory)
        {
            let l2 = self.build_l2(&result.prev_scene_summary, &result.chapter_summary, &result.recent_events);
            layers.push((
                ContextLayerType::L2RecentHistory,
                l2,
                ContextScore {
                    relevance: 0.6,
                    importance: 0.5,
                    recency: 0.8,
                    explicitness: 0.5,
                    visibility: 1.0,
                },
            ));
        }

        // L3: Narrative Context
        if policy.required_layers.contains(&ContextLayerType::L3NarrativeContext)
            || policy.optional_layers.contains(&ContextLayerType::L3NarrativeContext)
        {
            let l3 = self.build_l3(&result.volume_summary, &result.arc_summary);
            layers.push((
                ContextLayerType::L3NarrativeContext,
                l3,
                ContextScore {
                    relevance: 0.5,
                    importance: 0.6,
                    recency: 0.5,
                    explicitness: 0.3,
                    visibility: 1.0,
                },
            ));
        }

        // L4: Character Knowledge
        if policy.required_layers.contains(&ContextLayerType::L4CharacterKnowledge)
            || policy.optional_layers.contains(&ContextLayerType::L4CharacterKnowledge)
        {
            let l4 = ContextLayer {
                content: result.knowledge,
                token_estimate: 0,
                included: true,
            };
            layers.push((
                ContextLayerType::L4CharacterKnowledge,
                l4,
                ContextScore {
                    relevance: 0.7,
                    importance: 0.7,
                    recency: 0.6,
                    explicitness: 0.4,
                    visibility: 0.8,
                },
            ));
        }

        // L5: World Background
        if policy.required_layers.contains(&ContextLayerType::L5WorldBackground)
            || policy.optional_layers.contains(&ContextLayerType::L5WorldBackground)
        {
            let l5 = ContextLayer {
                content: result.world_rules,
                token_estimate: 0,
                included: true,
            };
            layers.push((
                ContextLayerType::L5WorldBackground,
                l5,
                ContextScore {
                    relevance: 0.4,
                    importance: 0.5,
                    recency: 0.3,
                    explicitness: 0.2,
                    visibility: 1.0,
                },
            ));
        }

        // 按评分排序
        layers.sort_by(|a, b| b.2.total_score().partial_cmp(&a.2.total_score()).unwrap_or(std::cmp::Ordering::Equal));

        Ok(FilteredContext { layers })
    }

    /// Token Budget 分配
    ///
    /// Required layers are ALWAYS included regardless of budget.
    /// Optional layers fill remaining budget, sorted by score/token_cost.
    fn allocate_budget(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
        filtered: FilteredContext,
        policy: &ContextPolicy,
    ) -> ContextPackage {
        let max_tokens = (token_budget as f64 * policy.max_budget_ratio) as i32;
        let mut used_tokens = 0;
        let mut included_layers = Vec::new();

        // Step 1: Separate required and optional layers
        let mut required: Vec<(ContextLayerType, ContextLayer, ContextScore)> = Vec::new();
        let mut optional: Vec<(ContextLayerType, ContextLayer, ContextScore)> = Vec::new();

        for (layer_type, mut layer, score) in filtered.layers {
            let estimated_tokens = self.estimate_tokens(&layer.content);
            layer.token_estimate = estimated_tokens;

            if policy.required_layers.contains(&layer_type) {
                required.push((layer_type, layer, score));
            } else {
                optional.push((layer_type, layer, score));
            }
        }

        // Step 2: Required layers are ALWAYS included (forced, no budget check)
        for (layer_type, mut layer, score) in required {
            layer.included = true;
            used_tokens += layer.token_estimate;
            included_layers.push((layer_type, layer, score));
        }

        // Step 3: Optional layers fill remaining budget, sorted by score/token_cost
        optional.sort_by(|a, b| {
            let score_a = if a.1.token_estimate > 0 { a.2.total_score() / a.1.token_estimate as f64 } else { f64::MAX };
            let score_b = if b.1.token_estimate > 0 { b.2.total_score() / b.1.token_estimate as f64 } else { f64::MAX };
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (layer_type, mut layer, score) in optional {
            if used_tokens + layer.token_estimate <= max_tokens {
                layer.included = true;
                used_tokens += layer.token_estimate;
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
        let now = Utc::now();
        let empty_layer = ContextLayer {
            content: String::new(),
            token_estimate: 0,
            included: false,
        };

        ContextPackage {
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
        }
    }

    // ============================================================
    // 辅助方法
    // ============================================================

    async fn get_chapter_summary(&self, scene_node: &NarrativeNode, narrative_repo: &narrative_repo::NarrativeRepo) -> Result<Option<String>> {
        if let Some(parent_id) = scene_node.parent_id {
            if let Some(chapter) = narrative_repo.get_node_by_id(parent_id).await? {
                return Ok(Some(format!("Chapter: {} - {}", chapter.title, chapter.description.unwrap_or_default())));
            }
        }
        Ok(None)
    }

    async fn get_volume_summary(&self, project_id: Uuid, scene_node: &NarrativeNode, narrative_repo: &narrative_repo::NarrativeRepo) -> Result<Option<String>> {
        // Walk up: Scene → Chapter → Arc → Volume
        let volume = if let Some(chapter_id) = scene_node.parent_id {
            if let Some(chapter) = narrative_repo.get_node_by_id(chapter_id).await? {
                if let Some(arc_id) = chapter.parent_id {
                    if let Some(arc) = narrative_repo.get_node_by_id(arc_id).await? {
                        arc.parent_id
                    } else { None }
                } else { None }
            } else { None }
        } else { None };

        if let Some(vol_id) = volume {
            if let Some(vol) = narrative_repo.get_node_by_id(vol_id).await? {
                let attrs: VolumeAttributes = serde_json::from_value(vol.attributes.clone()).unwrap_or_default();
                let mut summary = format!("Current Volume: {}", vol.title);
                if let Some(mission) = &attrs.mission {
                    summary.push_str(&format!("\nMission: {}", mission));
                }
                return Ok(Some(summary));
            }
        }

        // Fallback: if scene has no parent chain, try first volume
        let all = narrative_repo.list_nodes_by_project(project_id).await?;
        let volumes: Vec<&NarrativeNode> = all.iter().filter(|n| n.node_type == NarrativeNodeType::Volume).collect();
        if let Some(vol) = volumes.first() {
            let attrs: VolumeAttributes = serde_json::from_value(vol.attributes.clone()).unwrap_or_default();
            let mut summary = format!("Current Volume: {}", vol.title);
            if let Some(mission) = &attrs.mission {
                summary.push_str(&format!("\nMission: {}", mission));
            }
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }

    async fn get_arc_summary(&self, scene_node: &NarrativeNode, narrative_repo: &narrative_repo::NarrativeRepo) -> Result<Option<String>> {
        if let Some(chapter_id) = scene_node.parent_id {
            if let Some(chapter) = narrative_repo.get_node_by_id(chapter_id).await? {
                if let Some(arc_id) = chapter.parent_id {
                    if let Some(arc) = narrative_repo.get_node_by_id(arc_id).await? {
                        return Ok(Some(format!("Current Arc: {} - {}", arc.title, arc.description.unwrap_or_default())));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn get_relevant_characters(
        &self,
        project_id: Uuid,
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

        // Batch query entities
        let entities = entity_repo.list_by_ids(project_id, &character_ids).await?;
        let entity_map: std::collections::HashMap<Uuid, Entity> = entities.into_iter().map(|e| (e.id, e)).collect();

        // Batch query states
        let all_states = state_repo.list_current_states_batch(project_id, &character_ids).await?;
        let mut state_map: std::collections::HashMap<Uuid, Vec<CurrentState>> = std::collections::HashMap::new();
        for state in all_states {
            state_map.entry(state.entity_id).or_default().push(state);
        }

        let mut result = Vec::new();
        for cid in &character_ids {
            if let Some(entity) = entity_map.get(cid) {
                let states = state_map.remove(cid).unwrap_or_default();
                result.push((entity.clone(), states));
            }
        }
        Ok(result)
    }

    async fn get_relevant_location(
        &self,
        project_id: Uuid,
        scene_attrs: &SceneAttributes,
        entity_repo: &entity_repo::EntityRepo,
        state_repo: &state_repo::StateRepo,
    ) -> Result<Option<(Entity, Vec<CurrentState>)>> {
        if let Some(loc_id) = scene_attrs.location_id {
            if let Some(entity) = entity_repo.get_by_id(loc_id).await? {
                let states = state_repo.list_current_states(project_id, loc_id).await?;
                return Ok(Some((entity, states)));
            }
        }
        Ok(None)
    }

    async fn get_relevant_relations(&self, project_id: Uuid, scene_attrs: &SceneAttributes) -> Result<Vec<Relation>> {
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

        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Single batch query instead of N+1
        let rows = match sqlx::query_as::<_, RelationRow>(
            "SELECT id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, valid_from, valid_until, created_at, updated_at \
             FROM relation WHERE project_id = $1 AND (source_entity_id = ANY($2) OR target_entity_id = ANY($2)) ORDER BY created_at DESC"
        )
        .bind(project_id)
        .bind(&entity_ids)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Failed to query relations for project {}: {}. Treating as empty.", project_id, e);
                return Ok(Vec::new());
            }
        };

        let relations: Vec<Relation> = rows.into_iter().map(|r| r.into()).collect();
        Ok(relations)
    }

    async fn get_character_knowledge(
        &self,
        project_id: Uuid,
        character_id: Uuid,
        knowledge_repo: &knowledge_repo::KnowledgeRepo,
    ) -> Result<String> {
        let items = knowledge_repo.get_character_known_facts(character_id, project_id).await?;
        if items.is_empty() {
            return Ok(String::new());
        }

        let mut content = String::from("## Character Knowledge (what this character actually knows)\n");
        for item in &items {
            let level_str = match item.knowledge_level {
                KnowledgeLevel::Unknown => "unknown",
                KnowledgeLevel::Hearsay => "hearsay",
                KnowledgeLevel::Partial => "partial",
                KnowledgeLevel::Complete => "complete",
                KnowledgeLevel::Misunderstood => "misunderstood",
            };
            let certainty_tag = if item.fact_certainty != "CANON" {
                format!(" [{}]", item.fact_certainty)
            } else {
                String::new()
            };
            let source_info = item.source.as_deref().unwrap_or("unknown");
            content.push_str(&format!("- {} ({}){}: {}\n", level_str, source_info, certainty_tag, item.fact_content));
        }
        Ok(content)
    }

    async fn get_world_rules_summary(&self, project_id: Uuid) -> Result<String> {
        let mut content = String::new();

        // Get world rules from canon_rule table
        let rows = match sqlx::query_as::<_, (String, String, String)>(
            "SELECT rule_level, rule_content, affected_scope FROM canon_rule WHERE project_id = $1 ORDER BY rule_level"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Failed to query canon rules for project {}: {}. World rules will be incomplete.", project_id, e);
                Vec::new()
            }
        };

        if !rows.is_empty() {
            content.push_str("## World Rules (Canon Constitution)\n");
            for (level, rule_content, scope) in rows {
                content.push_str(&format!("- [{}] {}: {}\n", level, scope, rule_content));
            }
        }

        // Also get world description/rules from world table
        let world_rules: Option<String> = match sqlx::query_scalar(
            "SELECT world_rules FROM world WHERE project_id = $1 AND is_main = TRUE"
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(opt) => opt,
            Err(e) => {
                tracing::warn!("Failed to query world rules for project {}: {}. World rules will be incomplete.", project_id, e);
                None
            }
        };

        if let Some(rules) = world_rules {
            if !rules.is_empty() {
                if content.is_empty() {
                    content.push_str("## World Rules\n");
                }
                content.push_str(&format!("{}\n", rules));
            }
        }

        Ok(content)
    }

    async fn get_previous_scene_summary(
        &self,
        _project_id: Uuid,
        current_scene: &NarrativeNode,
        narrative_repo: &narrative_repo::NarrativeRepo,
    ) -> Result<Option<String>> {
        if let Some(parent_id) = current_scene.parent_id {
            let siblings = narrative_repo.list_children(parent_id).await?;
            let scenes: Vec<&NarrativeNode> = siblings.iter().filter(|n| n.node_type == NarrativeNodeType::Scene).collect();

            if let Some(idx) = scenes.iter().position(|s| s.id == current_scene.id) {
                if idx > 0 {
                    let prev = scenes[idx - 1];
                    return Ok(Some(format!("Previous Scene: {} - {}", prev.title, prev.description.as_deref().unwrap_or_default())));
                }
            }
        }
        Ok(None)
    }

    async fn get_recent_events(&self, project_id: Uuid, scene_attrs: &SceneAttributes) -> Result<Vec<Event>> {
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

        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Single batch query instead of N+1
        let rows = match sqlx::query_as::<_, EventRow>(
            "SELECT DISTINCT e.id, e.project_id, e.name, e.description, e.event_type, e.event_time, e.duration, e.created_at, e.updated_at \
             FROM event e INNER JOIN event_entity ee ON e.id = ee.event_id \
             WHERE e.project_id = $1 AND ee.entity_id = ANY($2) ORDER BY e.created_at DESC LIMIT 20"
        )
        .bind(project_id)
        .bind(&entity_ids)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Failed to query events for project {}: {}. Treating as empty.", project_id, e);
                return Ok(Vec::new());
            }
        };

        let events: Vec<Event> = rows.into_iter().map(|r| r.into()).collect();
        Ok(events)
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

        ContextLayer {
            content,
            token_estimate: 0,
            included: true,
        }
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
            content.push_str(&format!("\n## Location: {}\n", loc.name));
            for state in states {
                content.push_str(&format!("  {}: {}\n", state.state_key, state.state_value));
            }
        }
        if !relations.is_empty() {
            content.push_str("\n## Relations\n");
            for rel in relations {
                content.push_str(&format!("- {} --{}--> {}\n", rel.source_entity_id, rel.relation_type, rel.target_entity_id));
            }
        }

        ContextLayer {
            content,
            token_estimate: 0,
            included: true,
        }
    }

    fn build_l2(
        &self,
        prev_scene_summary: &Option<String>,
        chapter_summary: &Option<String>,
        recent_events: &[Event],
    ) -> ContextLayer {
        let mut content = String::new();
        if let Some(prev) = prev_scene_summary {
            content.push_str(&format!("## {}\n", prev));
        }
        if let Some(chapter) = chapter_summary {
            content.push_str(&format!("## {}\n", chapter));
        }
        if !recent_events.is_empty() {
            content.push_str("## Recent Events\n");
            for event in recent_events {
                content.push_str(&format!("- {}: {}\n", event.name, event.description));
            }
        }

        ContextLayer {
            content,
            token_estimate: 0,
            included: true,
        }
    }

    fn build_l3(&self, volume_summary: &Option<String>, arc_summary: &Option<String>) -> ContextLayer {
        let mut content = String::new();
        if let Some(vol) = volume_summary {
            content.push_str(&format!("## {}\n", vol));
        }
        if let Some(arc) = arc_summary {
            content.push_str(&format!("## {}\n", arc));
        }

        ContextLayer {
            content,
            token_estimate: 0,
            included: true,
        }
    }

    fn estimate_tokens(&self, content: &str) -> i32 {
        self.token_estimator.estimate(content)
    }
}

// ============================================================
// 内部数据结构
// ============================================================

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

struct FilteredContext {
    layers: Vec<(ContextLayerType, ContextLayer, ContextScore)>,
}

#[derive(sqlx::FromRow)]
struct RelationRow {
    id: Uuid,
    project_id: Uuid,
    source_entity_id: Uuid,
    target_entity_id: Uuid,
    relation_type: String,
    description: Option<String>,
    attributes: Option<serde_json::Value>,
    valid_from: Option<String>,
    valid_until: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<RelationRow> for Relation {
    fn from(r: RelationRow) -> Self {
        Relation {
            id: r.id,
            project_id: r.project_id,
            source_entity_id: r.source_entity_id,
            target_entity_id: r.target_entity_id,
            relation_type: r.relation_type,
            description: r.description,
            attributes: r.attributes.unwrap_or_default(),
            valid_from: r.valid_from,
            valid_until: r.valid_until,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: String,
    event_type: Option<String>,
    event_time: Option<String>,
    duration: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EventRow> for Event {
    fn from(r: EventRow) -> Self {
        Event {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            event_type: r.event_type,
            timestamp: None,
            event_time: r.event_time,
            duration: r.duration,
            involved_entity_ids: Vec::new(),
            state_changes: Vec::new(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}