# Commit 契约（commit contract）

> 本文件只定义**不可破坏的契约**，不写实现细节。任何修改 `commit` 路径的
> PR 都必须保证以下不变量继续成立（由 `db/tests/transaction_invariants.rs`、
> `application/tests/commit_contract_tests.rs` 守护）。

## 输入保证（Input Guarantee）

1. **批内同键唯一**：同一个 commit batch 内，不能存在两个 `StateChange`
   命中相同的 `(entity_id, state_key)`。第二个必须失败，且**不允许部分写入**。
   - 依据：`commit_changes` 在循环内维护 `committed_state_keys` 集合，命中即
     返回 `CAS conflict` 错误，整笔事务回滚。
   - 反例（已被修复的 bug）：曾因按无 tag 的 `ChangePayload` 反序列化，导致
     commit 永远返回 `Unsupported payload`，使该不变式完全失效。

2. **payload 必须匹配 change_type**：`commit` 按 `change_type` 分派并解析
   payload（`StateChange` → `{state_key, new_value}`；`EntityCreate` →
   `{entity_type, name, attributes}`；`RelationCreate` →
   `{target_entity_id, relation_type, attributes}`）。payload 形状与
   `change_type` 不符必须报错，而不是静默忽略。
   - 依据：`domain/tests/change_payload_property.rs` 锁定各 `ProposedChangeType`
     期望的 payload 结构。

## 原子性保证（Atomicity Guarantee）

3. **成功 = 完整链路**：一次成功的 commit 必须**同时**产出：
   - `system_events` 中一条事件记录（`event_id` 可追溯）；
   - `state_change` 中对应的状态变更行，且 `state_change.event_id` 正确指向
     该事件；
   - `current_state` 中状态被更新（含 per-state 版本 CAS 递增）；
   - `world_version` 中追加一条新版本行（见 `world-version.md`），`version`
     较上一次 +1。
   上述任一项缺失都视为 bug。

4. **失败 = 全部回滚**：commit 过程中任何一步失败（含批内同键冲突、payload
   解析失败、实体不存在、并发修改），必须**整笔事务回滚**——数据库回到
   commit 前状态，没有任何 `system_events` / `state_change` / `current_state`
   / `world_version` 的部分写入，且所有 `proposed_change` 仍保持 `Approved`
   （未被改成 `Applied`）。
   - 守护：`test_commit_atomicity` 与 `transaction_invariants` 在失败后核对
     `world.version / events.count / state_changes.count / world_version.count`
     与失败前一致，而不仅验证返回错误。

## 设计约束

- `commit` 是**唯一**变更 canonical world state 的入口。
- 不要在 `e2e` 测试里承担数据库一致性保护；一致性由本契约测试守护。
