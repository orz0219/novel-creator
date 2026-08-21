//! Narrative - 叙事结构模型
//!
//! 统一的 NarrativeNode 树形结构，支持 Volume -> Arc -> Sequence -> Chapter -> Scene -> Beat 层级。
//! 使用单一表 + node_type 枚举实现，便于未来扩展新的节点类型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 叙事节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NarrativeNodeType {
    /// 卷
    Volume,
    /// 故事弧线
    Arc,
    /// 序列
    Sequence,
    /// 章节
    Chapter,
    /// 场景
    Scene,
    /// 节拍（场景内的最小生成单位）
    Beat,
    /// 故事线（可选，用于并行叙事）
    Storyline,
    /// 子弧线（可选）
    SubArc,
    /// 特殊节点（可选）
    Special,
}

/// 叙事节点 - 统一的树形结构
///
/// 所有叙事层级（卷/弧/序列/章/场景/节拍）都用同一个结构表示。
/// 通过 node_type 区分层级，通过 parent_id 建立树形关系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeNode {
    pub id: Uuid,
    pub project_id: Uuid,
    pub world_id: Uuid,
    pub node_type: NarrativeNodeType,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    /// 场景/章节正文（编辑器草稿内容）。Volume/Arc 等也可承载概述。
    pub content: Option<String>,
    /// 节点特有的属性（JSON），不同类型有不同结构
    pub attributes: serde_json::Value,
    pub sort_order: i32,
    pub status: NarrativeNodeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 叙事节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeNodeStatus {
    /// 草稿
    Draft,
    /// 已规划
    Planned,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 已归档
    Archived,
}

/// 卷的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct VolumeAttributes {
    pub mission: Option<String>,
    pub theme: Option<String>,
    pub conflict: Option<String>,
    pub goal: Option<String>,
    pub start_state: Option<String>,
    pub end_state: Option<String>,
    pub important_character_ids: Vec<Uuid>,
    pub important_location_ids: Vec<Uuid>,
    pub major_events: Vec<String>,
    pub secrets: Vec<String>,
    pub foreshadowing: Vec<String>,
    pub resolution: Option<String>,
    /// Story Contract ID for this volume
    pub story_contract_id: Option<Uuid>,
}

/// 弧线的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ArcAttributes {
    /// 弧线目标
    pub goal: Option<String>,
    /// 核心冲突
    pub conflict: Option<String>,
    /// 参与者
    pub participants: Vec<String>,
    /// 开始条件
    pub start_condition: Option<String>,
    /// 结束条件
    pub end_condition: Option<String>,
    /// 关键事件
    pub key_events: Vec<String>,
    /// 转折点
    pub twists: Vec<String>,
    /// Story Contract ID for this arc
    pub story_contract_id: Option<Uuid>,
}

/// 场景的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SceneAttributes {
    pub objective: Option<String>,
    pub conflict: Option<String>,
    pub pov_character_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub time: Option<String>,
    /// 情绪目标（这一幕要传达什么情绪）
    pub emotional_goal: Option<String>,
    /// 信息目标（这一幕要让读者/角色知道什么）
    pub information_goal: Option<String>,
    pub required_events: Vec<String>,
    /// 禁止发生的事件
    pub forbidden_events: Vec<String>,
    /// 预期的世界状态变化
    pub expected_changes: Vec<String>,
    pub required_facts: Vec<String>,
    pub characters_present: Vec<Uuid>,
}

/// 节拍的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BeatAttributes {
    pub action: String,
    pub emotion: Option<String>,
    pub dialogue_needed: bool,
    pub word_count_target: Option<i32>,
}

/// 场景 - 叙事节点的具体化视图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: Uuid,
    pub narrative_node_id: Uuid,
    pub objective: Option<String>,
    pub conflict: Option<String>,
    pub pov_character_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub time: Option<String>,
    pub scene_start_time: Option<String>,
    pub scene_end_time: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 场景中涉及的实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: Uuid,
    pub scene_id: Uuid,
    pub entity_id: Uuid,
    pub role: Option<String>,
    pub notes: Option<String>,
}

/// 场景需求 - 该场景必须满足的条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneRequirement {
    pub id: Uuid,
    pub scene_id: Uuid,
    pub requirement_type: String,
    pub content: String,
    pub priority: RequirementPriority,
}

/// 需求优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequirementPriority {
    Must,
    Should,
    Could,
}

/// 角色弧线 - 追踪角色在叙事中的成长/变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterArc {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    pub volume_id: Option<Uuid>,
    pub arc_type: String,
    pub start_state: Option<String>,
    pub mid_state: Option<String>,
    pub end_state: Option<String>,
    pub key_moments: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// NarrativeState - 叙事状态（已从人物模块移出，归叙事引擎）
///
/// 区分 World State（世界发生了什么）和 Narrative State（叙事已揭示什么）。
/// 例如：World State = 王家已灭亡，Narrative State = 读者不知道王家已灭亡。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeState {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 状态维度（World/Narrative/Character/Reader）
    pub state_dimension: StateDimension,
    /// 状态键
    pub state_key: String,
    /// 状态值
    pub state_value: Value,
    /// 关联的场景 ID
    pub scene_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 状态维度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateDimension {
    /// 世界状态（客观事实）
    World,
    /// 叙事状态（已揭示给读者的）
    Narrative,
    /// 角色状态（角色当前状态）
    Character,
    /// 读者状态（读者当前知道的）
    Reader,
}

impl StateDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            StateDimension::World => "World",
            StateDimension::Narrative => "Narrative",
            StateDimension::Character => "Character",
            StateDimension::Reader => "Reader",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "World" => StateDimension::World,
            "Narrative" => StateDimension::Narrative,
            "Character" => StateDimension::Character,
            "Reader" => StateDimension::Reader,
            _ => StateDimension::World,
        }
    }
}
