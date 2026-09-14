-- ============================================================
-- 伏笔的节点锚与伏笔树
--
-- 背景（来自 AI 使用方反馈，已核实）：伏笔原先只有 importance / hint_level / status
-- 三个属性，**没有关联任何章节节点**。对连载来说「这条钩子埋在哪一章、打算哪一章收」
-- 是刚需，此前只能写在 description 里当散文，排章节时查不出来。
--
-- 三列：
--   planted_node_id      埋点所在节点（哪一章埋的）
--   payoff_node_id       计划回收所在节点（打算哪一章收）
--   parent_foreshadow_id 父伏笔：一条大伏笔挂几个小钩子（伏笔树）
--
-- 沿本表既有风格**不加外键约束**（表上从来没有 FK，跨表一致性由应用层在写入前校验：
-- 节点/父伏笔不存在就直接报错，不写悬空 uuid）。
--
-- 幂等，可重复执行。
-- ============================================================

ALTER TABLE foreshadowing ADD COLUMN IF NOT EXISTS planted_node_id UUID;
ALTER TABLE foreshadowing ADD COLUMN IF NOT EXISTS payoff_node_id UUID;
ALTER TABLE foreshadowing ADD COLUMN IF NOT EXISTS parent_foreshadow_id UUID;
