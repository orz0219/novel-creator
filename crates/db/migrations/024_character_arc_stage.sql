-- ============================================================
-- 人物阶段弧线（arc_stages）
--
-- 背景：人物有「前期 / 中期 / 后期」的身份与戏份变化，
-- 但 role_in_story 是单值枚举、drive 只有"当下 / 远景"、conflicts 也没有生效阶段，
-- 一个角色装不下三个身份。
--
-- 本表与 character_arc_potential 正交：
--   character_arc_potential = 内在曲线（为什么会变、什么在抵抗）
--   character_arc_stage     = 外部时间线（何时上场、演什么、戏份多大）
--
-- 可选字段：不填就不显示，不影响现有角色。幂等，可重复执行。
-- ============================================================

CREATE TABLE IF NOT EXISTS character_arc_stage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    -- 阶段名：前期/中期/后期，或卷1/卷2，或按地点命名
    stage TEXT NOT NULL,
    -- 排序用整数（前端好展示、引擎按时序推进）；越小越早
    order_index INTEGER NOT NULL DEFAULT 0,
    -- 此阶段的身份 / 功能位（比 role_in_story 更具体，如「第一个合伙人」）
    role TEXT,
    -- 戏份：Light / Medium / Heavy（枚举，便于按戏份排序）
    screen_weight TEXT,
    -- 此阶段的目标（补 drive 的中间档）
    goal TEXT,
    -- 此阶段的叙事功能
    "function" TEXT,
    -- 什么事件把他推进这一阶段
    entry_trigger TEXT,
    -- 该阶段的现状 / 备注
    status TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_char_arc_stage_entity ON character_arc_stage(entity_id);

-- 冲突的生效阶段：周浩「怕被排除在外」在主角开口之前根本不存在，
-- 标上阶段后，前端可以按阶段过滤，不会从第一章就显示。
ALTER TABLE character_conflict ADD COLUMN IF NOT EXISTS phase TEXT;
