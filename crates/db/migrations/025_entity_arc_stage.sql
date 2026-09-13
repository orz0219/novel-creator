-- ============================================================
-- 阶段弧线泛化：character_arc_stage -> entity_arc_stage
--
-- 背景：阶段弧线（何时上场 / 演什么 / 戏份多大 / 该阶段现状）不是人物独有的，
-- 势力（崛起→鼎盛→分裂）和地点（主角家→据点→主战场）同样需要。
-- 三种实体的字段完全一致，因此收敛为一张按 entity_id 索引的通用表。
--
-- 幂等，可重复执行；旧表存在时把数据搬过来再删掉。
-- ============================================================

CREATE TABLE IF NOT EXISTS entity_arc_stage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    -- 阶段名：前期/中期/后期，或卷1/卷2，或按地点命名
    stage TEXT NOT NULL,
    -- 排序用整数（前端好展示、引擎按时序推进）；越小越早
    order_index INTEGER NOT NULL DEFAULT 0,
    -- 此阶段的身份 / 功能位
    role TEXT,
    -- 戏份：Light / Medium / Heavy
    screen_weight TEXT,
    -- 此阶段的目标（势力尤其需要：这一卷它想要什么）
    goal TEXT,
    -- 此阶段的叙事功能
    "function" TEXT,
    -- 什么事件把它推进这一阶段
    entry_trigger TEXT,
    -- 该阶段的现状快照（势力：多少人/占哪/盟友是谁；地点：被谁占据、什么样子）
    status TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_entity_arc_stage_entity ON entity_arc_stage(entity_id);

-- 把旧表的数据搬过来（旧表可能不存在，比如全新库已经直接建了新表）
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'character_arc_stage'
    ) THEN
        INSERT INTO entity_arc_stage (
            id, entity_id, stage, order_index, role, screen_weight,
            goal, "function", entry_trigger, status, created_at, updated_at
        )
        SELECT
            id, entity_id, stage, order_index, role, screen_weight,
            goal, "function", entry_trigger, status, created_at, updated_at
        FROM character_arc_stage
        ON CONFLICT (id) DO NOTHING;

        DROP TABLE character_arc_stage;
    END IF;
END $$;
