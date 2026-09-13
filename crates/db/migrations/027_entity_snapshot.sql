-- 027: 实体版本快照表。
--
-- 背景：`/entities/{id}/versions` 那三个接口此前是**占位桩**——完全不查库，
-- 永远返回一条写死的 "Initial"（版本号恒为 1、changes 为空、created_at 还是
-- 请求时刻的时间）。电脑端的「版本历史」面板看到的都是编出来的数据。
--
-- 要做出真的历史，必须有地方存「改动前的样子」。mutation_ledger 只有
-- command_id / project_id / status / created_at，不记改了哪个实体、改了什么；
-- entity 表只有一个 version 计数。所以新建这张快照表：
-- 每次实体被修改**之前**，把当前这一版留档（见 EntityRepo::snapshot_tx）。
--
-- 为什么存"被取代的那一版"而不是"新的一版"：写入点就在更新前，
-- 一次 INSERT ... SELECT 即可拿到旧值，不必先改再读回；而当前状态本来就在
-- entity 表里，读历史时把它当成"最新一版"拼上即可。

CREATE TABLE IF NOT EXISTS entity_snapshot (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id uuid NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    project_id uuid NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    version integer NOT NULL,
    name text NOT NULL,
    summary text,
    description text,
    attributes jsonb,
    status text NOT NULL,
    -- 谁改的：'system' / 'user' / 模型相关标识，直接沿用 entity.updated_by
    updated_by text,
    -- 这一版被取代的时间（也就是下一次修改发生的时间）
    replaced_at timestamptz NOT NULL DEFAULT NOW(),
    -- 同一实体的同一版本只留一份
    UNIQUE (entity_id, version)
);

CREATE INDEX IF NOT EXISTS idx_entity_snapshot_entity
    ON entity_snapshot (entity_id, version DESC);
