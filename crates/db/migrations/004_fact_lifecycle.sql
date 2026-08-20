-- 事实生命周期（提案 六 / 二十二）：用状态标记取代物理 DELETE。
-- 事实可被 Superseded / Invalid 取代，但不可被业务层物理删除。
ALTER TABLE fact ADD COLUMN IF NOT EXISTS status VARCHAR NOT NULL DEFAULT 'Active';
ALTER TABLE fact ADD COLUMN IF NOT EXISTS superseded_by UUID REFERENCES fact(id);
CREATE INDEX IF NOT EXISTS idx_fact_project_status ON fact (project_id, status);
