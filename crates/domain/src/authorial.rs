//! AuthorialIntent - 作者层指令
//!
//! 给 AI 的"如何讲故事"的指令，不能改变 World Truth。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 作者意图 - 控制叙事风格和节奏
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthorialIntent {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 关联的 Scene 或 NarrativeNode ID
    pub target_id: Option<Uuid>,
    /// 目标类型（Scene/Volume/Arc）
    pub target_type: Option<String>,
    /// 节奏（快/慢/中等）
    pub pacing: Option<String>,
    /// 情绪基调
    pub emotional_tone: Option<String>,
    /// 重点表现
    pub focus: Option<String>,
    /// 不要做什么
    pub avoid: Option<String>,
    /// 视角（第一人称/第三人称/上帝视角）
    pub perspective: Option<String>,
    /// 叙事距离（亲密/疏离）
    pub narrative_distance: Option<String>,
    /// 其他指令
    pub additional_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
