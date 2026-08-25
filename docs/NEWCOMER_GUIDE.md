# 新人地图（Newcomer Map）

> 目标读者：**第一次接触本项目的新人 / 实习生**。目标是让你在 30 分钟内建立「系统由哪几块拼成、一次请求怎么走、第一天该读哪些文件」的全局心智模型。
>
> 本文档是**入口与地图**，不是细节文档。所有深度内容都在它指向的其它文档里（见末尾「下一步去哪」）。
> 想把它跑起来、避坑，见 `ONBOARDING.md`；想读源码级逐模块说明，见 `docs/modules/*`。

---

## 0. 一句话理解这个项目

**Novel Creator 是一个「AI 辅助长篇小说创作」系统。**

它的核心设计哲学只有一句话：

```text
AI 只提案（Proposal）。
确定性世界状态（Canon）是唯一真源。
任何写操作，都必须经过统一边界裁决后才能落库。
```

- 后端：Rust workspace，8 个 crate，承载领域建模、AI 编排、Canonical 世界状态、HTTP API。
- 前端：Vue 3 + TypeScript，提供世界 / 故事 / 提案 / 引导对话等创作界面。
- 数据库：Postgres 16（docker 容器 `novel-postgres`，端口 5432）。

> 完整的设计原则（为什么这样设计）在根目录 `ARCHITECTURE.md`。新人不必第一天啃完它（1191 行），但建议第二周回看。

---

## 1. 系统由哪几块拼成（高层地图）

```text
┌──────────────────────────────────────────────────────────────┐
│  前端 (Vue3 + TS)                                             │
│  types → api → stores → composables → pages/app/components    │
└───────────────────────────┬──────────────────────────────────┘
                            │  HTTP / SSE   /api/v1/*
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  narrative-engine  （组合根 + axum API，不含业务逻辑）          │
└───────────────────────────┬──────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  application  （用例编排 / service 层）                         │
│  runtime      （Context Engine / Validator / Commit）          │
│  agent        （引导式会话编排：session / 工具 / 流式）          │
│  ai           （检索 retrieval / 异步任务 job）                 │
└───────────────────────────┬──────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  domain       （实体 + 端口 ports —— 一切真源，不依赖任何人）    │
└──────────┬───────────────────────────────────┬───────────────┘
           ▼                                   ▼
┌──────────────────────┐            ┌──────────────────────────┐
│  db                  │            │  infrastructure          │
│  （Postgres Repo +   │            │  （LLM / artifacts /      │
│   迁移，实现 ports）  │            │    observability）        │
└──────────┬───────────┘            └──────────┬───────────────┘
           ▼                                   │
      ┌─────────┐                              │ 实现 LlmPort
      │ Postgres│                              │
      └─────────┘                              ▼
                                         ┌─────────┐
                                         │  LLM    │
                                         └─────────┘
```

**读图要点（按依赖方向，越往下越"地基"）：**

1. `domain` 是地基：定义实体（World/Entity/Character/Event/Fact…）和端口（ports）。它**不依赖任何上层、数据库或 HTTP**，所以谁都能安全引用它。
2. `db` 和 `infrastructure` 是"适配器"：分别实现 domain 定义的数据库端口和 LLM 端口。
3. `application` / `runtime` / `agent` / `ai` 是"编排层"：只依赖 domain 的端口与实体，具体实现由组合根注入。
4. `narrative-engine` 是"组合根 + API"：把 LLM 客户端、各 Repo、领域工具装配起来，并对外提供 HTTP 接口。**它不应承载业务逻辑。**

---

## 2. 一次核心请求怎么走（核心生命周期）

理解下面这条主链路，你就理解了整个系统 80% 的行为：

```text
用户意图
   ↓
Application 创建用例（use-case）
   ↓
检索相关 Canonical 状态
   ↓
构建 Context 快照（Context Engine：检索 → 可见性 → 排序 → 预算）
   ↓
执行生成能力（调用 LLM）
   ↓
产出 Draft 和/或 ProposedChange（AI 的"提案"，不是真源）
   ↓
Validation（确定性校验，必要时请求人工审批）
   ↓
Commit（唯一的写边界，落 Canon）
   ↓
追加 Domain Event / State Change
   ↓
更新 Current State 投影
   ↓
持久化到 Postgres
```

**最关键的一条边界（务必记住）：**

```text
Probabilistic World（AI 生成，允许不确定）
        │
        ▼
   Proposal（提案）
        │
        ▼
Deterministic Validation（确定性校验）
        │
        ▼
Canonical World（确定性的世界状态，唯一真源）
```

> 「一切在 Proposal 之前允许是不确定的；一切在 Commit 之后必须遵守确定性领域规则。」

---

## 3. 8 个后端 crate 各管什么（速查表）

| Crate | 一句话职责 | 新人优先读 |
|---|---|---|
| `domain` | 领域模型 + 端口（一切真源）。实体、规则、提案、验证、生成运行时。 | ⭐⭐⭐ `docs/modules/domain.md` |
| `db` | Postgres 仓储实现 + 21 个迁移（001 为权威 schema）。实现 domain 的 ports。 | ⭐⭐ `docs/modules/db.md` |
| `infrastructure` | 外部系统适配：LLM 客户端、artifacts、可观测性。 | ⭐ `docs/modules/infrastructure.md` |
| `application` | 用例编排 / service 层（事务边界、项目隔离）。 | ⭐⭐ `docs/modules/application.md` |
| `ai` | AI 过程原语：retrieval（检索）与 job（异步任务）。 | ⭐ `docs/modules/ai.md` |
| `agent` | 引导式会话编排：session、工具、非流式/流式聊天、记忆。 | ⭐ `docs/modules/agent.md` |
| `runtime` | 执行层：Context Engine、Validator、Commit（写边界）。 | ⭐⭐ `docs/modules/runtime.md` |
| `narrative-engine` | 组合根 + axum API（HTTP / SSE / 装配）。 | ⭐ `docs/modules/narrative-engine.md` |

> 模块文档统一结构：概述 → 职责 → 依赖(mermaid) → 源码对照 → 字段设计 → 核心流程 → 接口签名 → 小结。

---

## 4. 前端技术层（速查表）

```text
frontend/src
├── types/        契约层（13 子模块，与后端 domain 形成契约镜像）
├── api/          网络边界（client.ts 统一 fetch；agent.ts 独立 SSE）
├── stores/       Pinia 状态（8 个 store）
├── composables/  视图无关 UI 逻辑（5 个）
├── pages/        23 个路由级视图
├── layouts/      AppLayout / ProjectLayout / WritingLayout
├── app/          App.vue / router.ts
└── components/   56 个 .vue 组件
```

**数据流向：** 页面 → store → api → 后端；对话走 `agentStore` 的 SSE 流式。

| 前端层 | 对应文档 |
|---|---|
| `types` | `docs/modules/types.md` |
| `api` | `docs/modules/api.md` |
| `stores` | `docs/modules/stores.md` |
| `composables` | `docs/modules/composables.md` |
| `pages` | `docs/modules/pages.md` |
| `app` | `docs/modules/app.md` |
| `components` | `docs/modules/components.md` |

---

## 5. 第一天 / 第一周的阅读路线

### 第一天（建立全局观，约 30–60 分钟）
1. 本文档（你正在读）。
2. `docs/ARCHITECTURE_OVERVIEW.md` —— 跨模块架构图、依赖拓扑、核心原则。
3. `docs/GLOSSARY.md` —— 术语表（Canon / ProposedChange / ContextEngine / world_version 等），读源码时随时对照。

### 第一周（深入某一条线时）
- **想改后端业务逻辑**：`domain.md` → `application.md` → `runtime.md`（按依赖从底向上）。
- **想改 AI / 对话**：`ai.md` → `agent.md` → `runtime.md`（Context Engine）。
- **想改前端**：`types.md` → `api.md` → `stores.md` → `pages.md` / `components.md`。
- **想接数据库**：`db.md` + `crates/db/migrations/`。

### 进阶（第二周+）
- 根目录 `ARCHITECTURE.md` —— 22 条设计原则与边界约束（信息密度高，适合带着问题回看）。
- `docs/contracts/*` —— 关键契约：commit / events / world-version。

---

## 6. 几个必须记住的核心概念（否则读不懂代码）

| 概念 | 是什么 | 为什么重要 |
|---|---|---|
| **Canon / Canonical State** | 确定性世界状态（world/entity/event/fact… 表），系统唯一权威 | AI 不直接写它，必须经写边界 |
| **ProposedChange（提案）** | AI / 用户提出的世界状态变更 | 是「概率世界」与「确定世界」的分界 |
| **Validator** | 对提案做确定性校验 | Critical→拒绝，Warning→待审批，否则通过 |
| **Commit / 写边界** | 唯一能把变更落到 Canon 的地方 | 一切写操作必须经此 |
| **Context Engine** | 检索 → 可见性 → 排序 → 预算，组装给 LLM 的上下文 | AI 不能拿到整个数据库 |
| **world_version** | 状态推进的版本号（git-commit 式递增） | 支持可复现与回滚 |
| **domain::ports** | 依赖倒置边界（trait） | 上层只依赖接口，实现在 db/infrastructure 注入 |

> 完整术语与源码出处见 `docs/GLOSSARY.md`。

---

## 7. 下一步去哪（文档导航）

| 我想… | 去读 |
|---|---|
| 把它跑起来 / 构建 / 避坑 | `docs/ONBOARDING.md` |
| 看跨模块架构图与依赖拓扑 | `docs/ARCHITECTURE_OVERVIEW.md` |
| 查术语 / 概念 | `docs/GLOSSARY.md` |
| 读设计原则（为什么这样设计） | 根目录 `ARCHITECTURE.md` |
| 逐 crate / 逐前端层源码级说明 | `docs/modules/*.md` |
| 看关键契约（commit / events / world-version） | `docs/contracts/*.md` |
| 了解重构路线（分阶段治理计划） | `docs/ROADMAP.md` |
| 文档总索引 | `docs/README.md` |

---

> 本地图不重复任何细节，只负责「指路」。遇到与源码不符处，**以源码为准**，并可回头核对对应模块文档。
