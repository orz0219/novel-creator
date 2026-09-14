//! Runtime-facing Repository Ports.
//!
//! These traits are the dependency-inversion boundary between the runtime
//! execution layer and the concrete PostgreSQL storage in the db crate.
//!
//! runtime depends ONLY on these traits (never on db or sqlx). The db crate
//! provides the concrete implementations, injected at the application
//! composition root.

use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::approval::{ApprovalRecord, ApprovalTargetType};
use crate::canon::CanonRule;
use crate::entity::{Entity, Event, Fact, Relation, StateChange};
use crate::generation::{ContextPackage, GenerationTask, Skill};
use crate::knowledge::CharacterKnowledgeItem;
use crate::narrative::NarrativeNode;
use crate::state::{CurrentState, ResourceState};
use crate::storyline::Storyline;
use crate::validation::{
    IssueSeverity, ProposedChange, ProposedChangeStatus, ProposedChangeType,
    ValidationIssueType, ValidationRun,
};
use crate::world::World;

/// Narrative node retrieval (project-scoped).
#[async_trait]
pub trait NarrativePort: Send + Sync {
    async fn get_node_by_id_with_project(
        &self,
        project_id: Uuid,
        node_id: Uuid,
    ) -> Result<Option<NarrativeNode>>;
    async fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>>;
}

/// Entity retrieval (project-scoped).
#[async_trait]
pub trait EntityPort: Send + Sync {
    async fn list_entities_by_ids(&self, project_id: Uuid, ids: &[Uuid]) -> Result<Vec<Entity>>;
    async fn get_entity_by_id_with_project(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Option<Entity>>;
}

/// Current-state (projection) retrieval.
#[async_trait]
pub trait StatePort: Send + Sync {
    async fn list_current_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>>;
    async fn list_current_states_batch(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<CurrentState>>;
}

/// Character-knowledge retrieval.
#[async_trait]
pub trait KnowledgePort: Send + Sync {
    async fn get_character_known_facts(
        &self,
        character_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<CharacterKnowledgeItem>>;
}

/// Relation retrieval (was a raw SQL query inside ContextEngine).
#[async_trait]
pub trait RelationPort: Send + Sync {
    async fn find_relations_by_entities(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<Relation>>;
}

/// Story-event retrieval (was a raw SQL query inside ContextEngine).
#[async_trait]
pub trait EventPort: Send + Sync {
    async fn find_events_by_entities(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<Event>>;
}

/// Canon-rule + world-rule retrieval (was raw SQL inside ContextEngine/Validator).
#[async_trait]
pub trait CanonRulePort: Send + Sync {
    async fn list_canon_rules(&self, project_id: Uuid) -> Result<Vec<CanonRule>>;
    async fn get_main_world_rules_text(&self, project_id: Uuid) -> Result<Option<String>>;
}

/// Context snapshot persistence.
#[async_trait]
pub trait ContextSnapshotPort: Send + Sync {
    async fn save(&self, package: &ContextPackage) -> Result<()>;
}

/// Validation run + issue persistence (used by the Validator).
#[async_trait]
pub trait ValidationPort: Send + Sync {
    async fn create_validation_run(&self, project_id: Uuid, task_id: Uuid)
        -> Result<ValidationRun>;
    async fn update_status(&self, change_id: Uuid, status: ProposedChangeStatus) -> Result<()>;
    async fn create_issue(
        &self,
        validation_run_id: Uuid,
        proposed_change_id: Uuid,
        issue_type: ValidationIssueType,
        severity: IssueSeverity,
        message: &str,
        suggestion: Option<&str>,
    ) -> Result<()>;
    async fn update_validation_run(&self, run: &ValidationRun) -> Result<()>;
}

/// Approval-record creation (used by the Validator for warning-level changes).
#[async_trait]
pub trait ApprovalPort: Send + Sync {
    async fn create(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        proposal_content: serde_json::Value,
    ) -> Result<()>;
}

/// ProposedChange queries (used by the Validator).
#[async_trait]
pub trait ProposedChangeQueryPort: Send + Sync {
    async fn list_approved_changes(
        &self,
        project_id: Uuid,
        task_id: Uuid,
    ) -> Result<Vec<ProposedChange>>;
}

/// Generation（生成任务）仓储端口。
///
/// 与 P1 的 runtime 端口一致：具体 SQL 留在 db 实现，
/// application 的 GenerationService 只依赖此抽象，不直接接触 db / sqlx。
#[async_trait]
pub trait GenerationRepositoryPort: Send + Sync {
    async fn list_tasks(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn get_task(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_task(
        &self,
        project_id: Uuid,
        task_type: &str,
        target_id: Option<Uuid>,
        model: Option<&str>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value>;
    async fn cancel_task(&self, id: Uuid) -> Result<()>;

    /// 取结构化 GenerationTask（供 GenerationExecutor 编排）。
    async fn get_task_struct(&self, id: Uuid) -> Result<Option<GenerationTask>>;
    /// 取 Skill（含 prompt 模板）。
    async fn get_skill_by_id(&self, id: Uuid) -> Result<Option<Skill>>;
    /// 写回任务产出。
    async fn update_task_output(&self, id: Uuid, output: serde_json::Value) -> Result<()>;
    /// 记录一次 GenerationRun（提案 十 / 十一）。context_snapshot_id 关联 ContextSnapshot（提案 十二）。
    ///
    /// `reproducibility` 携带模型 / 温度 / 检索策略 / prompt hash 等可复现元数据（ChatGPT 评审 P1）。
    async fn create_run(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        context_snapshot_id: Option<Uuid>,
        llm_model: &str,
        provider: Option<&str>,
        prompt_sent: &str,
        response_received: &str,
        token_usage: Option<serde_json::Value>,
        latency_ms: Option<i64>,
        reproducibility: crate::generation::ReproducibilityMeta,
    ) -> Result<()>;
}

/// LLM 用量统计（含提示缓存命中信息）。
///
/// 数据来自网关返回的 `usage` 字段。不同网关给的细节不同：
/// 有的给 `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens`，
/// 有的给 `prompt_tokens_details.cached_tokens`，两者都解析为 [`cached_tokens`]。
///
/// [`cached_tokens`]: LlmUsage::cached_tokens
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// 命中提示缓存的 prompt token 数；网关未返回该信息时为 `None`。
    pub cached_tokens: Option<u32>,
}

impl LlmUsage {
    /// 提示缓存命中率（`cached / prompt`）；无缓存信息或 prompt 为 0 时返回 `None`。
    ///
    /// 命中率高说明会话前缀被网关缓存复用（省钱、更快）；
    /// 持续为 0 通常意味着每轮都在重算，或网关侧缓存被驱逐。
    pub fn cache_hit_rate(&self) -> Option<f32> {
        let cached = self.cached_tokens?;
        if self.prompt_tokens == 0 {
            return None;
        }
        Some(cached as f32 / self.prompt_tokens as f32)
    }
}

/// 流式返回的一段内容。
///
/// 之所以不是单纯的 `String`：网关通常把用量统计放在**最后一个 chunk**，
/// 若只用字符串通道就没法把缓存命中信息带给上层。
#[derive(Debug, Clone)]
pub enum LlmStreamChunk {
    /// 正文片段。
    Token(String),
    /// 本次请求的用量统计。
    Usage(LlmUsage),
    /// 流结束原因（如 stop / length / content_filter）。
    ///
    /// `length` 表示输出撞到了 `max_tokens` 上限：此时 JSON 工具调用很可能
    /// 被截断，调用方应给模型/用户明确提示，而不是只报一个 JSON EOF。
    Finish(String),
}

/// LLM 调用端口（提案 十 / 十一）。
///
/// GenerationExecutor 只依赖此抽象，具体实现在 infrastructure 中包裹 LlmClient。
#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn complete(&self, system_prompt: &str, user_prompt: &str, model: &str) -> Result<String>;

    /// 流式补全：逐段产出正文 token，末尾可能跟一条用量统计。
    ///
    /// 默认实现回退到 `complete`，把整段文本作为单个 chunk 产出（不含用量），
    /// 因此未覆盖该方法的端口（如测试用 Mock）无需改动即可使用。
    async fn stream_complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        model: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>> {
        let text = self.complete(system_prompt, user_prompt, model).await?;
        Ok(Box::pin(futures::stream::once(async move {
            Ok(LlmStreamChunk::Token(text))
        })))
    }
}

/// 运行时 AI 配置（模型网关参数）。
///
/// 真源是「设置」页写入的 `app_settings` 中 `aiBaseUrl / aiApiKey / defaultModel`；
/// 环境变量（`OPENCODE_BASE_URL` / `OPENCODE_API_KEY` / `OPENCODE_MODEL`）只在
/// 设置页未配置对应字段时充当默认值。
#[derive(Debug, Clone)]
pub struct AiRuntimeConfig {
    /// OpenAI 兼容网关前缀（不含 `/chat/completions`）。
    pub base_url: String,
    /// 未配置密钥时为 None（请求不带 Authorization 头）。
    pub api_key: Option<String>,
    /// 模型名，透传给网关的 `model` 字段。
    pub model: String,
    /// 单次对话的上下文上限（token），用于聊天页用量预警。
    ///
    /// 注意：网关的 `/models` 不返回上下文长度，因此这是**用户在设置页配置的预算**，
    /// 而非模型硬上限的权威声明；仅用于「快超了」的提前提醒。
    pub context_limit: usize,
    /// 单次请求允许模型输出的最大 token 数。
    ///
    /// 中文长文本 + 工具调用 JSON 很容易撞上限；设置页可调，默认 22000。
    pub max_output_tokens: u32,
}

/// 运行时 AI 配置读取端口。
///
/// 每次 LLM 调用前都会调用 [`AiSettingsPort::load`]，因此设置页改完立即对
/// 对话 / 生成 / 抽取全部生效，无需重启服务。
#[async_trait]
pub trait AiSettingsPort: Send + Sync {
    async fn load(&self) -> Result<AiRuntimeConfig>;
}

/// 引导进度读取端口。
///
/// 真源是 `project.config.current_step`（由 `confirm_step` 工具推进，项目级、多会话共享）。
/// 注意：`agent_sessions.current_step` 是早期遗留字段，值恒为「项目初始化」，
/// **不能**作为判断当前引导阶段的依据——否则 LLM 会一直以为还停在第一步。
#[async_trait]
pub trait GuideProgressPort: Send + Sync {
    /// 读取项目当前引导步骤 key（如 `premise` / `golden_finger`）。
    ///
    /// 项目存在但尚未写入步骤时返回 `Ok(None)`。
    async fn current_step(&self, project_id: Uuid) -> Result<Option<String>>;
}

/// Agent 系统提示词自定义配置（落库实体）。
#[derive(Debug, Clone)]
pub struct AgentPromptConfig {
    pub id: Uuid,
    /// 作用域：'global' 或 'project:<uuid>'
    pub scope: String,
    pub project_id: Option<Uuid>,
    /// 用户可编辑的「人格 / 引导策略」基座文本（工具列表、提问协议等结构性段落会由运行时自动追加）。
    pub system_prompt: String,
    pub updated_at: DateTime<Utc>,
}

/// 提示词持久化端口（Agent 系统提示词的自定义基座，落库用）。
#[async_trait]
pub trait PromptRepositoryPort: Send + Sync {
    /// 读取某作用域下当前生效的提示词配置（不存在返回 None）。
    async fn load(&self, scope: &str) -> Result<Option<AgentPromptConfig>>;
    /// 幂等保存（按 scope upsert）。
    async fn save(&self, config: &AgentPromptConfig) -> Result<()>;
    /// 删除某作用域的自定义覆盖（恢复为内置默认）。
    async fn delete(&self, scope: &str) -> Result<()>;
}

/// 上下文快照仓储端口（提案 十二）。
///
/// GenerationExecutor 在每次执行时保存一份 ContextSnapshot，
/// 并将其 id 关联到 generation_run.context_snapshot_id。
#[async_trait]
pub trait ContextSnapshotRepositoryPort: Send + Sync {
    async fn save(&self, package: &ContextPackage) -> Result<Uuid>;
}

/// Narrative（叙事节点）仓储端口。
///
/// 与 GenerationRepositoryPort 一致：具体 SQL 留在 db 实现，
/// application 的 NarrativeService 只依赖此抽象。
#[async_trait]
pub trait NarrativeRepositoryPort: Send + Sync {
    async fn list_nodes(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn get_node(&self, id: Uuid) -> Result<Option<serde_json::Value>>;

    /// 新建叙事节点（细纲字段齐全：挂线挂阶段、挂实体、元数据、显式序号、初始状态）。
    ///
    /// 校验（父节点归属、故事线归属、实体存在性、阶段名）由实现内部完成，
    /// 缺前置条件直接报错，不做猜测。
    async fn create_node_full(
        &self,
        input: crate::narrative::NewNarrativeNode,
    ) -> Result<serde_json::Value>;

    /// 兼容入口：只带基础字段的新建。
    ///
    /// 内部委托 [`Self::create_node_full`]（追加到同父末尾），
    /// 因此「建节点」只有一份实现，不会出现两条路径各写一套 SQL。
    async fn create_node(
        &self,
        project_id: Uuid,
        node_type: &str,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.create_node_full(crate::narrative::NewNarrativeNode {
            project_id,
            node_type: node_type.to_string(),
            parent_id,
            title: title.to_string(),
            description: description.map(str::to_string),
            content: None,
            attributes,
            sort_order: None,
            status: None,
            storyline_id: None,
            arc_stage: None,
            stage_refs: Vec::new(),
            participant_entity_ids: Vec::new(),
            location_id: None,
            item_ids: Vec::new(),
            estimated_chapters: None,
            estimated_words: None,
            story_time: None,
        })
        .await
    }

    async fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<serde_json::Value>;

    /// 列表 + 过滤 + **真分页**（LIMIT/OFFSET 下推到 SQL）：返回本页与命中总数。
    ///
    /// 过渡期的 `list_nodes` 是全量取回，300 章时「拉全量自己拼树」已经拉不动；
    /// 细纲树的下钻（按父节点 / 类型 / 故事线筛）走这条。
    async fn list_nodes_page(
        &self,
        project_id: Uuid,
        filter: &crate::narrative::NarrativeNodeFilter,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<serde_json::Value>, usize)>;

    async fn delete_node(&self, id: Uuid) -> Result<()>;
}

/// Approval（人工审批闸门）仓储端口。
///
/// 与 GenerationRepositoryPort / NarrativeRepositoryPort 一致：
/// 具体 SQL 留在 db 实现，application 的 ApprovalService 只依赖此抽象。
#[async_trait]
pub trait ApprovalRepositoryPort: Send + Sync {
    async fn create(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        content: serde_json::Value,
    ) -> Result<ApprovalRecord>;
    async fn approve(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>)
        -> Result<()>;
    async fn reject(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>)
        -> Result<()>;
    async fn list_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>>;
}

/// Proposal（提案）仓储端口。
///
/// 提案的创建、状态转换（含 CAS）与读取都集中在 port 实现里，
/// application 的 ProposalService 只负责编排，不直接接触 db / sqlx。
#[async_trait]
pub trait ProposalRepositoryPort: Send + Sync {
    async fn list_proposals(&self, project_id: Uuid) -> Result<Vec<ProposedChange>>;
    async fn get_proposal(&self, id: Uuid) -> Result<Option<ProposedChange>>;
    async fn create_proposal(
        &self,
        project_id: Uuid,
        task_id: Option<Uuid>,
        change_type: ProposedChangeType,
        target_entity_id: Uuid,
        description: &str,
        payload: serde_json::Value,
    ) -> Result<ProposedChange>;
    async fn approve_proposal(&self, id: Uuid) -> Result<ProposedChange>;
    async fn reject_proposal(&self, id: Uuid) -> Result<ProposedChange>;
}

/// Timeline（时间线）仓储端口。
///
/// 排序与冲突检测属于应用层逻辑，保留在 TimelineService；
/// 此端口只负责按项目拉取事件。
#[async_trait]
pub trait TimelineRepositoryPort: Send + Sync {
    async fn list_events_by_project(&self, project_id: Uuid) -> Result<Vec<Event>>;
}

/// Storyline（剧情线）仓储端口。
#[async_trait]
pub trait StorylineRepositoryPort: Send + Sync {
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Storyline>>;
    /// 列出项目全部剧情线（host 层 list_storylines 语义）。
    async fn list_storylines(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    /// 创建剧情线（默认 status = Planned）。
    ///
    /// `parent_id` 可选：传 Some(uuid) 表示这条副线挂到哪条 story line 下。
    /// 主线（importance=Main）必须 parent_id=None。
    /// 创建剧情线。
    ///
    /// `status` 之前是硬编码 `'Planned'`、且没有任何更新入口——于是剧情线状态
    /// 永远停在初始值，调用方能读到却改不了。现在创建即可指定，更新也能改。
    async fn create_storyline(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        status: &str,
        importance: &str,
        tone: &str,
        visibility: &str,
        parent_id: Option<Uuid>,
        // 阶段弧线（与人物 / 势力 / 地点**同构**）：数组对象，
        // 元素含 stage / role / screen_weight / goal / function / entry_trigger / status。
        // `None` 视为空阶段列表。
        arc_stages: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value>;
    /// 按 id 读取单条剧情线。
    ///
    /// `update_storyline` 的 arc_stages 合并需要旧值（先读再合并），
    /// AI 也可以用它「按 id 看一条」而不必 list 全部。
    async fn get_storyline(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    /// 更新剧情线名称/描述/状态/明暗/可见性。
    ///
    /// `None` 表示保持原值（与其它字段一致），因此「只改状态」不会清掉描述。
    /// 修改剧情线。`name` / `description` 都是「不传就不改」（`None` 保持原值）——
    /// 与 `update_*_profile` 一致；原先 name 必填、且 name/description 无条件写入，
    /// 于是「只改描述」也得把名字抄一遍，而「只改名字」会把描述清空。
    async fn update_storyline(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        tone: Option<&str>,
        visibility: Option<&str>,
        // 阶段弧线：整块替换；调用方若要走 merge，自己先读旧值合并后传入
        // （合并语义在工具层，见 `apply_arc_stages`）。
        arc_stages: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value>;
    /// 删除剧情线（按 id）。
    async fn delete_storyline(&self, id: Uuid) -> Result<()>;
    /// 列出某项目所有剧情线关系（parent → child，含 relation_type）
    async fn list_storyline_relations(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<serde_json::Value>>;
    /// 建一条剧情线关系（带类型）。同 (from, to) 已存在时**更新类型**（幂等）。
    ///
    /// 对外用 from / to 的中性说法：驱动 / 依赖 / 交汇 / 对冲这些横向关系里
    /// "parent / child" 的读法并不成立（列名是历史包袱，落库仍映射到那两列）。
    async fn relate_storylines(
        &self,
        project_id: Uuid,
        from_storyline_id: Uuid,
        to_storyline_id: Uuid,
        relation_type: &str,
    ) -> Result<serde_json::Value>;
    /// 删除一条剧情线关系（按 id）。
    async fn unrelate_storylines(&self, id: Uuid) -> Result<()>;
}

/// Foreshadow（伏笔）仓储端口。
///
/// `storyline_id` 是伏笔归属的剧情线：`foreshadowing.storyline_id` 这一列一直存在，
/// 只是此前没被读写——结果是伏笔永远挂不上线，前端也做不出「点开一条暗线看它埋了哪些钩子」。
#[async_trait]
pub trait ForeshadowRepositoryPort: Send + Sync {
    async fn list_foreshadows(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    /// 创建伏笔。
    ///
    /// `status` / `importance` / `hint_level` 都走**枚举校验后的英文值**
    /// （见 `domain::foreshadowing` 的各 `parse`）：此处原先直接写字符串，
    /// 默认值甚至是非法值 `"low"`，模型建出来的伏笔暗示级别是脏数据。
    async fn create_foreshadow(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        status: &str,
        importance: &str,
        hint_level: &str,
        // 等级之外的**备注**（如「前期只透风，不揭示」）：等级是枚举、说明是自由文本，
        // 原先两者挤在 hint_level 一列里，导致前端无法按等级筛选（见迁移 034）。
        hint_note: Option<&str>,
        // 埋点 / 预期回收的时机锚（自由文本：「前期」「第三卷」「第 12 章」）
        introduced_at: Option<&str>,
        expected_reveal_at: Option<&str>,
        // 节点锚：埋点 / 计划回收所在的叙事节点（哪一章埋、打算哪一章收）
        planted_node_id: Option<Uuid>,
        payoff_node_id: Option<Uuid>,
        // 父伏笔：一条大伏笔挂几个小钩子（伏笔树）
        parent_foreshadow_id: Option<Uuid>,
        // 归属的剧情线（可空：伏笔也可以暂时无主）
        storyline_id: Option<Uuid>,
    ) -> Result<serde_json::Value>;
    /// 修改伏笔。
    ///
    /// `storyline_id` 用两层 Option 区分三种意图，避免「改个名字」顺手把归属清掉：
    /// `Some(Some(id))` 改挂到该线；`Some(None)` 解除挂载；`None` 保持原值。
    ///
    /// `name` / `description` 同样是「不传就不改」（`None` 保持原值）：
    /// 只改归属或状态时，不该被迫把名字抄一遍——抄错就是一次误改名。
    /// 修改伏笔。返回**修改后的完整对象**（而不是 `{"updated":true}`）：
    /// 调用方改完就能看到写进去的是什么，不必再 list 一遍全文——本项目的
    /// storyline / 伏笔全文近万字，重复拉取代价很大（AI 上下文也是钱）。
    async fn update_foreshadow(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        hint_note: Option<&str>,
        introduced_at: Option<&str>,
        expected_reveal_at: Option<&str>,
        actual_reveal_at: Option<&str>,
        // 节点锚与伏笔树：`None` = 不改（想清空见工具层的 clear_* 说明）
        planted_node_id: Option<Uuid>,
        payoff_node_id: Option<Uuid>,
        parent_foreshadow_id: Option<Uuid>,
        storyline_id: Option<Option<Uuid>>,
    ) -> Result<serde_json::Value>;
    /// 删除伏笔（按 id）。
    async fn delete_foreshadow(&self, id: Uuid) -> Result<()>;
}

/// World（世界管理）仓储端口。
///
/// 这是 application 中最大的一个端口：WorldService 原本直接持有 PgPool，
/// 并自行实现 set_entity_state / record_event 的事务。P3 把这些 SQL 与事务
/// 下沉到 db 的具体实现，使 application 不再依赖 db / sqlx。
///
/// 注意：set_entity_state / record_event 直接写入 canonical state，属于
/// "系统级" 写入（与 AI 提案经 Proposal → Validate → Commit 的路径不同）。
/// 这里只是把现有实现搬进 port，不改变写入语义；未来如需统一写入边界，
/// 可再让 WorldService 改走 Proposal 路径。
#[async_trait]
pub trait WorldRepositoryPort: Send + Sync {
    async fn create_world(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World>;
    async fn get_world(&self, world_id: Uuid) -> Result<Option<World>>;
    async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>>;
    async fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World>;
    /// 获取项目的主要世界；若不存在则按 project 名称自动创建（host 层 get_world 语义）。
    async fn get_or_create_main_world(&self, project_id: Uuid) -> Result<Option<World>>;
    /// 更新主要世界的基础字段（name / description / world_rules）。
    async fn update_main_world(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        world_rules: Option<&str>,
    ) -> Result<World>;

    async fn create_entity(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Entity>;
    async fn get_entity(&self, project_id: Uuid, entity_id: Uuid) -> Result<Option<Entity>>;
    async fn list_entities(&self, project_id: Uuid) -> Result<Vec<Entity>>;
    async fn list_entities_by_type(
        &self,
        project_id: Uuid,
        entity_type_name: &str,
    ) -> Result<Vec<Entity>>;
    async fn create_relation(
        &self,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Relation>;
    async fn list_relations(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<Relation>>;

    async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact>;
    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Fact>>;

    async fn set_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState>;
    async fn get_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>>;
    async fn list_entity_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>>;

    async fn upsert_resource(
        &self,
        project_id: Uuid,
        location_id: Uuid,
        resource_name: &str,
        quantity: Option<f64>,
        production_rate: Option<f64>,
        controlled_by: Option<Uuid>,
    ) -> Result<ResourceState>;
    async fn list_resources(&self, location_id: Uuid) -> Result<Vec<ResourceState>>;

    async fn record_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        involved_entity_ids: &[Uuid],
        state_changes: Vec<StateChange>,
    ) -> Result<Event>;
}

/// Project（项目）仓储端口。
///
/// 与 P3 端口一致：具体 SQL 留在 db 实现，application 的 ProjectService 只
/// 依赖此抽象。create_project 同时自动创建项目的主要世界（与 host 层语义一致）。
#[async_trait]
pub trait ProjectRepositoryPort: Send + Sync {
    async fn list_projects(&self) -> Result<Vec<serde_json::Value>>;
    async fn get_project(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        language: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn update_project(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        premise: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_project(&self, id: Uuid) -> Result<()>;
}

/// Rule（canon_rule 规则）仓储端口。
#[async_trait]
pub trait RuleRepositoryPort: Send + Sync {
    async fn list_rules(&self, world_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_rule(
        &self,
        world_id: Uuid,
        rule_content: &str,
        rule_level: Option<&str>,
        affected_scope: Option<&str>,
        enforcement: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn get_rule(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn update_rule(
        &self,
        id: Uuid,
        rule_content: Option<&str>,
        rule_level: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_rule(&self, id: Uuid) -> Result<()>;
}

/// History（event / fact / version）仓储端口。
///
/// version 相关为占位实现，仍由 host 层返回 stub，这里只覆盖有真实 SQL 的
/// event / fact 读写。
#[async_trait]
pub trait HistoryRepositoryPort: Send + Sync {
    /// 列出事件。`order_by`：`"recent"`（默认，按创建时间倒序）/ `"era"`（按历史轴锚
    /// `era_order` 升序，未标定的排最后）。中文时间没法比大小，所以排序要显式选。
    async fn list_events(
        &self,
        project_id: Uuid,
        limit: i64,
        order_by: &str,
    ) -> Result<Vec<serde_json::Value>>;
    /// 创建事件。
    ///
    /// 事件天然是结构化的（发生时间 / 地点 / 参与方 / 直接后果 / 揭示时机），此前却只能
    /// 写 name + description，于是「同一地点发生过哪些事」「哪些事件属于暗线」都查不了。
    /// `attributes` 承载 where / participants / consequences / reveal_at 四个键。
    async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        event_time: Option<&str>,
        duration: Option<&str>,
        attributes: &serde_json::Value,
        // 历史轴排序锚（越小越早）。`when` 是给人看的自由文本（「远昔」「缓变」），
        // 中文没法比大小，所以另给一个数值锚；`None` = 未标定。
        era_order: Option<i64>,
        // 事件发生在哪个叙事节点（章 / 场）。`None` = 不挂节点。
        narrative_node_id: Option<Uuid>,
    ) -> Result<serde_json::Value>;
    /// 修改事件（含结构化字段）。`None` 表示保持原值。
    async fn update_event(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        event_type: Option<&str>,
        event_time: Option<&str>,
        duration: Option<&str>,
        attributes: Option<&serde_json::Value>,
        era_order: Option<i64>,
        // 改挂到另一个叙事节点；`None` 且 `clear_narrative_node = false` 时保持原值。
        narrative_node_id: Option<Uuid>,
        clear_narrative_node: bool,
    ) -> Result<serde_json::Value>;
    /// 语义化结束事件（`status='Deleted'`，不物理删除）。
    ///
    /// 与 `entity` / `narrative_node` 同一套语义：创作数据误删无法挽回，
    /// 因此 AI 侧的删除统一是逻辑删除，保留可追溯性。
    async fn delete_event(&self, id: Uuid) -> Result<()>;
    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
    ) -> Result<serde_json::Value>;
    /// 语义化结束事实（`status='Retired'`）。
    ///
    /// `fact` 表本来就有 `status`（默认 `Active`）与 `superseded_by` 两列，
    /// 但没有写入路径——事实一旦建立就永远生效，写错了也撤不回。
    async fn delete_fact(&self, id: Uuid) -> Result<()>;
}

/// Snapshot（novel_state_snapshot）仓储端口。
#[async_trait]
pub trait SnapshotRepositoryPort: Send + Sync {
    async fn list_snapshots(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_snapshot(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        story_time: Option<&str>,
        world_summary: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_snapshot(&self, id: Uuid) -> Result<()>;
    /// 按 id 读取单个快照（含全部列），不存在返回 None。
    async fn find_snapshot(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
}

/// 叙事状态（narrative_state）写入端口 —— 快照恢复等状态回写的唯一通道。
#[async_trait]
pub trait NarrativeStateWritePort: Send + Sync {
    /// 幂等写入：同 (project_id, dimension, key) 已存在则更新，否则插入。
    async fn upsert_state(
        &self,
        project_id: Uuid,
        dimension: crate::narrative::StateDimension,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<()>;
}

/// AI 可追溯查询端口：generation_run / validation_run 的只读视图。
///
/// 返回值是 API 所需的 JSON 形状（id 等已字符串化），供前端追溯页直接渲染：
/// prompt_sent / response_received / token_usage / latency_ms 等 AI 审计字段全量可见。
#[async_trait]
pub trait TraceQueryPort: Send + Sync {
    /// 最近 generation_run 列表（按 created_at 倒序）。
    async fn list_generation_runs(&self, project_id: Uuid, limit: i64) -> Result<Vec<serde_json::Value>>;
    /// 最近 validation_run 列表（按 started_at 倒序），附带每轮的 validation_issue 明细。
    async fn list_validation_runs(&self, project_id: Uuid, limit: i64) -> Result<Vec<serde_json::Value>>;
}

/// Entity（实体 + 关系 + 角色子数据）仓储端口。
///
/// 覆盖 host 层 entity.rs 的全部真实 SQL 查询（实体 CRUD、关系 CRUD，以及
/// 角色档案 / 状态 / 知识 / 关系查询）。返回值已是 API 所需的 JSON 形状，
/// 与 GenerationRepositoryPort / NarrativeRepositoryPort 一致。
#[async_trait]
pub trait EntityRepositoryPort: Send + Sync {
    async fn list_entities(
        &self,
        world_id: Uuid,
        entity_type: Option<&str>,
    ) -> Result<Vec<serde_json::Value>>;
    async fn get_entity(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_entity(
        &self,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn update_entity(
        &self,
        id: Uuid,
        name: Option<&str>,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value>;
    async fn delete_entity(&self, id: Uuid) -> Result<serde_json::Value>;

    async fn list_relations(&self, world_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_relation(
        &self,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_relation(&self, id: Uuid) -> Result<()>;

    async fn get_character_profile(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn get_character_state(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn update_character_profile(&self, id: Uuid, profile: serde_json::Value, actor: &str) -> Result<serde_json::Value>;
    async fn update_character_state(&self, id: Uuid, state: serde_json::Value, actor: &str) -> Result<serde_json::Value>;
    async fn get_location_profile(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn upsert_location_profile(&self, id: Uuid, profile: serde_json::Value, actor: &str) -> Result<serde_json::Value>;
    async fn get_faction_profile(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn upsert_faction_profile(&self, id: Uuid, profile: serde_json::Value, actor: &str) -> Result<serde_json::Value>;
    async fn get_character_knowledge(&self, id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<serde_json::Value>>;
    /// 把「改动前」的档案留档，供版本历史回看。
    ///
    /// `actor` 沿用 MutationSource::as_str()：user / ai / system。
    async fn snapshot_profile(
        &self,
        id: Uuid,
        kind: &str,
        before: &serde_json::Value,
        actor: &str,
    ) -> Result<()>;
}

/// 解析对象 -> project_id 的低层读端口（仅查询，不修改）。
///
/// 用于把 world-scoped / entity-scoped 的调用收敛到 project-scoped 的
/// MutationCommand（提案要求 Command 必须携带 project_id）。
#[async_trait]
pub trait ProjectResolverPort: Send + Sync {
    async fn project_id_for_entity(&self, entity_id: Uuid) -> Result<Option<Uuid>>;
    async fn project_id_for_world(&self, world_id: Uuid) -> Result<Option<Uuid>>;
    async fn project_id_for_relation(&self, relation_id: Uuid) -> Result<Option<Uuid>>;
    async fn project_id_for_narrative_node(&self, node_id: Uuid) -> Result<Option<Uuid>>;
}
