# 重构路线图（ROADMAP）

> 按「**混乱性价比**」（先易后难、先高 ROI 低风险后高价值高成本）排序的分阶段计划。每条均引用 `docs/modules/*.md` 与 `文件:行号` 证据。目标不是推倒重来，而是**让现有架构意图（Canonical 唯一真源 + AI 只提案 + 依赖倒置）真正落地**，消除已坐实的死代码、双写入口、字段漂移与功能断裂。

排序原则：
1. **阶段 0 止血**：纯删除/收窄，无行为变更，风险极低，立即见效。
2. **阶段 1 修架构违规**：双写入口是唯一违反核心原则的结构性问题，必须早做。
3. **阶段 2 契约对齐**：前后端字段/枚举错位是「直接致 bug」的高频源，ROI 极高。
4. **阶段 3 功能断裂**：用户可见的 501/假数据，影响可用性。
5. **阶段 4–5 能力缺口与重复**：提升质量，可并行持续。

---

## 阶段 0：止血与清理（低风险 · 高 ROI · 纯删除/收窄）

| 项 | 证据 | 动作 | 工作量 |
|---|---|---|---|
| 0.1 删除 `ai` crate 未编译死代码 | `crates/ai/src/lib.rs:22-28` 仅 `pub use domain::*`，`character_mind.rs`/`state_mgmt.rs`/`repair.rs` 未被 `mod` 声明，与 `domain` 逐行重复 | 直接删除这三文件，依赖方改为 `use domain::*` | S |
| 0.2 删除前端未引用组件 | `grep` 全仓库无匹配：`EntityHighlight.vue`/`NeTooltip.vue`/`ContextInspector.vue`/`HistoryInspector.vue`（`docs/modules/components.md` §6.1） | 删除 4 文件（或先确认后再删） | S |
| 0.3 移除重复组合根构造 | `narrative-engine` 的装配在 `api/*` handler 与 `agent_tools.rs` 两处重复（`docs/modules/narrative-engine.md` §8） | 抽取到 `main.rs` 单一 `build_app_state()` | S |
| 0.4 收窄 CORS | `api/mod.rs:28-31` `AllowOrigin::Any` | 改为显式 origins（开发/生产白名单） | S |
| 0.5 清死占位/死代码 | `application::MutationResultExt::conflict_to_409`（`docs/modules/application.md` §8）、`agent` 占位注释块/`get_by_type`（`docs/modules/agent.md` §8.8）、`runtime` 冗余排序/`default_score` 未用（`docs/modules/runtime.md` §8.4）、`api/client.ts:32-39` `createSSE` 无人调用 | 删除或接上 | S |

**验收**：`cargo build` 通过；前端无 unused 导入告警；CORS 仅放行白名单。

---

## 阶段 1：统一 Canon 写入口（架构违规修复 · 中风险 · 高价值）

| 项 | 证据 | 动作 | 工作量 |
|---|---|---|---|
| 1.1 合并两套写入口 | `runtime/src/commit/state_committer.rs:15-31`（走 `StateCommitterPort`+`ProposedChange`）与 `application/src/mutation/committer.rs:14-63`（走 `MutationCommitterPort`+`MutationCommand`）并存，二者都声称「唯一落 Canon」 | 选定单一边界（建议 `application::MutationCommitter` 为应用层写入口），`runtime::DbStateCommitter` 改为委托它；废弃 `StateCommitterPort` 或反之内聚 | M |
| 1.2 收敛双实体写路径 | `WorldService` 与 `EntityService` 两套实体写路径（`docs/modules/application.md` §8） | 统一经 1.1 的写入口 | M |

**风险**：涉及事务边界与状态机，需配套集成测试（参考 `runtime/tests/integration_tests.rs` 的 CAS/隔离用例）验证 Approved-only 提交不变。
**验收**：全仓库只有一个「落 Canon」函数；现有 proposal→commit 流程行为不变。

---

## 阶段 2：前后端契约对齐（高 ROI · 直接修 bug · 中工作量）

| 项 | 证据 | 动作 | 工作量 |
|---|---|---|---|
| 2.1 刷新角色类型 | `types/character.ts:5-29` 仍用 `real_name/health/cultivation/...`，后端 R2 后已无（`domain/src/character.rs:278-328`，`017_character_r2_refactor.sql`） | 以 `domain/src/character.rs` + 迁移为基线重生成 `character.ts` | M |
| 2.2 FactCertainty 枚举对齐 | 前端 `'Confirmed'\|'Likely'\|'Rumor'\|'Uncertain'` vs 后端 `CANON/PROBABLE/RUMOR/BELIEF/SPECULATION/FALSE_BELIEF/UNKNOWN`（`types/world.ts:69` vs `domain/src/canon.rs:100-115`） | 前端枚举改对齐 | S |
| 2.3 GenerationTaskStatus 对齐 | 前端缺 `Running`、多 `BuildingContext/Generating/Validating`（`types/generation.ts:14-21` vs `domain/src/generation.rs:56-62`），导致轮询/进度 UI 失真 | 引入 `Running`，去掉虚构态；`useGenerationStore`/`useGeneration` 阶段映射同步修 | M |
| 2.4 ApprovalRecord 字段对齐 | `reviewer`(string)/`review_notes`(string) vs `reviewer_id`(UUID)/`reviewer_comment`（`types/approval.ts:15-25` vs `001:889-902`） | 改名对齐 | S |
| 2.5 补缺失字段 | `Foreshadowing` 缺 `storyline_id`（`types/narrative.ts` vs `001:914`）；`Rules` 缺 `constraints`/`source`（`types/rules` vs `domain/src/canon.rs:91-93`） | 补字段 | S |
| 2.6 去 agent 双 BASE 前缀 | `agent.ts:7` 硬编码 `/api/v1/agent` 绕开 `client.ts:3` BASE_URL；`createSSE` 死代码 | 复用 `BASE_URL`；删除 `createSSE` | S |
| 2.7 去弱类型返回 | `settings`/`rules`/`snapshots` 后端返回裸 `serde_json::Value`（`docs/modules/api.md` §4.5/§4.6） | 后端显式序列化结构体，或前端用 `zod` 校验 | M |

**验收**：`npm run type-check` 全绿；前端角色/事实/生成进度/审批在真实数据下显示正确。

---

## 阶段 3：功能断裂修复（用户可见 · 中高工作量）

| 项 | 证据 | 动作 | 工作量 |
|---|---|---|---|
| 3.1 Context 端点实现 | 后端 6 个 handler 全 501（`api/context.rs:8-52`），前端 `stores/context.ts:15-33` 必失败 | 接线 `runtime::ContextEngine`，实现 get/build_context + pin/exclude；**或**前端对 501 显式提示而非静默空白 | L |
| 3.2 死路由 stream | 前端 `generation.ts:14` 调 `/generations/{id}/stream`，后端 `api/mod.rs` 未注册 | 注册后端 SSE handler，或前端改走 `/execute`+轮询 | M |
| 3.3 Validation 真校验 | `ContractValidator::required_characters_met` 硬编码 `true`（`runtime/src/validation/contract_validator.rs:47-50`）；`validation` 三端点假通过（`docs/modules/narrative-engine.md` §8） | 接 DB 名字解析；改为真实校验 | M |
| 3.4 假数据落库 | history version 伪造、proposal change 级 accept/reject 假成功不落库、location 子资源返空（`docs/modules/narrative-engine.md` §8） | 接真实仓储与事务 | M |
| 3.5 ChatMessage.role 契约 | 后端注释 `system` 实际写 `tool`，前端联合类型无 `system` 有 `tool`（`docs/modules/agent.md` §5.3/§8.3） | 统一为一处权威枚举并同步 TS | S |

**验收**：Context 面板能展示真实上下文；生成/校验/审批在 UI 上行为真实可信。

---

## 阶段 4：能力缺口（中工作量）

| 项 | 证据 | 动作 | 工作量 |
|---|---|---|---|
| 4.1 LlmPort 透传 model | `port_impl.rs:44` `let _ = model;`，硬编码 `max_tokens:4096/temperature:0.7` | 真正使用传入 model；参数改由配置/请求驱动 | S |
| 4.2 可观测性 | `health_check` 恒 `true`；`Metrics` 无导出（`docs/modules/infrastructure.md` §8） | health 真正检查 PG；Metrics 接入 Prometheus/日志 | M |
| 4.3 职责边界文档化 | `agent`（引导会话编排）与 `ai`（检索/异步任务）边界模糊（`docs/modules/agent.md` §8.1） | 在 `ARCHITECTURE.md` 显式划分，避免功能在两 crate 间摇摆 | S |
| 4.4 项目级提示词定制 | `resolve_base` 硬编码 `global` 使 scope 级自定义不生效（`docs/modules/agent.md` §8.4） | 支持 `project:<uuid>` scope 读取 | S |

---

## 阶段 5：重复逻辑收敛与质量（持续 · 低优先级）

| 项 | 证据 | 动作 | 工作量 |
|---|---|---|---|
| 5.1 三页抽象 | `Characters/Locations/Factions` 结构雷同（`docs/modules/pages.md` §7.10） | 抽 `EntityDetailPage` 通用组件 | M |
| 5.2 取数去重 | `ProjectLayout` 与子页面重复拉取同一份数据（`docs/modules/app.md` §7.1） | 统一预取到 layout，子页只读缓存 | M |
| 5.3 scoring 模型 | `ContextScore` 乘积脆弱、ranking 内排序冗余（`docs/modules/runtime.md` §8.4/§8.6） | 文档化或改加权和；清理冗余排序 | M |
| 5.4 前端 mock 接真 | `ActivityCenter`/多处 Inspector 硬编码（`docs/modules/components.md` §6.2） | 接 store/api | M |
| 5.5 状态类名统一 | `StatusBadge` 小写 vs `StoryNode` 大写（`docs/modules/components.md` §6.4） | 统一枚举/类名 | S |

---

## 执行建议

- **阶段 0–1 可在一个迭代内完成**（纯删/合并，不增功能），立即降低维护熵。
- **阶段 2 与 3 应结对进行**：先对齐契约（2）再修功能断裂（3），否则 3 的实现又会踩 2 的旧类型。
- 每个阶段结束跑：`cargo test` + `vitest` + `npm run type-check`，并以 `docs/ONBOARDING.md` §7 自检清单过一遍。
- 所有改动需同步 `domain 实体 ↔ db 迁移 ↔ frontend/types` 三源，避免复现阶段 2 的漂移。
