# 前端字段/按钮/状态完整性 检查报告（Inspector Audit）

> 审计时间：2026-08-21（目标轮次 1/256）
> 审计对象： `/Users/wangxingchao/Documents/novel/frontend` （Vue 3）+ 运行中的后端 `:8080` + 前端 `:5173`
> 对照基准： **真实数据库 schema**（`crates/db/migrations/001_canonical_schema.sql` 及 `002–016` 增量），而非已过期的 `DATABASE_SCHEMA.md`（该文档与实际库严重漂移，详见下文）
> 检查方式： agent-browser 实机渲染检查 + 4 个并行子代理的静态代码审计（读 .vue / types / api / store / migration）
> 测试项目： `测试项目-玄幻`（`430cbfa0-...`）

---

## 一、结论（TL;DR）

**前端存在系统性“偷懒”，远未完整呈现数据库应展示的字段。** 在 29 个页面中，**没有任何一个页面是 COMPLETE 的**；其中 **9 个页面/组件是纯静态假数据（不调用任何后端）**，其余均为 PARTIAL（字段被大量丢弃）。

最关键的三类问题：

1. **实体详情页只编辑 `entity` 基表**（name/summary/description），数据库为支撑角色/地点/势力准备的 **30+ 列子表全部不可见**（见第二节）。
2. **9 个页面是硬编码 mock**，渲染的是伪造数据（如 History 显示 `林凡/黑石城/场景1`，而非真实项目数据），与真实库零绑定。
3. **AI 可追溯性两张表 `generation_run` / `validation_run` 在前端完全没有体现**，连 `payload`、`prompt_sent`、`response_received`、`token_usage`、`latency_ms` 都看不到。

> ⚠️ 与既有 `QA_REPORT.md` 的冲突：旧报告声称“全量 19 页零报错、所有缺陷已修复”。但旧报告只验证了 **“页面能否加载、有无 console 报错”**，**从未核对字段是否完整呈现**。本审计证明：页面能打开 ≠ 字段已展示。旧报告属于“通过假象”。

---

## 二、实体 CRUD 页：只碰基表，子表全丢（最严重）

`Characters / Locations / Factions / Items` 四页的“编辑/新建”弹窗经浏览器实机确认，均**只含 名称 / 摘要 / 详细描述 三个输入框**（见第四节截图证据）。数据库为不同实体类型准备的详尽属性子表，前端类型里根本没有、界面上也无处可填：

| 页面 | 数据库应展示但前端缺失的子表 / 字段 |
|------|------------------------------------|
| **人物 Characters** | `character_profile`（real_name, nickname, age, gender, identity, appearance, background, social_status, core_personality, values）、`character_state`（location, health, cultivation, money, wanted, extra）、`character_goal`（long_term/current/immediate）、`character_trait`（trait_type/name/description/intensity）。`entity.status`（Active/Deleted）也不显示。 |
| **地点 Locations** | 前端**根本没有 `location.ts` 类型**；`location_identity`（location_type/size/climate/era/accessibility）、`location_geography`（terrain/natural_resources/hazards）、`location_facilities`、`location_rules`、`location_threats`、`location_secrets`、`location_narrative_hooks`、`resource_state`（resource_name/quantity/production_rate/controlled_by）—— 约 25 列全部无模型、无界面。 |
| **势力 Factions** | `faction_profile`（goals/leader/values/resources/territory/members/enemies/allies/internal_conflicts/secrets/modus_operandi）11 字段从不渲染；`frontend/src/api/faction.ts` **文件不存在**（靠通用 entity 接口勉强跑通）。 |
| **物品 Items** | 物品是纯 `entity`，无专属子表 → 此项**相对完整**（仅缺 `entity.status` 展示）。 |

附带： `CharacterCard / LocationCard / FactionCard` 三个组件是**死代码**（从未被引用）。

---

## 三、纯静态 mock 页面（不调用后端，显示伪造数据）

| 页面 / 组件 | 证据 | 应展示的真实表 |
|------|------|------|
| **History（历史）** | 浏览器实机显示 `林凡/黑石城/地下遗迹/场景1` 等硬编码条目，与真实项目（林惊羽/陆雪琪/道玄真人…）不符；子组件 `EventLog.vue`、`VersionDiff.vue` 渲染字面量数组，从未调用 `historyApi` | `state_change` / `current_state` / `system_event` / entity 版本 |
| **Search（搜索）** | `allResults` 是写死的 6 条常量，无任何搜索 API 调用 | entity / scene / fact |
| **KnowledgePanel（知识）** | 4 个字面量数组（knownItems 等），形状与 `KnowledgeState` 类型不符，无 API | `knowledge_state` / `reader_knowledge` / `revelation` |
| **ConstraintPanel（约束）** | `levels` 硬编码；`+ 添加` 不接任何 `canon_rule` 接口 | `canon_rule` |
| **StoryBoard（看板）** | 硬编码 mock 数组，零绑定 | `narrative_node` |
| **StructuredEditor（结构化编辑器）** | 硬编码 8 个实体名列表，点击仅 `console.log` | entity / scene |
| **Graph（图谱）** | 子代理确认纯演示数据，无后端调用 | entity / relation |
| **ProjectDashboard（项目仪表盘）** | 标题写死 `"天玄大陆"`，**无任何 `onMounted`/fetch**，不显示真实项目任何字段 | `project` |

---

## 四、浏览器实机验证记录（agent-browser）

| 页面 | 实际渲染 | 结论 |
|------|---------|------|
| `/`（首页） | 项目卡片含 名称/描述/状态徽章(概念)/更新日期 | 字段偏少，但非 mock |
| `/world/characters` → 点开“林惊羽” | 弹窗仅 **名称 / 摘要 / 详细描述** + 取消/更新 | ✅ 证实实体子表全丢 |
| `/world/locations` → 点开“剑冢” | 弹窗仅 **名称 / 摘要 / 详细描述** | ✅ 同上 |
| `/proposals` | 显示 状态(Approved)/原因/“变更 (0)”/验证结果引用，但 **`payload` 具体内容不展开** | PARTIAL |
| `/history` | 显示 `林凡/黑石城/场景1` 等**假数据** | ✅ 证实 STATIC-ONLY |
| `/settings` | 输入框带 `v-model`，有保存按钮（Round 9 已修复，非旧 bug） | COMPLETE（功能壳） |
| `/world` | 仅实体/事实计数卡片，世界自身的 name/rules/is_main 均不显示 | PARTIAL |

---

## 五、其余 PARTIAL 页面（字段被丢弃）

| 页面 | 缺失字段 / 问题 |
|------|----------------|
| **Proposals** | 缺 `proposed_change.change_type/description/payload/target_entity_id/resolved_at/content_hash`；验证缺 `issue_type/suggestion`；整张 `validation_run`（validated/approved/rejected/status/时间）无体现；**无“运行验证”按钮**；`PartiallyAccepted/Expired` 状态不可操作。 |
| **Snapshots** | 缺 `world_summary/current_location/main_character_state/scene_id/volume·arc id/state_data`；**无“恢复/回滚”动作**（只有删除）。 |
| **Extract** | 缺候选 `attributes`、fact `category/certainty`；无逐条编辑。 |
| **Story** | 仅“新建”树，无编辑/删除；`attributes`（卷/弧/场景/节拍）不展示。 |
| **Storylines** | 只读列表，无编辑/删除；缺 `created_volume_id/resolved_volume_id`；status 不可编辑。 |
| **Foreshadows** | 类型与 DB 均缺 `introduced_at/expected_reveal_at/actual_reveal_at`；`hint_level` 枚举不匹配（多出 `Obvious`，缺 `Explicit/Hidden`）。 |
| **Timeline** | 缺 `event_type/event_time/duration/timeline_id`；`timeline_event` 连接表未用；只读。 |
| **Writing / SceneEditor** | 只暴露 objective/conflict/time；真正的 `scene` 表（pov_character_id, location_id, scene_start/end_time）未建模；story-state 区块硬编码。 |
| **Home** | `project.status` 可显示但不可编辑；7 个类型字段（language/world_setting/system_setting/default_model/default_style/default_params/config）从不显示；无编辑/删除按钮。 |
| **World** | 世界自身的 name/description/world_rules/config/is_main 均不显示（仅 Rules 页只读显示 name/description）。 |
| **Rules** | CRUD 完整，但 `affected_scope` 列从不显示/可编辑。 |
| **GenerationPanel (+ store)** | 缺 `model/target/parameters/context_tokens/error`；`skill_id/scene_id`（迁移 016）缺失；整张 `generation_run` 追踪（prompt/response/tokens/latency）不可见；无取消按钮；**store 是 mock**。 |

---

## 六、数据库表 → 前端覆盖矩阵（用户域）

**有页面但字段不全**：entity(人物/地点/势力)、project、world、proposed_change、validation_issue、narrative_node、storyline、foreshadowing、event、context_snapshot(novel_state_snapshot)、fact（抽取时）。

**完全无前端表示的用户域表（零 UI）**：
- 角色：`character_profile`、`character_state`、`character_goal`、`character_trait`、`character_arc`
- 地点：`location_identity`、`location_geography`、`location_facilities`、`location_rules`、`location_threats`、`location_secrets`、`location_narrative_hooks`、`resource_state`
- 势力：`faction_profile`
- 叙事/场景：`scene`、`scene_entity`、`scene_requirement`、`scene_contract`、`plot`、`causal_relation`、`storyline_scene`、`timeline_event`
- 知识/揭示：`knowledge_state`、`reader_knowledge`、`fact_visibility`、`revelation`
- 世界/分支：`authorial_intent`、`world_branch`、`narrative_branch`、`narrative_state`、`narrative_thread`
- 其它域：`quality_score`(类型存在但无任何组件引用)

**内部/系统表（无 UI 可接受，但 AI 可追溯性应补）**：`generation_run`*`、`generation_task`、`validation_run`*`、`validation_issue`、`context_snapshot`、`state_snapshot`、`system_event`、`event_outbox`、`test_case`、`test_result`、`decision_trace`、`agent_runs`、`revision_plan`、`scene_ledger` 等。（带 `*` 为强烈建议补 UI 的可追溯性表）

---

## 七、建议修复优先级

1. **P0 — 消除 mock 页**：History / Search / KnowledgePanel / ConstraintPanel / StoryBoard / StructuredEditor / Graph / ProjectDashboard 改为接真实 API（这些页面目前对用户是“谎言”）。
2. **P0 — 实体子表**：为 Characters / Locations / Factions 增加对应的 profile/state/goal/trait 及 location_* / resource_state 编辑区（至少把类型补齐、弹窗加字段）。
3. **P1 — AI 可追溯性**：新增 `generation_run` / `validation_run` 详情视图（prompt/response/tokens/latency/校验汇总）。
4. **P1 — Proposals / Snapshots / History 字段补全**：展开 `payload`、补 `resolved_at`、加“运行验证”“恢复快照”动作。
5. **P2 — 项目/世界级字段**：`world_setting/system_setting/default_model/default_style/default_params/config/world_rules/is_main/authorial_intent` 至少在某个编辑界面可写可读。
6. **P2 — 状态可编辑**：`project.status`、`entity.status`、`narrative_node/foreshadowing/storyline.status` 当前仅展示不可改，应提供状态切换。

---

## 八、审计产物与证据链

- 实机验证：agent-browser 会话 `audit`，已逐一打开上述路由并比对 DOM 与真实项目数据。
- 静态审计：4 个并行子代理分别覆盖 [实体 CRUD]、[叙事/故事]、[AI/提案/历史/快照/抽取/搜索]、[项目/世界/设置/规则]，结论与本报告一致并附文件行号。
- 对比基准：真实 schema 来自 `crates/db/migrations/001_canonical_schema.sql`（92 张表），`DATABASE_SCHEMA.md` 已确认过期、不可作为对照。

**判定：前端代码存在明显偷懒，未完整展示数据库应展示的字段；需按第七节优先级补齐。**
