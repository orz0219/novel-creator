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

/// 伏笔状态的合法取值（DB 存的英文值），供工具层列出「可写什么」。
pub const FORESHADOWING_STATUSES: &[&str] =
    &["Planned", "Introduced", "Active", "Revealed", "Abandoned"];

/// 伏笔重要性的合法取值。
pub const FORESHADOWING_IMPORTANCES: &[&str] = &["Core", "Important", "Normal", "Minor"];

/// 伏笔暗示级别的合法取值。
///
/// 注意：工具层此前把这些字符串直接写库，默认值甚至是 `"low"`——
/// 而合法取值里根本没有 `low`，于是模型建出来的伏笔暗示级别是个非法值。
pub const HINT_LEVELS: &[&str] = &["Explicit", "Direct", "Subtle", "Hidden"];

impl ForeshadowingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::Introduced => "Introduced",
            Self::Active => "Active",
            Self::Revealed => "Revealed",
            Self::Abandoned => "Abandoned",
        }
    }

    /// 解析状态，接受前端表单里的中文写法（计划中 / 已引入 / 进行中 / 已揭示 / 已放弃）。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Planned" | "计划中" => Some(Self::Planned),
            "Introduced" | "已引入" => Some(Self::Introduced),
            "Active" | "进行中" => Some(Self::Active),
            "Revealed" | "已揭示" => Some(Self::Revealed),
            "Abandoned" | "已放弃" | "已废弃" => Some(Self::Abandoned),
            _ => None,
        }
    }
}

impl ForeshadowingImportance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Important => "Important",
            Self::Normal => "Normal",
            Self::Minor => "Minor",
        }
    }

    /// 解析重要性（伏笔用 Core，与剧情线的 Main 不同）。
    ///
    /// 中文别名按"作者会怎么随口写"来收：「主要」也算 Important。
    /// 注意 `Main` **不**接受——它是剧情线的等级，混用会导致前端展示错位。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Core" | "核心" => Some(Self::Core),
            "Important" | "重要" | "主要" => Some(Self::Important),
            "Normal" | "普通" => Some(Self::Normal),
            "Minor" | "次要" => Some(Self::Minor),
            _ => None,
        }
    }
}

impl HintLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Explicit => "Explicit",
            Self::Direct => "Direct",
            Self::Subtle => "Subtle",
            Self::Hidden => "Hidden",
        }
    }

    /// 解析暗示级别（明示 / 直接 / 隐晦 / 隐藏）。
    ///
    /// `Hidden` 的注释在枚举里写作「隐晦」，前端标签是「隐藏」——两种写法都接受，
    /// 免得模型按枚举注释写就解析失败。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Explicit" | "明示" | "高" => Some(Self::Explicit),
            "Direct" | "直接" | "中" => Some(Self::Direct),
            "Subtle" | "隐晦" | "暗示" | "低" => Some(Self::Subtle),
            "Hidden" | "隐藏" => Some(Self::Hidden),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_accepts_english_and_chinese() {
        // 英文值必须与前端 ForeshadowingStatus 类型、DB 列取值完全一致
        for raw in ["Planned", "Introduced", "Active", "Revealed", "Abandoned"] {
            assert_eq!(
                ForeshadowingStatus::parse(raw).map(|s| s.as_str()),
                Some(raw),
                "英文值 {} 应自洽",
                raw
            );
        }
        // 中文写法（前端表单标签）也要认
        assert_eq!(ForeshadowingStatus::parse("进行中"), Some(ForeshadowingStatus::Active));
        assert_eq!(ForeshadowingStatus::parse("已揭示"), Some(ForeshadowingStatus::Revealed));
        // 未知值必须返回 None 由调用方报错，不能静默降级成某个默认值
        assert_eq!(ForeshadowingStatus::parse("随便写的"), None);
        assert_eq!(ForeshadowingStatus::parse("planned"), None, "大小写不匹配应报错而非猜测");
    }

    #[test]
    fn hint_level_rejects_legacy_invalid_default() {
        // 工具层曾把 "low" 当默认值写库，而合法集合里没有它——这条断言锁住不再回流
        assert_eq!(HintLevel::parse("low"), None);
        assert_eq!(HintLevel::parse("Subtle"), Some(HintLevel::Subtle));
        assert_eq!(HintLevel::parse("隐晦"), Some(HintLevel::Subtle));
        assert_eq!(HintLevel::parse("隐藏"), Some(HintLevel::Hidden));
        // 作者视角的自然写法：暗示程度 低 / 中 / 高
        assert_eq!(HintLevel::parse("低"), Some(HintLevel::Subtle));
        assert_eq!(HintLevel::parse("中"), Some(HintLevel::Direct));
        assert_eq!(HintLevel::parse("高"), Some(HintLevel::Explicit));
        // 但历史脏值不再回流：大小写变体与自由文本一律拒绝
        assert_eq!(HintLevel::parse("Low"), None);
        assert_eq!(HintLevel::parse("低（前期只透风，不揭示）"), None);
        for raw in HINT_LEVELS {
            assert_eq!(HintLevel::parse(raw).map(|v| v.as_str()), Some(*raw));
        }
    }

    #[test]
    fn importance_is_core_not_main() {
        // 伏笔用 Core，剧情线用 Main——两者不可混用（曾由此产生前端展示错位）
        for raw in FORESHADOWING_IMPORTANCES {
            assert_eq!(
                ForeshadowingImportance::parse(raw).map(|v| v.as_str()),
                Some(*raw)
            );
        }
        assert_eq!(ForeshadowingImportance::parse("Main"), None);
        assert_eq!(ForeshadowingImportance::parse("主要"), Some(ForeshadowingImportance::Important));
        assert_eq!(ForeshadowingImportance::parse("核心"), Some(ForeshadowingImportance::Core));
    }
}
