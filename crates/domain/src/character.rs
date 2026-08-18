//! Character 子结构 - 人物的详细信息模型
//!
//! Character 是 Entity 的一种，但复杂信息不应全部放进 Entity。
//! 通过 CharacterProfile/CharacterState/CharacterGoal/CharacterTrait/CharacterArc 子结构拆分。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Character Profile - 人物的基础信息（稳定不变）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterProfile {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 姓名
    pub real_name: Option<String>,
    /// 昵称/别名
    pub nickname: Option<String>,
    /// 年龄
    pub age: Option<String>,
    /// 性别
    pub gender: Option<String>,
    /// 身份（如"边境散修"、"王家家主"）
    pub identity: Option<String>,
    /// 外貌描述
    pub appearance: Option<String>,
    /// 背景故事
    pub background: Option<String>,
    /// 社会身份
    pub social_status: Option<String>,
    /// 核心性格
    pub core_personality: Option<String>,
    /// 价值观
    pub values: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character State - 人物的当前状态（频繁变化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterState {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 当前位置
    pub location: Option<String>,
    /// 健康状态
    pub health: Option<String>,
    /// 修炼等级
    pub cultivation: Option<String>,
    /// 金钱/资源
    pub money: Option<String>,
    /// 是否被通缉
    pub wanted: bool,
    /// 自由扩展状态
    pub extra: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character Goal - 人物的目标（多层级）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGoal {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 长期目标
    pub long_term: Option<String>,
    /// 当前目标
    pub current: Option<String>,
    /// 当前场景目标
    pub immediate: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character Trait - 人物的特征（分层结构）
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
    /// 父特征 ID（支持层级嵌套）
    pub parent_trait_id: Option<Uuid>,
    /// 强度（1-10）
    pub intensity: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 特征类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TraitType {
    Personality,
    Behavior,
    Value,
    Fear,
    Preference,
    Habit,
    Strength,
    Weakness,
}
