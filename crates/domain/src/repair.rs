//! PlotRepair - 剧情修复
//!
//! 自动检测和修复剧情问题。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 修复类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepairType {
    /// 自动修复
    Automatic,
    /// 建议修复
    Suggested,
    /// 手动修复
    Manual,
}

/// 修复状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepairStatus {
    /// 待处理
    Pending,
    /// 已应用
    Applied,
    /// 已拒绝
    Rejected,
}

/// 剧情修复记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotRepair {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    pub issue_description: String,
    pub repair_suggestion: String,
    pub repair_type: RepairType,
    pub status: RepairStatus,
    pub applied_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
