//! Approval - Human Approval Gate
//!
//! 人工审批门：AI 提案 -> 系统检查 -> 用户确认 -> Canon。
//! 特别是在世界观、人物、地点、Volume、Arc、重大剧情阶段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 审批状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// 待审批
    Pending,
    /// 已批准
    Approved,
    /// 已拒绝
    Rejected,
    /// 需要编辑
    NeedsEdit,
    /// 已过期
    Expired,
}

/// 审批目标类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalTargetType {
    /// 世界观
    World,
    /// 实体（人物/地点/势力）
    Entity,
    /// 卷
    Volume,
    /// 弧线
    Arc,
    /// 场景
    Scene,
    /// 故事线
    Storyline,
    /// 事实
    Fact,
    /// 自定义
    Custom(String),
}

/// 审批记录 - 人工审批门
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: Uuid,
    pub project_id: Uuid,
    pub target_type: ApprovalTargetType,
    pub target_id: Uuid,
    /// 提案者（通常是 AI 或用户）
    pub proposed_by: String,
    /// 提案内容（JSON）
    pub proposal_content: serde_json::Value,
    pub status: ApprovalStatus,
    /// 审阅者 ID
    pub reviewer_id: Option<String>,
    /// 审阅评论
    pub reviewer_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}
