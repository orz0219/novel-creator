//! World Version - 世界（Canon）版本边界（ChatGPT 评审 P2）。
//!
//! 类比 git commit：每次 AI 提案被提交，世界前进一个版本
//! （World v100 → v101）。用于支撑多人编辑 / 多 Agent 协同与回滚，
//! 也是"为什么这次生成和上次不同"的可解释性基础之一。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 版本来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldVersionKind {
    /// 初始基线
    Baseline,
    /// 用户手动编辑
    UserEdit,
    /// AI 提案提交
    AiProposal,
    /// 系统维护（修复 / 迁移等）
    System,
}

impl WorldVersionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WorldVersionKind::Baseline => "baseline",
            WorldVersionKind::UserEdit => "user_edit",
            WorldVersionKind::AiProposal => "ai_proposal",
            WorldVersionKind::System => "system",
        }
    }
}

/// 世界版本 —— Canon 前进的一个不可变检查点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldVersion {
    pub id: Uuid,
    pub world_id: Uuid,
    /// 单调递增版本号
    pub version: i32,
    pub kind: WorldVersionKind,
    /// 触发本次前进的提案 / 命令 id（若有）
    pub trigger_id: Option<Uuid>,
    /// 人类可读的变更摘要
    pub summary: Option<String>,
    /// 父版本 id（形成版本链，支撑回滚 / diff）
    pub parent_version_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl WorldVersion {
    pub fn new(
        world_id: Uuid,
        version: i32,
        kind: WorldVersionKind,
        trigger_id: Option<Uuid>,
        summary: Option<String>,
        parent_version_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            world_id,
            version,
            kind,
            trigger_id,
            summary,
            parent_version_id,
            created_at: Utc::now(),
        }
    }
}
