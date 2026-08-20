//! Context Ranking + Visibility
//!
//! 决定哪些 Layer 对当前 Scene 可见（Visibility），并给出 5 维评分与排序（Ranking）。
//! 不直接接触数据库，也不做 Token 预算。

use anyhow::Result;
use domain::*;
use domain::skill::ContextPolicy;
use crate::context::{FilteredContext, RetrievalResult};

/// 上下文项评分（5 维）
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
        self.relevance * self.importance * self.visibility * self.recency * self.explicitness
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

/// 按策略过滤 + 排序，产出 FilteredContext。
///
/// 可见性由 policy 的 required/optional layers 决定；
/// 排序由 ContextScore 决定（评分高者优先进入预算分配）。
pub fn filter(result: RetrievalResult, policy: &ContextPolicy) -> Result<FilteredContext> {
    let mut layers = Vec::new();

    // L0: Essential
    if policy.required_layers.contains(&ContextLayerType::L0Essential)
        || policy.optional_layers.contains(&ContextLayerType::L0Essential)
    {
        let l0 = build_l0(&result.scene_node, &result.scene_attrs, &result.characters, &result.location);
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
        let l1 = build_l1(&result.characters, &result.location, &result.relations);
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
        let l2 = build_l2(&result.prev_scene_summary, &result.chapter_summary, &result.recent_events);
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
        let l3 = build_l3(&result.volume_summary, &result.arc_summary);
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
    layers.sort_by(|a, b| {
        b.2.total_score()
            .partial_cmp(&a.2.total_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(FilteredContext { layers })
}

fn build_l0(
    scene_node: &NarrativeNode,
    scene_attrs: &SceneAttributes,
    characters: &[(Entity, Vec<CurrentState>)],
    location: &Option<(Entity, Vec<CurrentState>)>,
) -> ContextLayer {
    let mut content = String::new();
    content.push_str(&format!(
        "## Scene Objective\n{}\n",
        scene_attrs.objective.as_deref().unwrap_or(&scene_node.title)
    ));
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
            content.push_str(&format!(
                "- {} --{}--> {}\n",
                rel.source_entity_id, rel.relation_type, rel.target_entity_id
            ));
        }
    }

    ContextLayer {
        content,
        token_estimate: 0,
        included: true,
    }
}

fn build_l2(
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

fn build_l3(volume_summary: &Option<String>, arc_summary: &Option<String>) -> ContextLayer {
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
