//! Narrative Contract - 场景契约
//!
//! 每个 Scene 都有一个契约，定义：
//! - 必须发生什么
//! - 禁止发生什么
//! - 读者/主角会学到什么
//! - 世界会发生什么变化

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 场景契约 - 定义场景必须满足的条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContract {
    pub id: Uuid,
    pub scene_id: Uuid,
    /// 必须发生的事件
    pub required_events: Vec<String>,
    /// 禁止发生的事件
    pub forbidden_events: Vec<String>,
    /// 必须出现的角色
    pub required_characters: Vec<Uuid>,
    /// 必须提及的事实
    pub required_facts: Vec<String>,
    /// 读者会学到什么
    pub reader_learns: Vec<String>,
    /// 主角会学到什么
    pub protagonist_learns: Vec<String>,
    /// 世界会发生什么变化
    pub world_changes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
