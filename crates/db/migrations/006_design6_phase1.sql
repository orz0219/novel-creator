-- Design Doc 6 Phase 1: Story Contract
-- Story Contract table for Volume/Arc completion requirements

-- ============================================================
-- STORY_CONTRACT 故事契约
-- ============================================================
CREATE TABLE IF NOT EXISTS story_contract (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL,
    narrative_node_id VARCHAR NOT NULL,
    mission TEXT,
    objectives JSONB,
    required_events JSONB,
    required_revelations JSONB,
    required_character_changes JSONB,
    required_world_changes JSONB,
    forbidden_events JSONB,
    exit_conditions JSONB,
    completion_progress DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    completed_events JSONB,
    completed_character_changes JSONB,
    completed_world_changes JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_story_contract_project ON story_contract(project_id);
CREATE INDEX IF NOT EXISTS idx_story_contract_narrative ON story_contract(narrative_node_id);
