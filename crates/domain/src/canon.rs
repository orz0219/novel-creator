//! Canon Constitution - 世界规则宪法
//!
//! 四级规则系统，Validator 据此判断 AI Proposal 的合法性：
//! - RULE-0: 绝对规则（AI 永远不能违反）
//! - RULE-1: 世界规则（违反需 Reject 或 Require Approval）
//! - RULE-2: 剧情既定事实（违反需 Require Approval）
//! - RULE-3: 可变设定（允许 AI 提出修改）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 规则级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleLevel {
    /// 绝对规则：死者不能复活、时间不能倒流
    Rule0,
    /// 世界规则：修炼体系、魔法体系、经济体系
    Rule1,
    /// 剧情既定事实：王家已灭亡、林凡已加入青云宗
    Rule2,
    /// 可变设定：路人名字、酒馆位置
    Rule3,
}

impl RuleLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleLevel::Rule0 => "RULE-0",
            RuleLevel::Rule1 => "RULE-1",
            RuleLevel::Rule2 => "RULE-2",
            RuleLevel::Rule3 => "RULE-3",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "RULE-0" => RuleLevel::Rule0,
            "RULE-1" => RuleLevel::Rule1,
            "RULE-2" => RuleLevel::Rule2,
            "RULE-3" => RuleLevel::Rule3,
            _ => RuleLevel::Rule3,
        }
    }
}

/// 规则执行动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnforcementAction {
    /// 直接拒绝
    Reject,
    /// 需要人工审批
    RequireApproval,
    /// 允许
    Allow,
}

impl EnforcementAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnforcementAction::Reject => "Reject",
            EnforcementAction::RequireApproval => "RequireApproval",
            EnforcementAction::Allow => "Allow",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Reject" => EnforcementAction::Reject,
            "RequireApproval" => EnforcementAction::RequireApproval,
            "Allow" => EnforcementAction::Allow,
            _ => EnforcementAction::Allow,
        }
    }
}

/// Canon Rule - 世界规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonRule {
    pub id: Uuid,
    pub project_id: Uuid,
    pub world_id: Uuid,
    pub rule_level: RuleLevel,
    /// 规则内容描述
    pub rule_content: String,
    /// 规则影响范围（如 "cultivation_system", "economy", "politics"）
    pub affected_scope: String,
    /// 违反时的执行动作
    pub enforcement: EnforcementAction,
    /// 规则的具体约束条件（JSON，用于 Validator 自动检查）
    pub constraints: serde_json::Value,
    /// 规则来源（如 "author_defined", "world_setting"）
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// FactCertainty - 事实确定性等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FactCertainty {
    /// 确定事实（世界客观真理）
    Canon,
    /// 可能的事实（有证据支持）
    Probable,
    /// 传闻（未经证实）
    Rumor,
    /// 角色信念（角色认为是真的）
    Belief,
    /// 猜测（无证据）
    Speculation,
    /// 错误认知（角色认为是真的，但世界不是）
    FalseBelief,
    /// 未知
    Unknown,
}

impl FactCertainty {
    pub fn as_str(&self) -> &'static str {
        match self {
            FactCertainty::Canon => "CANON",
            FactCertainty::Probable => "PROBABLE",
            FactCertainty::Rumor => "RUMOR",
            FactCertainty::Belief => "BELIEF",
            FactCertainty::Speculation => "SPECULATION",
            FactCertainty::FalseBelief => "FALSE_BELIEF",
            FactCertainty::Unknown => "UNKNOWN",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CANON" => FactCertainty::Canon,
            "PROBABLE" => FactCertainty::Probable,
            "RUMOR" => FactCertainty::Rumor,
            "BELIEF" => FactCertainty::Belief,
            "SPECULATION" => FactCertainty::Speculation,
            "FALSE_BELIEF" => FactCertainty::FalseBelief,
            _ => FactCertainty::Unknown,
        }
    }
}

/// SourceType - 事实来源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceType {
    /// 世界规则
    CanonRule,
    /// 世界状态
    WorldState,
    /// 作者确认数据
    AuthorConfirmed,
    /// 结构化设计
    StructuredDesign,
    /// 之前的草稿
    PreviousDraft,
    /// AI 生成文本
    AIGenerated,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::CanonRule => "CanonRule",
            SourceType::WorldState => "WorldState",
            SourceType::AuthorConfirmed => "AuthorConfirmed",
            SourceType::StructuredDesign => "StructuredDesign",
            SourceType::PreviousDraft => "PreviousDraft",
            SourceType::AIGenerated => "AIGenerated",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CanonRule" => SourceType::CanonRule,
            "WorldState" => SourceType::WorldState,
            "AuthorConfirmed" => SourceType::AuthorConfirmed,
            "StructuredDesign" => SourceType::StructuredDesign,
            "PreviousDraft" => SourceType::PreviousDraft,
            _ => SourceType::AIGenerated,
        }
    }
}
