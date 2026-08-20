//! State Management - 状态管理、回滚、知识缺口、多级记忆
//!
//! Rollback: 每次 Scene Commit 保存 State Before/After，支持回滚到任意 Scene。
//! Knowledge Gap: 发现缺失信息时记录，由 Designer Agent 自动补充。
//! Multi-level Memory: Scene→Chapter→Arc→Volume→Global 多级记忆层次。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// State Snapshot - 状态快照（用于回滚）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    /// 场景提交前的状态
    pub state_before: serde_json::Value,
    /// 场景产生的变更
    pub changes: serde_json::Value,
    /// 场景提交后的状态
    pub state_after: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Knowledge Gap - 知识缺口
///
/// 当 Context Engine 发现缺失信息时记录，由 Designer Agent 自动补充。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 缺口类型（如 "LOCATION_DETAIL", "CHARACTER_BACKGROUND", "WORLD_RULE"）
    pub gap_type: String,
    /// 缺口描述
    pub description: String,
    /// 重要性（HIGH, MEDIUM, LOW）
    pub importance: String,
    /// 需要此信息的场景 ID
    pub required_by_scene_id: Option<Uuid>,
    /// 缺口状态（Open, Filled, Ignored）
    pub status: GapStatus,
    /// 建议使用的 Designer Skill
    pub designer_skill_hint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 缺口状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GapStatus {
    Open,
    Filled,
    Ignored,
}

impl GapStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GapStatus::Open => "Open",
            GapStatus::Filled => "Filled",
            GapStatus::Ignored => "Ignored",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Filled" => GapStatus::Filled,
            "Ignored" => GapStatus::Ignored,
            _ => GapStatus::Open,
        }
    }
}

/// Chapter Summary - 章节摘要（多级记忆：最近 Chapter 中等详细度）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub id: Uuid,
    pub project_id: Uuid,
    pub chapter_id: Uuid,
    /// 章节摘要内容
    pub summary: String,
    /// 关键事件列表
    pub key_events: Vec<String>,
    /// 涉及的角色
    pub involved_characters: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Arc Summary - 弧线摘要（多级记忆：当前 Arc 摘要）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcSummary {
    pub id: Uuid,
    pub project_id: Uuid,
    pub arc_id: Uuid,
    /// 弧线摘要内容
    pub summary: String,
    /// 关键转折点
    pub key_turning_points: Vec<String>,
    /// 弧线状态
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Volume Summary - 卷摘要（多级记忆：当前/过去 Volume 摘要）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSummary {
    pub id: Uuid,
    pub project_id: Uuid,
    pub volume_id: Uuid,
    /// 卷摘要内容
    pub summary: String,
    /// 重要人物变化
    pub character_changes: Vec<String>,
    /// 世界变化
    pub world_changes: Vec<String>,
    /// 伏笔进展
    pub foreshadowing_progress: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Global Story State - 全局故事状态（多级记忆：最顶层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStoryState {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 当前进度描述
    pub current_progress: String,
    /// 未解决的伏笔
    pub open_foreshadowing: Vec<String>,
    /// 未解决的剧情线
    pub open_storylines: Vec<String>,
    /// 世界当前状态概要
    pub world_state_summary: String,
    /// 主要角色当前状态概要
    pub character_state_summary: String,
    pub updated_at: DateTime<Utc>,
}
