//! CausalRelation - 因果链
//!
//! 追踪事件之间的因果关系，让 AI 不只是知道"发生过什么"，
//! 而是知道"为什么会发生"。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 因果关系类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalRelationType {
    /// 直接原因
    DirectCause,
    /// 间接原因
    IndirectCause,
    /// 触发条件
    Trigger,
    /// 前提条件
    Prerequisite,
    /// 促进因素
    ContributingFactor,
}

/// 因果关系强度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CausalStrength {
    /// 强因果
    Strong,
    /// 中等因果
    Moderate,
    /// 弱因果
    Weak,
}

/// 因果关系 - 事件之间的因果链
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalRelation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub cause_event_id: Uuid,
    pub effect_event_id: Uuid,
    pub relation_type: CausalRelationType,
    pub strength: CausalStrength,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
