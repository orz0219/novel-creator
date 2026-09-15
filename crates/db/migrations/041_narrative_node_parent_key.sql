-- ============================================================
-- 修掉 narrative_node 序号唯一约束在「根节点」上的盲区
--
-- 背景（实测，非推断）：
--   040 把 idx_narrative_sort_order_unique 换成了
--     UNIQUE (project_id, parent_id, sort_order) DEFERRABLE INITIALLY DEFERRED
--   可延迟是为了让兄弟重排能在事务内瞬时重复（上移/下移必需）。
--   但这个约束对 parent_id IS NULL 的行**完全不生效**——PostgreSQL 的
--   唯一索引里 NULL 互不相等，而「卷」这类根节点的 parent_id 恰好永远是 NULL。
--
--   实测：往根层级插一条与「卷一 · 破庙立神」同为 sort_order=1 的行，
--   返回 `INSERT 0 1`，不报冲突；同一条插入挂到具体父节点下则会正常报
--   `duplicate key value violates unique constraint`。
--   于是根层级实际上没有任何去重保护：并发算号（MAX+1）或显式传号撞车时，
--   根节点会**静默**产生重复序号，排序退化成创建时间顺序，且无错可查。
--
-- 为什么不用部分唯一索引（WHERE status != 'Deleted'）：
--   DEFERRABLE 只支持**约束**，而约束不支持 WHERE 子句；
--   `CREATE UNIQUE INDEX ... WHERE` 又不能声明 DEFERRABLE。
--   两者不可兼得，所以保留约束形态，改约束键。
--
-- 做法：把约束键里的 parent_id 换成**非空**的生成列 parent_key
--   （根节点用全零 UUID 作哨兵）。列非空 → NULL 盲区消失；
--   仍是表约束 → DEFERRABLE 保住，让位/重排不受影响。
--   生成列 STORED 由 PostgreSQL 自动维护：parent_id 变化时自动重算，
--   因此 revise_node 换父、move_to_root 都不需要应用层改动，
--   也没有「两处各写一遍、忘记同步」的风险。
--
-- 迁移前已核对全库：按 (project_id, COALESCE(parent_id, 全零), sort_order)
-- 分组无重复行，故本迁移不会因历史脏数据失败。
--
-- 幂等，可重复执行。
-- ============================================================

-- 根节点的父作用域键：全零 UUID 作哨兵（真实节点 id 不可能取到它）
ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS parent_key UUID
    GENERATED ALWAYS AS (COALESCE(parent_id, '00000000-0000-0000-0000-000000000000'::uuid)) STORED;

-- 旧约束键里含可空列 parent_id，对根节点形同虚设
ALTER TABLE narrative_node
    DROP CONSTRAINT IF EXISTS narrative_node_sort_order_unique;

ALTER TABLE narrative_node
    ADD CONSTRAINT narrative_node_sort_order_unique
    UNIQUE (project_id, parent_key, sort_order) DEFERRABLE INITIALLY DEFERRED;
