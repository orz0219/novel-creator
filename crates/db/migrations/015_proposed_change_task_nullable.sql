-- 提案（proposed_change）不一定依附于生成任务：
--   · 抽取链路（M1 / generation 文本）产生的提案可能不关联某个 generation_task；
--   · 仅 generation flow 的提案会带真实 task_id。
-- 因此 task_id 改为可空；保留对 generation_task 的外键（外键允许 NULL），
-- 以便仍对“有 task 的提案”做引用完整性校验。
ALTER TABLE proposed_change ALTER COLUMN task_id DROP NOT NULL;
