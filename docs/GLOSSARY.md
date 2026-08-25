# 领域术语表（Glossary）

> 本文档整理 Novel Creator 中与源码强绑定的专有名词，供阅读 `docs/modules/` 与 `docs/ARCHITECTURE_OVERVIEW.md` 时对照。每条给出中文释义、出处（文件:行号）与备注。**以源码为准**。

## A–C

- **AgentSession（引导会话）**：一次与用户的引导式对话。绑定到项目（`project_id NOT NULL`，`021_bind_session_to_project.sql`）。字段 `id/project_id/title/messages/current_step/created_at/updated_at`（`crates/domain/src/agent_store.rs:24-34`）。
- **AgentRuntime（Agent 运行时）**：引导层编排核心，持有 `llm/tools/sessions/memory/prompt_store` 与 `model`，提供 `chat_stream` 等（`crates/agent/src/runtime.rs:64-75`）。
- **AgentStreamEvent（流式事件）**：`Token`/`Question`/`Tool`/`Done`/`Error` 五变体（`crates/agent/src/runtime.rs:30-50`），驱动前端 SSE 渲染。
- **AgentTool / ToolRegistry（工具与注册表）**：`AgentTool` trait（`crates/agent/src/tool.rs:21-30`）+ `ToolRegistry`（RwLock HashMap，`tool.rs:33-35`）。内置 `EchoTool`/`AskQuestionTool`。
- **ApprovalRecord（审批记录）**：提案审批结果；表 `approval_record`（`001_canonical_schema.sql:889-902`）。注意前端 `reviewer`/`review_notes` 与后端 `reviewer_id`/`reviewer_comment` 错位（见 `docs/modules/types.md` §4.8）。
- **Canon / Canonical State（规范状态 / 唯一真源）**：确定性世界状态（world/entity/event/fact/narrative_node 等表），是系统唯一权威。AI 不直接写，须经写入口（`docs/ARCHITECTURE_OVERVIEW.md` §3）。
- **CanonRule（世界规则）**：约束世界一致性的规则，分 `rule_level`/`enforcement`（`domain/src/canon.rs:79-96`；`001:1063-1075`）。前端 `rules.ts` 缺 `constraints`/`source`。
- **ChatMessage（对话消息）**：`role`/`content`/`created_at`（`crates/domain/src/agent_store.rs:15-20`）。`role` 为自由 `String`，注释写 `user/assistant/system`，运行时实际写 `user/assistant/tool`（前后端契约未对齐，见 `docs/modules/agent.md` §5.3/§8.3）。
- **CharacterMind（角色心智）**：角色知识/信念/目标等心理模型，定义在 `domain/src/character_mind.rs`。**注意**：`ai/src/character_mind.rs` 为未编译死代码（`ai/src/lib.rs:22-28` 仅 `pub use domain::*`）。
- **CharacterProfile / CharacterState（角色档案 / 状态）**：R2 重构后 Profile 含 `name/aliases/age: AgeRange/social_position/...`（`domain/src/character.rs:278-328`），State 含 `physical_state/mental_state/resource_state/social_state/flags`（`001` 经 `017` 演进）。前端 `types/character.ts` 仍用旧字段，严重错位。
- **ContextEngine（上下文引擎）**：编排 7 步检索 → Ranking → Budget → 组装 `ContextPackage`（`crates/runtime/src/execution/context_engine.rs`）。
- **ContextLayer / ContextPackage（上下文层 / 包）**：动态上下文的分层聚合（L0–L6），schema 定义于 `domain::generation`（generation.rs:76-109）；runtime 的 `budget::allocate` 填充（`crates/runtime/src/context/budget.rs:125-164`）。**注意**：L6 恒为空层（`budget.rs:160`）。
- **ContextPolicy / ContextLayerType / excluded_layers**：skill 关联的上下文策略（`domain/src/skill.rs`）。`excluded_layers` 字段**未被任何代码消费**（死配置，见 `docs/modules/runtime.md` §8.2）。
- **ContextScore（上下文评分）**：五维乘积 `relevance*importance*recency*explicitness*visibility`（`crates/runtime/src/context/ranking.rs:13-24`），任一维为 0 则总分为 0（脆弱，见 §8.6）。

## D–F

- **db crate（数据层）**：实现 `domain::ports` 与各 Repo，含 21 个迁移（`crates/db/migrations/`）。`application_ports.rs` 约 2595 行手写 SQL。
- **DuckDB（历史遗留）**：早期方案曾考虑 DuckDB；`crates/db/src/ser.rs` 注释仍残留「DuckDB」字样，但实现已全面转向 Postgres（无需实际行动，仅注释噪音）。
- **Entity（实体）**：宽表模型，按 `entity_type_id` 区分人物/地点/势力/物品（`001:67-83`；`domain/src/entity.rs:46-66`）。
- **FactCertainty（事实确定性）**：枚举 `'CANON'|'PROBABLE'|'RUMOR'|'BELIEF'|'SPECULATION'|'FALSE_BELIEF'|'UNKNOWN'`（`domain/src/canon.rs:100-115`）。前端 `types/world.ts:69` 用 `'Confirmed'|'Likely'|'Rumor'|'Uncertain'`，完全不匹配。
- **Foreshadowing（伏笔）**：叙事伏笔，表含 `storyline_id`（`001:914`）；前端 `Foreshadowing` 接口缺该字段。

## G–I

- **GenerationTask（生成任务）**：AI 文本生成任务，`TaskStatus = Pending|Running|Completed|Failed|Cancelled`（`domain/src/generation.rs:56-62`）。前端 `types/generation.ts` 多出中间态、缺 `Running`，导致轮询 UI 失真。
- **GenerationRun / ValidationRun / ValidationIssue（生成/校验运行）**：可溯源审计记录（`domain/src/generation.rs:143-183`；`001:455-542`）。前端 `trace.ts` 与之一致，是字段对齐最好的模块。
- **Iron Triad / 铁三角评审**：本项目的协作评审模式（human 决策 + ChatGPT 提案/复审 + 本 agent 执行验证），见 `available_skills` 的 `iron-triad` 说明及 `ai/src/lib.rs:6-18` 的「铁三角评审 P2 清理」注释。
- **infrastructure（基础设施层）**：承载 LLM（`LlmClient`/`InfraLlmPort`）、artifacts、observability。注意 `InfraLlmPort` 丢弃 `model` 参数并硬编码 `max_tokens:4096/temperature:0.7`（`crates/infrastructure/src/llm/port_impl.rs:29-71`）。

## J–L

- **job（异步任务状态机）**：`ai` crate 的 `job` 模块（`crates/ai/src/job.rs`），承载 AI 异步任务。`ai/src/lib.rs:23` 已 `mod job`。
- **LlmPort（LLM 端口）**：领域端口，定义 `complete`/`stream_complete`（`domain/src/ports.rs:205-221`），由 `infrastructure::InfraLlmPort` 实现。

## M–O

- **MutationCommand / MutationBatch（变更命令 / 批）**：application 层的写模型，经 `MutationCommitter` 落 Canon（`application/src/mutation/`）。与 runtime 的 `ProposedChange`/`StateCommitter` 构成**两套写入口**（见 `docs/ARCHITECTURE_OVERVIEW.md` §5.1）。
- **P1 / P2 / P3（方案阶段标记）**：源自 GPT 方案的分期。`P1`=基础框架（agent 已实现范围，含 SSE 流式/工具框架）；`P2/P3`=待接入能力（持久化 4 表、Workflow 引擎、真实领域工具）。见 `crates/agent/src/lib.rs:10-18`。

## P–R

- **ProposedChange（提案变更）**：AI/用户提出的世界状态变更，经 `Validator` 裁决、`StateCommitter` 提交（`domain::validation`）。AI 只提案、不直接写 Canon。
- **Project（项目）**：顶层创作容器，绑定一个 World（`domain/src/project.rs:11-32`；`001:19-33`）。
- **R2 refactor（角色重构）**：迁移 `017_character_r2_refactor.sql` 把角色字段重命名/重组（real_name→name、background→background_origin、状态拆为 physical/mental/resource/social_state + flags）。前端尚未跟进。
- **ReproducibilityMeta（可复现元数据）**：记录 world_version、retrieval_strategy、retrieved_documents 等，支撑可复现（`domain/src/generation.rs:124-139`）；runtime 填充 world_version 与检索引用，model/temperature/prompt_hash 由 Generation Runtime 负责。
- **retrieval（检索）**：`ai` crate 的 `retrieval` 模块（`crates/ai/src/retrieval.rs`），承载 AI 上下文检索。

## S–U

- **SceneContract（场景契约）**：场景必须满足的约束（required/forbidden 事件、required_characters 等，`domain/src/contract.rs`）。`ContractValidator::required_characters_met` 硬编码 `true` 占位（`crates/runtime/src/validation/contract_validator.rs:47-50`）。
- **SkillType / ExtendedSkillType**：技能类型。`domain::SkillType` 与 `ai::ExtendedSkillType` 分裂孤立（见 `docs/modules/ai.md` §8）。
- **soft delete（软删除）**：项目采用 `status` 列或 `deleted_at` 标记删除而非物理删除（`db` 各 repo 普遍过滤 `Active`）。
- **StateCommitter / StateCommitterPort（状态提交器 / 端口）**：runtime 侧的 Canon 写边界，纯委托（`crates/runtime/src/commit/state_committer.rs:15-31`）。
- **StatePort / EntityPort / NarrativePort 等**：`domain::ports` 中定义的世界状态读写端口，由 `db` 实现。

## V–Z

- **Validator / ValidationRun / ValidationIssue**：`runtime::Validator` 校验 `ProposedChange`（Critical→Rejected、Warning→PendingApproval、否则 Approved，`crates/runtime/src/validation/validator.rs:49-112`）；`ValidationRun`/`ValidationIssue` 为可溯源结果（与前端 `trace.ts` 对齐）。
- **world_version（世界版本）**：状态推进的版本号（git-commit 式语义递增），是 Canonical State 的时间轴。
- **WorldService / EntityService（世界 / 实体服务）**：application 层的两套实体写路径之一（与 MutationCommitter 并存，见 `docs/modules/application.md` §8）。
