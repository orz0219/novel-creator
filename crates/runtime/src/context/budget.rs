//! Context Budget
//!
//! Token 预算分配：required layers 必含（不受预算限制），
//! optional layers 按 score/token 成本填充剩余预算。
//! 不直接接触数据库。

use chrono::Utc;
use domain::*;
use domain::skill::ContextPolicy;
use uuid::Uuid;
use crate::context::{ContextScore, FilteredContext};

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

/// Token 预算预设
pub struct TokenBudgets;

impl TokenBudgets {
    pub const SMALL: i32 = 8000;
    pub const MEDIUM: i32 = 12000;
    pub const LARGE: i32 = 20000;
}

/// 在过滤后的上下文上执行 Token 预算分配，产出 ContextPackage。
pub fn allocate(
    project_id: Uuid,
    scene_node_id: Uuid,
    token_budget: i32,
    filtered: FilteredContext,
    policy: &ContextPolicy,
    estimator: &dyn TokenEstimator,
) -> ContextPackage {
    let max_tokens = (token_budget as f64 * policy.max_budget_ratio) as i32;
    let mut used_tokens = 0;
    let mut included_layers = Vec::new();

    // Step 1: Separate required and optional layers
    let mut required: Vec<(ContextLayerType, ContextLayer, ContextScore)> = Vec::new();
    let mut optional: Vec<(ContextLayerType, ContextLayer, ContextScore)> = Vec::new();

    for (layer_type, mut layer, score) in filtered.layers {
        let estimated_tokens = estimator.estimate(&layer.content);
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
        let score_a = if a.1.token_estimate > 0 {
            a.2.total_score() / a.1.token_estimate as f64
        } else {
            f64::MAX
        };
        let score_b = if b.1.token_estimate > 0 {
            b.2.total_score() / b.1.token_estimate as f64
        } else {
            f64::MAX
        };
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
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
        l0_essential: included_layers
            .iter()
            .find(|(lt, _, _)| *lt == ContextLayerType::L0Essential)
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| empty_layer.clone()),
        l1_scene_relevant: included_layers
            .iter()
            .find(|(lt, _, _)| *lt == ContextLayerType::L1SceneRelevant)
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| empty_layer.clone()),
        l2_recent_history: included_layers
            .iter()
            .find(|(lt, _, _)| *lt == ContextLayerType::L2RecentHistory)
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| empty_layer.clone()),
        l3_narrative_context: included_layers
            .iter()
            .find(|(lt, _, _)| *lt == ContextLayerType::L3NarrativeContext)
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| empty_layer.clone()),
        l4_character_knowledge: included_layers
            .iter()
            .find(|(lt, _, _)| *lt == ContextLayerType::L4CharacterKnowledge)
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| empty_layer.clone()),
        l5_world_background: included_layers
            .iter()
            .find(|(lt, _, _)| *lt == ContextLayerType::L5WorldBackground)
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| empty_layer.clone()),
        l6_optional_supplement: empty_layer,
        actual_tokens: used_tokens,
        created_at: now,
    }
}
