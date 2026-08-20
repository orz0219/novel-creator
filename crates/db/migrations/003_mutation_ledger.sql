-- 幂等 ledger：保证同一 command_id 重复提交得到同一结果（提案 二十七）
CREATE TABLE IF NOT EXISTS mutation_ledger (
    command_id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES project(id),
    status     VARCHAR NOT NULL DEFAULT 'committed',
    result     JSON,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mutation_ledger_project ON mutation_ledger (project_id);
