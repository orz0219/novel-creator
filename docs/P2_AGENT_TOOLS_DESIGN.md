# P2：智能体真实领域工具接入设计（Agent Real Domain Tools）

> 状态：设计稿（待评审确认后实现）
> 关联代码：`crates/agent/*`、`crates/application/*`、`crates/domain/src/mutation.rs`、`crates/narrative-engine/src/main.rs`

---

## 1. 背景与问题

当前 P1 的智能体只暴露 3 个示例工具（`echo` / `create_character_draft` / `ask_question`）：

- `create_character_draft` 的 `execute` **只返回草稿 JSON，不落库**（注释明说"落库在 P2"）。
- `ask_question` 仅生成选择题标记，由前端渲染。
- `crate/agent/src/lib.rs` 与 `runtime.rs` 都注明："真实 create_* 工具在 P2 接入"。

因此现状是：**提问 / 回答停留在对话层，没有真正落到 World / Character / Narrative 等实体**。用户的核心疑问——"工具只有 create 吗？智能体能否按我的要求修改或删除（逻辑删除）产物？"——答案分两层：

1. **Agent 工具层**：确实只有 create 的雏形，且未落库。
2. **application / domain 层**：**CRUD 早已齐备，且 D 是逻辑删除 / 语义化软删除**。

所以 P2 不是"新增 CRUD 能力"，而是**把已有的 application service 方法包成领域级工具，暴露给智能体**。

---

## 2. 核心结论

- **必须覆盖 C / U / D + R**。其中 D 一律走逻辑删除 / 语义化软删除，**绝不物理 `DELETE`**。
- 底层能力已就绪（见下表），P2 工作量为"接线 + 工具语义封装 + 提示词引导"，非"从零造能力"。
- 设计张力：`tool.rs` 注释"工具是领域级 Action，不是 CRUD"——意思是**工具名应是领域动作**（如 `revise_character`、`retire_character`），但**底层必须能 update 和逻辑 delete**，否则智能体无法按用户要求修改 / 删除产物。用户的直觉正确。

### 2.1 底层已具备的 CRUD 能力（application service 实测）

| 聚合 | Create | Update | Delete（逻辑 / 语义） |
|---|---|---|---|
| Entity | `EntityService::create_entity` | `update_entity`（带乐观锁版本号） | `delete_entity` → **语义化软删除，绝不物理 DELETE** |
| Relation | `create_relation` | — | `delete_relation` → `end_relation`（语义化结束，带 `valid_until`） |
| Narrative Node | `NarrativeService::create_node` | `update_node` | `delete_node` |
| Storyline | `create_storyline` | `update_storyline` | `delete_storyline` |
| Foreshadow | `create_foreshadow` | `update_foreshadow` | `delete_foreshadow` |
| Rule | `create_rule` | `update_rule` | `delete_rule` |
| Snapshot | `create_snapshot` | — | `delete_snapshot` |
| World | `get_or_create_main_world` / `create_world` | `update_main_world` | — |
| Project | `create_project` | `update_project` | `delete_project` |
| History Event / Fact | `create_event` / `create_fact` | — | —（append-only，历史不可篡改） |

对应 `domain::mutation::MutationCommand` 已有：`update_entity / delete_entity / end_relation / update_narrative_node / delete_narrative_node / update_storyline / update_foreshadow`，均经 `MutationCommitter` 提交并推进 `world_version`。

---

## 3. 设计原则

1. **依赖倒置（必须遵守现有约定）**
   - `crates/agent` 不直接依赖 `application` / `infrastructure`（见 `agent/src/lib.rs` 注释）。
   - **真实领域工具在组合根 `narrative-engine/src/main.rs` 构造**（此处已有 `pool` 与各 application service 的构造），通过 `agent_tools.register(Arc::new(...))` 注入 `ToolRegistry`。
   - `agent` crate 只保留 `AgentTool` trait、`ToolRegistry`、`types`、`runtime` 编排。

2. **领域动作命名，而非裸 CRUD**
   - 推荐：`create_character` / `revise_character` / `retire_character`；`create_node` / `revise_node` / `remove_node` 等。
   - 工具 `description` 要明确"这会**修改 / 逻辑删除**已有产物"，以便 LLM 在合适时机调用。

3. **逻辑删除优先**
   - 所有"删除"类工具映射到 `delete_*` / `end_relation`，**禁止物理删除**。
   - 历史 Event / Fact 不提供删除工具（语义上为不可篡改记录）。

4. **统一走 Canon 写路径**
   - 写操作经 `MutationCommitter`（或对应 service 的 commit 封装），不在工具里直接操作 repo。
   - 实体 / 关系等影响 Canon 的写，显式声明 `affected_worlds`（参考 `EntityService::create_entity` 的 `commit_with_worlds`），在同一事务内推进 `world_version`。

5. **乐观锁**
   - `update_entity` / `delete_entity` 必须携带 `expected_version`（取自 `get_entity` 返回的 `version`）。冲突时返回明确错误，由智能体决定重试或提示用户。

6. **先读后改 / 先读后删**
   - 工具层提供 `get_entity` / `list_*` 等读工具，提示词引导智能体"修改 / 删除前先读取目标，确认 id 与当前版本"。

---

## 4. 工具清单（P2 建议全集）

> 前缀按领域动作；`落库` 列标注是否经 Committer 真正写入。

### 4.1 Entity / Character（Phase A 先行）

| 工具名 | 对应 service 方法 | 语义 | 落库 |
|---|---|---|---|
| `create_character` | `create_entity(world_id, "Character", …)` | 创建角色 | ✅ |
| `create_location` | `create_entity(…, "Location", …)` | 创建地点 | ✅ |
| `create_faction` | `create_entity(…, "Faction", …)` | 创建势力 | ✅ |
| `revise_entity` | `update_entity(id, name?, summary?, description?, attributes?, expected_version)` | 修改实体字段 | ✅ |
| `retire_entity` | `delete_entity(id, expected_version)` | **逻辑删除**实体 | ✅（软删除） |
| `create_relation` | `create_relation(src, tgt, type, desc)` | 创建关系 | ✅ |
| `end_relation` | `delete_relation(id)` | **语义化结束**关系 | ✅ |
| `get_entity` | `get_entity(id)` | 读取单一实体（含 version） | ❌ 读 |
| `list_entities` | `list_entities(world_id, type?)` | 列出某世界实体 | ❌ 读 |

### 4.2 Narrative / Storyline / Foreshadow / Rule（Phase B）

| 工具名 | 对应 service 方法 | 语义 | 落库 |
|---|---|---|---|
| `create_node` | `NarrativeService::create_node` | 创建叙事节点（卷/弧/章/场/节拍） | ✅ |
| `revise_node` | `update_node` | 修改叙事节点 | ✅ |
| `remove_node` | `delete_node` | **逻辑删除**叙事节点 | ✅ |
| `create_storyline` | `StorylineService::create_storyline` | 创建故事线 | ✅ |
| `revise_storyline` | `update_storyline` | 修改故事线 | ✅ |
| `retire_storyline` | `delete_storyline` | 删除故事线 | ✅ |
| `create_foreshadow` | `ForeshadowService::create_foreshadow` | 创建伏笔 | ✅ |
| `revise_foreshadow` | `update_foreshadow` | 修改伏笔 | ✅ |
| `retire_foreshadow` | `delete_foreshadow` | 删除伏笔 | ✅ |
| `create_rule` | `RuleService::create_rule` | 创建世界规则 | ✅ |
| `revise_rule` | `update_rule` | 修改规则 | ✅ |
| `retire_rule` | `delete_rule` | 删除规则 | ✅ |
| `list_nodes` / `list_storylines` / `list_foreshadows` / `list_rules` | 各 `list_*` | 列举供上下文检索 | ❌ 读 |

### 4.3 World / Project / Snapshot / History（Phase B / C）

| 工具名 | 对应 service 方法 | 语义 | 落库 |
|---|---|---|---|
| `update_main_world` | `WorldService::update_main_world` | 修改主世界设定 | ✅ |
| `create_fact` | `HistoryService::create_fact` | 添加事实（Canon） | ✅ |
| `create_event` | `HistoryService::create_event` | 添加历史事件 | ✅ |
| `create_snapshot` | `SnapshotService::create_snapshot` | 创建快照 | ✅ |
| `delete_snapshot` | `delete_snapshot` | 删除快照 | ✅ |
| `update_project` | `ProjectService::update_project` | 修改项目元信息 | ✅ |
| `get_world` / `list_facts` / `list_events` / `list_snapshots` | 各 `get_*`/`list_*` | 读 | ❌ 读 |

> 历史 Event / Fact **不提供修改 / 删除工具**：历史不可篡改，如需修正应追加新事实而非改写。

---

## 5. 工具执行链路

```
LLM 决定调用工具
   → POST /api/agent/tool/execute { name, input }
   → AgentRuntime::execute_tool
       1. ToolRegistry::get(name)            // 找不到 → 明确报错
       2. 输入须为 JSON 对象（P1 已有校验）
       3. 【P2 新增】完整 JSON Schema 校验（tool.input_schema）
       4. tool.execute(input)
            ├─ 解析参数
            ├─ 调用 application service（已在组合根注入 service Arc）
            ├─ service 内部经 MutationCommitter 提交
            └─ 返回结构化结果 { ok, id?, version?, status? }
   → 返回 ExecuteToolResponse { name, result }
```

- 工具 `execute` 返回 `serde_json::Value`，建议统一结构：
  ```json
  { "ok": true, "id": "<uuid>", "version": 3, "action": "retire_entity" }
  ```
- 失败时返回明确错误（包含乐观锁冲突 / 找不到实体等），**不静默吞掉**。

---

## 6. 逻辑删除 / 语义删除约定

| 聚合 | 删除语义 | 实现途径 |
|---|---|---|
| Entity | **语义化软删除**（绝不物理 DELETE） | `MutationCommand::delete_entity` → `MutationCommitter` |
| Relation | **语义化结束**（带 `valid_until`，保留历史边） | `MutationCommand::end_relation` |
| Narrative Node / Storyline / Foreshadow / Rule / Snapshot / Project | 各自 `delete_*`（落库层语义化，非物理 删） | 对应 service `delete_*` |
| History Event / Fact | 不可删除 | 无工具 |

> 注：当前 `delete_entity` 注释已声明"绝不物理 DELETE"；需确认仓库层 `entity_repo` 实际是用 `status`/`deleted_at` 标记而非 `DELETE FROM`。若实现不符，P2 一并修正。

---

## 7. 工具注册方式（组合根改造）

在 `crates/narrative-engine/src/main.rs` 中（当前 `agent_tools.register(EchoTool/CharacterDraftTool/AskQuestionTool)` 处）改为：

```rust
let agent_tools = Arc::new(ToolRegistry::new());
// 保留：对话协议与调试
agent_tools.register(Arc::new(EchoTool));
agent_tools.register(Arc::new(AskQuestionTool));
// P2 真实领域工具（构造时注入对应 application service Arc）
agent_tools.register(Arc::new(EntityTools::new(entity_service.clone())));
agent_tools.register(Arc::new(NarrativeTools::new(narrative_service.clone())));
agent_tools.register(Arc::new(StorylineTools::new(storyline_service.clone())));
// … 其余聚合
```

- **`create_character_draft` 去留**：建议收敛。两种方案——
  - 方案 A（推荐）：直接删除 `create_character_draft`，由真实 `create_character` 落库，提示词引导"创建后请向用户确认"。
  - 方案 B：保留草稿工具作为"先产出草稿、用户确认后再调真实 create"的两段式流程（需前端配合）。

---

## 8. Prompt 拼接与工具发现

- `agent/src/prompt.rs::build_system_prompt` 已自动把 `tools` 列表（name + description + input_schema）追加到系统提示词。**新增真实工具后无需改这里**，但 `description` 要写清楚"会修改 / 逻辑删除已有产物"。
- P2 在基座提示词（`DEFAULT_SYSTEM_PROMPT_BASE`）补充一条引导：
  > 修改或删除已有产物前，先用读工具确认目标 id 与当前版本；删除为逻辑删除，会保留历史。

---

## 9. 错误处理与校验

- **完整 JSON Schema 校验在 P2**（`tool.rs` 注释已标注）。`execute_tool` 在调用 `tool.execute` 前用 `input_schema` 校验入参，失败返回明确错误。
- 乐观锁冲突：`update/delete` 返回版本不符时，工具返回 `{ "ok": false, "error": "version_conflict", "current_version": N }`，由智能体重试或提示用户。
- 不静默吞异常；每个错误都带可定位信息。

---

## 10. 分阶段实施计划

- **Phase A（最小可用闭环，建议先做）**
  - Entity 全套：`create_character` / `create_location` / `create_faction` / `revise_entity` / `retire_entity` / `create_relation` / `end_relation` + 读工具 `get_entity` / `list_entities`。
  - 组合根接 `EntityService`，`execute_tool` 增加 Schema 校验。
  - 收敛 `create_character_draft`（采用方案 A 或 B）。
  - 测试：`agent → EntityService → MutationCommitter → DB`，验证落库与逻辑删除非物理删。

- **Phase B（其余聚合）**
  - Narrative / Storyline / Foreshadow / Rule / Snapshot / Project 的 C/U/D + 读工具。
  - World / History 的创建与修改工具。

- **Phase C（提示词与协议收敛）**
  - 基座提示词增加"先读后改 / 先读后删"引导。
  - 确认仓库层逻辑删除实现符合约定；补 `status`/`deleted_at` 字段与查询过滤。

---

## 11. 测试策略

- **单元测试**：每个工具 `execute` 的输入解析与 schema、错误分支（找不到 / 版本冲突）。
- **集成测试**：`agent_tests` 或新建 `tool_integration_tests`，覆盖 `register → execute_tool → service → db`，断言：
  - 创建后实体存在且 `version` 起始合理；
  - `revise_entity` 后字段更新、`version+1`；
  - `retire_entity` 后实体**仍在表中**（逻辑删除），查询结果反映已删除状态；
  - `end_relation` 后关系保留但语义结束。
- **契约测试**：参照 `crates/application/tests/commit_contract_tests.rs` 风格，验证写路径经 `MutationCommitter`。

---

## 12. 风险与开放议题

- `delete_entity` 在仓库层的**真实实现**是否软删除需核实（见 §6 注）；若为物理删，P2 必须改。
- `create_character_draft` 收敛方案（A/B）需与前端确认是否要保留"草稿确认"交互。
- 乐观锁冲突时智能体的重试 / 提示策略需在提示词中明确，避免死循环。
- 历史 Event / Fact 不可删的语义是否要在工具层显式拒绝（返回明确错误）。
- **narrative_node 软删读取一致性（已修复）**：原先 `narrative_repo` 的 `get_node_by_id` / `get_node_by_id_with_project` / `list_nodes_by_project` / `list_children` / `get_node_by_id_with_project_tx` 以及 `application_ports` 的 `get_node` 均不带 `status` 过滤，而 committer 的 `DeleteNarrativeNode`（→ `soft_delete_node_tx`）走软删 `UPDATE ... SET status='Deleted'`（递归含子节点）。导致 `remove_node` 后 `get_node` 仍返回已删节点，与 Entity 聚合（`get_entity` 过滤 `status != 'Deleted'`）行为不一致。现已统一：所有"按 id 读取"与"列出节点"查询追加 `AND status != 'Deleted'`（`list_nodes` 此前已过滤）；事务内校验 `get_node_by_id_with_project_tx` 一并过滤，已软删节点不可再被引用。集成测试相应改回断言 `remove_node` 后 `get_node` 失败，并保留查库确认 `status='Deleted'`（证明软删而非物理删）；`db` crate 全部测试通过，无回归。

---

## 13. 实施记录（Phase A，已落地）

Phase A 已按本设计执行，覆盖 Entity 聚合的 C/U/D+R，并把工具从组合根注入 `ToolRegistry`。

### 13.1 变更文件
- **新增** `crates/narrative-engine/src/agent_tools.rs`：定义 `EntityTool`（`EntityAction` 枚举分派）+ `register_entity_tools`，实现 `agent::AgentTool`，直接调用 `EntityService`。
- **新增** `crates/narrative-engine/tests/entity_tools_integration.rs`：DB 集成测试，验证 create→revise→get→retire（逻辑删除）+ create_relation→end_relation（语义结束），并直接查库确认非物理 DELETE。
- `crates/narrative-engine/src/main.rs`：构造 `EntityService`，调用 `register_entity_tools` 注册；移除 `CharacterDraftTool` 注册。
- `crates/narrative-engine/Cargo.toml`：新增 `async-trait` 依赖。
- `crates/narrative-engine/src/lib.rs`：暴露 `pub mod agent_tools;`（供二进制与集成测试共用）。
- `crates/agent/src/tool.rs`：删除 `CharacterDraftTool`（收敛草稿工具）。
- `crates/agent/src/runtime.rs`：移除 `CharacterDraftTool` 引用；`execute_tool` 增加 JSON Schema 轻量校验（required + type）。
- `crates/agent/src/lib.rs`、`crates/agent/tests/agent_tests.rs`：同步清理 `CharacterDraftTool` 引用，新增 Schema 校验单测。
- `crates/agent/src/prompt.rs`：基座提示词增加「先读后改/删、删除为逻辑删除」引导。

### 13.2 接入的工具

**Entity 聚合（Phase A）**
`create_character` / `create_location` / `create_faction` / `create_item` / `revise_entity` / `retire_entity`（逻辑删除）/ `create_relation` / `end_relation`（语义结束）/ `get_entity` / `list_entities`。

**Phase B 新增聚合**
- Narrative：`create_node` / `revise_node` / `remove_node`（软删除）/ `get_node` / `list_nodes`
- Storyline：`create_storyline` / `revise_storyline` / `retire_storyline` / `list_storylines`
- Foreshadow：`create_foreshadow` / `revise_foreshadow` / `retire_foreshadow` / `list_foreshadows`
- Rule：`create_rule` / `revise_rule` / `retire_rule` / `get_rule` / `list_rules`
- Snapshot：`create_snapshot` / `delete_snapshot` / `list_snapshots`
- Project：`create_project` / `update_project` / `delete_project` / `get_project` / `list_projects`
- World：`get_main_world` / `update_main_world`
- History（仅创建 / 读取，历史不可篡改）：`create_event` / `create_fact` / `list_events` / `list_facts`

> 统一注册入口 `register_all_domain_tools(registry, &pool)` 在组合根构造所有聚合的 service
> （repo + committer + resolver，与 `api/*` handler 的 `service()` 同构）并一次性注册全部工具；
> `main.rs` 仅一行调用即可。

### 13.3 变更文件（Phase B 增量）
- `crates/narrative-engine/src/agent_tools.rs`：在 EntityTool 基础上新增 NarrativeTool / StorylineTool / ForeshadowTool / RuleTool / SnapshotTool / ProjectTool / WorldTool / HistoryTool 八种聚合工具（枚举分派 + 各自 `register_*_tools`），并新增统一入口 `register_all_domain_tools`。
- `crates/narrative-engine/src/main.rs`：`agent_tools` 注册改为调用 `register_all_domain_tools(&agent_tools, &pool)`，移除原先手动构造 `EntityService` 的代码与多余 import。
- `crates/narrative-engine/tests/phase_b_tools_integration.rs`：DB 集成测试，覆盖 Project/Narrative/Storyline/Rule/World/History 的创建、修改、读取与逻辑删除（remove_node 后 `get_node` 失败 + 查库确认 `status='Deleted'`）。

### 13.4 软删读取不一致修复（narrative_node）
- **问题**：`narrative_node` 删除走 committer 软删（`UPDATE ... SET status='Deleted'`），但其全部读取方法（`get_node_by_id` / `get_node_by_id_with_project` / `list_nodes_by_project` / `list_children` / `get_node_by_id_with_project_tx` 及 port 的 `get_node`）均不过滤 `status`，导致 `get_node` 在软删后仍返回已删节点，与 Entity 聚合（`get_entity` 过滤 `status != 'Deleted'`）行为不一致。
- **修复**：`crates/db/src/repos/narrative_repo.rs` 与 `crates/db/src/application_ports.rs` 中所有"按 id 读取"与"列出节点"查询统一追加 `AND status != 'Deleted'`（`list_nodes` 此前已过滤）；事务内校验 `get_node_by_id_with_project_tx` 一并过滤，使已软删节点不可被引用。
- **验证**：`phase_b_tools_integration` 中 `remove_node` 后 `get_node` 现应失败（断言已改回），并查库确认 `status='Deleted'`；`db` crate 全部测试通过，无回归。

### 13.3 验证状态
- `cargo build -p narrative-engine`：通过（仅既有 warning）。
- `cargo test -p agent`：8 项单测全过（含新增的 Schema 校验测试）。
- `cargo test -p narrative-engine --no-run`：集成测试编译通过。
- **逻辑删除已核实（代码级）**：`entity_repo::delete_tx` 为 `UPDATE entity SET status='Deleted'`（乐观锁），属软删除；`end_relation` 经 `MutationCommitter` 调 `RelationRepo::end_relation_tx` 设 `valid_until`，非物理删（注释明确"业务层禁止物理 DELETE"）。
- 本地 DB 经 colima + docker 启动 `novel-postgres` 后，集成测试已实际运行通过
  （`test entity_tools_create_revise_retire_logical_delete ... ok`），完整覆盖
  create→revise→get→retire（逻辑删除）+ create_relation→end_relation（语义结束），并查库确认非物理删；启动与运行步骤见 §14。
- Phase B 集成测试也已实跑通过（`test phase_b_tools_crud_logical_delete ... ok`），覆盖 Project / Narrative（create→revise→remove 软删）/ Storyline / Rule（经 get_main_world 取主世界）/ World / History 的创建、修改、读取与逻辑删除；两个集成测试同时运行均无回归。

---

## 14. 本地数据库启动与集成测试运行

本机开发数据库跑在 **colima + docker** 的 `novel-postgres` 容器中（镜像 `postgres:16-alpine`，
数据持久化在 docker volume `pgdata`，容器删除数据不丢）。`.env` 中 `DATABASE_URL` 已指向
`postgresql://novel:novel_pass@localhost:5432/novel_engine`。

### 14.1 启动步骤
```bash
# 1) 启动 colima（docker daemon 的 VM；default profile 已存在，镜像已缓存，约 10s）
colima start

# 2) 启动 Postgres 容器（容器已创建过，直接 start 即可；首次用 docker compose up -d）
docker start novel-postgres

# 3) 等待就绪（pg_isready 在容器内）
for i in $(seq 1 40); do
  docker exec novel-postgres pg_isready -U novel -d novel_engine >/dev/null 2>&1 && break
  sleep 2
done
```

> 注：本机未安装 `docker compose` 插件 / `docker-compose` standalone，故用 `docker start` 直接拉起已有容器；
> 若容器不存在，再补 `docker run -d --name novel-postgres -e POSTGRES_USER=novel -e POSTGRES_PASSWORD=novel_pass -e POSTGRES_DB=novel_engine -p 5432:5432 postgres:16-alpine`。

### 14.2 前置：schema 与 seed
- **Schema（migrations）**：由 `narrative-engine` 服务首次启动时 `run_migrations` 自动应用；
  也可直接启动一次服务完成迁移。
- **Seed（entity_type 等）**：`main.rs` 启动时向 `entity_type` 写入
  `Character / Location / Faction / Item / Creature / Organization`（幂等 `ON CONFLICT DO NOTHING`）。
  若 `entity_type` 为空，先启动一次服务 seed，否则 `create_character` 等会因外键缺失失败。
- 验证：`docker exec novel-postgres psql -U novel -d novel_engine -tc "SELECT name FROM entity_type;"`。

### 14.3 运行集成测试
```bash
cd /Users/wangxingchao/Documents/novel
DATABASE_URL=postgresql://novel:novel_pass@localhost:5432/novel_engine \
  cargo test -p narrative-engine --test entity_tools_integration -- --nocapture
```
期望输出：
```
running 1 test
test entity_tools_create_revise_retire_logical_delete ... ok
test result: ok. 1 passed; 0 failed; ...
```

### 14.4 注意事项
- 测试会在数据库中**新建 project / world / 实体 / 关系**（每次用新 UUID），建议在开发库运行，勿连生产。
- 测试断言"retire 后直接查库 `status='Deleted'`、end_relation 后 `valid_until` 非空"，即验证删除为逻辑/语义删除、非物理 DELETE。
- 停止开发时：`docker stop novel-postgres`；再停 colima：`colima stop`（数据保留在 volume，下次 `docker start` 仍在）。

