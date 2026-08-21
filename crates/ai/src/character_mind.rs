//! Character Mind Model - 角色认知模型
//!
//! Character Mind Model = Knowledge + Belief + Memory + Goal + Fear + Emotion
//! 三者（Knowledge/Belief/Memory）不要混，对人物行为影响完全不同。
//!
//! 注意：NarrativeState 已移出人物模块，归属叙事引擎（见 domain::narrative）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Belief - 角色信念（角色认为是真的，但不代表世界真的是真的）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    /// 信念内容
    pub belief_content: String,
    /// 信念置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 信念来源（如 "personal_observation", "told_by_someone", "inference"）
    pub source: Option<String>,
    /// 来源场景 ID
    pub source_scene_id: Option<Uuid>,
    /// 信念是否仍然有效
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MemoryType - 记忆类型（R2 新增，区分记忆对行为的影响方式）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    /// 创伤记忆（强驱动，常引发恐惧/回避）
    Traumatic,
    /// 重要记忆（影响决策与价值观）
    Important,
    /// 虚假记忆（角色以为真，实际为假 —— 制造信息差）
    False,
    /// 秘密记忆（角色刻意隐瞒）
    Secret,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Traumatic => "Traumatic",
            MemoryType::Important => "Important",
            MemoryType::False => "False",
            MemoryType::Secret => "Secret",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Traumatic" => MemoryType::Traumatic,
            "Important" => MemoryType::Important,
            "False" => MemoryType::False,
            "Secret" => MemoryType::Secret,
            _ => MemoryType::Important,
        }
    }
}

/// Memory - 角色记忆（过去的经历，影响当前行为）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    /// 记忆内容
    pub memory_content: String,
    /// 记忆类型（R2 新增）
    pub memory_type: Option<MemoryType>,
    /// 情感影响（如 "positive", "negative", "traumatic"）
    pub emotional_impact: Option<String>,
    /// 记忆发生的场景 ID
    pub scene_id: Option<Uuid>,
    /// 记忆重要性 (1-10)
    pub importance: i32,
    /// 记忆是否仍然活跃（某些记忆会随时间淡化）
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// CharacterGoalMind - 角色目标（从 Mind Model 视角）
///
/// 注意：与 character::CharacterDrive 不同，这里关注的是目标的认知维度。
/// CharacterDrive 是结构化的驱动力（目标+动机+紧迫度）。
/// CharacterGoalMind 是目标的"心理"层面：为什么想要这个目标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGoalMind {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    /// 目标描述
    pub goal_content: String,
    /// 目标优先级 (1-10)
    pub priority: i32,
    /// 目标状态
    pub status: GoalStatus,
    /// 目标来源（如 "survival", "revenge", "love"）
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 目标状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Completed,
    Abandoned,
    Blocked,
}

impl GoalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalStatus::Active => "Active",
            GoalStatus::Completed => "Completed",
            GoalStatus::Abandoned => "Abandoned",
            GoalStatus::Blocked => "Blocked",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Active" => GoalStatus::Active,
            "Completed" => GoalStatus::Completed,
            "Abandoned" => GoalStatus::Abandoned,
            "Blocked" => GoalStatus::Blocked,
            _ => GoalStatus::Active,
        }
    }
}

/// Fear - 角色恐惧（影响决策和行为）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterFear {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    /// 恐惧描述
    pub fear_content: String,
    /// 恐惧强度 (1-10)
    pub intensity: i32,
    /// 恐惧来源（如 "past_trauma", "known_threat"）
    pub source: Option<String>,
    /// 是否仍然活跃
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// EmotionState - 角色当前情绪状态
///
/// 性格 ≠ 当前情绪。性格是稳定的，情绪是变化的。
/// Scene 结束后情绪会衰减（decay_rate），下一场 Scene 的人物行为有连续性。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    /// 情绪类型（如 "fear", "anger", "joy", "sadness", "anxiety", "calm"）
    pub emotion_type: String,
    /// 情绪强度 (0-100)
    pub intensity: i32,
    /// 衰减率（每 Scene 衰减多少强度）
    pub decay_rate: f64,
    /// 触发场景 ID
    pub trigger_scene_id: Option<Uuid>,
    /// 触发事件描述
    pub trigger_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
