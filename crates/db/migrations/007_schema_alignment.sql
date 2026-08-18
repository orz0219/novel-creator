-- Migration 007: Schema/Domain Alignment
-- Fixes mismatches between domain structs and database tables

-- ============================================================
-- 1. ENTITY: Add missing columns
-- ============================================================
ALTER TABLE entity ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE entity ADD COLUMN IF NOT EXISTS created_by VARCHAR NOT NULL DEFAULT 'system';
ALTER TABLE entity ADD COLUMN IF NOT EXISTS updated_by VARCHAR;
ALTER TABLE entity ADD COLUMN IF NOT EXISTS source_generation_id VARCHAR;

-- ============================================================
-- 2. NARRATIVE_NODE: Add missing world_id column
-- ============================================================
ALTER TABLE narrative_node ADD COLUMN IF NOT EXISTS world_id VARCHAR NOT NULL DEFAULT '';
ALTER TABLE narrative_node ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

-- ============================================================
-- 3. FACT: Add certainty column for FactCertainty levels
-- ============================================================
ALTER TABLE fact ADD COLUMN IF NOT EXISTS certainty VARCHAR NOT NULL DEFAULT 'CANON';

CREATE INDEX IF NOT EXISTS idx_fact_certainty ON fact(project_id, certainty);

-- ============================================================
-- 4. CHARACTER_PROFILE - Character stable info
-- ============================================================
CREATE TABLE IF NOT EXISTS character_profile (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    real_name VARCHAR,
    nickname VARCHAR,
    age VARCHAR,
    gender VARCHAR,
    identity VARCHAR,
    appearance TEXT,
    background TEXT,
    social_status VARCHAR,
    core_personality TEXT,
    values_desc TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_profile_entity ON character_profile(entity_id);

-- ============================================================
-- 5. CHARACTER_STATE - Character dynamic state
-- ============================================================
CREATE TABLE IF NOT EXISTS character_state (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    location VARCHAR,
    health VARCHAR,
    cultivation VARCHAR,
    money VARCHAR,
    wanted BOOLEAN NOT NULL DEFAULT FALSE,
    extra JSON,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_state_entity ON character_state(entity_id);

-- ============================================================
-- 6. CHARACTER_GOAL - Character multi-level goals
-- ============================================================
CREATE TABLE IF NOT EXISTS character_goal (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    long_term TEXT,
    current_goal TEXT,
    immediate TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_goal_entity ON character_goal(entity_id);

-- ============================================================
-- 7. CHARACTER_TRAIT - Character personality traits
-- ============================================================
CREATE TABLE IF NOT EXISTS character_trait (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    trait_type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    parent_trait_id VARCHAR,
    intensity INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_trait_entity ON character_trait(entity_id);

-- ============================================================
-- 8. FACTION_PROFILE - Faction structured info
-- ============================================================
CREATE TABLE IF NOT EXISTS faction_profile (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    goals TEXT,
    leader VARCHAR,
    values_desc TEXT,
    resources TEXT,
    territory TEXT,
    members TEXT,
    enemies TEXT,
    allies TEXT,
    internal_conflicts TEXT,
    secrets TEXT,
    modus_operandi TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_faction_profile_entity ON faction_profile(entity_id);

-- ============================================================
-- 9. RELATION: Add version for optimistic locking
-- ============================================================
ALTER TABLE relation ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

-- ============================================================
-- 10. LOCATION_PROFILE - Location structured info
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
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_profile_entity ON location_profile(entity_id);

-- ============================================================
-- 11. LOCATION_FACILITY - Location facilities
-- ============================================================
CREATE TABLE IF NOT EXISTS location_facility (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    facility_type VARCHAR,
    description TEXT,
    controlled_by_entity_id VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_facility_entity ON location_facility(entity_id);

-- ============================================================
-- 12. LOCATION_THREAT - Location threats
-- ============================================================
CREATE TABLE IF NOT EXISTS location_threat (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    threat_type VARCHAR,
    description TEXT,
    severity VARCHAR DEFAULT 'Normal',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_threat_entity ON location_threat(entity_id);

-- ============================================================
-- 13. LOCATION_SECRET - Location secrets
-- ============================================================
CREATE TABLE IF NOT EXISTS location_secret (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    description TEXT NOT NULL,
    discovered BOOLEAN NOT NULL DEFAULT FALSE,
    discovered_by_entity_id VARCHAR,
    discovered_at_scene_id VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_secret_entity ON location_secret(entity_id);

-- ============================================================
-- 14. LOCATION_CONNECTION - Travel graph edges
-- ============================================================
CREATE TABLE IF NOT EXISTS location_connection (
    id VARCHAR PRIMARY KEY,
    source_location_id VARCHAR NOT NULL REFERENCES entity(id),
    target_location_id VARCHAR NOT NULL REFERENCES entity(id),
    connection_type VARCHAR NOT NULL DEFAULT 'path',
    travel_time VARCHAR,
    travel_description TEXT,
    is_bidirectional BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_conn_source ON location_connection(source_location_id);
CREATE INDEX IF NOT EXISTS idx_loc_conn_target ON location_connection(target_location_id);

-- ============================================================
-- 15. NARRATIVE_BUDGET - Word count allocation
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_budget (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    narrative_node_id VARCHAR NOT NULL,
    allocated_words INTEGER NOT NULL DEFAULT 0,
    used_words INTEGER NOT NULL DEFAULT 0,
    action_ratio DOUBLE,
    dialogue_ratio DOUBLE,
    description_ratio DOUBLE,
    exposition_ratio DOUBLE,
    internal_monologue_ratio DOUBLE,
    pacing_warning_threshold DOUBLE DEFAULT 0.9,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narr_budget_project ON narrative_budget(project_id);
CREATE INDEX IF NOT EXISTS idx_narr_budget_node ON narrative_budget(narrative_node_id);

-- ============================================================
-- 16. NOVEL_STATE_SNAPSHOT - Full novel state at a point in time
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
    state_data JSON,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_novel_snap_project ON novel_state_snapshot(project_id);
CREATE INDEX IF NOT EXISTS idx_novel_snap_scene ON novel_state_snapshot(scene_id);

-- ============================================================
-- 17. NARRATIVE_THREAD - Extended storyline with progression
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
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narr_thread_project ON narrative_thread(project_id);
CREATE INDEX IF NOT EXISTS idx_narr_thread_storyline ON narrative_thread(storyline_id);

-- ============================================================
-- 18. NARRATIVE_THREAD_PARTICIPANT
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_thread_participant (
    id VARCHAR PRIMARY KEY,
    thread_id VARCHAR NOT NULL REFERENCES narrative_thread(id),
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    role VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narr_thread_part_thread ON narrative_thread_participant(thread_id);

-- ============================================================
-- 19. Update scene table: add version column
-- ============================================================
ALTER TABLE scene ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

-- ============================================================
-- 20. Update current_state: add version column
-- ============================================================
ALTER TABLE current_state ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
