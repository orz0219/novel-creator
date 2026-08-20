# Events 契约（events contract）

> 记录 `system_events` 的当前语义边界与 P2 开放议题。

## 当前角色（as-built）

`system_events` 目前是 **append-only 的 domain history**，承担三类用途：

1. **commit event**：`commit_changes` 每提交一个 `ProposedChange` 就写入一条
   `DomainEventType::ProposalCommitted` 事件，`event_id` 被 `state_change`
   外键引用（见迁移 `010` 的修复）。
2. **state_change 的持久化锚点**：`state_change.event_id` → `system_events(id)`
   是状态变更的可追溯链路。
3. **（未来）AI trace**：计划承载 AI 生成的溯源（prompt / 链路），迁移 010
   注释已预留。

## 语义决议（P2 已定稿）

`system_events` = **append-only domain event history**（可 replay 的世界事实），
**不是** audit log，**不存储** AI trace。

- domain event（永久历史 / 可 replay / 参与世界状态演进 / 属于业务事实）：
  `EntityCreated` / `EntityMoved` / `RelationshipChanged` / `WorldAdvanced` 等。
- audit log（谁操作 / 何时 / 从哪 / 权限）属于运行系统的访问追踪，**不进入** system_events。
- AI trace（prompt / response / model / token / latency / cost）属于
  **infrastructure observability**，**不进入** system_events。

### AI trace 归属（P2 已定稿）
AI trace 放 infrastructure observability 层，例如新建
`crates/observability/ai_trace.rs`（或 `crates/infrastructure/src/ai_trace/`）+ `ai_traces` 表
（`id, request_id, model, prompt_hash, token_input, token_output, latency_ms, cost, created_at`）。
**禁止**：
- `system_events.ai_trace_id` 外键；
- `domain::AiTrace` / `domain::AiCallEvent` 之类进入 domain model；
- `system_events` 写入 `event_type="AI_REQUEST"`。
模型变化（GPT-X → GPT-Y）不应改变世界模型；世界事实只记录 `ProposalAccepted`，
不记录 `GPT-Y called`。

## 契约约束（已锁定）

- `commit` 写入的事件必须可被 `state_change.event_id` 正确回溯（不变量见
  `commit.md` 第 3 条）。
- `system_events` 为 append-only：只允许 INSERT，不允许 UPDATE/DELETE 业务行
  （修正/作废通过新事件表达）。
- 任何把 `event`（narrative 事件表）与 `system_events` 混淆的改动都是 bug：
  二者是**不同表**，`state_change` 指向 `system_events`，而
  `event_entity` / `timeline_event` 指向 `event`。

## AI trace 归属（参考结论，待 P2 确认）

AI 调用链路（prompt / response / model / tokens / latency / cost）更偏向
**infrastructure observability**，应在 domain 之外（infrastructure 层）承载，
**不要污染 domain**。
