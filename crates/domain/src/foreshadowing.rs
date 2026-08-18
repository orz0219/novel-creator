//! Foreshadowing - 伏笔系统
//!
//! 正式的伏笔对象，不是仅仅当成 Prompt。
//! 可以追踪伏笔的状态、推进和揭示。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 伏笔状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForeshadowingStatus {
    /// 已规划
    Planned,
    /// 已引入
    Introduced,
    /// 活跃中
    Active,
    /// 已揭示
    Revealed,
    /// 已废弃
    Abandoned,
}

/// 伏笔重要性
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForeshadowingImportance {
    /// 核心
    Core,
    /// 重要
    Important,
    /// 普通
    Normal,
    /// 微小
    Minor,
}

/// 伏笔暗示级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HintLevel {
    /// 明示
    Explicit,
    /// 直接
    Direct,
    /// 暗示
    Subtle,
    /// 隐晦
    Hidden,
}

/// 伏笔 - 正式的叙事元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foreshadowing {
    pub id: Uuid,
    pub project_id: Uuid,
    pub storyline_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub status: ForeshadowingStatus,
    pub importance: ForeshadowingImportance,
    pub hint_level: HintLevel,
    /// 引入时的章节/场景
    pub introduced_at: Option<String>,
    /// 预期揭示时的章节/场景
    pub expected_reveal_at: Option<String>,
    /// 实际揭示时的章节/场景
    pub actual_reveal_at: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
