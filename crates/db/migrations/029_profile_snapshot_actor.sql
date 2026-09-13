-- 029: 档案快照记录「是谁改的」。
--
-- 028 建表时漏了这一列，导致档案历史里只能显示「改了哪些字段」，
-- 无法区分「我自己改的」还是「AI 改的」——而后者恰恰是用户最想追溯的。
--
-- 取值沿用 MutationSource::as_str()：user / ai / system，
-- 与 entity_snapshot.updated_by 保持同一套词汇。

ALTER TABLE entity_profile_snapshot ADD COLUMN IF NOT EXISTS actor text;
