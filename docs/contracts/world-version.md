# World Version 契约（world-version contract）

> 机械不变量（mechanical invariant）现在就被锁定；**语义（semantic）含义**是
> P2 架构评审的开放议题，暂不定稿。本文件记录两者。

## 机械不变量（已锁定，由契约测试守护）

- 每一次**成功的 commit** 都会为项目的 main world 在 `world_version` 表
  追加一行：`version = (该 world 已有最大 version) + 1`，并通过
  `parent_version_id` 链接上一条版本。
- 因此「世界前进一个版本」与 commit 一一对应（类比 git commit），是
  `commit` 完整链路的一部分（见 `commit.md` 不变量 3）。
- 失败的 commit 不产生 `world_version` 行（随事务整体回滚）。

## 语义决议（P2 已定稿）

`world_version.version` 定义为**某个 world 的不可逆领域历史序号（domain history
sequence）**：世界发生过多少次被接受的合法状态演进。它**不是**乐观锁版本号、
**不是** narrative 时间线序号、**不是**数据库修订号。

- 乐观锁请另用 `current_state.revision` / `current_state.updated_at`（或
  `entity_revision`）实现，避免与 world_version 语义混淆。
- 三概念（乐观锁 / 世界历史 / 时间线）不得混为一谈。

## 实现边界决议（P2 已定稿）

- **推进权唯一**：`world_version` 只能由 **canonical commit** 推进。`DbMutationCommitter`
  （`crates/db/src/mutation_committer.rs`）是其唯一拥有者；`DbStateCommitterPort::commit`
  现在也通过同一个 `WorldVersionRepo` 推进版本（不再保留第二套裸写逻辑）。任何
  caller 不得直接 `INSERT world_version`。
- **kind 语义**：`world_version.kind` 仅表示"世界为何推进"
  （`UserEdit` / `AiProposal` / `System`），**不是**"如何推进"。canonical 路径
  始终写入非空的 `kind`；裸写 `kind=NULL` 视为违反契约（见
  `db/tests/transaction_invariants.rs::world_version_only_advanced_by_canonical_commit`）。
- **snapshot 暂不做**：当前 `system_events + state_change + world_version` 已具备
  event sourcing 最小基础；未来若需"回到某版本状态 / 分支剧情 / AI 重生成"，
  新增 `world_snapshots(version_id, state_blob)` 表，而非改动 `world_version` 语义。
  这属于 P3，不阻塞当前收敛。

## 当前实现边界

- 仅 canonical commit 会推进 `world_version`：
  - 路径二（canonical）：`DbMutationCommitter::commit_batch` 经 `WorldVersionRepo` 推进，
    并据 `MutationSource` 设置 `kind`（UserEdit / AiProposal / System）。
  - 路径一（兼容）：`DbStateCommitterPort::commit` 现在也经同一个 `WorldVersionRepo`
    推进（不再有独立的裸写逻辑），`kind` 固定为 `AiProposal`。
- 其它写路径（直接 `upsert_state` / `create_entity`）**不**推进版本。
- 并发提交下 `UNIQUE(world_id, version)` 的冲突处理留待 P3（当前顺序提交成立）。
