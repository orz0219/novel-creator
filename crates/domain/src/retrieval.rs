//! Retrieval System - 四种检索接口 + RevisionPlanner
//!
//! 不要把 RAG 当核心。小说世界的信息高度结构化，SQL/Graph 查询远比向量搜索可靠。
//! 检索体系：Structured + Graph + Temporal + Semantic，最后合并。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RetrievalQuery - 统一检索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalQuery {
    pub project_id: Uuid,
    pub scene_id: Option<Uuid>,
    pub character_id: Option<Uuid>,
    pub query_text: Option<String>,
    pub time_range: Option<TimeRange>,
    pub entity_ids: Vec<Uuid>,
    pub max_results: usize,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

/// RetrievalResult - 检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub items: Vec<RetrievalItem>,
    pub total_count: usize,
    pub retrieval_type: RetrievalType,
}

/// 检索结果条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalItem {
    pub id: Uuid,
    pub item_type: String, // "entity", "fact", "event", "relation", "knowledge"
    pub content: serde_json::Value,
    pub relevance_score: f64,
    pub source: String,
}

/// 检索类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RetrievalType {
    /// 结构化检索（SQL 查询）
    Structured,
    /// 图检索（关系图遍历）
    Graph,
    /// 时间检索（时间线查询）
    Temporal,
    /// 语义检索（向量搜索，补充用）
    Semantic,
    /// 合并结果
    Merged,
}

/// RevisionPlanner - 修订规划器
///
/// Validator 不应该直接修改正文，只报告问题。
/// Revision Planner 接收问题列表，生成修订方案。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionPlan {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    /// 原始 draft ID
    pub original_draft_id: Uuid,
    /// Validator 发现的问题
    pub issues: Vec<RevisionIssue>,
    /// 修订方案
    pub revision_strategy: String,
    /// 修订后的 prompt
    pub revision_prompt: String,
    pub created_at: DateTime<Utc>,
}

/// 修订问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionIssue {
    /// 问题类型（"knowledge_violation", "rule_violation", "timeline_conflict"）
    pub issue_type: String,
    /// 严重程度
    pub severity: String,
    /// 问题描述
    pub description: String,
    /// 建议修复方式
    pub suggestion: Option<String>,
    /// 涉及的行/段落范围
    pub location: Option<String>,
}

/// SkillType 扩展 - 增加 RevisionPlanner
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtendedSkillType {
    // 原有类型
    WorldPlanner,
    VolumePlanner,
    ArcPlanner,
    ScenePlanner,
    CharacterDesigner,
    LocationDesigner,
    FactionDesigner,
    PlotDesigner,
    Writer,
    Polisher,
    Analyzer,
    ContinuityValidator,
    KnowledgeExtractor,
    StateChangeExtractor,
    // 新增类型
    RevisionPlanner,
    Custom(String),
}
