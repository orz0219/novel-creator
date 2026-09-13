-- 档案快照的来源列（与 PG 侧 029 对应）。
-- SQLite 的 ADD COLUMN 没有 IF NOT EXISTS，靠 _migrations 保证只执行一次。

ALTER TABLE "entity_profile_snapshot" ADD COLUMN "actor" TEXT;
