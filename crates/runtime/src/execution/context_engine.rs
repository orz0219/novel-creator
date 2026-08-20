//! Context Engine - 动态上下文组装（编排层）
//!
//! 核心职责：根据当前 Scene，检索相关信息，按 L0~L6 分层组织，
//! 根据 Token Budget 动态选择，生成最小充分上下文给 Skill。
//!
//! 具体逻辑已拆到 `context` 子系统：
//! - `context::ranking` 负责可见性（Visibility）+ 评分排序（Ranking）
//! - `context::budget` 负责 Token 预算分配（Budget）
//! 本文件只做编排。

use anyhow::Result;
use crate::context::{budget, ranking, CharacterTokenEstimator, RetrievalResult, TokenEstimator};
use crate::retrieval::Retriever;
use domain::*;
use domain::generation::{ReproducibilityMeta, RetrievedDocRef};
use domain::ports::*;
use domain::skill::ContextPolicy;
use domain::{deterministic_uuid, sha256_hex};
use std::sync::Arc;
use uuid::Uuid;

// 保持 `runtime::context_engine::{ContextScore, TokenBudgets}` 对外可达
pub use crate::context::{ContextScore, TokenBudgets};

// ============================================================
// Context Engine - 编排
// ============================================================

/// Ports required by the Context Engine. Injected at the composition root.
pub struct ContextEngineDeps {
    pub narrative: Arc<dyn NarrativePort>,
    pub entity: Arc<dyn EntityPort>,
    pub state: Arc<dyn StatePort>,
    pub knowledge: Arc<dyn KnowledgePort>,
    pub relation: Arc<dyn RelationPort>,
    pub event: Arc<dyn EventPort>,
    pub canon: Arc<dyn CanonRulePort>,
    pub snapshot: Arc<dyn ContextSnapshotPort>,
}

pub struct ContextEngine {
    narrative: Arc<dyn NarrativePort>,
    entity: Arc<dyn EntityPort>,
    state: Arc<dyn StatePort>,
    retriever: Arc<Retriever>,
    snapshot: Arc<dyn ContextSnapshotPort>,
    token_estimator: Box<dyn TokenEstimator>,
}

impl ContextEngine {
    pub fn new(deps: ContextEngineDeps) -> Self {
        Self {
            narrative: deps.narrative,
            entity: deps.entity,
            state: deps.state,
            retriever: Arc::new(Retriever::new(
                deps.relation.clone(),
                deps.event.clone(),
                deps.knowledge.clone(),
                deps.canon.clone(),
            )),
            snapshot: deps.snapshot,
            token_estimator: Box::new(CharacterTokenEstimator),
        }
    }

    pub fn with_token_estimator(deps: ContextEngineDeps, estimator: Box<dyn TokenEstimator>) -> Self {
        Self {
            narrative: deps.narrative,
            entity: deps.entity,
            state: deps.state,
            retriever: Arc::new(Retriever::new(
                deps.relation.clone(),
                deps.event.clone(),
                deps.knowledge.clone(),
                deps.canon.clone(),
            )),
            snapshot: deps.snapshot,
            token_estimator: estimator,
        }
    }

    /// 为指定 Scene 组装上下文包（使用默认策略）
    pub async fn build_context(
        &self,
        project_id: Uuid,
        scene_node_id: Uuid,
        token_budget: i32,
        world_version: Option<i32>,
    ) -> Result<ContextPackage> {
        self.build_context_with_policy(
            project_id,
            scene_node_id,
            token_budget,
            &ContextPolicy::scene_writer(),
            world_version,
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
        world_version: Option<i32>,
    ) -> Result<ContextPackage> {
        // 获取 World ID
        // Project-scoped query ensures the scene belongs to the current project.
        let scene_node = self
            .narrative
            .get_node_by_id_with_project(project_id, scene_node_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found: {}", scene_node_id))?;
        let world_id = scene_node.world_id;

        // 1) 检索
        let retrieval_result = self
            .retrieve_context(project_id, world_id, scene_node_id, policy)
            .await?;

        // 1.5) 先固化「检索文档引用」——必须在 ranking 消费 retrieval_result 之前，
        // 因为 ranking::filter 会 move 它。可复现性需要原始检索载荷的 hash。
        let retrieved_doc_refs = build_retrieved_doc_refs(&retrieval_result);

        // 2) 可见性 + 排序（Ranking）
        let filtered = ranking::filter(retrieval_result, policy)?;

        // 3) Token 预算分配（Budget）
        let mut context_package = budget::allocate(
            project_id,
            scene_node_id,
            token_budget,
            filtered,
            policy,
            &*self.token_estimator,
        );

        // 3.5) 填充可复现性元数据（ChatGPT 评审 P1）。
        // 上下文确定性信息在此固定；生成请求参数（model/temperature/prompt_hash）
        // 由 Generation Runtime 负责，不回写 ContextPackage。
        // world_version 由调用方显式传入——不让 Context Engine 自己查 latest，
        // 否则相同输入在不同时间可能得到不同快照，破坏 reproducibility。
        context_package.reproducibility = ReproducibilityMeta {
            world_version,
            // ContextPolicy 本身无数值版本（以 name 标识），故此处留空；
            // 策略身份已并入 retrieval_strategy 文本，保证快照可追溯到具体策略。
            context_policy_version: None,
            retrieval_strategy: Some(format!(
                "relation+event+knowledge+canon (policy={})",
                policy.name
            )),
            retrieved_documents: retrieved_doc_refs,
            ..Default::default()
        };

        // 4) 持久化 ContextSnapshot
        if let Err(e) = self.snapshot.save(&context_package).await {
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
        // 第1步: Scene Explicit Entities
        let scene_node = self
            .narrative
            .get_node_by_id_with_project(project_id, scene_node_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scene node not found"))?;
        let scene_attrs: SceneAttributes =
            serde_json::from_value(scene_node.attributes.clone()).unwrap_or_default();

        // 第2步: Current State
        let characters = self
            .get_relevant_characters(project_id, &scene_attrs, &*self.entity, &*self.state)
            .await?;
        let location = self
            .get_relevant_location(project_id, &scene_attrs, &*self.entity, &*self.state)
            .await?;

        // 第3步: Relations
        let relations = self.retriever.get_relevant_relations(project_id, &scene_attrs).await?;

        // 第4步: Recent Events
        let recent_events = self.retriever.get_recent_events(project_id, &scene_attrs).await?;

        // 第5步: Knowledge
        let knowledge = if let Some(pov_id) = scene_attrs.pov_character_id {
            self.retriever.get_character_knowledge(project_id, pov_id).await?
        } else {
            String::new()
        };

        // 第6步: Narrative Context (all project-scoped)
        let chapter_summary = self.get_chapter_summary(project_id, &scene_node, &*self.narrative).await?;
        let volume_summary = self.get_volume_summary(project_id, &scene_node, &*self.narrative).await?;
        let arc_summary = self.get_arc_summary(project_id, &scene_node, &*self.narrative).await?;
        let prev_scene_summary = self
            .get_previous_scene_summary(project_id, &scene_node, &*self.narrative)
            .await?;

        // 第7步: World Rules
        let world_rules = self.retriever.get_world_rules_summary(project_id).await?;

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

    // ============================================================
    // 辅助方法
    // ============================================================

    async fn get_chapter_summary(
        &self,
        project_id: Uuid,
        scene_node: &NarrativeNode,
        narrative: &dyn NarrativePort,
    ) -> Result<Option<String>> {
        if let Some(parent_id) = scene_node.parent_id {
            // P2-2: 使用 project-scoped 查询确保 chapter 属于当前 project
            if let Some(chapter) = narrative.get_node_by_id_with_project(project_id, parent_id).await? {
                return Ok(Some(format!(
                    "Chapter: {} - {}",
                    chapter.title,
                    chapter.description.unwrap_or_default()
                )));
            }
        }
        Ok(None)
    }

    async fn get_volume_summary(
        &self,
        project_id: Uuid,
        scene_node: &NarrativeNode,
        narrative: &dyn NarrativePort,
    ) -> Result<Option<String>> {
        // Walk up: Scene → Chapter → Arc → Volume
        let volume = if let Some(chapter_id) = scene_node.parent_id {
            if let Some(chapter) = narrative.get_node_by_id_with_project(project_id, chapter_id).await? {
                if let Some(arc_id) = chapter.parent_id {
                    if let Some(arc) = narrative.get_node_by_id_with_project(project_id, arc_id).await? {
                        arc.parent_id
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(vol_id) = volume {
            if let Some(vol) = narrative.get_node_by_id_with_project(project_id, vol_id).await? {
                let attrs: VolumeAttributes =
                    serde_json::from_value(vol.attributes.clone()).unwrap_or_default();
                let mut summary = format!("Current Volume: {}", vol.title);
                if let Some(mission) = &attrs.mission {
                    summary.push_str(&format!("\nMission: {}", mission));
                }
                return Ok(Some(summary));
            }
        }

        // P2-1: 删除 first-volume fallback
        Ok(None)
    }

    async fn get_arc_summary(
        &self,
        project_id: Uuid,
        scene_node: &NarrativeNode,
        narrative: &dyn NarrativePort,
    ) -> Result<Option<String>> {
        if let Some(chapter_id) = scene_node.parent_id {
            if let Some(chapter) = narrative.get_node_by_id_with_project(project_id, chapter_id).await? {
                if let Some(arc_id) = chapter.parent_id {
                    if let Some(arc) = narrative.get_node_by_id_with_project(project_id, arc_id).await? {
                        return Ok(Some(format!(
                            "Current Arc: {} - {}",
                            arc.title,
                            arc.description.unwrap_or_default()
                        )));
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
        entity: &dyn EntityPort,
        state: &dyn StatePort,
    ) -> Result<Vec<(Entity, Vec<CurrentState>)>> {
        let mut character_ids = scene_attrs.characters_present.clone();
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !character_ids.contains(&pov_id) {
                character_ids.push(pov_id);
            }
        }

        // Batch query entities
        let entities = entity.list_entities_by_ids(project_id, &character_ids).await?;
        let entity_map: std::collections::HashMap<Uuid, Entity> =
            entities.into_iter().map(|e| (e.id, e)).collect();

        // Batch query states
        let all_states = state.list_current_states_batch(project_id, &character_ids).await?;
        let mut state_map: std::collections::HashMap<Uuid, Vec<CurrentState>> =
            std::collections::HashMap::new();
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
        entity: &dyn EntityPort,
        state: &dyn StatePort,
    ) -> Result<Option<(Entity, Vec<CurrentState>)>> {
        if let Some(loc_id) = scene_attrs.location_id {
            // P1-6: 使用 project-scoped 查询确保 location 属于当前 project
            if let Some(entity) = entity.get_entity_by_id_with_project(project_id, loc_id).await? {
                let states = state.list_current_states(project_id, loc_id).await?;
                return Ok(Some((entity, states)));
            }
        }
        Ok(None)
    }

    async fn get_previous_scene_summary(
        &self,
        _project_id: Uuid,
        current_scene: &NarrativeNode,
        narrative: &dyn NarrativePort,
    ) -> Result<Option<String>> {
        if let Some(parent_id) = current_scene.parent_id {
            let siblings = narrative.list_children(parent_id).await?;
            let scenes: Vec<&NarrativeNode> = siblings
                .iter()
                .filter(|n| n.node_type == NarrativeNodeType::Scene)
                .collect();

            if let Some(idx) = scenes.iter().position(|s| s.id == current_scene.id) {
                if idx > 0 {
                    let prev = scenes[idx - 1];
                    return Ok(Some(format!(
                        "Previous Scene: {} - {}",
                        prev.title,
                        prev.description.as_deref().unwrap_or_default()
                    )));
                }
            }
        }
        Ok(None)
    }
}

/// 把检索结果转为可复现的文档引用列表。
///
/// 每个引用带一个「确定性 id」（由 label 派生，保证跨次生成一致）与
/// 一份内容 sha256。hash 的是「规范化后的检索载荷」而非数据库行——
/// 这样 DB 行更新时间的变化不会改变语义 hash，能检测检索内容漂移，
/// 也支持未来不同的 storage backend。
fn build_retrieved_doc_refs(r: &RetrievalResult) -> Vec<RetrievedDocRef> {
    let mut docs = Vec::new();
    let mut push = |label: &str, content: String| {
        if !content.trim().is_empty() {
            docs.push(RetrievedDocRef {
                id: deterministic_uuid(label),
                hash: sha256_hex(&content),
            });
        }
    };

    if let Ok(s) = serde_json::to_string(&r.characters) {
        push("characters", s);
    }
    if let Some((e, st)) = &r.location {
        if let Ok(s) = serde_json::to_string(&(e, st)) {
            push("location", s);
        }
    }
    if let Ok(s) = serde_json::to_string(&r.relations) {
        push("relations", s);
    }
    if let Ok(s) = serde_json::to_string(&r.recent_events) {
        push("recent_events", s);
    }
    push("knowledge", r.knowledge.clone());
    push("world_rules", r.world_rules.clone());
    if let Some(c) = &r.chapter_summary {
        push("chapter_summary", c.clone());
    }
    if let Some(c) = &r.volume_summary {
        push("volume_summary", c.clone());
    }
    if let Some(c) = &r.arc_summary {
        push("arc_summary", c.clone());
    }
    if let Some(c) = &r.prev_scene_summary {
        push("prev_scene_summary", c.clone());
    }
    docs
}
