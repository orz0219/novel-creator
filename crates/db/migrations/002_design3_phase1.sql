-- Design Doc 3: Phase 1 - V1 Core Missing Items
-- Timeline Enhancement + Storyline + Visibility + Human Approval

-- ============================================================
-- 1. STORYLINE 跨卷剧情线
-- ============================================================
CREATE TABLE IF NOT EXISTS storyline (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL DEFAULT 'Active',
    importance VARCHAR NOT NULL DEFAULT 'Normal',
    created_volume_id VARCHAR,
    resolved_volume_id VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_storyline_project ON storyline(project_id);

-- ============================================================
-- 2. STORYLINE_SCENE 剧情线-场景关联
-- ============================================================
CREATE TABLE IF NOT EXISTS storyline_scene (
    id VARCHAR PRIMARY KEY,
    storyline_id VARCHAR NOT NULL REFERENCES storyline(id),
    scene_id VARCHAR NOT NULL,
    significance VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_storyline_scene_storyline ON storyline_scene(storyline_id);
CREATE INDEX IF NOT EXISTS idx_storyline_scene_scene ON storyline_scene(scene_id);

-- ============================================================
-- 3. FACT_VISIBILITY 事实可见性控制
-- ============================================================
CREATE TABLE IF NOT EXISTS fact_visibility (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    fact_id VARCHAR NOT NULL,
    subject_type VARCHAR NOT NULL,
    subject_id VARCHAR,
    visibility_level VARCHAR NOT NULL DEFAULT 'Hidden',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_fact_visibility_project ON fact_visibility(project_id);
CREATE INDEX IF NOT EXISTS idx_fact_visibility_fact ON fact_visibility(fact_id);

-- ============================================================
-- 4. APPROVAL_RECORD 人工审批记录
-- ============================================================
CREATE TABLE IF NOT EXISTS approval_record (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    target_type VARCHAR NOT NULL,
    target_id VARCHAR NOT NULL,
    proposed_by VARCHAR NOT NULL,
    proposal_content JSON,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    reviewer_id VARCHAR,
    reviewer_comment TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_approval_project ON approval_record(project_id);
CREATE INDEX IF NOT EXISTS idx_approval_target ON approval_record(target_id);
