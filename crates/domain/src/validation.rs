//! Validation - 变更验证模型
//!
//! 核心原则：AI 只能提出世界状态变更（ProposedChange），
//! 所有变更必须经过 Validator 验证并事务化提交。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 拟议变更 - AI 提出的世界状态变更建议
///
/// 这是系统安全性的核心：AI 不能直接修改世界，
/// 只能提出 ProposedChange，经过 Validator 验证后才提交。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub change_type: ProposedChangeType,
    pub target_entity_id: Uuid,
    pub description: String,
    /// 变更的具体内容（JSON）
    pub payload: serde_json::Value,
    pub status: ProposedChangeStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// 拟议变更类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposedChangeType {
    /// 状态变化
    StateChange,
    /// 新增实体
    EntityCreate,
    /// 修改实体
    EntityUpdate,
    /// 删除实体
    EntityDelete,
    /// 新增关系
    RelationCreate,
    /// 修改关系
    RelationUpdate,
    /// 删除关系
    RelationDelete,
    /// 新增事件
    EventCreate,
    /// 新增知识
    KnowledgeUpdate,
    /// 自定义
    Custom(String),
}

/// 拟议变更状态 - 完整状态机
///
/// 状态转换:
/// DRAFT -> VALIDATING -> VALID -> APPROVED -> COMMITTED
/// DRAFT -> VALIDATING -> VALID -> PENDING_APPROVAL -> APPROVED -> COMMITTED
/// 异常: INVALID, REJECTED, CONFLICTED, EXPIRED
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposedChangeStatus {
    /// 草稿状态
    Draft,
    /// 验证中
    Validating,
    /// 验证通过
    Valid,
    /// 已批准
    Approved,
    /// 等待人工审批（Canon RULE-1/RULE-2 冲突）
    PendingApproval,
    /// 已提交
    Committed,
    /// 已应用
    Applied,
    /// 验证失败
    Invalid,
    /// 被拒绝
    Rejected,
    /// 版本冲突
    Conflicted,
    /// 已过期
    Expired,
    /// 提交失败
    Failed,
}

impl ProposedChangeStatus {
    /// 检查状态转换是否合法
    ///
    /// 完整状态机:
    /// Draft -> Validating -> Valid -> Approved -> Committed -> Applied
    /// Draft -> Validating -> Valid -> PendingApproval -> Approved -> Committed -> Applied
    /// 异常: Invalid, Rejected, Conflicted, Expired, Failed
    pub fn can_transition_to(&self, new_status: &ProposedChangeStatus) -> bool {
        matches!(
            (self, new_status),
            // 正常流程
            (ProposedChangeStatus::Draft, ProposedChangeStatus::Validating)
                | (ProposedChangeStatus::Validating, ProposedChangeStatus::Valid)
                | (ProposedChangeStatus::Validating, ProposedChangeStatus::Invalid)
                | (ProposedChangeStatus::Valid, ProposedChangeStatus::Approved)
                | (ProposedChangeStatus::Valid, ProposedChangeStatus::PendingApproval)
                | (ProposedChangeStatus::Valid, ProposedChangeStatus::Rejected)
                | (ProposedChangeStatus::PendingApproval, ProposedChangeStatus::Approved)
                | (ProposedChangeStatus::PendingApproval, ProposedChangeStatus::Rejected)
                | (ProposedChangeStatus::Approved, ProposedChangeStatus::Committed)
                | (ProposedChangeStatus::Approved, ProposedChangeStatus::Rejected)
                | (ProposedChangeStatus::Committed, ProposedChangeStatus::Applied)
                | (ProposedChangeStatus::Committed, ProposedChangeStatus::Failed)
                // Conflicted 可从多个状态进入
                | (ProposedChangeStatus::Validating, ProposedChangeStatus::Conflicted)
                | (ProposedChangeStatus::Valid, ProposedChangeStatus::Conflicted)
                | (ProposedChangeStatus::Approved, ProposedChangeStatus::Conflicted)
                | (ProposedChangeStatus::PendingApproval, ProposedChangeStatus::Conflicted)
                // Expired 可从 PendingApproval 进入
                | (ProposedChangeStatus::PendingApproval, ProposedChangeStatus::Expired)
        )
    }

    /// 获取状态描述
    pub fn description(&self) -> &str {
        match self {
            ProposedChangeStatus::Draft => "Draft",
            ProposedChangeStatus::Validating => "Validating",
            ProposedChangeStatus::Valid => "Valid",
            ProposedChangeStatus::Approved => "Approved",
            ProposedChangeStatus::PendingApproval => "PendingApproval",
            ProposedChangeStatus::Committed => "Committed",
            ProposedChangeStatus::Applied => "Applied",
            ProposedChangeStatus::Invalid => "Invalid",
            ProposedChangeStatus::Rejected => "Rejected",
            ProposedChangeStatus::Conflicted => "Conflicted",
            ProposedChangeStatus::Expired => "Expired",
            ProposedChangeStatus::Failed => "Failed",
        }
    }
}

/// 验证运行 - 对一批 ProposedChange 的验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub changes_validated: i32,
    pub changes_approved: i32,
    pub changes_rejected: i32,
    pub status: ValidationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 验证状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationStatus {
    Running,
    Completed,
    Failed,
}

/// 验证问题 - 验证过程中发现的问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub id: Uuid,
    pub validation_run_id: Uuid,
    pub proposed_change_id: Uuid,
    pub issue_type: ValidationIssueType,
    pub severity: IssueSeverity,
    pub message: String,
    pub suggestion: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 验证问题类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationIssueType {
    /// 与现有状态矛盾
    Contradiction,
    /// 实体不存在
    EntityNotFound,
    /// 类型不匹配
    TypeMismatch,
    /// 违反世界规则
    RuleViolation,
    /// 时间线矛盾
    TimelineConflict,
    /// 知识一致性问题
    KnowledgeInconsistency,
    /// 自定义
    Custom(String),
}

/// 问题严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    /// 阻断性问题，必须修复
    Critical,
    /// 警告，建议修复
    Warning,
    /// 信息性提示
    Info,
}
