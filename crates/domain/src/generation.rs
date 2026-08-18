//! Generation - 生成运行时模型
//!
//! 包含 Skill 定义、生成任务、Context Package 组装等。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::skill::{SkillType, SkillStatus};

/// Skill - 可版本化的生成技能
///
/// 每个 Skill 是一个独立的生成能力，如 location_designer、character_designer、writer 等。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub skill_type: SkillType,
    pub version: i32,
    /// Skill 的 Prompt 模板
    pub prompt_template: String,
    /// Skill 的输入 schema（JSON Schema）
    pub input_schema: Option<serde_json::Value>,
    /// Skill 的输出 schema（JSON Schema）
    pub output_schema: Option<serde_json::Value>,
    /// 默认参数
    pub default_params: serde_json::Value,
    pub status: SkillStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}





/// 生成任务 - 一次 LLM 调用的完整描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTask {
    pub id: Uuid,
    pub project_id: Uuid,
    pub skill_id: Uuid,
    pub scene_id: Option<Uuid>,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: TaskStatus,
    pub token_usage: Option<TokenUsage>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 生成任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

/// 上下文包 - Context Engine 组装的最小充分上下文
///
/// 按 L0~L6 分层，根据 Token Budget 动态选择。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub id: Uuid,
    pub project_id: Uuid,
    pub scene_id: Uuid,
    /// Token 预算
    pub token_budget: i32,
    /// L0: 绝对必须知道
    pub l0_essential: ContextLayer,
    /// L1: 当前场景相关
    pub l1_scene_relevant: ContextLayer,
    /// L2: 近期历史
    pub l2_recent_history: ContextLayer,
    /// L3: 当前剧情上下文
    pub l3_narrative_context: ContextLayer,
    /// L4: 角色知识
    pub l4_character_knowledge: ContextLayer,
    /// L5: 世界背景
    pub l5_world_background: ContextLayer,
    /// L6: 可选补充
    pub l6_optional_supplement: ContextLayer,
    /// 实际使用的总 token 数
    pub actual_tokens: i32,
    pub created_at: DateTime<Utc>,
}

/// 上下文层
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLayer {
    pub content: String,
    pub token_estimate: i32,
    pub included: bool,
}

/// 生成运行记录 - 完整的生成历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub context_snapshot_id: Option<Uuid>,
    pub llm_model: String,
    /// LLM 提供商（如 "openai", "deepseek", "local"）
    pub provider: Option<String>,
    pub prompt_sent: String,
    pub response_received: String,
    pub token_usage: Option<TokenUsage>,
    pub latency_ms: Option<i64>,
    /// Skill 版本
    pub skill_version: Option<i32>,
    /// Prompt 版本
    pub prompt_version: Option<i32>,
    /// Schema 版本
    pub schema_version: Option<i32>,
    /// Context Policy 版本
    pub context_policy_version: Option<i32>,
    /// 重试次数
    pub retry_count: i32,
    /// 最大重试次数
    pub max_retries: i32,
    /// 错误信息
    pub error: Option<String>,
    /// 输出 Artifact ID
    pub output_artifact_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
