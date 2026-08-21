//! Character 子结构 - 通用小说人物设定模型
//!
//! 设计原则（来自与 ChatGPT 协作修订的 R2 方案）：
//! - 人物是「剧情发动机」，每个字段都要能直接或间接驱动情节；
//! - 跨题材通用：标量身份约束 + 剧情核心 + 题材扩展(Extension) 三层；
//! - 不绑定单一题材（玄幻/都市/科幻/悬疑/言情共用同一模块）；
//! - 不是真人档案，也不是心理测评，是程序可读、供自动小说引擎消费的结构化数据。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===================== 通用枚举 =====================

/// 叙事年龄：生成用的年龄区间，不是真实年龄
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgeRange {
    Child,
    Teen,
    YoungAdult,
    Adult,
    MiddleAge,
    Elder,
    Unknown,
}

impl AgeRange {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgeRange::Child => "Child",
            AgeRange::Teen => "Teen",
            AgeRange::YoungAdult => "YoungAdult",
            AgeRange::Adult => "Adult",
            AgeRange::MiddleAge => "MiddleAge",
            AgeRange::Elder => "Elder",
            AgeRange::Unknown => "Unknown",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Child" => AgeRange::Child,
            "Teen" => AgeRange::Teen,
            "YoungAdult" => AgeRange::YoungAdult,
            "Adult" => AgeRange::Adult,
            "MiddleAge" => AgeRange::MiddleAge,
            "Elder" => AgeRange::Elder,
            _ => AgeRange::Unknown,
        }
    }
}

/// 性别：定位为身份约束(identity_constraint)，不是性格
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    NonBinary,
    Unknown,
    Other,
}

impl Gender {
    pub fn as_str(&self) -> &'static str {
        match self {
            Gender::Male => "Male",
            Gender::Female => "Female",
            Gender::NonBinary => "NonBinary",
            Gender::Unknown => "Unknown",
            Gender::Other => "Other",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Male" => Gender::Male,
            "Female" => Gender::Female,
            "NonBinary" => Gender::NonBinary,
            "Unknown" => Gender::Unknown,
            _ => Gender::Other,
        }
    }
}

/// 角色在故事中的功能位（role_in_story）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoryRole {
    Protagonist,
    Antagonist,
    Mentor,
    Ally,
    Rival,
    Catalyst,
    Victim,
    Observer,
}

impl StoryRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoryRole::Protagonist => "Protagonist",
            StoryRole::Antagonist => "Antagonist",
            StoryRole::Mentor => "Mentor",
            StoryRole::Ally => "Ally",
            StoryRole::Rival => "Rival",
            StoryRole::Catalyst => "Catalyst",
            StoryRole::Victim => "Victim",
            StoryRole::Observer => "Observer",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Protagonist" => StoryRole::Protagonist,
            "Antagonist" => StoryRole::Antagonist,
            "Mentor" => StoryRole::Mentor,
            "Ally" => StoryRole::Ally,
            "Rival" => StoryRole::Rival,
            "Catalyst" => StoryRole::Catalyst,
            "Victim" => StoryRole::Victim,
            _ => StoryRole::Observer,
        }
    }
}

/// 冲突类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConflictType {
    Internal,
    External,
    Relationship,
    Ideology,
}

impl ConflictType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictType::Internal => "Internal",
            ConflictType::External => "External",
            ConflictType::Relationship => "Relationship",
            ConflictType::Ideology => "Ideology",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Internal" => ConflictType::Internal,
            "External" => ConflictType::External,
            "Relationship" => ConflictType::Relationship,
            "Ideology" => ConflictType::Ideology,
            _ => ConflictType::Internal,
        }
    }
}

// ===================== 组合子结构 =====================

/// 社会位置（social_status 的抽象：不写死"贵族/平民"）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocialPosition {
    pub rank: Option<String>,
    pub authority_level: Option<i32>,
    pub social_access: Vec<String>,
}

/// 剧情必要性（R2 新增）：告诉引擎谁该重点写、谁可以死、谁可替换
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NarrativeNecessity {
    pub importance: i32,
    pub irreplaceability: i32,
    pub absence_effect: Option<String>,
    pub replacement_cost: Option<String>,
}

/// 驱动力（合并原 CharacterGoal 的多级目标，并补上恐惧/弱点/欲望/矛盾）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterDrive {
    pub primary_goal: Option<String>,
    pub motivation: Option<String>,
    pub urgency: i32,
    pub long_term: Option<String>,
    pub current: Option<String>,
    pub immediate: Option<String>,
    pub hidden_goal: Option<String>,
    pub fear: Option<String>,
    pub weakness: Option<String>,
    pub desire: Option<String>,
    pub contradiction: Option<String>,
}

/// 冲突（人物真正参与剧情的接口）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterConflict {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub conflict_type: ConflictType,
    pub description: String,
    pub target_entity_id: Option<Uuid>,
    pub resolution_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 关系（支撑联盟/背叛/冲突/羁绊）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRelationship {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub target_entity_id: Uuid,
    pub relationship_type: String,
    pub attitude: String,
    pub trust_level: i32,
    pub secret_knowledge: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 秘密（驱动悬念/反转/信息差）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSecret {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub content: String,
    pub importance: i32,
    pub reveal_condition: Option<String>,
    pub related_entities: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 能力边界（限制比能力更重要）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterCapability {
    pub skills: Vec<String>,
    pub limitations: Vec<String>,
}

/// 弧光潜力（人物成长线）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterArcPotential {
    pub starting_state: Option<String>,
    pub possible_change: Option<String>,
    pub resistance: Option<String>,
}

/// 题材扩展：类型化枚举 + JSON 兜底（约 80% typed / 20% extra）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CharacterExtension {
    Fantasy(FantasyExtension),
    Modern(ModernExtension),
    SciFi(SciFiExtension),
    Custom(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FantasyExtension {
    pub cultivation_level: Option<String>,
    pub realm: Option<String>,
    pub bloodline: Option<String>,
    pub magic_affinity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModernExtension {
    pub occupation: Option<String>,
    pub income_level: Option<String>,
    pub assets: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SciFiExtension {
    pub cybernetics: Option<Vec<String>>,
    pub augmentation_level: Option<String>,
    pub faction: Option<String>,
}

// ===================== 主结构 =====================

/// Character Profile - 人物的稳定身份约束（通用核心）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterProfile {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub name: Option<String>,
    pub aliases: Vec<String>,
    /// 叙事年龄区间（不是真实年龄）
    pub age: Option<AgeRange>,
    /// 性别（身份约束，不是性格）
    pub gender: Option<Gender>,
    /// 身份（如"边境散修"、"王家家主"）
    pub identity: Option<String>,
    /// 外貌描述
    pub appearance: Option<String>,
    /// 背景故事（导致现在状态的关键经历）
    pub background_origin: Option<String>,
    /// 社会位置（social_status 的抽象）
    pub social_position: Option<SocialPosition>,
    /// 核心性格
    pub core_personality: Option<String>,
    /// 价值观
    pub values: Option<String>,
    /// 角色在故事中的功能位
    pub role_in_story: Option<StoryRole>,
    /// 剧情必要性
    pub narrative_necessity: Option<NarrativeNecessity>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character State - 人物的当前状态（抽象通用层；题材相关进 Extension）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterState {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 当前位置
    pub location: Option<String>,
    /// 身体状态（替代 health，可含 limitations）
    pub physical_state: Option<String>,
    /// 心理状态（R2 新增，上一版缺失）
    pub mental_state: Option<String>,
    /// 资源状态（抽象：钱/灵石/能源本质都是资源）
    pub resource_state: Option<String>,
    /// 社会状态（替代 wanted，含 legal_status）
    pub social_state: Option<String>,
    /// 状态标记（如 flags）
    pub flags: Vec<String>,
    /// 自由扩展状态
    pub extra: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character Trait - 人物的特征（扁平化，去掉 parent_trait_id 层级嵌套）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTrait {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 特征类型
    pub trait_type: TraitType,
    /// 特征名称（如"谨慎"）
    pub name: String,
    /// 特征描述
    pub description: Option<String>,
    /// 强度（1-10）
    pub intensity: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 特征类型（保留 Personality/Behavior/Strength/Weakness/Habit/Fear/Value，去掉 Preference）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TraitType {
    Personality,
    Behavior,
    Value,
    Fear,
    Habit,
    Strength,
    Weakness,
}
