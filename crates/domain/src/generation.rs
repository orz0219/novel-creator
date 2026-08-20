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
    pub skill_id: Option<Uuid>,
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
    /// 可复现性元数据（见 [`ReproducibilityMeta`]）
    pub reproducibility: ReproducibilityMeta,
    pub created_at: DateTime<Utc>,
}

/// 上下文层
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLayer {
    pub content: String,
    pub token_estimate: i32,
    pub included: bool,
}

/// 检索到的文档引用（用于精确复现）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrievedDocRef {
    pub id: Uuid,
    /// 文档内容 hash（如 sha256 hex），用于校验"这次生成看到的就是这份"
    pub hash: String,
}

/// 可复现性元数据 —— Context Snapshot / GenerationRun 可复现性的核心（ChatGPT 评审 P1）。
///
/// 记录"这次生成上下文是怎么构成的"，使得同一结果可被审计 / 重放 / 解释
/// "为什么昨天生成 A、今天生成 B"。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReproducibilityMeta {
    /// 快照时的世界（Canon）版本
    pub world_version: Option<i32>,
    /// 上下文策略版本
    pub context_policy_version: Option<i32>,
    /// 使用的 LLM 模型
    pub model: Option<String>,
    /// 采样温度
    pub temperature: Option<f64>,
    /// 检索策略（如 "hybrid" / "semantic" / "structured"）
    pub retrieval_strategy: Option<String>,
    /// 实际检索到的文档（id + 内容 hash）
    pub retrieved_documents: Vec<RetrievedDocRef>,
    /// 最终拼装 prompt 的 hash
    pub prompt_hash: Option<String>,
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
    // ===== 可复现性字段（ChatGPT 评审 P1：AI Trace 必须可复现）=====
    /// 快照时的世界版本
    pub world_version: Option<i32>,
    /// 采样温度
    pub temperature: Option<f64>,
    /// 检索策略
    pub retrieval_strategy: Option<String>,
    /// 最终 prompt 的 hash
    pub prompt_hash: Option<String>,
    /// 检索到的文档引用（id + hash）
    pub retrieved_documents: Vec<RetrievedDocRef>,
    pub created_at: DateTime<Utc>,
}
