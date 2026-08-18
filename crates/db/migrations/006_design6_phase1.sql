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
    objectives JSON,
    required_events JSON,
    required_revelations JSON,
    required_character_changes JSON,
    required_world_changes JSON,
    forbidden_events JSON,
    exit_conditions JSON,
    completion_progress DOUBLE NOT NULL DEFAULT 0.0,
    completed_events JSON,
    completed_character_changes JSON,
    completed_world_changes JSON,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_story_contract_project ON story_contract(project_id);
CREATE INDEX IF NOT EXISTS idx_story_contract_narrative ON story_contract(narrative_node_id);
