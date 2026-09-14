//! 阶段弧线（arc stage）—— 实体在故事时间线上的"外部叙事轨迹"。
//!
//! 与 `CharacterArcPotential`（内在曲线：为什么会变、什么在抵抗）正交：
//! 本模块回答的是「它何时上场、演什么、戏份多大、当时什么状态」。
//!
//! **通用**：人物、势力、地点共用同一张表 / 同一套结构。
//! - 人物：前期只是背景板 → 中期成为合伙人 → 后期成重头戏
//! - 势力：传闻中的小教团 → 主角靠山 → 主要对手
//! - 地点：主角藏身处 → 教团总部 → 两军战场
//!
//! 只出现一段的实体可以只填一段，0 段表示"不填这个字段"，不影响现有数据。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 戏份权重：实体在某一故事阶段的出场分量。
///
/// 做成枚举是为了让前端可以按戏份排序 / 筛选
/// （「哪些角色在中期是重头戏」——排章节时很有用）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScreenWeight {
    Light,
    Medium,
    Heavy,
}

impl ScreenWeight {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenWeight::Light => "Light",
            ScreenWeight::Medium => "Medium",
            ScreenWeight::Heavy => "Heavy",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Light" => ScreenWeight::Light,
            "Heavy" => ScreenWeight::Heavy,
            _ => ScreenWeight::Medium,
        }
    }

    /// 严格解析（语义同 `AgeRange::parse`）：非法值返回 `None`，不静默兜底；
    /// 同时接受中文写法。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Light" | "轻" | "次要" | "背景" | "龙套" => Some(ScreenWeight::Light),
            "Medium" | "中" | "中等" | "重要" | "配角" => Some(ScreenWeight::Medium),
            "Heavy" | "重" | "主要" | "核心" | "重头戏" | "主角" => Some(ScreenWeight::Heavy),
            _ => None,
        }
    }

    pub const ALL: [&'static str; 3] = ["Light", "Medium", "Heavy"];
}

/// 阶段弧线的单个阶段。
///
/// `status` 承载"该阶段的现状快照"：势力写「三百人、占三座城、与主角结盟」，
/// 地点写「被烧毁一半，流民占据」。这是势力唯一能表达"这一卷多强"的地方
/// —— 势力没有 `faction_state` 表，只有一个冻结的 `faction_profile`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityArcStage {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 阶段名：前期 / 中期 / 后期，或卷1 / 卷2，或按地点命名
    pub stage: String,
    /// 排序用整数（越小越早）
    #[serde(rename = "order")]
    pub order: i32,
    /// 此阶段的定位（人物：他是谁；势力：它在主角眼里的分量；地点：叙事位置）。
    ///
    /// 列名沿用 `role`（不迁移已有数据），对外同时接受等价写法 `stage_role`——
    /// 「role」写在势力 / 地点上读起来别扭，但原地改列名会让已有数据与前端一起失效。
    pub role: Option<String>,
    /// 戏份：Light / Medium / Heavy
    pub screen_weight: Option<ScreenWeight>,
    /// 此阶段的目标
    pub goal: Option<String>,
    /// 此阶段的叙事功能
    pub function: Option<String>,
    /// 什么事件把它推进这一阶段（让阶段变成"可推演"的）
    pub entry_trigger: Option<String>,
    /// 该阶段的现状快照 / 备注
    pub status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
