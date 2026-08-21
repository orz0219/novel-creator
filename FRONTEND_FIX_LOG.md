# 前端修复日志 (Frontend Fix Log)

依据 `FRONTEND_AUDIT_REPORT.md`。本轮（Round 2）修复所有「纯前端写死 mock / 静态假数据」问题。
原则：前端只做绑定真实数据的修复；需要后端新增端点才能完成的项列入「待办（依赖后端）」。

## 已修复（纯前端，已浏览器验证）

### Tier A — 8 个写死 mock 的页面/组件
| 文件 | 修复 | 验证 |
|---|---|---|
| `components/version/EventLog.vue` | 真实 `historyApi.getEvents(projectId)`，加载/空态 | ✅ 测试项目显示「0 条记录 / 暂无事件」 |
| `components/version/VersionDiff.vue` | 真实实体 + `getVersions`/`compareVersions` 字段级 diff | ✅ 版本选择器填充真实实体（剑冢/林惊羽/演武场/青云山/青云门/魔教…） |
| `pages/ProjectDashboard.vue` | 真实 project/world/story 数据，计数卡片改 `router-link` | ✅ 显示「测试项目-玄幻 / 用于验证 LLM 抽取链路 / 构思中 / 6人物 4地点…」 |
| `pages/Graph.vue` | 真实 worldStore 实体 + 关系边，`console.log` 缩放改为真实 `scale` transform | ✅ 渲染真实节点与边（StudentOf/Friend/LocatedAt/ConflictWith） |
| `pages/Search.vue` | 客户端检索 `worldStore` 实体（`/search` 无 `:id`，改为从 projectStore 兜底取项目） | ✅ 列出真实实体（剑冢/山门初遇/林惊羽/青云门/魔教…） |
| `pages/StoryBoard.vue` | 真实 `storyStore.nodes` 按状态分列 | ✅ 测试项目显示「暂无节点」（真实空） |
| `components/constraint/ConstraintPanel.vue` | 真实 `rulesApi` 列表/增/删/改 | ✅ 显示「暂无约束规则」（真实空）+「+ 添加规则」 |
| `components/knowledge/KnowledgePanel.vue` | 真实 `characterApi.getKnowledge` 聚合 | ✅ 显示「知识状态共 0 条知识记录 / 暂无知识记录」 |
| `components/editor/StructuredEditor.vue` | 实体高亮列表改由 worldStore 驱动，`console.log` 改 `emit('entity-click')` | ✅ 类型检查通过 |

### Tier A+ — 实体档案补全
| 文件 | 修复 | 验证 |
|---|---|---|
| `pages/Characters.vue` | 编辑时调用 `getProfile`/`getState` 展示真实档案（只读面板，后端暂无 PUT） | ✅ 类型检查通过 |

### 额外发现并修复的第 9 个 mock — 写作上下文面板
`WritingLayout.vue` 的「上下文」面板此前用 `loadMockContext()` 写死 `林凡/苏晚晴/王家/黑石城/古井/天玄历381年` 等假数据。
- `stores/context.ts`：`loadMockContext` 删除，新增 `loadContext(sceneId)` 调用 `contextApi.getSceneContext`；无场景时 `reset()`。
- `WritingLayout.vue`：`watch(editorStore.currentSceneId)` 选场景即加载真实上下文；删除写死的「故事状态（第一卷：黑石城/王家追杀/…）」与「选择原因（林凡…）」，改为由 `storyStore.nodes` 派生（场景总数/已完成/节点数）与 `contextStore.entities[].reasons` 派生。
- 修复类型检查发现的 `VersionDiff.vue` `entity_type`→`entity_type_id` 错误。
- 验证：写作页上下文显示「0 tokens / 场景总数0 / 已完成场景0 / 叙事节点0 / 暂无选择原因」，无任何写死假数据。

类型检查：`npm run type-check` 全部通过（EXIT=0）。

## Round 3–4 修复（Tier B：前端可修复的字段/按钮缺失）
全部通过 `npm run type-check`（EXIT=0），并在测试项目浏览器验证真实数据：
- **Timeline** (`pages/Timeline.vue`)：新增 `event_type` 徽章、`event_time`（`toLocaleString` 格式化）、`duration`、`涉及 N 个实体`，可选字段 `v-if` 守卫。
- **Snapshots** (`pages/Snapshots.vue`)：完整展示 `name/story_time/current_location/world_summary` + 六项计数/进度，仅保留可工作的「删除」（无 restore 端点，未添加）。
- **Proposals** (`pages/Proposals.vue`)：展示真实 payload（`changes`：变更类型/目标/描述/风险/state_change）+ 真实「运行校验」按钮（`validationApi.validateProposal`）渲染 `validation_results`；accept/reject 接 `proposalApi`。修复了类型导入（`@/types`）。
- **Story** (`pages/Story.vue`)：每个叙事节点（卷/弧/章/场景）加「编辑 / 删除」按钮，编辑弹窗改 `title/description/content/status`，删除走 `storyStore.deleteNode`（端点存在）。测试项目无节点→显示「暂无故事结构」真实空态。
- **Foreshadows** (`pages/Foreshadows.vue`)：展示 `status/importance/hint_level/planted_scene_id/revealed_scene_id/关联实体数`，加编辑（`foreshadowApi.update`）与删除（`foreshadowApi.delete`，后端 Round 5 新增 `DELETE /foreshadows/{id}`）。
- **Storylines** (`pages/Storylines.vue`)：展示 `status/importance/created_volume_id/resolved_volume_id`，加编辑（`storylineApi.update`）与删除（`storylineApi.delete`，后端 Round 5 新增 `DELETE /storylines/{id}`）。
- **Home** (`pages/Home.vue`)：展示项目真实字段 + 「编辑 / 删除」（`projectStore.updateProject/deleteProject`）。编辑表单去除后端不支持的 `language/world_setting/system_setting` 字段避免无效提交。
- **World** (`pages/World.vue`)：完整展示 `World` 字段（`world_rules`/`is_main`/`config`）+ 「编辑」（`worldApi.update`）。
- **AI 生成面板**（`stores/generation.ts` + `WritingLayout.vue` + `composables/useGeneration.ts`）：删除 `loadMockData()` 与 `simulateProgress()` 假数据/假进度；改为 `generationApi.list` 拉取真实任务历史、`generationApi.start` 启动 + 轮询 `get` 真实状态直至终态。验证：`chapter_draft` 真实任务 `已完成/等待中` 正常展示，无 `gen-1/2/3` 假数据。

## Round 5 修复（后端 + 前端：Foreshadows / Storylines 删除端点缺口）

审计遗留的最后一个真实后端缺口已补齐，并浏览器端到端验证通过。

### 后端（Rust，crates/）
- `domain/src/ports.rs`：`StorylineRepositoryPort` / `ForeshadowRepositoryPort` 新增 `delete_storyline(id)` / `delete_foreshadow(id)`。
- `db/src/application_ports.rs`：两个 repository 实现新增 `DELETE FROM storyline/foreshadowing WHERE id=$1`。
- `application/src/storyline_service.rs` / `foreshadow_service.rs`：新增 `delete_storyline` / `delete_foreshadow`。
- `narrative-engine/src/api/narrative.rs`：新增 `delete_storyline` / `delete_foreshadow` 处理器（返回 `{"deleted":true,"id":...}`）。
- `narrative-engine/src/api/mod.rs`：为 `/api/v1/storylines/{id}` 与 `/api/v1/foreshadows/{id}` 追加 `.delete(...)` 路由。
- `cargo check` 通过（CARGO_EXIT=0）。

### 前端（Vue）
- `api/story.ts`：`storylineApi` / `foreshadowApi` 新增 `delete`。
- `stores/story.ts`：新增 `deleteStoryline` / `deleteForeshadow`（调用 API 后本地过滤）。
- `pages/Foreshadows.vue` / `pages/Storylines.vue`：每条卡片新增「删除」按钮（带 `confirm` 二次确认 + `.btn-danger` 样式），接 `store.deleteXxx`。
- `npm run type-check` 通过（EXIT=0）。

### 浏览器验证（测试项目 430cbfa0-…）
- 通过 API 创建测试剧情线/伏笔 → 页面渲染出「删除」按钮 → 点击 → 确认对话框 → 接受后卡片消失，侧栏计数由「剧情线 1 / 伏笔 1」降为「0」。端到端通过。

## 仍需做的事（依赖后端端点 / 可选增强）
- **Snapshots 恢复**：后端仅有 list/create/delete，无 restore 端点（需新增后端能力）。已与用户确认此项暂留。
- **AI 可追溯增强**（可选）：Proposals 已展示 `validation_results`、AI 面板已展示 generation 任务历史，可作为可追溯视图；跨页面联动（proposal→generation_task）未做。
- **地点/势力编辑（更正）**：此前列为待办属误报——`Locations.vue`/`Factions.vue` 已通过通用 `entityApi.update/delete`（`@/api/world.ts`）完整支持编辑/删除，无需新增端点。

> 注：测试项目 `430cbfa0-0cc0-48ae-8d0e-e9a30d58b7e3`（6 人物 / 4 地点 / 0 势力 / 0 节点），用于浏览器验证。

## Round 6 修复（实体字段完整性 + 创建/档案后端 bug）

用户确认：所有实体（人物 / 地点 / 势力 / 物品）的设计字段是否在 UI 中完整展示。结论：四类实体的字段设计均已完整覆盖，本轮修复了阻止它们实际生效的两个后端 bug 与两个前端 bug。

### 字段覆盖确认（四类实体）
- **人物 Character**：10 个档案字段（真名/别名/年龄/性别/身份/外貌/背景/社会地位/核心性格/价值观）+ 6 个状态字段（编辑基础信息走 EntityDialog）。`Characters.vue` 内联可编辑详情面板，已浏览器验证（林惊羽真实档案展示）。
- **地点 Location**：12 个设计字段（类型/规模/气候/地形/资源/居民/文化/历史/特殊/危险等级/连通性/控制者等）。`Locations.vue` 内联详情面板，已浏览器验证（剑冢真实档案展示）。
- **势力 Faction**：11 个设计字段（目标/领袖/信条/资源/领地/成员/敌对/盟友/内部冲突/秘密/行事风格）。`Factions.vue` 内联可编辑详情面板。
- **物品 Item**：`attributes` 自由 JSON 设计字段（材质/品级/特效…），`Items.vue` 单 NeDialog 编辑器。

### 后端（Rust，crates/）
- `domain/src/mutation.rs` `MutationCommand::create_entity`：第 2 个参数原本传入 `world_id` 作为 target，导致 committer 用 world_id 当实体主键 → 主键重复 `23505`。改为 `Uuid::nil()`（用户创建走「无目标自动生成 id」分支）。**此 bug 影响所有经 API 创建的实体**（人物/地点此前是直接 seed，未触发）。`cargo run` 重启验证：`POST /worlds/{id}/factions|entities` 现正常返回实体 id。
- `db/src/application_ports.rs` `get_faction_profile`：`query_as` 元组声明了 12 个 `Option<String>`，但 `SELECT` 仅返回 11 列（`ColumnIndexOutOfBounds { index: 11, len: 11 }`）→ 势力档案 GET 500。删除多余的元组元素。重启后 `GET /factions/{id}/profile` 返回全部 11 字段。

### 前端（Vue）
- `pages/Items.vue`：
  - 新增 `watch(() => worldStore.currentWorld?.id, …)` 加载物品。原 `onMounted(loadItems)` 在深链直达 `/world/items` 时早于 `ProjectLayout` 解析出 `currentWorld`，导致物品网格为空。改为 world 就绪后再加载（兼顾直链与页内导航）。
  - 修复属性保存：编辑提交原分两步（先 `updateCharacter` 再 `entityApi.update({attributes})`），而 `PUT /entities/{id}` 复用 `CreateEntityInput` 要求 `name`，只传 `{attributes}` 触发 `422 缺 name` 且被 `.catch` 吞掉 → 属性永不入库。改为一次 `entityApi.update({name, summary, description, attributes})` 全量提交，并让错误抛出到外层 `catch`（保存失败弹窗留驻）。
- `pages/Characters.vue` / `Locations.vue` / `Factions.vue`：经浏览器验证设计字段均正常展示（本轮仅后端修复使其生效，前端逻辑未改）。

### 浏览器验证（测试项目 430cbfa0-…）
- 势力：新建「天剑宗」→ 详情面板展示 11 字段（匡扶正义/道玄真人/侠义/灵脉/中州/三千弟子/魔教/散修/派系之争/上古秘宝/以剑证道），保存回读正常。
- 物品：新建「青锋剑」→ 深链直达正常出现卡片 → 打开对话框编辑 `attributes={"材质":"玄铁","品级":"上品","特效":"破甲"}` → 保存 → 重开确认属性持久化（v2）。
- 验证后清理测试数据，项目恢复 6 人物 / 4 地点 / 0 势力 / 0 物品 原始状态。
- `npm run type-check` 通过（EXIT=0）。
