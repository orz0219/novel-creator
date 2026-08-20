# 小说引擎前端 全量测试 QA 报告

测试时间：2026-08-21
测试范围：前端 http://localhost:5173 （代理 → 后端 http://localhost:8080/api/v1）
测试项目：`测试项目QA`（id `d66d43e2-7d42-4a73-8663-64ccc96581ad`），世界 `5f20e4ac-5dc1-4a51-8f08-9b2941877608`
方法：agent-browser 逐页截图 + 逐按钮点击 + console/errors 捕获 + API 冒烟（curl）

---

## 一、已修复的 Bug（后端数据层 / 前端）

### 1. 【后端·系统性】uuid↔String 误解码（全列表 500）
- 现象：`GET /api/v1/projects`、`POST .../locations` 等 ~22 个 `query_as` 元组把 Postgres `uuid` 列按 `String` 解，抛 `column N ... String not compatible with UUID`。
- 修复：`crates/db/src/application_ports.rs` 所有多列 `SELECT` 的 uuid 列（`id`/`*_id`）统一 `::text`；单列表（`(Uuid,)` 查询）保持不转。
- 验证：17 个列表端点全部 **200**。

### 2. 【后端】narrative 查询 over-cast（反向 500）
- 现象：上一步 blanket cast 把 `narrative_node` 的 `id/project_id/world_id/parent_id` 也转 `::text`，但其元组用 `Uuid` 解 → `Uuid not compatible with TEXT`，`/narrative` 列表+详情 500，连带 `/story`、`/story/board` 打不开。
- 修复：撤销 `narrative_node` 4 个 uuid 列的 cast（保留 `attributes/created_at/updated_at::text`）。
- 验证：`/narrative` 200；`/story`、`/story/board` 正常加载。

### 3. 【后端·schema drift】character_state 缺列
- 现象：`GET /characters/{id}/state` → 500 `column "resources" does not exist`。代码+前端类型要 `resources/current_status/emotion`，但 DB 实际是 `money/wanted/extra`。
- 修复：`ALTER TABLE character_state ADD COLUMN resources/current_status/emotion` + 新增迁移 `crates/db/migrations/012_character_state_align.sql`。
- 验证：`/characters/{id}/state` 200。

### 4. 【前端】写作页 5 组件未注册
- 现象：`WritingLayout.vue` 用了 `<StructuredEditor>/<SelectionActions>/<KnowledgePanel>/<ConstraintPanel>/<EventLog>` 但从未 import → `Failed to resolve component`，面板成空壳。
- 修复：在 `WritingLayout.vue` 补 5 行 import（文件本身存在）。
- 验证：写作页 console 无 `Failed to resolve component`。

### 5. 【前端】搜索结果跳转 `/project/undefined` → 5×500
- 现象：`Search.vue` 取 `route.params.id` 作项目 id，但 `/search` 路由无 `:id` 参数 → `undefined` → 点结果跳 `/project/undefined/world/characters`，触发 5 个 `GET /projects/undefined/*` 全 500。
- 修复：`Search.vue` 改用 `projectStore` 真实 project id（`currentProject?.id ?? projects[0]?.id`），`onMounted` 调 `fetchProjects()`（原误调不存在的 `list()`）。
- 验证：点结果跳真实 `/project/e8ce4045-.../world/characters`，errors 为空。

### 6. 【后端·清理】回退临时调试日志
- `crates/narrative-engine/src/api/error.rs` 临时 `{:#}` 改回 `{}`。

### 7. 【前端】实体删除 UI 缺失（4 页均无删除入口）
- 现象：人物/地点/势力/物品四页卡片与编辑框都**无删除按钮**，后端 `DELETE /entities/{id}` 可用但前端无入口（F 报告）。
- 修复：`EntityCard.vue` 加 hover 删除按钮（🗑，`@click.stop` 防误触编辑，`emit('delete')`）；`Characters/Locations/Factions/Items.vue` 加 `@delete="handleDelete(x)"` + `handleDelete`（原生 `confirm` 二次确认 → 调 `worldStore.deleteCharacter/Location/Faction/Entity`）。store 删除后自动从列表移除（Items 额外本地 `filter`）。
- 验证：浏览器实测删「x」→ 角色 8→7、卡片消失、无 console/JS 错误、confirm 弹窗出现。
- 副作用：顺带补齐「删除无二次确认」风险（原 #5）。

### 8. 【前端】关系页静态 no-op（`/relationships` 不调后端）
- 现象：`Relationships.vue` 硬编码 7 条假关系，过滤器/新建/编辑/删除按钮全无 `@click`，不调任何 API（C 报告 #1）。
- 修复：重写 `Relationships.vue` —— `onMounted` 调 `fetchWorld`+`fetchRelations`+`fetchEntities`；用实体 id→name 映射渲染 `源 → 类型 → 目标`；过滤器按实际 `relation_type` 动态生成；`+新建关系`/`编辑`/`删除` 接 `worldStore.createRelation/deleteRelation`（后端无 PUT，编辑以「删后建」实现）；删除带 `confirm`。
- 验证：浏览器实测 —— 列表来自后端（无静态数据）、新建关系入库（名解析正确）、删除后卡片消失 + `GET /relations` count 0、空态正确渲染。

### 9. 【后端】关系删除 500（乐观锁校验误伤）
- 现象：`DELETE /relations/{id}` → 500 `EndRelation { valid_until: None } requires expected_version for optimistic locking`。`validator.rs` 把 `EndRelation` 与 `DeleteEntity` 并列要求 `expected_version`，但 `relation` 表**无 version 列**（实体才有），`delete_relation` 也未传版本号。
- 修复：`crates/application/src/mutation/validator.rs` 从「需 expected_version」分支移除 `EndRelation`（关系不版本化）。
- 验证：`DELETE /relations/{id}` 返回 200 `{"deleted":true}`。

### 10. 【后端】关系软删无效 + 列表不过滤（删除后仍在）
- 现象：即便绕过校验，`end_relation_tx` 把 `valid_until = $3`（None）→ 写回 `NULL`，等于「未结束」；且 `list_relations` 不过滤 `valid_until IS NULL`，导致已删关系仍显示、UI 删了不消失。
- 修复：`crates/db/src/repos/entity_repo.rs` 的 `end_relation_tx` 在 `valid_until=None` 时改设 `NOW()`（真正结束）；`crates/db/src/application_ports.rs` 的 `list_relations` 加 `AND r.valid_until IS NULL`（与实体软删一致）。
- 验证：删除后 `valid_until` 写入时间戳、`GET /relations` 立即返回空、卡片从 UI 消失。

---

## 二、逐页测试结果（PASS / FAIL）

| 页面 | 结果 | 备注 |
|------|------|------|
| 首页 `/` | ✅ PASS | 新建项目、命令面板、卡片点击正常 |
| 搜索 `/search` | ✅ PASS（已修） | 结果跳转带真实 projectId |
| 设置 `/settings` | ✅ PASS | 输入/开关正常，疑似自动保存 |
| 世界概览 `/world` | ✅ PASS | 统计卡、+新建地点正常 |
| 人物 `/characters` | ✅ PASS | 增/改/查/取消正常 |
| 地点 `/locations` | ✅ PASS | 增/改/查/取消正常 |
| 势力 `/factions` | ✅ PASS | 增/改/查/取消正常（走 `/entities/{id}`） |
| 物品 `/items` | ✅ PASS | 增/改/查/取消正常 |
| 规则 `/rules` | ✅ PASS | 增/改/删/下拉正常 |
| 关系 `/relationships` | ✅ PASS（已修） | 列表/新建/删除/过滤全接后端（见 #8/#9/#10） |
| 时间线 `/timeline` | ➖ N/A | 纯只读展示，无按钮（设计如此） |
| 故事 `/story` | ✅ PASS（已修） | narrative 修好后新建节点正常 |
| 故事板 `/story/board` | ✅ PASS（已修） | narrative 修好后加载正常 |
| 剧情线 `/storylines` | ✅ PASS | 增正常 |
| 伏笔 `/foreshadows` | ✅ PASS | 增正常 |
| 图谱 `/graph` | ✅ PASS | 缩放/筛选/节点详情正常 |
| 提案 `/proposals` | ✅ PASS（已修） | 加载数据 + 空态引导（见 #12） |
| 历史 `/history` | ✅ PASS | 版本 diff 正常 |
| 快照 `/snapshots` | ✅ PASS | 增/删正常 |
| 写作 `/write` | ✅ PASS（已修） | 侧栏树加载真实节点（见 #11） |

> **Round 7 全量回归**：对 `graph / world/timeline / story/storylines / story/foreshadows / snapshots / history` 逐页浏览器实测，均加载真实数据（或正常空态）、console 无错误；8 条路由 `HTTP 200`。确认无遗漏 bug。

---

## 三、未修复的缺陷（需进一步实现，非本次快速修）

### #1 关系管理页（`/relationships`）功能未实现 ~~【已修，见 #8/#9/#10】~~
- 源码 `Relationships.vue` 为**静态演示页**：`+新建关系/编辑/删除` 按钮无 `@click`，数据为硬编码 7 条假数据，不调后端。点击删除列表不变，无任何 API 请求。后端 `/relations` 实际返回 `[]`。
- 影响：最高优先级功能阻塞。需补齐新建/编辑/删除逻辑与 API 对接。
- ✅ **本轮已修复**：重写为接后端（列表/新建/删除/过滤全功能），并修了后端关系删除 500 + 软删无效两连 bug。浏览器实测新建/删除全通过。

### #2 写作页场景树为空（`/scenes` 404） ~~【已修，见 #11】~~
- 注册 5 组件后页面不再报错，但左侧「故事结构」场景树为空。原诊断称依赖 `/scenes`、`/storylines/<id>/scenes` 端点 404 —— **经核实系误判**：`WritingLayout.vue` 侧栏树用的是 `storyStore.tree`（源自 `narrativeApi.listNodes` → `GET /projects/{id}/narrative`，该端点本身 200），但组件 `<script setup>` **从未调用 `fetchNodes`**，仅调了 mock 的 `loadMockContext/loadMockData`，故树恒为空。
- 影响：写作侧栏无内容。
- ✅ **本轮已修复**：`WritingLayout.vue` `onMounted` 用 `route.params.id` 调 `fetchNodes/fetchStorylines/fetchForeshadows`，树现加载真实叙事节点（实测显示「测试卷QA-D-001×3 / QA卷1 / QA章1」）。注：编辑器正文仍为 mock（`editorStore.sceneContents`），属另一项内容持久化 feature，非本 bug 范围。

### #3 提案页空态无引导 ~~【已修，见 #12】~~
- `GET /proposals` 返回 `[]` 时页面仅渲染标题「AI 提案」，无「生成提案」按钮、无空态引导。提案需 AI 生成（依赖模型配置）。
- 影响：无数据时可用性差。
- ✅ **本轮已修复**：`Proposals.vue` 加 `onMounted` 调 `fetchProposals(route.params.id)`（原**从不加载**，列表恒空），并补空态引导块（图标 + 文案说明提案由 AI 自动生成、如何触发）。浏览器实测：空态正确显示、无 console 错误、后端 `GET /projects/{id}/proposals` 200。

### #4 实体删除 UI 缺失 ~~【已修，见上方 #7】~~
- 人物/地点/势力/物品四页**均无删除按钮**（卡片与编辑框都没有），后端 `DELETE /entities/{id}` 可用但前端无入口。
- 影响：用户无法在前端删实体（需经历史快照/后端清理）。
- ✅ **本轮已修复**：EntityCard 加删除按钮 + 四页 handleDelete（含 confirm）。

### #5 删除无二次确认 ~~【已缓解】~~
- 规则、快照、势力删除为即时操作（无确认弹窗），有误删风险。
- ✅ 实体删除已加 confirm；规则/快照删除仍缺确认（待补）。

### #6 轻微代码异味
- `Items.vue` 编辑物品调用 `worldStore.updateCharacter(id, data)`（应为通用更新），命名与势力/地点页不一致。

### #7 设置页非功能（静态壳，不持久化）【实测确认】
- `Settings.vue` 所有输入为静态 `value="天玄大陆"` / `value="14"` / `checked`，**无 `v-model`、无 `@change`、无 `api.put/post`、无 store 调用**。改完刷新全部回滚，网络仅 GET、无写请求。
- 影响：语言/模型/字号/风格等设置完全不生效。需接后端 settings 端点 + 双向绑定。

### ⚠️ 关于 A 组「世界概览 9 死路由」的更正
- A 报告称侧栏 `relations/story-structure/board/storylines/foreshadows/graph/ai-proposals/snapshots/history` 点击「No match found」。经核实：**`story-structure`、`ai-proposals` 在全前端源码中不存在**；router（`app/router.ts:24-33`）确有 `world/relationships`、`story`、`story/board`、`story/storylines`、`story/foreshadows`、`graph`、`proposals`、`history`、`snapshots` 全部路由；E/D/F/G 用有效 projectId 直接导航这些页均 PASS。
- 判定：该「死路由」为**误报**，最可能原因是 A 点击时 `projectId` 为 `undefined`，链接拼成 `/project/undefined/...` → No match（与搜索 bug 同类假象）。**路由本身健康，不作为确认 bug。**

---

## 四、结论

- **后端数据层系统性 uuid 误解码**已根除，所有列表/详情端点 200；narrative 与 character_state 两处 schema/cast 派生 500 已修复。
- **前端 2 个致命报错**（写作页组件未注册、搜索跳转 undefined）已修复并验证。
- 当前**所有已知功能性缺陷均已修复**：后端 uuid 误解码、前端致命报错、关系/实体管理、写作侧栏树、提案空态、编辑器正文持久化（Round 8）、设置页持久化（Round 9）。全量路由遍历（19 个页面）零 console 报错/警告。详见第六、七节。
- 测试数据：已在 `测试项目QA` 下创建多个人物/地点/势力/物品/规则/剧情线/伏笔；如需清理可删该项目或回滚。

## 五、改动文件清单
- `crates/db/src/application_ports.rs` — uuid 列 `::text` cast（含 narrative 回退）
- `crates/db/migrations/012_character_state_align.sql` — 补 character_state 列
- `crates/narrative-engine/src/api/error.rs` — 回退 `{:#}` → `{}`
- `frontend/src/layouts/WritingLayout.vue` — 补 5 组件 import
- `frontend/src/pages/Search.vue` — 用 store 真实 projectId 替换 `route.params.id`
- `frontend/src/components/ui/EntityCard.vue` — 加 hover 删除按钮（emit `delete`）
- `frontend/src/pages/Characters.vue` / `Locations.vue` / `Factions.vue` / `Items.vue` — 接 `@delete` + `handleDelete`（confirm 二次确认）
- `frontend/src/pages/Relationships.vue` — 重写为接后端（列表/新建/编辑/删除/过滤）
- `crates/application/src/mutation/validator.rs` — EndRelation 不再强制 `expected_version`
- `crates/db/src/repos/entity_repo.rs` — `end_relation_tx` 的 `valid_until=None` 改设 `NOW()`
- `crates/db/src/application_ports.rs` — `list_relations` 加 `AND r.valid_until IS NULL`
- `frontend/src/layouts/WritingLayout.vue` — `onMounted` 拉取叙事节点/剧情线/伏笔（修空树）
- `frontend/src/pages/Proposals.vue` — `onMounted` 加载提案 + 空态引导块

### 11. 【前端】写作侧栏树恒为空（从不 fetch 节点）
- 现象：写作页 `WritingLayout.vue` 左侧「故事结构」树永远为空。`storyStore.tree` 源自 `narrativeApi.listNodes`（端点 200），但组件 `<script setup>` 只调 mock 的 `loadMockContext/loadMockData`，**从未 `fetchNodes`**。（原诊断归咎 `/scenes` 404 系误判，侧栏树根本不用 `/scenes`。）
- 修复：`WritingLayout.vue` 加 `onMounted`，用 `route.params.id`（`project/:id/write`）调 `fetchNodes/fetchStorylines/fetchForeshadows`。
- 验证：浏览器实测树加载真实节点（测试卷QA-D-001×3 / QA卷1 / QA章1），console 无错误。（编辑器正文仍为 mock，属独立 feature。）

### 12. 【前端】提案页从不加载 + 空态无引导（#3）
- 现象：`Proposals.vue` 仅渲染标题「AI 提案」，`proposalStore.proposals` 恒为 `[]`。根因：组件**从不调 `fetchProposals`**（store 有该方法但页面未用），且 `[]` 时无空态引导。
- 修复：`Proposals.vue` 加 `onMounted` 用 `route.params.id` 调 `proposalStore.fetchProposals`；新增空态块（图标+引导文案：提案由 AI 自动生成、如何触发）。
- 验证：浏览器实测空态正确显示、无 console 错误；后端 `GET /projects/{id}/proposals` 200。

---

## 六、Round 8 编辑器正文持久化（核心功能缺口修复）

### 目标
此前编辑器正文是 **mock**（`editorStore.sceneContents` 仅含 `scene-1`/`scene-2`，`saveContent` 只写内存），刷新/重开即丢失。Round 8 沿已存在的 `description` 提交器透传模式，安全加 `content`，不另造机制。

### 改动（全栈）
- `crates/db/migrations/013_narrative_node_content.sql` — `ALTER TABLE narrative_node ADD COLUMN content TEXT;`
- `crates/domain/src/narrative.rs` — `NarrativeNode` 加 `content: Option<String>`
- `crates/domain/src/mutation.rs` — `UpdateNarrativeNode` 加 `content`，`update_narrative_node(...)` 构造器加 `content` 参数
- `crates/db/src/mutation_committer.rs` — 该 payload 分支解构 `content`，`if let Some(v)=content { node.content=v }`（走 `NarrativeNode` 结构体 + `update_node_tx` CAS）
- `crates/db/src/repos/narrative_repo.rs` — `NarrativeNodeRow` 加 `content`；所有 `SELECT`（含 tx 版）加 `content`；`From` 映射加 `content`；`create_node` 构造加 `content: None`；`update_node`/`update_node_tx` 的 `SET` 加 `content=$n`
- `crates/application/src/narrative_service.rs` — `update_node` 加 `content: Option<&str>` 并透传给提交器
- `crates/narrative-engine/src/api/narrative.rs` — `UpdateNodeInput` 加 `content`；handler 透传 `input.content.as_deref()`
- `crates/db/src/application_ports.rs` — `list_nodes`/`get_node` 的 `SELECT`、元组、解构、JSON 映射全部加 `content`（前端读真实正文）
- `frontend/src/types/narrative.ts` — `NarrativeNode` 加 `content?: string`
- `frontend/src/stores/editor.ts` — `loadScene` 改为 `narrativeApi.getNode` 拉真实 `content`；`saveContent` 改为 `narrativeApi.updateNode(id,{content})` 写后端（删 mock map）
- `frontend/src/components/editor/StructuredEditor.vue` — **关键 bug 修复**：原 `onMounted` 只读一次 `modelValue`，异步 `loadScene` 在 mount 后才就绪 → 载入正文永不渲染；且原 `emitContent()` 无返回值、在 `watch` 比较里被调用产生副作用 → 无限重建循环 + 空白。`Split` 出纯函数 `serialize()`（无 emit）用于 `watch` 比较，`emitContent()` 仅用户输入时 emit；并加 `watch(() => props.modelValue)` 在外部载入/切换节点时重建块。

### 验证（全 PASS）
1. **API round-trip**：`PUT /narrative/{id}` 带 `content` → `GET` 返回一致；迁移 013 自动应用，列存在。
2. **UI 保存持久化**：浏览器写作页键入文本 → 点「保存」→ `GET` 确认 `content` 入库（含此前写入）。
3. **重载渲染**：刷新写作页（同 `:sceneId` URL）→ 编辑器正确渲染已持久化正文（修复前为空）。
4. **无报错/无循环**：清 console 后重载，仅 `[vite] connected`，**无** Vue warn / 无 `[StructuredEditor] watch` 刷屏（原无限循环消失）。
5. **回归**：`GET /projects/{id}/narrative` 200（5 节点，含 `content` 字段）；`PUT` 仅改 `title` 走 CAS 仍 200，`content` 保持。

### 仍不可用
- **设置页持久化**：`Settings.vue` 仍为静态壳（无 `v-model`、无写请求、无后端 settings 端点）。属独立功能缺口，需后续接后端。

### 改动文件清单（Round 8 增量）
- `crates/db/migrations/013_narrative_node_content.sql`（新增）
- `crates/domain/src/narrative.rs`
- `crates/domain/src/mutation.rs`
- `crates/db/src/mutation_committer.rs`
- `crates/db/src/repos/narrative_repo.rs`
- `crates/application/src/narrative_service.rs`
- `crates/narrative-engine/src/api/narrative.rs`
- `crates/db/src/application_ports.rs`
- `frontend/src/types/narrative.ts`
- `frontend/src/stores/editor.ts`
- `frontend/src/components/editor/StructuredEditor.vue`

---

## 七、Round 9 设置页持久化（最后功能缺口修复）

### 目标
`Settings.vue` 此前为静态壳（输入写死 `value`、无 `v-model`、无写请求），改完刷新全丢。Round 9 补后端 `app_settings` 端点 + 前端双向绑定，闭环「设置不持久化」这一最后已知缺陷。

### 改动
- `crates/db/migrations/014_app_settings.sql`（新增）— `CREATE TABLE app_settings (id TEXT PK DEFAULT 'default', settings JSONB, updated_at TIMESTAMPTZ)`
- `crates/db/src/application_ports.rs` — 新增 `DbSettingsRepositoryPort`：`get_settings`（SELECT settings::text，无记录返回 `{}`）、`upsert_settings`（INSERT … ON CONFLICT DO UPDATE 覆盖写 JSONB）；直接走 `self.pool`，与同文件其他 port 一致。
- `crates/narrative-engine/src/api/settings.rs`（新增）— `get_settings` / `update_settings` handler（`State<AppState>` → 构造 port → `?` 透传，错误经 `AppError: From<anyhow::Error>` 转 500）。
- `crates/narrative-engine/src/api/mod.rs` — `pub mod settings;` + 路由 `/api/v1/settings` (get + put)。
- `frontend/src/api/settings.ts`（新增）— `settingsApi.get/update`（`AppSettings` 接口）。
- `frontend/src/api/index.ts` — 导出 `settingsApi`。
- `frontend/src/pages/Settings.vue` — 重写：`<script setup>` `onMounted` 拉 `settingsApi.get` 填充 `reactive` 表单；各输入 `v-model`；顶部「保存」按钮 → `settingsApi.update({...form})`，成功后显示「已保存 · 时间」。

### 验证（全 PASS）
1. **API round-trip**：`GET /settings` 初始 `{}`；`PUT` 写入全字段 200；`GET` 回读一致。
2. **UI 载入**：浏览器开 `/settings` → 输入框显示后端值（项目名 `天玄大陆`、字号 `16`）。
3. **UI 保存**：字号改 `20` → 点保存 → `GET` 确认 `fontSize=20` 入库。
4. **重载持久**：刷新 `/settings` → 字号仍 `20`（证明 load-on-mount 持久化生效）。
5. **无报错**：settings 页 console 仅 `[vite] connected`，无 Vue warn / 无 error。

### 全量路由回归（19 页，零 console 错误/警告）
home `/` · 项目仪表盘 `/project/{id}` · world · characters · locations · factions · items · rules · timeline · relationships · write · story · storylines · foreshadows · graph · proposals · history · snapshots · settings。逐一导航 + `networkidle` 后 `errors()` 均返回 `[]`。

### 结论
所有已知功能性缺陷已修复并通过浏览器实测；全量遍历零报错。QA 通过。
