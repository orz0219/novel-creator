//! Storyline - 跨卷剧情线（树形结构）
//!
//! 剧情线是跨越多个 Volume/Arc 的长期叙事线。
//! 例如：主角成长线、王家线、地下遗迹线、幕后黑手线。
//!
//! ## 树形结构（P2 新增）
//!
//! 每项目有 1 条 Main 主线（树干），可以挂多条 Normal 副线（树枝）。
//! 副线可以继续挂副线（任意深度）。
//!
//! ```
//! 主线: 侦探追凶
//!   ├─ 副线 A（明线）: 搭档感情线
//!   └─ 副线 B（暗线）: 反派真实身份
//!        └─ 副线 C（明线）: 警局内部派系
//! ```
//!
//! ## 明/暗线
//!
//! - 明线（tone=light）: 正常故事推进，读者可见
//! - 暗线（tone=dark + visibility=hidden）: 伏笔/钩子，对用户暴露但对读者隐藏
//!
//! ## 与伏笔（foreshadow）的关系
//!
//! 暗线 = 长期隐藏的剧情线（"幕后黑手是谁"）
//! 伏笔 = 单点事件标记（"第 3 章凶手用左手"）
//! 两者语义不同，存不同表。

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
    /// 主线（树干，每项目 1 条）
    Main,
    /// 重要支线
    Important,
    /// 普通支线
    Normal,
    /// 微小支线
    Minor,
}

/// 剧情线明暗
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorylineTone {
    /// 明线（用户可见）
    Light,
    /// 暗线（伏笔/钩子，用户隐藏）
    Dark,
}

impl StorylineTone {
    /// 字符串解析（与 DB 列值一致：light / dark）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

/// 剧情线可见性
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorylineVisibility {
    /// 暴露给读者
    Visible,
    /// 隐藏（暗线专属）
    Hidden,
}

impl StorylineVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            _ => None,
        }
    }
}

/// 剧情线 - 跨卷的长期叙事线（可挂载到主线/其他副线）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyline {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: StorylineStatus,
    pub importance: StorylineImportance,
    /// 明/暗线
    pub tone: StorylineTone,
    /// 可见性（暗线一般 hidden）
    pub visibility: StorylineVisibility,
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

/// 剧情线挂载关系（树形 parent → child）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorylineRelation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub created_at: DateTime<Utc>,
}
