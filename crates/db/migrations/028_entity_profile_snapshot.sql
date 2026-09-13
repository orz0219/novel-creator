-- 028: 实体「档案」的历史快照。
--
-- 背景：027 的 entity_snapshot 只覆盖 entity 表（名称 / 摘要 / 描述 / 属性 / 状态）。
-- 而角色档案（身份、外貌、性格…）、当前状态、地点/势力档案都存在**另外几张表**里，
-- 更新走的是 application_ports 的 upsert_* —— 那些改动此前**完全不留痕**：
-- 用户把「身份」改错了、或者 AI 改掉了性格，都无处回看。
--
-- 这里为档案类改动单独留档。与 entity_snapshot 的区别：
--   - entity_snapshot 记录的是「行级」的实体字段
--   - 这张表记录的是「整份档案的 JSON」（改动前那一份），
--     因为档案字段分散在多张表（location_profile + location_identity 等），
--     逐列存没有意义，存整份 JSON 更简单也更完整。

CREATE TABLE IF NOT EXISTS entity_profile_snapshot (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id uuid NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    project_id uuid NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    -- character / location / faction（与前端详情页的类型对应）
    kind text NOT NULL,
    -- 改动前的完整档案（就是 GET .../profile 当时会返回的那份 JSON）
    payload jsonb NOT NULL,
    replaced_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_entity_profile_snapshot_entity
    ON entity_profile_snapshot (entity_id, replaced_at DESC);
