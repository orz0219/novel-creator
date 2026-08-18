//! Branch - 版本分支系统
//!
//! 支持世界和叙事的分支，允许 "what if" 场景。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 分支状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BranchStatus {
    /// 活跃
    Active,
    /// 已合并
    Merged,
    /// 已废弃
    Abandoned,
}

/// 世界分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBranch {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_branch_id: Option<Uuid>,
    pub is_main: bool,
    pub status: BranchStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 叙事分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeBranch {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_branch_id: Option<Uuid>,
    pub fork_point_scene_id: Option<Uuid>,
    pub is_main: bool,
    pub status: BranchStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
