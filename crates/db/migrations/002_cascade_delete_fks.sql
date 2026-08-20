-- 002: 将引用 project / entity / world 的外键统一改为 ON DELETE CASCADE
--
-- 背景与修复目标：
--   1. DELETE FROM project 当前会因所有子表的 project_id 外键（默认 NO ACTION = RESTRICT）
--      直接触发外键冲突而报错，导致 delete_project 端点实际上不可用。
--      把所有引用 project(id) 的外键改为 CASCADE，使删除项目能一次性级联清理全部子数据。
--   2. 同时把引用 entity(id) / world(id) 的外键改为 CASCADE，保证整条依赖链语义一致：
--      删除 world 或（将来）硬删除 entity 时能级联清理其子行。
--      注意：本项目的 delete_entity 走软删除（UPDATE status='Deleted'），并不触发级联，
--      因此不会因 relation 的 ON DELETE RESTRICT 而报错 —— 这正是此前 delete_entity 端点
--      不可用的根因之一，已由 API 层改用软删除修复。
--
-- 幂等性：migration runner 通过 _migrations 表保证本文件只执行一次；
--   DO 块对每个匹配外键先 DROP 再 ADD，统一重建为 ON DELETE CASCADE。

DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT
            tc.constraint_name  AS cn,
            tc.table_name      AS tn,
            kcu.column_name    AS col,
            ccu.table_name     AS ref_tn
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
           AND tc.table_schema   = kcu.table_schema
        JOIN information_schema.constraint_column_usage ccu
            ON ccu.constraint_name = tc.constraint_name
           AND ccu.table_schema   = tc.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
          AND tc.table_schema   = 'public'
          AND ccu.table_name IN ('project', 'entity', 'world')
    LOOP
        EXECUTE format(
            'ALTER TABLE %I DROP CONSTRAINT %I',
            r.tn, r.cn
        );
        EXECUTE format(
            'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (%I) REFERENCES %I(id) ON DELETE CASCADE',
            r.tn, r.cn, r.col, r.ref_tn
        );
    END LOOP;
END $$;
