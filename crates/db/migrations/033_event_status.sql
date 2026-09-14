-- ============================================================
-- 事件软删：event 表补 status
--
-- 背景：事件此前**没有任何删除路径**（连 HTTP 接口都没有），AI 建错事件无法撤回。
-- 补删除时选择了「语义化结束」而不是物理 DELETE，与 entity / narrative_node 一致：
--   - entity：`UPDATE entity SET status='Deleted'`
--   - narrative_node：`UPDATE narrative_node SET status='Deleted'`
--   - fact：表里本就有 status 列（默认 Active）+ superseded_by
-- 创作数据误删无法挽回，而逻辑删除保留了可追溯性。
--
-- 因此 event 也需要一个 status 列，取值与 node 对齐：Active / Deleted。
--
-- 幂等，可重复执行。
-- ============================================================

ALTER TABLE event
    ADD COLUMN IF NOT EXISTS status VARCHAR NOT NULL DEFAULT 'Active';
