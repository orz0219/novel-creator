-- Design Doc 4: V2 Schema
-- Canon Constitution + Character Mind Model + Narrative Ledger + State Management + Identity + Retrieval

-- ============================================================
-- 1. CANON RULE 世界规则宪法
-- ============================================================
CREATE TABLE IF NOT EXISTS canon_rule (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    world_id VARCHAR NOT NULL,
    rule_level VARCHAR NOT NULL, -- RULE-0, RULE-1, RULE-2, RULE-3
    rule_content TEXT NOT NULL,
    affected_scope VARCHAR NOT NULL, -- "cultivation_system", "economy", etc.
    enforcement VARCHAR NOT NULL, -- "Reject", "RequireApproval", "Allow"
    constraints JSON, -- 具体约束条件（用于 Validator 自动检查）
    source VARCHAR, -- "author_defined", "world_setting"
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_canon_rule_project ON canon_rule(project_id);
CREATE INDEX IF NOT EXISTS idx_canon_rule_world ON canon_rule(world_id);
CREATE INDEX IF NOT EXISTS idx_canon_rule_level ON canon_rule(rule_level);

-- ============================================================
-- 2. BELIEF 角色信念
-- ============================================================
CREATE TABLE IF NOT EXISTS belief (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    character_id VARCHAR NOT NULL,
    belief_content TEXT NOT NULL,
    confidence DOUBLE NOT NULL DEFAULT 0.5, -- 0.0 - 1.0
    source VARCHAR, -- "personal_observation", "told_by_someone", "inference"
    source_scene_id VARCHAR,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_belief_project ON belief(project_id);
CREATE INDEX IF NOT EXISTS idx_belief_character ON belief(character_id);

-- ============================================================
-- 3. MEMORY 角色记忆
-- ============================================================
CREATE TABLE IF NOT EXISTS character_memory (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    character_id VARCHAR NOT NULL,
    memory_content TEXT NOT NULL,
    emotional_impact VARCHAR, -- "positive", "negative", "traumatic"
    scene_id VARCHAR,
    importance INTEGER NOT NULL DEFAULT 5, -- 1-10
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memory_project ON character_memory(project_id);
CREATE INDEX IF NOT EXISTS idx_memory_character ON character_memory(character_id);

-- ============================================================
-- 4. CHARACTER_GOAL_MIND 角色目标认知
-- ============================================================
CREATE TABLE IF NOT EXISTS character_goal_mind (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    character_id VARCHAR NOT NULL,
    goal_content TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 5, -- 1-10
    status VARCHAR NOT NULL DEFAULT 'Active', -- Active, Completed, Abandoned, Blocked
    source VARCHAR, -- "survival", "revenge", "love"
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_goal_mind_project ON character_goal_mind(project_id);
CREATE INDEX IF NOT EXISTS idx_goal_mind_character ON character_goal_mind(character_id);

-- ============================================================
-- 5. CHARACTER_FEAR 角色恐惧
-- ============================================================
CREATE TABLE IF NOT EXISTS character_fear (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    character_id VARCHAR NOT NULL,
    fear_content TEXT NOT NULL,
    intensity INTEGER NOT NULL DEFAULT 5, -- 1-10
    source VARCHAR, -- "past_trauma", "known_threat"
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_fear_project ON character_fear(project_id);
CREATE INDEX IF NOT EXISTS idx_fear_character ON character_fear(character_id);

-- ============================================================
-- 6. EMOTION_STATE 角色情绪状态
-- ============================================================
CREATE TABLE IF NOT EXISTS emotion_state (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    character_id VARCHAR NOT NULL,
    emotion_type VARCHAR NOT NULL, -- "fear", "anger", "joy", etc.
    intensity INTEGER NOT NULL DEFAULT 50, -- 0-100
    decay_rate DOUBLE NOT NULL DEFAULT 0.1, -- 每 Scene 衰减率
    trigger_scene_id VARCHAR,
    trigger_description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_emotion_project ON emotion_state(project_id);
CREATE INDEX IF NOT EXISTS idx_emotion_character ON emotion_state(character_id);

-- ============================================================
-- 7. NARRATIVE_STATE 叙事状态（四维状态）
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_state (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    state_dimension VARCHAR NOT NULL, -- "World", "Narrative", "Character", "Reader"
    state_key VARCHAR NOT NULL,
    state_value JSON NOT NULL,
    scene_id VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narrative_state_project ON narrative_state(project_id);
CREATE INDEX IF NOT EXISTS idx_narrative_state_dimension ON narrative_state(state_dimension);
CREATE INDEX IF NOT EXISTS idx_narrative_state_key ON narrative_state(state_key);

-- ============================================================
-- 8. SCENE_LEDGER 场景账本
-- ============================================================
CREATE TABLE IF NOT EXISTS scene_ledger (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR NOT NULL,
    events JSON, -- Vec<LedgerEvent>
    gains JSON, -- Vec<LedgerItem>
    losses JSON, -- Vec<LedgerItem>
    relationship_changes JSON, -- Vec<RelationshipChange>
    knowledge_changes JSON, -- Vec<KnowledgeChange>
    world_changes JSON, -- Vec<WorldChange>
    foreshadowing_mentions JSON, -- Vec<ForeshadowingMention>
    storyline_progress JSON, -- Vec<StorylineProgress>
    character_growth JSON, -- Vec<CharacterGrowth>
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ledger_project ON scene_ledger(project_id);
CREATE INDEX IF NOT EXISTS idx_ledger_scene ON scene_ledger(scene_id);

-- ============================================================
-- 9. DECISION_TRACE AI 决策追踪
-- ============================================================
CREATE TABLE IF NOT EXISTS decision_trace (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR NOT NULL,
    character_id VARCHAR NOT NULL,
    decision TEXT NOT NULL,
    factors JSON NOT NULL, -- Vec<DecisionFactor>
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_decision_project ON decision_trace(project_id);
CREATE INDEX IF NOT EXISTS idx_decision_scene ON decision_trace(scene_id);
CREATE INDEX IF NOT EXISTS idx_decision_character ON decision_trace(character_id);

-- ============================================================
-- 10. STATE_SNAPSHOT 状态快照（用于回滚）
-- ============================================================
CREATE TABLE IF NOT EXISTS state_snapshot (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR NOT NULL,
    state_before JSON NOT NULL,
    changes JSON NOT NULL,
    state_after JSON NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_snapshot_project ON state_snapshot(project_id);
CREATE INDEX IF NOT EXISTS idx_snapshot_scene ON state_snapshot(scene_id);

-- ============================================================
-- 11. KNOWLEDGE_GAP 知识缺口
-- ============================================================
CREATE TABLE IF NOT EXISTS knowledge_gap (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    gap_type VARCHAR NOT NULL, -- "LOCATION_DETAIL", "CHARACTER_BACKGROUND", etc.
    description TEXT NOT NULL,
    importance VARCHAR NOT NULL DEFAULT 'MEDIUM', -- HIGH, MEDIUM, LOW
    required_by_scene_id VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'Open', -- Open, Filled, Ignored
    designer_skill_hint VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_gap_project ON knowledge_gap(project_id);
CREATE INDEX IF NOT EXISTS idx_gap_status ON knowledge_gap(status);

-- ============================================================
-- 12. CHAPTER_SUMMARY 章节摘要
-- ============================================================
CREATE TABLE IF NOT EXISTS chapter_summary (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    chapter_id VARCHAR NOT NULL,
    summary TEXT NOT NULL,
    key_events JSON, -- Vec<String>
    involved_characters JSON, -- Vec<Uuid>
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_chapter_summary_project ON chapter_summary(project_id);

-- ============================================================
-- 13. ARC_SUMMARY 弧线摘要
-- ============================================================
CREATE TABLE IF NOT EXISTS arc_summary (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    arc_id VARCHAR NOT NULL,
    summary TEXT NOT NULL,
    key_turning_points JSON, -- Vec<String>
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_arc_summary_project ON arc_summary(project_id);

-- ============================================================
-- 14. VOLUME_SUMMARY 卷摘要
-- ============================================================
CREATE TABLE IF NOT EXISTS volume_summary (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    volume_id VARCHAR NOT NULL,
    summary TEXT NOT NULL,
    character_changes JSON, -- Vec<String>
    world_changes JSON, -- Vec<String>
    foreshadowing_progress JSON, -- Vec<String>
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_volume_summary_project ON volume_summary(project_id);

-- ============================================================
-- 15. GLOBAL_STORY_STATE 全局故事状态
-- ============================================================
CREATE TABLE IF NOT EXISTS global_story_state (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    current_progress TEXT NOT NULL,
    open_foreshadowing JSON, -- Vec<String>
    open_storylines JSON, -- Vec<String>
    world_state_summary TEXT NOT NULL,
    character_state_summary TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_global_state_project ON global_story_state(project_id);

-- ============================================================
-- 16. ENTITY_ALIAS 实体别名
-- ============================================================
CREATE TABLE IF NOT EXISTS entity_alias (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL,
    alias_type VARCHAR NOT NULL, -- "Canonical", "Alias", "Title", "HistoricalName"
    alias VARCHAR NOT NULL,
    valid_from_scene_id VARCHAR,
    valid_until_scene_id VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_alias_entity ON entity_alias(entity_id);

-- ============================================================
-- 17. IDENTITY_TIMELINE 身份时间线
-- ============================================================
CREATE TABLE IF NOT EXISTS identity_timeline (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL,
    identity TEXT NOT NULL,
    start_scene_id VARCHAR NOT NULL,
    end_scene_id VARCHAR,
    change_reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_identity_entity ON identity_timeline(entity_id);

-- ============================================================
-- 18. TEST_CASE 测试用例
-- ============================================================
CREATE TABLE IF NOT EXISTS test_case (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    test_type VARCHAR NOT NULL, -- 6 个维度
    preconditions JSON NOT NULL,
    expected_result TEXT NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'Pending', -- Pending, Passed, Failed, Skipped
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_test_project ON test_case(project_id);
CREATE INDEX IF NOT EXISTS idx_test_type ON test_case(test_type);

-- ============================================================
-- 19. TEST_RESULT 测试结果
-- ============================================================
CREATE TABLE IF NOT EXISTS test_result (
    id VARCHAR PRIMARY KEY,
    test_case_id VARCHAR NOT NULL REFERENCES test_case(id),
    passed BOOLEAN NOT NULL,
    actual_result TEXT NOT NULL,
    issues JSON, -- Vec<String>
    model_version VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_test_result_case ON test_result(test_case_id);

-- ============================================================
-- 20. REVISION_PLAN 修订计划
-- ============================================================
CREATE TABLE IF NOT EXISTS revision_plan (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    scene_id VARCHAR NOT NULL,
    original_draft_id VARCHAR NOT NULL,
    issues JSON NOT NULL, -- Vec<RevisionIssue>
    revision_strategy TEXT NOT NULL,
    revision_prompt TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_revision_project ON revision_plan(project_id);
CREATE INDEX IF NOT EXISTS idx_revision_scene ON revision_plan(scene_id);
