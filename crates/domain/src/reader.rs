//! ReaderKnowledge - 读者认知状态
//!
//! 控制读者知道什么、不知道什么。
//! 与 CharacterKnowledge 分离，实现信息差控制。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 读者知识程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReaderKnowledgeLevel {
    /// 完全不知道
    Unknown,
    /// 听说过
    Hearsay,
    /// 怀疑
    Suspected,
    /// 部分知道
    Partial,
    /// 完全知道
    Complete,
    /// 误解
    Misunderstood,
}

/// 读者知识信心度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReaderConfidence {
    /// 确定
    Certain,
    /// 可能
    Likely,
    /// 不确定
    Uncertain,
    /// 猜测
    Speculative,
}

/// 读者认知 - 控制读者知道什么
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderKnowledge {
    pub id: Uuid,
    pub project_id: Uuid,
    pub fact_id: Uuid,
    pub knowledge_level: ReaderKnowledgeLevel,
    /// 引入这个知识的场景
    pub source_scene_id: Option<Uuid>,
    pub confidence: ReaderConfidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
