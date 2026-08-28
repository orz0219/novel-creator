-- ============================================================
-- 022_project_premise.sql
-- 项目表新增 premise 字段：脑洞/故事前提
--
-- 用途：在世界观展开前，由 Agent 引导用户与 AI 持续沟通打磨的故事核心前提
-- （如"我捡了一块钱怎么也花不完"）。存放在项目级而非 world.config，因为：
--   1. premise 是项目级元数据，与 genre 同性质，应在同一层
--   2. world 可能尚未存在 / 未来可能多 world，premise 应共享
--   3. 后续所有世界观/角色/叙事生成都受 premise 约束，作为最强 prompt 注入
--
-- 设计原则：自由文本（与 description / world_setting 同类型），不约束结构；
-- Agent 负责在用户确认后写入，由用户决定何时落库（"持续沟通"模式）。
-- ============================================================

ALTER TABLE project
    ADD COLUMN IF NOT EXISTS premise TEXT;
