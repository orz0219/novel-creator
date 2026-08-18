-- Migration 007: Schema/Domain Alignment
-- Fixes mismatches between domain structs and database tables
-- NOTE: DuckDB does not support ADD COLUMN with NOT NULL constraints.
-- Columns are added nullable first, then backfilled, then constrained.

-- ============================================================
-- 1. ENTITY: Add missing columns
-- ============================================================
ALTER TABLE entity ADD COLUMN IF NOT EXISTS version INTEGER DEFAULT 1;
ALTER TABLE entity ADD COLUMN IF NOT EXISTS created_by VARCHAR DEFAULT 'system';
ALTER TABLE entity ADD COLUMN IF NOT EXISTS updated_by VARCHAR;
ALTER TABLE entity ADD COLUMN IF NOT EXISTS source_generation_id VARCHAR;
UPDATE entity SET version = 1 WHERE version IS NULL;
UPDATE entity SET created_by = 'system' WHERE created_by IS NULL;

-- ============================================================
-- 2. NARRATIVE_NODE: Add missing world_id column
-- ============================================================
ALTER TABLE narrative_node ADD COLUMN IF NOT EXISTS world_id VARCHAR DEFAULT '';
ALTER TABLE narrative_node ADD COLUMN IF NOT EXISTS version INTEGER DEFAULT 1;
UPDATE narrative_node SET world_id = '' WHERE world_id IS NULL;
UPDATE narrative_node SET version = 1 WHERE version IS NULL;

-- ============================================================
-- 3. FACT: Add certainty column for FactCertainty levels
-- ============================================================
ALTER TABLE fact ADD COLUMN IF NOT EXISTS certainty VARCHAR DEFAULT 'CANON';
UPDATE fact SET certainty = 'CANON' WHERE certainty IS NULL;

CREATE INDEX IF NOT EXISTS idx_fact_certainty ON fact(project_id, certainty);

-- ============================================================
-- 4. RELATION: Add version for optimistic locking
-- ============================================================
ALTER TABLE relation ADD COLUMN IF NOT EXISTS version INTEGER DEFAULT 1;
UPDATE relation SET version = 1 WHERE version IS NULL;

-- ============================================================
-- 5. LOCATION_PROFILE - Location structured info
-- ============================================================
CREATE TABLE IF NOT EXISTS location_profile (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    geography TEXT,
    appearance TEXT,
    population TEXT,
    economy TEXT,
    rules TEXT,
    history TEXT,
    narrative_usage TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_profile_entity ON location_profile(entity_id);

-- ============================================================
-- 6. LOCATION_FACILITY - Location facilities
-- ============================================================
CREATE TABLE IF NOT EXISTS location_facility (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    facility_type VARCHAR,
    description TEXT,
    controlled_by_entity_id VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_facility_entity ON location_facility(entity_id);

-- ============================================================
-- 7. LOCATION_THREAT - Location threats
-- ============================================================
CREATE TABLE IF NOT EXISTS location_threat (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    threat_type VARCHAR,
    description TEXT,
    severity VARCHAR DEFAULT 'Normal',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_threat_entity ON location_threat(entity_id);

-- ============================================================
-- 8. LOCATION_SECRET - Location secrets
-- ============================================================
CREATE TABLE IF NOT EXISTS location_secret (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    description TEXT NOT NULL,
    discovered BOOLEAN NOT NULL DEFAULT FALSE,
    discovered_by_entity_id VARCHAR,
    discovered_at_scene_id VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_secret_entity ON location_secret(entity_id);

-- ============================================================
-- 9. LOCATION_CONNECTION - Travel graph edges
-- ============================================================
CREATE TABLE IF NOT EXISTS location_connection (
    id VARCHAR PRIMARY KEY,
    source_location_id VARCHAR NOT NULL REFERENCES entity(id),
    target_location_id VARCHAR NOT NULL REFERENCES entity(id),
    connection_type VARCHAR NOT NULL DEFAULT 'path',
    travel_time VARCHAR,
    travel_description TEXT,
    is_bidirectional BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_conn_source ON location_connection(source_location_id);
CREATE INDEX IF NOT EXISTS idx_loc_conn_target ON location_connection(target_location_id);

-- ============================================================
-- 10. NARRATIVE_BUDGET - Word count allocation
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_budget (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    narrative_node_id VARCHAR NOT NULL,
    allocated_words INTEGER NOT NULL DEFAULT 0,
    used_words INTEGER NOT NULL DEFAULT 0,
    action_ratio DOUBLE PRECISION,
    dialogue_ratio DOUBLE PRECISION,
    description_ratio DOUBLE PRECISION,
    exposition_ratio DOUBLE PRECISION,
    internal_monologue_ratio DOUBLE PRECISION,
    pacing_warning_threshold DOUBLE PRECISION DEFAULT 0.9,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narr_budget_project ON narrative_budget(project_id);
CREATE INDEX IF NOT EXISTS idx_narr_budget_node ON narrative_budget(narrative_node_id);

-- ============================================================
-- 11. NOVEL_STATE_SNAPSHOT - Full novel state at a point in time
-- ============================================================
CREATE TABLE IF NOT EXISTS novel_state_snapshot (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR,
    story_time VARCHAR,
    world_summary TEXT,
    main_character_state TEXT,
    current_location VARCHAR,
    active_threads_count INTEGER DEFAULT 0,
    unresolved_foreshadows_count INTEGER DEFAULT 0,
    known_characters_count INTEGER DEFAULT 0,
    known_locations_count INTEGER DEFAULT 0,
    current_volume_id VARCHAR,
    current_arc_id VARCHAR,
    state_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_novel_snap_project ON novel_state_snapshot(project_id);
CREATE INDEX IF NOT EXISTS idx_novel_snap_scene ON novel_state_snapshot(scene_id);

-- ============================================================
-- 12. NARRATIVE_THREAD - Extended storyline with progression
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_thread (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    storyline_id VARCHAR,
    name VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL DEFAULT 'Active',
    importance VARCHAR NOT NULL DEFAULT 'Normal',
    current_stage TEXT,
    recent_progress TEXT,
    next_step TEXT,
    goal TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narr_thread_project ON narrative_thread(project_id);
CREATE INDEX IF NOT EXISTS idx_narr_thread_storyline ON narrative_thread(storyline_id);

-- ============================================================
-- 13. NARRATIVE_THREAD_PARTICIPANT
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_thread_participant (
    id VARCHAR PRIMARY KEY,
    thread_id VARCHAR NOT NULL REFERENCES narrative_thread(id),
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    role VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narr_thread_part_thread ON narrative_thread_participant(thread_id);

-- ============================================================
-- 14. Update scene table: add version column
-- ============================================================
ALTER TABLE scene ADD COLUMN IF NOT EXISTS version INTEGER DEFAULT 1;
UPDATE scene SET version = 1 WHERE version IS NULL;

-- ============================================================
-- 15. Update current_state: add version column
-- ============================================================
ALTER TABLE current_state ADD COLUMN IF NOT EXISTS version INTEGER DEFAULT 1;
UPDATE current_state SET version = 1 WHERE version IS NULL;
