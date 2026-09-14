-- ============================================================
-- 剧情线的阶段弧线（arc_stages）
--
-- 背景（来自 AI 使用方反馈，已核实）：人物 / 势力 / 地点都有 arc_stages，
-- 唯独剧情线没有——「破庙 → 聚落 → 村落 → 社区化」这种阶段推进只能塞进
-- description 当散文；后果是剧情线面板没有阶段视图，也无法按「推进到哪个阶段」筛选。
--
-- 为什么用 JSONB，而不是像人物那样建 entity_arc_stage 表：
--   1. storyline 的阶段不需要按列过滤 / 排序（前端只读展示、AI 整体读写）；
--   2. 建表要新增 repo + 端口 + 两套实现 + 两套迁移，收益不匹配。
--
-- 形状与实体侧**完全一致**（数组，元素为对象：
-- stage / role（等价写法 stage_role）/ screen_weight / goal / function /
-- entry_trigger / status），因此前端可以复用同一套展示组件。
--
-- 幂等，可重复执行。
-- ============================================================

ALTER TABLE storyline
    ADD COLUMN IF NOT EXISTS arc_stages JSONB NOT NULL DEFAULT '[]'::jsonb;
