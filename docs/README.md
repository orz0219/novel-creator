# Novel Creator 文档索引

> **👋 新人 / 实习生先看这里**：`NEWCOMER_GUIDE.md` —— 30 分钟建立全局心智模型的轻量入口（系统由哪几块拼成、一次请求怎么走、第一天该读哪些文件）。

> 本目录是 Novel Creator 项目的**源码级**文档集，按用户要求「以源码为准、逐文件通读、带行号对照」，覆盖 8 个 Rust crate 与前端全部技术层。所有论断均可在对应文件 `文件:行号` 处核对。

## 阅读顺序（推荐）

1. **ARCHITECTURE_OVERVIEW.md** — 跨模块架构总览、依赖拓扑、核心原则、已识别痛点汇总。
2. **后端模块**（底层 → 顶层）：
   - `modules/domain.md` — 实体与端口（一切真源）
   - `modules/db.md` — Repo 实现 + 迁移
   - `modules/infrastructure.md` — LLM / artifacts / observability
   - `modules/application.md` — service 层
   - `modules/ai.md` — retrieval / job（**含未编译死代码告警**）
   - `modules/agent.md` — 引导式 Agent 核心层
   - `modules/runtime.md` — Context Engine / Validator / Commit
   - `modules/narrative-engine.md` — 组合根 + axum API（**含死路由 / 501 / 假数据告警**）
3. **前端模块**（契约 → 网络 → 状态 → 逻辑 → 视图）：
   - `modules/types.md` — 类型契约（**含 R2 字段错位告警**）
   - `modules/api.md` — HTTP 客户端
   - `modules/stores.md` — Pinia 状态
   - `modules/composables.md` — 组合式函数
   - `modules/pages.md` — 路由级视图
   - `modules/app.md` — 应用装配（router/layouts）
   - `modules/components.md` — 组件层
4. **全局配套**：
   - `GLOSSARY.md` — 领域术语表
   - `ONBOARDING.md` — 如何构建/运行/避坑
   - `ROADMAP.md` — 按混乱性价比排序的分阶段重构路线

## 文档结构（每篇统一模板）

概述 → 模块职责 → 依赖关系（mermaid）→ 目录与源码对照（表）→ 字段设计（三源对照表）→ 核心流程（mermaid 时序/流程图）→ 接口/类型签名（表 + 代码块）→ **问题/代码异味（逐条附 `文件:行号` 证据）** → 小结。

## 已坐实的关键痛点（速览）

| 类型 | 代表问题 | 出处 |
|---|---|---|
| 架构违规 | 两套 Canon 写入口并存 | `ARCHITECTURE_OVERVIEW.md` §5.1 |
| 死代码 | `ai` crate 三文件未编译；前端 4 组件未引用 | §5.2 |
| 字段漂移 | 角色/事实/生成/审批前后端枚举错位 | §5.3 |
| 功能断裂 | Context 全 501；死路由 stream；Validation 假通过 | §5.4 |
| 能力缺口 | LlmPort 丢 model；可观测性缺位 | §5.5 |
| 重复 | 组合根重复构造；三页雷同；取数重复 | §5.6 |

完整证据与修复方案见 `ROADMAP.md`。
