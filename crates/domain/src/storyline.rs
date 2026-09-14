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
//! ```text
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

/// 剧情线状态的合法取值（DB 存的英文值）。
///
/// 供工具层在报错时列出「可写什么」，避免模型凭印象造值——
/// `status` 此前根本没有写入路径，模型即便看到这个字段也无从改起。
pub const STORYLINE_STATUSES: &[&str] = &["Planned", "Active", "Resolved", "Abandoned"];

/// 剧情线重要性的合法取值。
pub const STORYLINE_IMPORTANCES: &[&str] = &["Main", "Important", "Normal", "Minor"];

/// 剧情线之间关系的合法取值。
///
/// 为什么需要它：副线之间除了"挂载"，还有**有向的横向关系**——
///「《破庙居委会》吃得越多 → 《现实线的建设》压力越大」是驱动，
///「与《旧秩序的墙》在神迹外传上对接」是交汇。只有父子树的话，
/// 这些关系只能写在正文散文里，前端也画不出"线咬合图"。
pub const STORYLINE_RELATION_TYPES: &[&str] =
    &["Contains", "Drives", "DependsOn", "Intersects", "Counters"];

/// 剧情线之间的关系类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorylineRelationType {
    /// 包含：主线 - 支线的挂载关系（历史上的唯一语义）
    Contains,
    /// 驱动：A 的推进推动 B（「居委会吃得越多，现实线压力越大」）
    Drives,
    /// 依赖：A 需要 B 先成立
    DependsOn,
    /// 交汇：两条线在某处对接
    Intersects,
    /// 对冲：两条线互相抵消 / 拉扯
    Counters,
}

impl StorylineRelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contains => "Contains",
            Self::Drives => "Drives",
            Self::DependsOn => "DependsOn",
            Self::Intersects => "Intersects",
            Self::Counters => "Counters",
        }
    }

    /// 解析关系类型（含中文别名：包含 / 驱动 / 依赖 / 交汇 / 对冲）。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Contains" | "包含" | "父子" | "挂载" => Some(Self::Contains),
            "Drives" | "驱动" | "推动" => Some(Self::Drives),
            "DependsOn" | "依赖" | "取决于" => Some(Self::DependsOn),
            "Intersects" | "交汇" | "交叉" => Some(Self::Intersects),
            "Counters" | "对冲" | "相抵" => Some(Self::Counters),
            _ => None,
        }
    }
}

impl StorylineStatus {
    /// DB 列取值（英文）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::Active => "Active",
            Self::Resolved => "Resolved",
            Self::Abandoned => "Abandoned",
        }
    }

    /// 解析状态。除英文值外，接受前端表单里的中文写法——这是中文创作系统，
    /// 模型写「进行中」比写 "Active" 自然得多；不认识的取值返回 `None` 由调用方报错。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Planned" | "计划中" => Some(Self::Planned),
            "Active" | "进行中" => Some(Self::Active),
            "Resolved" | "已解决" => Some(Self::Resolved),
            "Abandoned" | "已放弃" | "已废弃" => Some(Self::Abandoned),
            _ => None,
        }
    }
}

impl StorylineImportance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Important => "Important",
            Self::Normal => "Normal",
            Self::Minor => "Minor",
        }
    }

    /// 解析重要性（含中文别名：主线 / 重要 / 普通 / 次要）。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Main" | "主线" => Some(Self::Main),
            "Important" | "重要" | "重要支线" => Some(Self::Important),
            "Normal" | "普通" | "普通支线" => Some(Self::Normal),
            "Minor" | "次要" | "次要支线" => Some(Self::Minor),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_accepts_english_and_chinese() {
        for raw in STORYLINE_STATUSES {
            assert_eq!(
                StorylineStatus::parse(raw).map(|v| v.as_str()),
                Some(*raw),
                "英文值 {} 应自洽",
                raw
            );
        }
        assert_eq!(StorylineStatus::parse("进行中"), Some(StorylineStatus::Active));
        assert_eq!(StorylineStatus::parse("已解决"), Some(StorylineStatus::Resolved));
        assert_eq!(StorylineStatus::parse("Active"), Some(StorylineStatus::Active));
        // 伏笔的状态值不能混用到这里
        assert_eq!(StorylineStatus::parse("Revealed"), None);
    }

    #[test]
    fn importance_is_main_first() {
        for raw in STORYLINE_IMPORTANCES {
            assert_eq!(
                StorylineImportance::parse(raw).map(|v| v.as_str()),
                Some(*raw)
            );
        }
        assert_eq!(StorylineImportance::parse("主线"), Some(StorylineImportance::Main));
        assert_eq!(StorylineImportance::parse("Core"), None, "Core 是伏笔的重要性格，不该被接受");
    }
}
