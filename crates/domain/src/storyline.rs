//! Storyline - 跨卷剧情线
//!
//! 剧情线是跨越多个 Volume/Arc 的长期叙事线。
//! 例如：主角成长线、王家线、地下遗迹线、幕后黑手线。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 剧情线状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorylineStatus {
    /// 已规划
    Planned,
    /// 进行中
    Active,
    /// 已解决
    Resolved,
    /// 已废弃
    Abandoned,
}

/// 剧情线重要性
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorylineImportance {
    /// 主线
    Main,
    /// 重要支线
    Important,
    /// 普通支线
    Normal,
    /// 微小支线
    Minor,
}

/// 剧情线 - 跨卷的长期叙事线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyline {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: StorylineStatus,
    pub importance: StorylineImportance,
    /// 创建时的 Volume ID
    pub created_volume_id: Option<Uuid>,
    /// 解决时的 Volume ID
    pub resolved_volume_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 剧情线-场景关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorylineScene {
    pub id: Uuid,
    pub storyline_id: Uuid,
    pub scene_id: Uuid,
    /// 该场景对剧情线的叙事意义
    pub significance: Option<String>,
    pub created_at: DateTime<Utc>,
}
