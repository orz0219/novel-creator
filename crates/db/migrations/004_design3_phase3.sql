-- Design Doc 3: Phase 3 - V2 Advanced Features
-- Branch/Version + AutoPlotRepair

-- ============================================================
-- 1. WORLD_BRANCH 世界分支
-- ============================================================
CREATE TABLE IF NOT EXISTS world_branch (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    parent_branch_id VARCHAR,
    is_main BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_world_branch_project ON world_branch(project_id);

-- ============================================================
-- 2. NARRATIVE_BRANCH 叙事分支
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_branch (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    parent_branch_id VARCHAR,
    fork_point_scene_id VARCHAR,
    is_main BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narrative_branch_project ON narrative_branch(project_id);

-- ============================================================
-- 3. PLOT_REPAIR 剧情修复
-- ============================================================
CREATE TABLE IF NOT EXISTS plot_repair (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR NOT NULL,
    issue_description TEXT NOT NULL,
    repair_suggestion TEXT NOT NULL,
    repair_type VARCHAR NOT NULL DEFAULT 'Automatic',
    status VARCHAR NOT NULL DEFAULT 'Pending',
    applied_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_plot_repair_project ON plot_repair(project_id);
CREATE INDEX IF NOT EXISTS idx_plot_repair_scene ON plot_repair(scene_id);
