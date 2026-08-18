-- Design Doc 3: Phase 2 - V1.5 Enhancement Items
-- Foreshadowing + CausalRelation + ReaderKnowledge + NarrativeContract + QualityScore

-- ============================================================
-- 1. FORESHADOWING 伏笔系统
-- ============================================================
CREATE TABLE IF NOT EXISTS foreshadowing (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    storyline_id VARCHAR,
    name VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL DEFAULT 'Planned',
    importance VARCHAR NOT NULL DEFAULT 'Normal',
    hint_level VARCHAR NOT NULL DEFAULT 'Subtle',
    introduced_at VARCHAR,
    expected_reveal_at VARCHAR,
    actual_reveal_at VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_foreshadowing_project ON foreshadowing(project_id);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_status ON foreshadowing(status);

-- ============================================================
-- 2. CAUSAL_RELATION 因果链
-- ============================================================
CREATE TABLE IF NOT EXISTS causal_relation (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    cause_event_id VARCHAR NOT NULL,
    effect_event_id VARCHAR NOT NULL,
    relation_type VARCHAR NOT NULL DEFAULT 'DirectCause',
    strength VARCHAR NOT NULL DEFAULT 'Strong',
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_causal_project ON causal_relation(project_id);
CREATE INDEX IF NOT EXISTS idx_causal_cause ON causal_relation(cause_event_id);
CREATE INDEX IF NOT EXISTS idx_causal_effect ON causal_relation(effect_event_id);

-- ============================================================
-- 3. READER_KNOWLEDGE 读者认知状态
-- ============================================================
CREATE TABLE IF NOT EXISTS reader_knowledge (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    fact_id VARCHAR NOT NULL,
    knowledge_level VARCHAR NOT NULL DEFAULT 'Unknown',
    source_scene_id VARCHAR,
    confidence VARCHAR NOT NULL DEFAULT 'Certain',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reader_knowledge_project ON reader_knowledge(project_id);
CREATE INDEX IF NOT EXISTS idx_reader_knowledge_fact ON reader_knowledge(fact_id);

-- ============================================================
-- 4. SCENE_CONTRACT 场景契约
-- ============================================================
CREATE TABLE IF NOT EXISTS scene_contract (
    id VARCHAR PRIMARY KEY,
    scene_id VARCHAR NOT NULL,
    required_events JSON,
    forbidden_events JSON,
    required_characters JSON,
    required_facts JSON,
    reader_learns JSON,
    protagonist_learns JSON,
    world_changes JSON,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_scene_contract_scene ON scene_contract(scene_id);

-- ============================================================
-- 5. QUALITY_SCORE 质量评分
-- ============================================================
CREATE TABLE IF NOT EXISTS quality_score (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR NOT NULL,
    run_id VARCHAR,
    continuity_score INTEGER,
    character_score INTEGER,
    plot_score INTEGER,
    knowledge_score INTEGER,
    world_score INTEGER,
    style_score INTEGER,
    overall_score INTEGER,
    issues JSON,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_quality_score_project ON quality_score(project_id);
CREATE INDEX IF NOT EXISTS idx_quality_score_scene ON quality_score(scene_id);
