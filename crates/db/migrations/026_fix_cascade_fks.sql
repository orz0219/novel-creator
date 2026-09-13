-- 026: 补齐缺失的 ON DELETE CASCADE（承接 002 迁移的遗漏）。
--
-- 背景：002 迁移只把「引用 project / entity / world 且当时已存在」的外键改成了 CASCADE。
-- 而它之后新增的表（mutation_ledger、storyline_relation、scene、timeline_event、
-- generation_run、validation_run、revelation …）又用了默认的 NO ACTION，
-- 于是 `DELETE FROM project` 依旧会撞外键：
--
--   error returned from database: update or delete on table "project"
--   violates foreign key constraint "mutation_ledger_project_id_fkey"
--
-- 表现就是「删除项目失败」（手机端点删除直接 500）。
--
-- 这里做一次全量扫描：把所有**非字典表**引用的外键统一改成 ON DELETE CASCADE。
-- 刻意跳过的三类：
--   1. 引用 entity_type / skill 的外键 —— 这些是字典表，删掉一个类型不该连带删数据
--   2. 自引用（如 fact.superseded_by）—— 删父行连带删子行过于激进
--   3. 已经是 CASCADE 的 —— 不动
--
-- 幂等：迁移表保证只执行一次；即使重复执行，条件里的 confdeltype <> 'c' 也会跳过已改好的。

DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT
            c.conname                        AS cn,
            c.conrelid::regclass::text       AS tn,
            pg_get_constraintdef(c.oid)      AS def
        FROM pg_constraint c
        WHERE c.contype = 'f'
          AND c.connamespace = 'public'::regnamespace
          AND c.confdeltype <> 'c'
          AND c.conrelid <> c.confrelid
          AND c.confrelid::regclass::text NOT IN ('entity_type', 'skill', 'test_case')
    LOOP
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', r.tn, r.cn);
        -- 把原有的 ON DELETE xxx（若有）去掉，统一换成 CASCADE
        EXECUTE format(
            'ALTER TABLE %s ADD CONSTRAINT %I %s',
            r.tn,
            r.cn,
            regexp_replace(r.def, '\s+ON DELETE\s+[A-Z ]+$', '', 'i') || ' ON DELETE CASCADE'
        );
    END LOOP;
END $$;
