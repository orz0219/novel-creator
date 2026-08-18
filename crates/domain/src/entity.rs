//! Entity - 统一的世界实体模型
//!
//! 所有世界实体（人物、地点、势力、物品等）统一用 EntityType + Entity 表示。
//! EntityType 定义实体类别，Entity 定义具体实体实例。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::canon::FactCertainty;

/// 实体类型 - 定义世界的分类体系
///
/// EntityType 是可扩展的枚举，用户可以自定义新的实体类别。
/// V1 提供常见的内置类型，但不写死。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityType {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// 该类型特有的字段定义（JSON Schema 格式）
    pub schema: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 内置的实体类型名称常量
impl EntityType {
    pub const CHARACTER: &'static str = "Character";
    pub const LOCATION: &'static str = "Location";
    pub const FACTION: &'static str = "Faction";
    pub const ITEM: &'static str = "Item";
    pub const CREATURE: &'static str = "Creature";
    pub const ORGANIZATION: &'static str = "Organization";
    pub const NATION: &'static str = "Nation";
    pub const CITY: &'static str = "City";
    pub const SECT: &'static str = "Sect";
    pub const RACE: &'static str = "Race";
    pub const DEITY: &'static str = "Deity";
    pub const TECHNOLOGY: &'static str = "Technology";
    pub const CONCEPT: &'static str = "Concept";
}

/// 世界实体 - 所有世界对象的统一表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub project_id: Uuid,
    pub world_id: Uuid,
    pub entity_type_id: Uuid,
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    /// 实体的自由格式属性（JSON），用于存储类型特有的字段
    pub attributes: serde_json::Value,
    /// 乐观锁版本号
    pub version: i32,
    /// 创建者
    pub created_by: String,
    /// 更新者
    pub updated_by: Option<String>,
    /// 来源生成记录 ID
    pub source_generation_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 实体之间的关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub source_entity_id: Uuid,
    pub target_entity_id: Uuid,
    pub relation_type: String,
    pub description: Option<String>,
    /// 关系的属性（如权重、起始时间等）
    pub attributes: serde_json::Value,
    /// 关系生效时间
    pub valid_from: Option<String>,
    /// 关系失效时间（None 表示持续有效）
    pub valid_until: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 世界事实 - 描述世界的客观真理
///
/// 例如："地下遗迹存在"、"天玄大陆有三大帝国"
/// 事实本身不随时间变化，但谁知道这个事实会变化（Knowledge Model）
/// 相关实体通过 fact_entity 关联表管理，不在本结构中。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub content: String,
    pub category: Option<String>,
    /// 事实确定性等级
    pub certainty: FactCertainty,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 事件 - 世界中发生的事情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub event_type: Option<String>,
    /// 事件发生的时间点（可以是相对时间）
    pub timestamp: Option<String>,
    /// 事件发生的具体时间
    pub event_time: Option<String>,
    /// 事件持续时间
    pub duration: Option<String>,
    /// 参与的实体 ID 列表
    pub involved_entity_ids: Vec<Uuid>,
    /// 事件产生的状态变更
    pub state_changes: Vec<StateChange>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 状态变更 - 描述世界状态的变化
///
/// 这是 ProposedChange 的核心结构之一。
/// AI 提出 StateChange，经过 Validator 验证后才提交到世界。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub change_type: StateChangeType,
    pub target_entity_id: Uuid,
    pub state_key: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
}

/// 状态变更的类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateChangeType {
    /// 位置变化
    LocationChange,
    /// 状态标记变化（如 status = WANTED）
    StatusChange,
    /// 属性值变化（如 cultivation = 炼气三层）
    AttributeChange,
    /// 关系变化
    RelationshipChange,
    /// 资源变化
    ResourceChange,
    /// 知识变化（获得/失去信息）
    KnowledgeChange,
    /// 自定义变更
    Custom(String),
}
