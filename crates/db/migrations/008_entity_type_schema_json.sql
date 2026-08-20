-- 008: align entity_type schema column with code
--
-- 代码（crates/db/src/repos/entity_repo.rs）实际查询的列名是 `schema_json`，
-- 但 001_canonical_schema.sql 中 entity_type 定义的是 `schema`（JSONB）。
-- 运行期 raw sqlx 查询不校验列名，故编译通过但运行报
-- "column schema_json does not exist"。此处将列名对齐到代码期望的 `schema_json`。
--
-- 使用 DO 块做幂等处理：仅当 `schema` 列存在时才重命名，
-- 避免该迁移在已重命名的库上重复执行时报错。

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'entity_type' AND column_name = 'schema'
    ) THEN
        ALTER TABLE entity_type RENAME COLUMN schema TO schema_json;
    END IF;
END $$;
