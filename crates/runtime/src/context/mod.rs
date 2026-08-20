//! Context 子系统 - 可见性 / 排序 / 预算 / 组装
//!
//! ContextEngine 只负责编排：
//!
//! ```text
//! Retrieval (Retriever)
//!     ↓
//! Visibility + Ranking (context::ranking)
//!     ↓
//! Token Budget (context::budget)
//!     ↓
//! ContextPackage
//! ```
//!
//! 策略（ContextPolicy / ContextLayerType）定义在 domain::skill；
//! 这里只负责"按策略决定可见性 + 评分排序 + 预算分配"。

pub mod budget;
pub mod ranking;

use domain::*;
use domain::skill::{ContextLayerType, SkillType};
use uuid::Uuid;

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

/// 检索结果的内部聚合结构（Retriever 产出，交给 ranking 过滤）
pub struct RetrievalResult {
    pub scene_node: NarrativeNode,
    pub scene_attrs: SceneAttributes,
    pub characters: Vec<(Entity, Vec<CurrentState>)>,
    pub location: Option<(Entity, Vec<CurrentState>)>,
    pub relations: Vec<Relation>,
    pub recent_events: Vec<Event>,
    pub knowledge: String,
    pub chapter_summary: Option<String>,
    pub volume_summary: Option<String>,
    pub arc_summary: Option<String>,
    pub prev_scene_summary: Option<String>,
    pub world_rules: String,
}

/// 按策略过滤后的上下文（待 budget 分配）
pub struct FilteredContext {
    pub layers: Vec<(ContextLayerType, ContextLayer, ContextScore)>,
}

// 集中导出 Context 相关公共类型
pub use crate::context::budget::{CharacterTokenEstimator, TokenBudgets, TokenEstimator};
pub use crate::context::ranking::ContextScore;
