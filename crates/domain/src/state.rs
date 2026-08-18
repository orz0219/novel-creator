//! State - 世界状态管理
//!
//! World State 描述"世界现在发生了什么"，与 World Model（世界有什么）分离。
//! 通过 StateChange 机制，AI 只能提出状态变更建议，不能直接修改世界。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 世界状态的当前快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub entity_id: Uuid,
    pub state_key: String,
    pub state_value: serde_json::Value,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 状态变更记录 - 已提交的变更历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeRecord {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_id: Option<Uuid>,
    /// 变更类型定义在 entity 模块中
    pub change_type: String,
    pub target_entity_id: Uuid,
    pub state_key: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub committed_at: DateTime<Utc>,
    pub committed_by: Option<String>,
}

/// 资源状态 - 用于追踪地点/势力的资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub location_id: Uuid,
    pub resource_name: String,
    pub quantity: Option<f64>,
    pub production_rate: Option<f64>,
    pub controlled_by_entity_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
