//! Serialization utilities for DuckDB
//!
//! DuckDB 存储 enum 为 VARCHAR，但 serde_json::to_string 会产生带引号的字符串。
//! 这些函数直接返回不带引号的字符串表示。

use domain::*;

/// NarrativeNodeType -> String
///
/// 规范词表与中文别名定义在 `domain::NarrativeNodeType`，这里只做委托：
/// 存库形态只有一份定义，不会出现「domain 认一种写法、db 认另一种」。
pub fn narrative_node_type_str(nt: &NarrativeNodeType) -> String {
    nt.as_db_str()
}

/// NarrativeNodeStatus -> String（委托 domain，存库形态只有一份定义）
pub fn narrative_node_status_str(ns: &NarrativeNodeStatus) -> String {
    ns.as_db_str()
}

/// String -> NarrativeNodeType（**严格**：不认识的取值直接报错）
///
/// 旧实现把未知取值静默当成 `Scene`，后果是「拼错的类型写进库、读回来变成另一种类型」
/// 而两头都不报错。现在解析失败会带着原始值与合法取值列表一起抛出。
pub fn parse_narrative_node_type(s: &str) -> anyhow::Result<NarrativeNodeType> {
    NarrativeNodeType::parse_strict(s)
}

/// String -> NarrativeNodeStatus（**严格**：未知取值报错，不静默落回 `Draft`）
pub fn parse_narrative_node_status(s: &str) -> anyhow::Result<NarrativeNodeStatus> {
    NarrativeNodeStatus::parse_strict(s)
}

/// TaskStatus -> String
pub fn task_status_str(ts: &TaskStatus) -> String {
    match ts {
        TaskStatus::Pending => "Pending".into(),
        TaskStatus::Running => "Running".into(),
        TaskStatus::Completed => "Completed".into(),
        TaskStatus::Failed => "Failed".into(),
        TaskStatus::Cancelled => "Cancelled".into(),
    }
}

/// String -> TaskStatus
pub fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "Pending" => TaskStatus::Pending,
        "Running" => TaskStatus::Running,
        "Completed" => TaskStatus::Completed,
        "Failed" => TaskStatus::Failed,
        "Cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Pending,
    }
}

/// SkillType -> String
pub fn skill_type_str(st: &SkillType) -> String {
    match st {
        SkillType::WorldPlanner => "WorldPlanner".into(),
        SkillType::VolumePlanner => "VolumePlanner".into(),
        SkillType::ArcPlanner => "ArcPlanner".into(),
        SkillType::ScenePlanner => "ScenePlanner".into(),
        SkillType::LocationDesigner => "LocationDesigner".into(),
        SkillType::CharacterDesigner => "CharacterDesigner".into(),
        SkillType::FactionDesigner => "FactionDesigner".into(),
        SkillType::PlotDesigner => "PlotDesigner".into(),
        SkillType::Writer => "Writer".into(),
        SkillType::Polisher => "Polisher".into(),
        SkillType::Analyzer => "Analyzer".into(),
        SkillType::ContinuityValidator => "ContinuityValidator".into(),
        SkillType::KnowledgeExtractor => "KnowledgeExtractor".into(),
        SkillType::StateChangeExtractor => "StateChangeExtractor".into(),
        SkillType::Custom(s) => format!("Custom:{}", s),
    }
}

/// String -> SkillType
pub fn parse_skill_type(s: &str) -> SkillType {
    if let Some(custom) = s.strip_prefix("Custom:") {
        SkillType::Custom(custom.to_string())
    } else {
        match s {
            "WorldPlanner" => SkillType::WorldPlanner,
            "VolumePlanner" => SkillType::VolumePlanner,
            "ArcPlanner" => SkillType::ArcPlanner,
            "ScenePlanner" => SkillType::ScenePlanner,
            "LocationDesigner" => SkillType::LocationDesigner,
            "CharacterDesigner" => SkillType::CharacterDesigner,
            "FactionDesigner" => SkillType::FactionDesigner,
            "PlotDesigner" => SkillType::PlotDesigner,
            "Writer" => SkillType::Writer,
            "Polisher" => SkillType::Polisher,
            "Analyzer" => SkillType::Analyzer,
            "ContinuityValidator" => SkillType::ContinuityValidator,
            "KnowledgeExtractor" => SkillType::KnowledgeExtractor,
            "StateChangeExtractor" => SkillType::StateChangeExtractor,
            _ => SkillType::Custom(s.to_string()),
        }
    }
}

/// SkillStatus -> String
pub fn skill_status_str(ss: &SkillStatus) -> String {
    match ss {
        SkillStatus::Draft => "Draft".into(),
        SkillStatus::Active => "Active".into(),
        SkillStatus::Deprecated => "Deprecated".into(),
    }
}

/// ProposedChangeType -> String
pub fn proposed_change_type_str(ct: &ProposedChangeType) -> String {
    match ct {
        ProposedChangeType::StateChange => "StateChange".into(),
        ProposedChangeType::EntityCreate => "EntityCreate".into(),
        ProposedChangeType::EntityUpdate => "EntityUpdate".into(),
        ProposedChangeType::EntityDelete => "EntityDelete".into(),
        ProposedChangeType::RelationCreate => "RelationCreate".into(),
        ProposedChangeType::RelationUpdate => "RelationUpdate".into(),
        ProposedChangeType::RelationDelete => "RelationDelete".into(),
        ProposedChangeType::EventCreate => "EventCreate".into(),
        ProposedChangeType::KnowledgeUpdate => "KnowledgeUpdate".into(),
        ProposedChangeType::Custom(s) => format!("Custom:{}", s),
    }
}

/// String -> ProposedChangeType
pub fn parse_proposed_change_type(s: &str) -> ProposedChangeType {
    if let Some(custom) = s.strip_prefix("Custom:") {
        ProposedChangeType::Custom(custom.to_string())
    } else {
        match s {
            "StateChange" => ProposedChangeType::StateChange,
            "EntityCreate" => ProposedChangeType::EntityCreate,
            "EntityUpdate" => ProposedChangeType::EntityUpdate,
            "EntityDelete" => ProposedChangeType::EntityDelete,
            "RelationCreate" => ProposedChangeType::RelationCreate,
            "RelationUpdate" => ProposedChangeType::RelationUpdate,
            "RelationDelete" => ProposedChangeType::RelationDelete,
            "EventCreate" => ProposedChangeType::EventCreate,
            "KnowledgeUpdate" => ProposedChangeType::KnowledgeUpdate,
            _ => ProposedChangeType::Custom(s.to_string()),
        }
    }
}

/// ValidationStatus -> String
pub fn validation_status_str(vs: &ValidationStatus) -> String {
    match vs {
        ValidationStatus::Running => "Running".into(),
        ValidationStatus::Completed => "Completed".into(),
        ValidationStatus::Failed => "Failed".into(),
    }
}

/// ValidationIssueType -> String
pub fn validation_issue_type_str(it: &ValidationIssueType) -> String {
    match it {
        ValidationIssueType::Contradiction => "Contradiction".into(),
        ValidationIssueType::EntityNotFound => "EntityNotFound".into(),
        ValidationIssueType::TypeMismatch => "TypeMismatch".into(),
        ValidationIssueType::RuleViolation => "RuleViolation".into(),
        ValidationIssueType::TimelineConflict => "TimelineConflict".into(),
        ValidationIssueType::KnowledgeInconsistency => "KnowledgeInconsistency".into(),
        ValidationIssueType::Custom(s) => format!("Custom:{}", s),
    }
}

/// IssueSeverity -> String
pub fn issue_severity_str(sev: &IssueSeverity) -> String {
    match sev {
        IssueSeverity::Critical => "Critical".into(),
        IssueSeverity::Warning => "Warning".into(),
        IssueSeverity::Info => "Info".into(),
    }
}

/// KnowledgeSubjectType -> String
pub fn knowledge_subject_type_str(kt: &KnowledgeSubjectType) -> String {
    match kt {
        KnowledgeSubjectType::Author => "Author".into(),
        KnowledgeSubjectType::Character => "Character".into(),
        KnowledgeSubjectType::Reader => "Reader".into(),
        KnowledgeSubjectType::Faction => "Faction".into(),
    }
}

/// String -> KnowledgeSubjectType
pub fn parse_knowledge_subject_type(s: &str) -> KnowledgeSubjectType {
    match s {
        "Author" => KnowledgeSubjectType::Author,
        "Character" => KnowledgeSubjectType::Character,
        "Reader" => KnowledgeSubjectType::Reader,
        "Faction" => KnowledgeSubjectType::Faction,
        _ => KnowledgeSubjectType::Author,
    }
}

/// KnowledgeLevel -> String
pub fn knowledge_level_str(kl: &KnowledgeLevel) -> String {
    match kl {
        KnowledgeLevel::Unknown => "Unknown".into(),
        KnowledgeLevel::Hearsay => "Hearsay".into(),
        KnowledgeLevel::Partial => "Partial".into(),
        KnowledgeLevel::Complete => "Complete".into(),
        KnowledgeLevel::Misunderstood => "Misunderstood".into(),
    }
}

/// String -> KnowledgeLevel
pub fn parse_knowledge_level(s: &str) -> KnowledgeLevel {
    match s {
        "Unknown" => KnowledgeLevel::Unknown,
        "Hearsay" => KnowledgeLevel::Hearsay,
        "Partial" => KnowledgeLevel::Partial,
        "Complete" => KnowledgeLevel::Complete,
        "Misunderstood" => KnowledgeLevel::Misunderstood,
        _ => KnowledgeLevel::Unknown,
    }
}

/// ProjectStatus -> String
pub fn project_status_str(ps: &ProjectStatus) -> String {
    match ps {
        ProjectStatus::Concept => "Concept".into(),
        ProjectStatus::Planning => "Planning".into(),
        ProjectStatus::Writing => "Writing".into(),
        ProjectStatus::Paused => "Paused".into(),
        ProjectStatus::Completed => "Completed".into(),
        ProjectStatus::Archived => "Archived".into(),
    }
}

/// String -> ProjectStatus
pub fn parse_project_status(s: &str) -> ProjectStatus {
    match s {
        "Concept" => ProjectStatus::Concept,
        "Planning" => ProjectStatus::Planning,
        "Writing" => ProjectStatus::Writing,
        "Paused" => ProjectStatus::Paused,
        "Completed" => ProjectStatus::Completed,
        "Archived" => ProjectStatus::Archived,
        _ => ProjectStatus::Concept,
    }
}

/// ProposedChangeStatus -> String
pub fn proposed_change_status_str(ps: &ProposedChangeStatus) -> String {
    match ps {
        ProposedChangeStatus::Draft => "Draft".into(),
        ProposedChangeStatus::Validating => "Validating".into(),
        ProposedChangeStatus::Valid => "Valid".into(),
        ProposedChangeStatus::Approved => "Approved".into(),
        ProposedChangeStatus::PendingApproval => "PendingApproval".into(),
        ProposedChangeStatus::Committed => "Committed".into(),
        ProposedChangeStatus::Applied => "Applied".into(),
        ProposedChangeStatus::Invalid => "Invalid".into(),
        ProposedChangeStatus::Rejected => "Rejected".into(),
        ProposedChangeStatus::Conflicted => "Conflicted".into(),
        ProposedChangeStatus::Expired => "Expired".into(),
        ProposedChangeStatus::Failed => "Failed".into(),
    }
}

/// String -> ProposedChangeStatus
pub fn parse_proposed_change_status(s: &str) -> ProposedChangeStatus {
    match s {
        "Draft" => ProposedChangeStatus::Draft,
        "Validating" => ProposedChangeStatus::Validating,
        "Valid" => ProposedChangeStatus::Valid,
        "Approved" => ProposedChangeStatus::Approved,
        "PendingApproval" => ProposedChangeStatus::PendingApproval,
        "Committed" => ProposedChangeStatus::Committed,
        "Applied" => ProposedChangeStatus::Applied,
        "Invalid" => ProposedChangeStatus::Invalid,
        "Rejected" => ProposedChangeStatus::Rejected,
        "Conflicted" => ProposedChangeStatus::Conflicted,
        "Expired" => ProposedChangeStatus::Expired,
        "Failed" => ProposedChangeStatus::Failed,
        _ => ProposedChangeStatus::Draft,
    }
}

/// String -> SkillStatus
pub fn parse_skill_status(s: &str) -> SkillStatus {
    match s {
        "Draft" => SkillStatus::Draft,
        "Active" => SkillStatus::Active,
        "Deprecated" => SkillStatus::Deprecated,
        _ => SkillStatus::Draft,
    }
}

/// String -> ValidationIssueType
pub fn parse_validation_issue_type(s: &str) -> ValidationIssueType {
    if let Some(custom) = s.strip_prefix("Custom:") {
        ValidationIssueType::Custom(custom.to_string())
    } else {
        match s {
            "Contradiction" => ValidationIssueType::Contradiction,
            "EntityNotFound" => ValidationIssueType::EntityNotFound,
            "TypeMismatch" => ValidationIssueType::TypeMismatch,
            "RuleViolation" => ValidationIssueType::RuleViolation,
            "TimelineConflict" => ValidationIssueType::TimelineConflict,
            "KnowledgeInconsistency" => ValidationIssueType::KnowledgeInconsistency,
            _ => ValidationIssueType::Custom(s.to_string()),
        }
    }
}

/// String -> IssueSeverity
pub fn parse_issue_severity(s: &str) -> IssueSeverity {
    match s {
        "Critical" => IssueSeverity::Critical,
        "Warning" => IssueSeverity::Warning,
        "Info" => IssueSeverity::Info,
        _ => IssueSeverity::Warning,
    }
}
