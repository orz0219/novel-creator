//! Knowledge - 知识模型
//!
//! 核心原则：世界知道什么 ≠ 人物知道什么 ≠ 读者知道什么
//! 通过 Fact + KnowledgeState + Revelation 机制控制信息差。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 知识主体类型 - 谁在"知道"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeSubjectType {
    /// 作者本人（全知视角）
    Author,
    /// 某个角色
    Character,
    /// 读者
    Reader,
    /// 某个势力/组织
    Faction,
}

/// 知识状态 - 某个主体对某个事实的认知状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub fact_id: Uuid,
    pub subject_type: KnowledgeSubjectType,
    pub subject_id: Option<Uuid>,
    pub knows: bool,
    /// 知道的程度（部分知道 / 完全知道 / 误解）
    pub knowledge_level: KnowledgeLevel,
    /// 知道的来源（如何得知的）
    pub source: Option<String>,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 知识程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeLevel {
    /// 完全不知道
    Unknown,
    /// 听说过但不确定
    Hearsay,
    /// 部分知道
    Partial,
    /// 完全知道
    Complete,
    /// 误解（知道的是错误的信息）
    Misunderstood,
}

/// 揭示 - 某个知识在某个场景中被揭示
///
/// 这是控制伏笔、悬念、信息差的核心机制。
/// 当一个 Revelation 发生时，对应主体的 KnowledgeState 会更新。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revelation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub fact_id: Uuid,
    pub scene_id: Uuid,
    /// 哪些主体获得了这个知识
    pub revealed_to: Vec<RevelationTarget>,
    /// 揭示的方式（亲眼看到 / 听说 / 推理 / 被告知）
    pub revelation_method: Option<String>,
    /// 这个揭示的叙事意义
    pub narrative_significance: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 揭示目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevelationTarget {
    pub subject_type: KnowledgeSubjectType,
    pub subject_id: Option<Uuid>,
    pub knowledge_level: KnowledgeLevel,
}
/// A fact a character actually knows, joined with fact content.
///
/// Returned by the knowledge retrieval port so the Context Engine can render
/// what a point-of-view character genuinely knows (vs. world truth).
pub struct CharacterKnowledgeItem {
    pub fact_content: String,
    pub fact_category: Option<String>,
    pub fact_certainty: String,
    pub knowledge_level: KnowledgeLevel,
    pub source: Option<String>,
}