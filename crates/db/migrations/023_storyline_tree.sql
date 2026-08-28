-- ============================================================
-- 023_storyline_tree.sql
-- Storyline 树形结构 + 明/暗线区分
--
-- 设计：
--   1. 每项目 1 条 Main 主线（代码 + 校验层强约束，DB 不加 UNIQUE 保持灵活）
--   2. 副线（Normal importance）可以挂到主线或另一条副线（树形）
--   3. tone: light=明线（用户可见）/ dark=暗线（隐藏）
--   4. visibility: visible=暴露给读者 / hidden=伏笔/钩子剧情
--      暗线 = tone=dark + visibility=hidden 的副线
--   5. 切入时间点：本次不加（用户故事还没展开）；留给细纲阶段
--      未来加 narrative_node_storyline 关联表
-- ============================================================

ALTER TABLE storyline
    ADD COLUMN IF NOT EXISTS tone VARCHAR NOT NULL DEFAULT 'light';
ALTER TABLE storyline
    ADD COLUMN IF NOT EXISTS visibility VARCHAR NOT NULL DEFAULT 'visible';

-- 树形挂载关系表
CREATE TABLE IF NOT EXISTS storyline_relation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    parent_id UUID NOT NULL REFERENCES storyline(id) ON DELETE CASCADE,
    child_id UUID NOT NULL REFERENCES storyline(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 同一对父子关系只能建一次
    UNIQUE (parent_id, child_id),
    -- 防止自挂
    CHECK (parent_id <> child_id)
);

CREATE INDEX IF NOT EXISTS idx_storyline_rel_project
    ON storyline_relation(project_id);
CREATE INDEX IF NOT EXISTS idx_storyline_rel_parent
    ON storyline_relation(parent_id);
CREATE INDEX IF NOT EXISTS idx_storyline_rel_child
    ON storyline_relation(child_id);
