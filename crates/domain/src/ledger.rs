//! Narrative Ledger - 叙事账本
//!
//! 每个 Scene 结束后生成结构化账本，记录发生的变化。
//! 这是解决"第500章不需要重新阅读前499章"的关键机制。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Scene Ledger - 场景账本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLedger {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    /// 场景发生的事件
    pub events: Vec<LedgerEvent>,
    /// 获得的物品/能力
    pub gains: Vec<LedgerItem>,
    /// 失去的物品/能力
    pub losses: Vec<LedgerItem>,
    /// 关系变化
    pub relationship_changes: Vec<RelationshipChange>,
    /// 知识变化
    pub knowledge_changes: Vec<KnowledgeChange>,
    /// 世界变化
    pub world_changes: Vec<WorldChange>,
    /// 伏笔提及
    pub foreshadowing_mentions: Vec<ForeshadowingMention>,
    /// 剧情线推进
    pub storyline_progress: Vec<StorylineProgress>,
    /// 人物成长
    pub character_growth: Vec<CharacterGrowth>,
    pub created_at: DateTime<Utc>,
}

/// 账本事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub description: String,
    pub event_type: Option<String>,
    pub involved_entity_ids: Vec<Uuid>,
}

/// 账本条目（获得/失去）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerItem {
    pub item_name: String,
    pub item_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub quantity: Option<f64>,
}

/// 关系变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChange {
    pub entity_a_id: Uuid,
    pub entity_b_id: Uuid,
    pub relation_type: String,
    pub change_delta: i32,
    pub description: Option<String>,
}

/// 知识变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChange {
    pub character_id: Uuid,
    pub fact_description: String,
    pub change_type: String, // "gained", "lost", "updated"
}

/// 世界变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldChange {
    pub entity_id: Uuid,
    pub state_key: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

/// 伏笔提及
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeshadowingMention {
    pub foreshadowing_id: Option<Uuid>,
    pub description: String,
    pub mention_type: String, // "hint", "progress", "reveal"
}

/// 剧情线推进
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorylineProgress {
    pub storyline_id: Uuid,
    pub progress_delta: i32,
    pub description: Option<String>,
}

/// 人物成长
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGrowth {
    pub character_id: Uuid,
    pub growth_type: String, // "skill", "personality", "knowledge"
    pub description: String,
    pub magnitude: i32,
}

/// Context Trace - 上下文来源追踪
///
/// 解释"为什么 AI 会知道这个？"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTraceItem {
    /// 上下文内容
    pub content: String,
    /// 来源实体/事件 ID
    pub source_id: Uuid,
    /// 来源类型（"entity", "event", "fact", "knowledge", "relation"）
    pub source_type: String,
    /// 选择原因
    pub selection_reason: String,
    /// 相关性分数 (0.0 - 1.0)
    pub relevance: f64,
    /// 可见性（谁能看到这个信息）
    pub visibility: Vec<VisibilityInfo>,
}

/// 可见性信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityInfo {
    pub subject_type: String, // "character", "reader", "author"
    pub subject_id: Option<Uuid>,
    pub can_see: bool,
}

/// Decision Trace - AI 决策追踪
///
/// 记录"结构化决策依据"，用于调试人物行为。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTrace {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    pub character_id: Uuid,
    /// 决策内容
    pub decision: String,
    /// 影响因素
    pub factors: Vec<DecisionFactor>,
    pub created_at: DateTime<Utc>,
}

/// 决策因素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionFactor {
    /// 因素类型（如 "emotion", "personality", "goal", "memory", "threat"）
    pub factor_type: String,
    /// 因素描述
    pub description: String,
    /// 因素影响力 (0.0 - 1.0)
    pub influence: f64,
    /// 关联的实体/状态 ID
    pub related_id: Option<Uuid>,
}
