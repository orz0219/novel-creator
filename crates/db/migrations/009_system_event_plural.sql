-- 009: align system_event table name with code expectation
--
-- crates/db/src/schema.rs 的 expected_tables 校验期望表名为 `system_events`（复数），
-- 但 001_canonical_schema.sql 创建的是 `system_event`（单数）。
-- 目前代码尚未对这张表发起查询，仅 verify_schema 会报 Missing tables；
-- 这里将表名对齐为复数，消除校验告警，避免后续接入时踩坑。
--
-- 幂等：仅当 `system_event` 存在且 `system_events` 不存在时才重命名。

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'system_event'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'system_events'
    ) THEN
        ALTER TABLE system_event RENAME TO system_events;
    END IF;
END $$;
