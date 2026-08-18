//! Faction 子结构 - 势力的详细信息模型
//!
//! Faction 是 Entity 的一种，通过 FactionProfile 提供结构化的势力信息。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Faction Profile - 势力的详细信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactionProfile {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 组织目标
    pub goals: Option<String>,
    /// 领导人
    pub leader: Option<String>,
    /// 价值观
    pub values: Option<String>,
    /// 资源
    pub resources: Option<String>,
    /// 领地
    pub territory: Option<String>,
    /// 成员
    pub members: Option<String>,
    /// 敌人
    pub enemies: Option<String>,
    /// 盟友
    pub allies: Option<String>,
    /// 内部矛盾
    pub internal_conflicts: Option<String>,
    /// 秘密
    pub secrets: Option<String>,
    /// 行动方式
    pub modus_operandi: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
