//! Quality Score - 质量评分
//!
//! 6维度评分，帮助发现问题而不是直接决定好坏。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 质量评分 - 6维度评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    pub run_id: Option<Uuid>,
    /// 连续性评分 (0-100)
    pub continuity_score: Option<i32>,
    /// 人物一致性评分 (0-100)
    pub character_score: Option<i32>,
    /// 剧情目标评分 (0-100)
    pub plot_score: Option<i32>,
    /// 知识边界评分 (0-100)
    pub knowledge_score: Option<i32>,
    /// 世界规则评分 (0-100)
    pub world_score: Option<i32>,
    /// 语言风格评分 (0-100)
    pub style_score: Option<i32>,
    /// 综合评分 (0-100)
    pub overall_score: Option<i32>,
    /// 发现的问题列表
    pub issues: Vec<QualityIssue>,
    pub created_at: DateTime<Utc>,
}

/// 质量问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub dimension: String,
    pub severity: String,
    pub description: String,
    pub suggestion: Option<String>,
}
