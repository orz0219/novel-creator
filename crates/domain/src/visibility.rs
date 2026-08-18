//! Visibility - 可见性控制
//!
//! 控制谁能看到什么信息。
//! 核心原则：World Truth ≠ Character Knowledge ≠ Reader Knowledge。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 可见性级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VisibilityLevel {
    /// 完全可见
    Visible,
    /// 只能看到"有这个事实"，不能看内容
    ExistsOnly,
    /// 完全隐藏
    Hidden,
}

/// 知识主体类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VisibilitySubjectType {
    /// 作者本人
    Author,
    /// 叙事规划者
    NarrativePlanner,
    /// 场景作者
    SceneWriter,
    /// 角色
    Character,
    /// 读者
    Reader,
}

/// 事实可见性 - 控制 Fact 对不同主体的可见性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactVisibility {
    pub id: Uuid,
    pub project_id: Uuid,
    pub fact_id: Uuid,
    pub subject_type: VisibilitySubjectType,
    pub subject_id: Option<Uuid>,
    pub visibility_level: VisibilityLevel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
