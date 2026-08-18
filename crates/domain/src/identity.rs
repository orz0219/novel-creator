//! Entity Identity - 实体别名、身份和评估数据集
//!
//! Alias/Identity: 支持同一个 Entity 有多个名字（别名/称号/历史名称）。
//! Identity Timeline: 身份随时间变化的历史。
//! Evaluation Dataset: 小说生成测试集框架。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Entity Alias - 实体别名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAlias {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 别名类型
    pub alias_type: AliasType,
    /// 别名内容
    pub alias: String,
    /// 生效时间（场景 ID）
    pub valid_from_scene_id: Option<Uuid>,
    /// 失效时间（场景 ID）
    pub valid_until_scene_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// 别名类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AliasType {
    /// 正式名称
    Canonical,
    /// 别名
    Alias,
    /// 称号/头衔
    Title,
    /// 历史名称
    HistoricalName,
}

impl AliasType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AliasType::Canonical => "Canonical",
            AliasType::Alias => "Alias",
            AliasType::Title => "Title",
            AliasType::HistoricalName => "HistoricalName",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Canonical" => AliasType::Canonical,
            "Alias" => AliasType::Alias,
            "Title" => AliasType::Title,
            "HistoricalName" => AliasType::HistoricalName,
            _ => AliasType::Alias,
        }
    }
}

/// Identity Timeline - 身份时间线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityTimeline {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 身份描述（如 "普通散修", "青云宗弟子", "玄天尊者传人"）
    pub identity: String,
    /// 开始场景 ID
    pub start_scene_id: Uuid,
    /// 结束场景 ID（None 表示当前身份）
    pub end_scene_id: Option<Uuid>,
    /// 身份变化原因
    pub change_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// TestCase - 测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 测试用例名称
    pub name: String,
    /// 测试描述
    pub description: String,
    /// 测试类型
    pub test_type: TestType,
    /// 测试前置条件（JSON）
    pub preconditions: serde_json::Value,
    /// 期望结果
    pub expected_result: String,
    /// 测试状态
    pub status: TestStatus,
    pub created_at: DateTime<Utc>,
}

/// 测试类型（6个维度）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestType {
    /// 人物一致性
    CharacterConsistency,
    /// 时间线准确性
    TimelineAccuracy,
    /// 知识边界正确性
    KnowledgeBoundary,
    /// 伏笔保持
    ForeshadowingMaintenance,
    /// 世界规则遵守
    WorldRuleCompliance,
    /// 剧情目标完成
    PlotGoalCompletion,
}

impl TestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestType::CharacterConsistency => "CharacterConsistency",
            TestType::TimelineAccuracy => "TimelineAccuracy",
            TestType::KnowledgeBoundary => "KnowledgeBoundary",
            TestType::ForeshadowingMaintenance => "ForeshadowingMaintenance",
            TestType::WorldRuleCompliance => "WorldRuleCompliance",
            TestType::PlotGoalCompletion => "PlotGoalCompletion",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CharacterConsistency" => TestType::CharacterConsistency,
            "TimelineAccuracy" => TestType::TimelineAccuracy,
            "KnowledgeBoundary" => TestType::KnowledgeBoundary,
            "ForeshadowingMaintenance" => TestType::ForeshadowingMaintenance,
            "WorldRuleCompliance" => TestType::WorldRuleCompliance,
            _ => TestType::PlotGoalCompletion,
        }
    }
}

/// 测试状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestStatus {
    Pending,
    Passed,
    Failed,
    Skipped,
}

impl TestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Pending => "Pending",
            TestStatus::Passed => "Passed",
            TestStatus::Failed => "Failed",
            TestStatus::Skipped => "Skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Passed" => TestStatus::Passed,
            "Failed" => TestStatus::Failed,
            "Skipped" => TestStatus::Skipped,
            _ => TestStatus::Pending,
        }
    }
}

/// TestResult - 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: Uuid,
    pub test_case_id: Uuid,
    /// 测试是否通过
    pub passed: bool,
    /// 实际结果描述
    pub actual_result: String,
    /// 问题描述（如果失败）
    pub issues: Vec<String>,
    /// 测试运行的模型版本
    pub model_version: Option<String>,
    pub created_at: DateTime<Utc>,
}
