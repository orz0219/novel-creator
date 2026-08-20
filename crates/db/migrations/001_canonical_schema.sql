-- ============================================================
-- Narrative Engine - Canonical PostgreSQL Schema
-- ============================================================
-- Replaces migrations 001~012 with a single authoritative schema.
--
-- Design rules:
--   - All PKs: UUID DEFAULT gen_random_uuid()
--   - All timestamps: TIMESTAMPTZ DEFAULT NOW()
--   - All JSON: JSONB
--   - All version columns: INTEGER DEFAULT 1
--   - Entity soft delete via status column
--   - generation_task aligned with Rust GenerationService
--   - system_event (singular) matching Rust code
-- ============================================================

-- ============================================================
-- 1. PROJECT
-- ============================================================
CREATE TABLE project (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    description TEXT,
    language VARCHAR,
    world_setting TEXT,
    system_setting TEXT,
    default_model VARCHAR,
    default_style VARCHAR,
    default_params JSONB,
    config JSONB,
    status VARCHAR NOT NULL DEFAULT 'Concept',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 2. WORLD
-- ============================================================
CREATE TABLE world (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    world_rules TEXT,
    config JSONB,
    is_main BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_world_project ON world(project_id);

-- ============================================================
-- 3. ENTITY_TYPE
-- ============================================================
CREATE TABLE entity_type (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL UNIQUE,
    description TEXT,
    schema JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 4. ENTITY
-- ============================================================
CREATE TABLE entity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    world_id UUID NOT NULL REFERENCES world(id),
    entity_type_id UUID NOT NULL REFERENCES entity_type(id),
    name VARCHAR NOT NULL,
    summary TEXT,
    description TEXT,
    attributes JSONB,
    version INTEGER NOT NULL DEFAULT 1,
    created_by VARCHAR NOT NULL DEFAULT 'system',
    updated_by VARCHAR,
    source_generation_id UUID,
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entity_project ON entity(project_id);
CREATE INDEX idx_entity_type ON entity(entity_type_id);
CREATE INDEX idx_entity_name ON entity(project_id, name);
CREATE INDEX idx_entity_status ON entity(project_id, status);

-- ============================================================
-- 5. RELATION
-- ============================================================
CREATE TABLE relation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    source_entity_id UUID NOT NULL REFERENCES entity(id) ON DELETE RESTRICT,
    target_entity_id UUID NOT NULL REFERENCES entity(id) ON DELETE RESTRICT,
    relation_type VARCHAR NOT NULL,
    description TEXT,
    attributes JSONB,
    valid_from VARCHAR,
    valid_until VARCHAR,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_relation_project ON relation(project_id);
CREATE INDEX idx_relation_source ON relation(source_entity_id);
CREATE INDEX idx_relation_target ON relation(target_entity_id);
CREATE INDEX idx_relation_type ON relation(project_id, relation_type);

-- ============================================================
-- 6. FACT
-- ============================================================
CREATE TABLE fact (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    content TEXT NOT NULL,
    category VARCHAR,
    certainty VARCHAR NOT NULL DEFAULT 'CANON',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fact_project ON fact(project_id);
CREATE INDEX idx_fact_certainty ON fact(project_id, certainty);

-- ============================================================
-- 7. FACT_ENTITY
-- ============================================================
CREATE TABLE fact_entity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fact_id UUID NOT NULL REFERENCES fact(id),
    entity_id UUID NOT NULL REFERENCES entity(id),
    role VARCHAR
);

CREATE INDEX idx_fact_entity_fact ON fact_entity(fact_id);
CREATE INDEX idx_fact_entity_entity ON fact_entity(entity_id);

-- ============================================================
-- 8. EVENT
-- ============================================================
CREATE TABLE event (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    event_type VARCHAR,
    timestamp VARCHAR,
    event_time VARCHAR,
    duration VARCHAR,
    timeline_id VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_event_project ON event(project_id);

-- ============================================================
-- 9. EVENT_ENTITY
-- ============================================================
CREATE TABLE event_entity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES event(id),
    entity_id UUID NOT NULL REFERENCES entity(id),
    role VARCHAR
);

CREATE INDEX idx_event_entity_event ON event_entity(event_id);

-- ============================================================
-- 10. STATE_CHANGE (append-only audit log)
-- ============================================================
CREATE TABLE state_change (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    event_id UUID REFERENCES event(id),
    change_type VARCHAR NOT NULL,
    target_entity_id UUID NOT NULL REFERENCES entity(id),
    state_key VARCHAR NOT NULL,
    old_value JSONB,
    new_value JSONB NOT NULL,
    committed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    committed_by VARCHAR
);

CREATE INDEX idx_state_change_project ON state_change(project_id);
CREATE INDEX idx_state_change_entity ON state_change(target_entity_id);

-- ============================================================
-- 11. CURRENT_STATE (latest projection, one active row per entity+key)
-- ============================================================
CREATE TABLE current_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    entity_id UUID NOT NULL REFERENCES entity(id),
    state_key VARCHAR NOT NULL,
    state_value JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_current_state_entity ON current_state(entity_id);
CREATE INDEX idx_current_state_project ON current_state(project_id, state_key);

-- Only one active (effective_to IS NULL) row per (project, entity, key)
CREATE UNIQUE INDEX idx_current_state_active_unique
    ON current_state(project_id, entity_id, state_key)
    WHERE effective_to IS NULL;

-- ============================================================
-- 12. RESOURCE_STATE
-- ============================================================
CREATE TABLE resource_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    location_id UUID NOT NULL REFERENCES entity(id),
    resource_name VARCHAR NOT NULL,
    quantity DOUBLE PRECISION,
    production_rate DOUBLE PRECISION,
    controlled_by_entity_id UUID REFERENCES entity(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, location_id, resource_name)
);

CREATE INDEX idx_resource_location ON resource_state(location_id);

-- ============================================================
-- 13. NARRATIVE_NODE
-- ============================================================
CREATE TABLE narrative_node (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    world_id UUID NOT NULL,
    node_type VARCHAR NOT NULL,
    parent_id UUID REFERENCES narrative_node(id),
    title VARCHAR NOT NULL,
    description TEXT,
    attributes JSONB,
    sort_order INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_narrative_project ON narrative_node(project_id);
CREATE INDEX idx_narrative_parent ON narrative_node(parent_id);
CREATE INDEX idx_narrative_type ON narrative_node(project_id, node_type);

-- Prevent concurrent sort_order duplicates within same parent
CREATE UNIQUE INDEX idx_narrative_sort_order_unique
    ON narrative_node(project_id, parent_id, sort_order);

-- ============================================================
-- 14. PLOT
-- ============================================================
CREATE TABLE plot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    plot_type VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plot_project ON plot(project_id);

-- ============================================================
-- 15. SCENE
-- ============================================================
CREATE TABLE scene (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    narrative_node_id UUID NOT NULL REFERENCES narrative_node(id),
    objective TEXT,
    conflict TEXT,
    pov_character_id UUID REFERENCES entity(id),
    location_id UUID REFERENCES entity(id),
    time VARCHAR,
    scene_start_time VARCHAR,
    scene_end_time VARCHAR,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scene_node ON scene(narrative_node_id);

-- ============================================================
-- 16. SCENE_ENTITY
-- ============================================================
CREATE TABLE scene_entity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_id UUID NOT NULL REFERENCES scene(id),
    entity_id UUID NOT NULL REFERENCES entity(id),
    role VARCHAR,
    notes TEXT
);

CREATE INDEX idx_scene_entity_scene ON scene_entity(scene_id);
CREATE INDEX idx_scene_entity_entity ON scene_entity(entity_id);

-- ============================================================
-- 17. SCENE_REQUIREMENT
-- ============================================================
CREATE TABLE scene_requirement (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_id UUID NOT NULL REFERENCES scene(id),
    requirement_type VARCHAR NOT NULL,
    content TEXT NOT NULL,
    priority VARCHAR NOT NULL DEFAULT 'Should',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scene_req_scene ON scene_requirement(scene_id);

-- ============================================================
-- 18. CHARACTER_ARC
-- ============================================================
CREATE TABLE character_arc (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    character_id UUID NOT NULL REFERENCES entity(id),
    volume_id UUID REFERENCES narrative_node(id),
    arc_type VARCHAR NOT NULL,
    start_state TEXT,
    mid_state TEXT,
    end_state TEXT,
    key_moments JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_char_arc_character ON character_arc(character_id);
CREATE INDEX idx_char_arc_volume ON character_arc(volume_id);

-- ============================================================
-- 19. KNOWLEDGE_STATE
-- ============================================================
CREATE TABLE knowledge_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    fact_id UUID NOT NULL REFERENCES fact(id),
    subject_type VARCHAR NOT NULL,
    subject_id UUID,
    knows BOOLEAN NOT NULL DEFAULT FALSE,
    knowledge_level VARCHAR NOT NULL DEFAULT 'Unknown',
    source TEXT,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_knowledge_fact ON knowledge_state(fact_id);
CREATE INDEX idx_knowledge_subject ON knowledge_state(subject_type, subject_id);
CREATE INDEX idx_knowledge_project ON knowledge_state(project_id);

-- ============================================================
-- 20. REVELATION
-- ============================================================
CREATE TABLE revelation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    fact_id UUID NOT NULL REFERENCES fact(id),
    scene_id UUID NOT NULL REFERENCES scene(id),
    revelation_method VARCHAR,
    narrative_significance TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_revelation_scene ON revelation(scene_id);
CREATE INDEX idx_revelation_fact ON revelation(fact_id);

-- ============================================================
-- 21. REVELATION_TARGET
-- ============================================================
CREATE TABLE revelation_target (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    revelation_id UUID NOT NULL REFERENCES revelation(id),
    subject_type VARCHAR NOT NULL,
    subject_id UUID,
    knowledge_level VARCHAR NOT NULL DEFAULT 'Complete'
);

CREATE INDEX idx_rev_target_revelation ON revelation_target(revelation_id);

-- ============================================================
-- 22. SKILL
-- ============================================================
CREATE TABLE skill (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    description TEXT,
    skill_type VARCHAR NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    prompt_template TEXT NOT NULL,
    input_schema JSONB,
    output_schema JSONB,
    default_params JSONB,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 23. SKILL_VERSION
-- ============================================================
CREATE TABLE skill_version (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id UUID NOT NULL REFERENCES skill(id),
    version INTEGER NOT NULL,
    prompt_template TEXT NOT NULL,
    input_schema JSONB,
    output_schema JSONB,
    default_params JSONB,
    changelog TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_skill_ver_skill ON skill_version(skill_id);

-- ============================================================
-- 24. GENERATION_TASK (aligned with Rust GenerationService)
-- ============================================================
CREATE TABLE generation_task (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    task_type VARCHAR NOT NULL,
    target_id UUID,
    model VARCHAR,
    parameters JSONB,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    result JSONB,
    context_tokens INTEGER,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gen_task_project ON generation_task(project_id);
CREATE INDEX idx_gen_task_status ON generation_task(status);

-- ============================================================
-- 25. GENERATION_RUN
-- ============================================================
CREATE TABLE generation_run (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    task_id UUID NOT NULL REFERENCES generation_task(id),
    context_snapshot_id UUID,
    llm_model VARCHAR NOT NULL,
    provider VARCHAR,
    prompt_sent TEXT NOT NULL,
    response_received TEXT NOT NULL,
    token_usage JSONB,
    latency_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gen_run_task ON generation_run(task_id);

-- ============================================================
-- 26. CONTEXT_SNAPSHOT
-- ============================================================
CREATE TABLE context_snapshot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    scene_id UUID NOT NULL,
    token_budget INTEGER NOT NULL,
    l0_essential JSONB,
    l1_scene_relevant JSONB,
    l2_recent_history JSONB,
    l3_narrative_context JSONB,
    l4_character_knowledge JSONB,
    l5_world_background JSONB,
    l6_optional_supplement JSONB,
    actual_tokens INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ctx_snap_scene ON context_snapshot(scene_id);

-- ============================================================
-- 27. PROPOSED_CHANGE
-- ============================================================
CREATE TABLE proposed_change (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    task_id UUID NOT NULL REFERENCES generation_task(id),
    change_type VARCHAR NOT NULL,
    target_entity_id UUID NOT NULL,
    description TEXT NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    content_hash VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_prop_change_project ON proposed_change(project_id);
CREATE INDEX idx_prop_change_task ON proposed_change(task_id);
CREATE INDEX idx_prop_change_status ON proposed_change(status);

-- ============================================================
-- 28. VALIDATION_RUN
-- ============================================================
CREATE TABLE validation_run (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    task_id UUID NOT NULL REFERENCES generation_task(id),
    changes_validated INTEGER NOT NULL DEFAULT 0,
    changes_approved INTEGER NOT NULL DEFAULT 0,
    changes_rejected INTEGER NOT NULL DEFAULT 0,
    status VARCHAR NOT NULL DEFAULT 'Running',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_val_run_task ON validation_run(task_id);

-- ============================================================
-- 29. VALIDATION_ISSUE
-- ============================================================
CREATE TABLE validation_issue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    validation_run_id UUID NOT NULL REFERENCES validation_run(id),
    proposed_change_id UUID NOT NULL REFERENCES proposed_change(id),
    issue_type VARCHAR NOT NULL,
    severity VARCHAR NOT NULL DEFAULT 'Warning',
    message TEXT NOT NULL,
    suggestion TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_val_issue_run ON validation_issue(validation_run_id);

-- ============================================================
-- 30. SCENE_DOCUMENT
-- ============================================================
CREATE TABLE scene_document (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_id UUID NOT NULL REFERENCES scene(id),
    generation_task_id UUID REFERENCES generation_task(id),
    content TEXT NOT NULL,
    word_count INTEGER,
    version INTEGER NOT NULL DEFAULT 1,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scene_doc_scene ON scene_document(scene_id);

-- ============================================================
-- 31. TIMELINE_EVENT
-- ============================================================
CREATE TABLE timeline_event (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    event_id UUID REFERENCES event(id),
    scene_id UUID REFERENCES scene(id),
    narrative_node_id UUID REFERENCES narrative_node(id),
    sort_key VARCHAR NOT NULL,
    label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_timeline_project ON timeline_event(project_id);
CREATE INDEX idx_timeline_sort ON timeline_event(project_id, sort_key);

-- ============================================================
-- 32. CHARACTER_PROFILE
-- ============================================================
CREATE TABLE character_profile (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    real_name VARCHAR,
    nickname VARCHAR,
    age VARCHAR,
    gender VARCHAR,
    identity VARCHAR,
    appearance TEXT,
    background TEXT,
    social_status VARCHAR,
    core_personality TEXT,
    values TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_char_profile_entity ON character_profile(entity_id);

-- ============================================================
-- 33. CHARACTER_STATE
-- ============================================================
CREATE TABLE character_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    location VARCHAR,
    health VARCHAR,
    cultivation VARCHAR,
    money VARCHAR,
    wanted BOOLEAN NOT NULL DEFAULT FALSE,
    extra JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_char_state_entity ON character_state(entity_id);

-- ============================================================
-- 34. CHARACTER_GOAL
-- ============================================================
CREATE TABLE character_goal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    long_term TEXT,
    current TEXT,
    immediate TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_char_goal_entity ON character_goal(entity_id);

-- ============================================================
-- 35. CHARACTER_TRAIT
-- ============================================================
CREATE TABLE character_trait (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    trait_type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    parent_trait_id UUID,
    intensity INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_char_trait_entity ON character_trait(entity_id);
CREATE INDEX idx_char_trait_parent ON character_trait(parent_trait_id);

-- ============================================================
-- 36. FACTION_PROFILE
-- ============================================================
CREATE TABLE faction_profile (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    goals TEXT,
    leader VARCHAR,
    values TEXT,
    resources TEXT,
    territory TEXT,
    members TEXT,
    enemies TEXT,
    allies TEXT,
    internal_conflicts TEXT,
    secrets TEXT,
    modus_operandi TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_faction_profile_entity ON faction_profile(entity_id);

-- ============================================================
-- 37. LOCATION_IDENTITY
-- ============================================================
CREATE TABLE location_identity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    location_type VARCHAR,
    size VARCHAR,
    climate VARCHAR,
    era VARCHAR,
    accessibility TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_identity_entity ON location_identity(entity_id);

-- ============================================================
-- 38. LOCATION_GEOGRAPHY
-- ============================================================
CREATE TABLE location_geography (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    terrain TEXT,
    climate TEXT,
    natural_resources TEXT,
    hazards TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 39. LOCATION_FACILITIES
-- ============================================================
CREATE TABLE location_facilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    facility_type VARCHAR,
    description TEXT,
    controlled_by VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_facilities_entity ON location_facilities(entity_id);

-- ============================================================
-- 40. LOCATION_RULES
-- ============================================================
CREATE TABLE location_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    rule_text TEXT NOT NULL,
    rule_type VARCHAR,
    enforced_by VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_rules_entity ON location_rules(entity_id);

-- ============================================================
-- 41. LOCATION_THREATS
-- ============================================================
CREATE TABLE location_threats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    threat_type VARCHAR,
    severity VARCHAR,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_threats_entity ON location_threats(entity_id);

-- ============================================================
-- 42. LOCATION_SECRETS
-- ============================================================
CREATE TABLE location_secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    secret_text TEXT NOT NULL,
    discovered_by VARCHAR,
    narrative_importance VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_secrets_entity ON location_secrets(entity_id);

-- ============================================================
-- 43. LOCATION_NARRATIVE_HOOKS
-- ============================================================
CREATE TABLE location_narrative_hooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    hook_text TEXT NOT NULL,
    hook_type VARCHAR,
    related_entities JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_hooks_entity ON location_narrative_hooks(entity_id);

-- ============================================================
-- 44. STANDARD_RELATION_TYPE (seed data)
-- ============================================================
CREATE TABLE standard_relation_type (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL UNIQUE,
    description TEXT,
    category VARCHAR,
    is_symmetric BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO standard_relation_type (name, description, category, is_symmetric) VALUES
    ('MEMBER_OF', '是成员', '组织', FALSE),
    ('CONTROLS', '控制', '权力', FALSE),
    ('ENEMY_OF', '敌人', '关系', TRUE),
    ('FRIEND_OF', '朋友', '关系', TRUE),
    ('ALLY_OF', '盟友', '关系', TRUE),
    ('PARENT_OF', '父母', '家族', FALSE),
    ('CHILD_OF', '子女', '家族', FALSE),
    ('SPOUSE_OF', '配偶', '家族', TRUE),
    ('LOCATED_AT', '位于', '位置', FALSE),
    ('CONNECTED_TO', '连接', '位置', TRUE),
    ('CONTAINS', '包含', '位置', FALSE),
    ('OWNS', '拥有', '资源', FALSE),
    ('PRODUCES', '产出', '资源', FALSE),
    ('TRAINS', '训练', '关系', FALSE),
    ('MENTORS', '指导', '关系', FALSE),
    ('SERVES', '服务', '关系', FALSE),
    ('OPPOSES', '反对', '关系', FALSE),
    ('PROTECTS', '保护', '关系', FALSE),
    ('HUNTS', '追捕', '关系', FALSE),
    ('LOVES', '爱慕', '关系', TRUE)
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- 45. AUTHORIAL_INTENT
-- ============================================================
CREATE TABLE authorial_intent (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    target_id UUID,
    target_type VARCHAR,
    pacing VARCHAR,
    emotional_tone VARCHAR,
    focus TEXT,
    avoid TEXT,
    perspective VARCHAR,
    narrative_distance VARCHAR,
    additional_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_authorial_project ON authorial_intent(project_id);
CREATE INDEX idx_authorial_target ON authorial_intent(target_id);

-- ============================================================
-- 46. STORYLINE (002)
-- ============================================================
CREATE TABLE storyline (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL DEFAULT 'Active',
    importance VARCHAR NOT NULL DEFAULT 'Normal',
    created_volume_id UUID,
    resolved_volume_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_storyline_project ON storyline(project_id);

-- ============================================================
-- 47. STORYLINE_SCENE
-- ============================================================
CREATE TABLE storyline_scene (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    storyline_id UUID NOT NULL REFERENCES storyline(id),
    scene_id UUID NOT NULL,
    significance VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_storyline_scene_storyline ON storyline_scene(storyline_id);
CREATE INDEX idx_storyline_scene_scene ON storyline_scene(scene_id);

-- ============================================================
-- 48. FACT_VISIBILITY
-- ============================================================
CREATE TABLE fact_visibility (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    fact_id UUID NOT NULL,
    subject_type VARCHAR NOT NULL,
    subject_id UUID,
    visibility_level VARCHAR NOT NULL DEFAULT 'Hidden',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fact_visibility_project ON fact_visibility(project_id);
CREATE INDEX idx_fact_visibility_fact ON fact_visibility(fact_id);

-- ============================================================
-- 49. APPROVAL_RECORD
-- ============================================================
CREATE TABLE approval_record (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    target_type VARCHAR NOT NULL,
    target_id UUID NOT NULL,
    proposed_by VARCHAR NOT NULL,
    proposal_content JSONB,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    reviewer_id UUID,
    reviewer_comment TEXT,
    content_hash VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ
);

CREATE INDEX idx_approval_project ON approval_record(project_id);
CREATE INDEX idx_approval_target ON approval_record(target_id);
CREATE INDEX idx_approval_content_hash ON approval_record(content_hash);

-- ============================================================
-- 50. FORESHADOWING (003)
-- ============================================================
CREATE TABLE foreshadowing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    storyline_id UUID,
    name VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL DEFAULT 'Planned',
    importance VARCHAR NOT NULL DEFAULT 'Normal',
    hint_level VARCHAR NOT NULL DEFAULT 'Subtle',
    introduced_at VARCHAR,
    expected_reveal_at VARCHAR,
    actual_reveal_at VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_foreshadowing_project ON foreshadowing(project_id);
CREATE INDEX idx_foreshadowing_status ON foreshadowing(status);

-- ============================================================
-- 51. CAUSAL_RELATION
-- ============================================================
CREATE TABLE causal_relation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    cause_event_id UUID NOT NULL,
    effect_event_id UUID NOT NULL,
    relation_type VARCHAR NOT NULL DEFAULT 'DirectCause',
    strength VARCHAR NOT NULL DEFAULT 'Strong',
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_causal_project ON causal_relation(project_id);
CREATE INDEX idx_causal_cause ON causal_relation(cause_event_id);
CREATE INDEX idx_causal_effect ON causal_relation(effect_event_id);

-- ============================================================
-- 52. READER_KNOWLEDGE
-- ============================================================
CREATE TABLE reader_knowledge (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    fact_id UUID NOT NULL,
    knowledge_level VARCHAR NOT NULL DEFAULT 'Unknown',
    source_scene_id UUID,
    confidence VARCHAR NOT NULL DEFAULT 'Certain',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reader_knowledge_project ON reader_knowledge(project_id);
CREATE INDEX idx_reader_knowledge_fact ON reader_knowledge(fact_id);

-- ============================================================
-- 53. SCENE_CONTRACT
-- ============================================================
CREATE TABLE scene_contract (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_id UUID NOT NULL,
    required_events JSONB,
    forbidden_events JSONB,
    required_characters JSONB,
    required_facts JSONB,
    reader_learns JSONB,
    protagonist_learns JSONB,
    world_changes JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scene_contract_scene ON scene_contract(scene_id);

-- ============================================================
-- 54. QUALITY_SCORE
-- ============================================================
CREATE TABLE quality_score (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID NOT NULL,
    run_id UUID,
    continuity_score INTEGER,
    character_score INTEGER,
    plot_score INTEGER,
    knowledge_score INTEGER,
    world_score INTEGER,
    style_score INTEGER,
    overall_score INTEGER,
    issues JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quality_score_project ON quality_score(project_id);
CREATE INDEX idx_quality_score_scene ON quality_score(scene_id);

-- ============================================================
-- 55. WORLD_BRANCH (004)
-- ============================================================
CREATE TABLE world_branch (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    parent_branch_id UUID,
    is_main BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_world_branch_project ON world_branch(project_id);

-- ============================================================
-- 56. NARRATIVE_BRANCH
-- ============================================================
CREATE TABLE narrative_branch (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    parent_branch_id UUID,
    fork_point_scene_id UUID,
    is_main BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_narrative_branch_project ON narrative_branch(project_id);

-- ============================================================
-- 57. PLOT_REPAIR
-- ============================================================
CREATE TABLE plot_repair (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID NOT NULL,
    issue_description TEXT NOT NULL,
    repair_suggestion TEXT NOT NULL,
    repair_type VARCHAR NOT NULL DEFAULT 'Automatic',
    status VARCHAR NOT NULL DEFAULT 'Pending',
    applied_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plot_repair_project ON plot_repair(project_id);
CREATE INDEX idx_plot_repair_scene ON plot_repair(scene_id);

-- ============================================================
-- 58. CANON_RULE (005)
-- ============================================================
CREATE TABLE canon_rule (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    world_id UUID NOT NULL,
    rule_level VARCHAR NOT NULL,
    rule_content TEXT NOT NULL,
    affected_scope VARCHAR NOT NULL,
    enforcement VARCHAR NOT NULL,
    constraints JSONB,
    source VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_canon_rule_project ON canon_rule(project_id);
CREATE INDEX idx_canon_rule_world ON canon_rule(world_id);
CREATE INDEX idx_canon_rule_level ON canon_rule(rule_level);

-- ============================================================
-- 59. BELIEF
-- ============================================================
CREATE TABLE belief (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    character_id UUID NOT NULL,
    belief_content TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    source VARCHAR,
    source_scene_id UUID,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_belief_project ON belief(project_id);
CREATE INDEX idx_belief_character ON belief(character_id);

-- ============================================================
-- 60. CHARACTER_MEMORY
-- ============================================================
CREATE TABLE character_memory (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    character_id UUID NOT NULL,
    memory_content TEXT NOT NULL,
    emotional_impact VARCHAR,
    scene_id UUID,
    importance INTEGER NOT NULL DEFAULT 5,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_memory_project ON character_memory(project_id);
CREATE INDEX idx_memory_character ON character_memory(character_id);

-- ============================================================
-- 61. CHARACTER_GOAL_MIND
-- ============================================================
CREATE TABLE character_goal_mind (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    character_id UUID NOT NULL,
    goal_content TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 5,
    status VARCHAR NOT NULL DEFAULT 'Active',
    source VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_goal_mind_project ON character_goal_mind(project_id);
CREATE INDEX idx_goal_mind_character ON character_goal_mind(character_id);

-- ============================================================
-- 62. CHARACTER_FEAR
-- ============================================================
CREATE TABLE character_fear (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    character_id UUID NOT NULL,
    fear_content TEXT NOT NULL,
    intensity INTEGER NOT NULL DEFAULT 5,
    source VARCHAR,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fear_project ON character_fear(project_id);
CREATE INDEX idx_fear_character ON character_fear(character_id);

-- ============================================================
-- 63. EMOTION_STATE
-- ============================================================
CREATE TABLE emotion_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    character_id UUID NOT NULL,
    emotion_type VARCHAR NOT NULL,
    intensity INTEGER NOT NULL DEFAULT 50,
    decay_rate DOUBLE PRECISION NOT NULL DEFAULT 0.1,
    trigger_scene_id UUID,
    trigger_description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_emotion_project ON emotion_state(project_id);
CREATE INDEX idx_emotion_character ON emotion_state(character_id);

-- ============================================================
-- 64. NARRATIVE_STATE (four-dimensional)
-- ============================================================
CREATE TABLE narrative_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    state_dimension VARCHAR NOT NULL,
    state_key VARCHAR NOT NULL,
    state_value JSONB NOT NULL,
    scene_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_narrative_state_project ON narrative_state(project_id);
CREATE INDEX idx_narrative_state_dimension ON narrative_state(state_dimension);
CREATE INDEX idx_narrative_state_key ON narrative_state(state_key);

-- ============================================================
-- 65. SCENE_LEDGER
-- ============================================================
CREATE TABLE scene_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID NOT NULL,
    events JSONB,
    gains JSONB,
    losses JSONB,
    relationship_changes JSONB,
    knowledge_changes JSONB,
    world_changes JSONB,
    foreshadowing_mentions JSONB,
    storyline_progress JSONB,
    character_growth JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ledger_project ON scene_ledger(project_id);
CREATE INDEX idx_ledger_scene ON scene_ledger(scene_id);

-- ============================================================
-- 66. DECISION_TRACE
-- ============================================================
CREATE TABLE decision_trace (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID NOT NULL,
    character_id UUID NOT NULL,
    decision TEXT NOT NULL,
    factors JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_decision_project ON decision_trace(project_id);
CREATE INDEX idx_decision_scene ON decision_trace(scene_id);
CREATE INDEX idx_decision_character ON decision_trace(character_id);

-- ============================================================
-- 67. STATE_SNAPSHOT (for rollback)
-- ============================================================
CREATE TABLE state_snapshot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID NOT NULL,
    state_before JSONB NOT NULL,
    changes JSONB NOT NULL,
    state_after JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_snapshot_project ON state_snapshot(project_id);
CREATE INDEX idx_snapshot_scene ON state_snapshot(scene_id);

-- ============================================================
-- 68. KNOWLEDGE_GAP
-- ============================================================
CREATE TABLE knowledge_gap (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    gap_type VARCHAR NOT NULL,
    description TEXT NOT NULL,
    importance VARCHAR NOT NULL DEFAULT 'MEDIUM',
    required_by_scene_id UUID,
    status VARCHAR NOT NULL DEFAULT 'Open',
    designer_skill_hint VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gap_project ON knowledge_gap(project_id);
CREATE INDEX idx_gap_status ON knowledge_gap(status);

-- ============================================================
-- 69. CHAPTER_SUMMARY
-- ============================================================
CREATE TABLE chapter_summary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    chapter_id UUID NOT NULL,
    summary TEXT NOT NULL,
    key_events JSONB,
    involved_characters JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chapter_summary_project ON chapter_summary(project_id);

-- ============================================================
-- 70. ARC_SUMMARY
-- ============================================================
CREATE TABLE arc_summary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    arc_id UUID NOT NULL,
    summary TEXT NOT NULL,
    key_turning_points JSONB,
    status VARCHAR NOT NULL DEFAULT 'Active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_arc_summary_project ON arc_summary(project_id);

-- ============================================================
-- 71. VOLUME_SUMMARY
-- ============================================================
CREATE TABLE volume_summary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    volume_id UUID NOT NULL,
    summary TEXT NOT NULL,
    character_changes JSONB,
    world_changes JSONB,
    foreshadowing_progress JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_volume_summary_project ON volume_summary(project_id);

-- ============================================================
-- 72. GLOBAL_STORY_STATE
-- ============================================================
CREATE TABLE global_story_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    current_progress TEXT NOT NULL,
    open_foreshadowing JSONB,
    open_storylines JSONB,
    world_state_summary TEXT NOT NULL,
    character_state_summary TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_global_state_project ON global_story_state(project_id);

-- ============================================================
-- 73. ENTITY_ALIAS
-- ============================================================
CREATE TABLE entity_alias (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL,
    alias_type VARCHAR NOT NULL,
    alias VARCHAR NOT NULL,
    valid_from_scene_id UUID,
    valid_until_scene_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alias_entity ON entity_alias(entity_id);

-- ============================================================
-- 74. IDENTITY_TIMELINE
-- ============================================================
CREATE TABLE identity_timeline (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL,
    identity TEXT NOT NULL,
    start_scene_id UUID NOT NULL,
    end_scene_id UUID,
    change_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_identity_entity ON identity_timeline(entity_id);

-- ============================================================
-- 75. TEST_CASE
-- ============================================================
CREATE TABLE test_case (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    test_type VARCHAR NOT NULL,
    preconditions JSONB NOT NULL,
    expected_result TEXT NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_test_project ON test_case(project_id);
CREATE INDEX idx_test_type ON test_case(test_type);

-- ============================================================
-- 76. TEST_RESULT
-- ============================================================
CREATE TABLE test_result (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    test_case_id UUID NOT NULL REFERENCES test_case(id),
    passed BOOLEAN NOT NULL,
    actual_result TEXT NOT NULL,
    issues JSONB,
    model_version VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_test_result_case ON test_result(test_case_id);

-- ============================================================
-- 77. REVISION_PLAN
-- ============================================================
CREATE TABLE revision_plan (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID NOT NULL,
    original_draft_id UUID NOT NULL,
    issues JSONB NOT NULL,
    revision_strategy TEXT NOT NULL,
    revision_prompt TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_revision_project ON revision_plan(project_id);
CREATE INDEX idx_revision_scene ON revision_plan(scene_id);

-- ============================================================
-- 78. STORY_CONTRACT (006)
-- ============================================================
CREATE TABLE story_contract (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    narrative_node_id UUID NOT NULL,
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_story_contract_project ON story_contract(project_id);
CREATE INDEX idx_story_contract_narrative ON story_contract(narrative_node_id);

-- ============================================================
-- 79-87. LOCATION & NARRATIVE TABLES (007 schema alignment)
-- ============================================================

-- LOCATION_PROFILE
CREATE TABLE location_profile (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    geography TEXT,
    appearance TEXT,
    population TEXT,
    economy TEXT,
    rules TEXT,
    history TEXT,
    narrative_usage TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_profile_entity ON location_profile(entity_id);

-- LOCATION_FACILITY (007 version, distinct from location_facilities)
CREATE TABLE location_facility (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    facility_type VARCHAR,
    description TEXT,
    controlled_by_entity_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_facility_entity ON location_facility(entity_id);

-- LOCATION_THREAT (007 version)
CREATE TABLE location_threat (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    threat_type VARCHAR,
    description TEXT,
    severity VARCHAR DEFAULT 'Normal',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_threat_entity ON location_threat(entity_id);

-- LOCATION_SECRET (007 version)
CREATE TABLE location_secret (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entity(id),
    description TEXT NOT NULL,
    discovered BOOLEAN NOT NULL DEFAULT FALSE,
    discovered_by_entity_id UUID,
    discovered_at_scene_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_secret_entity ON location_secret(entity_id);

-- LOCATION_CONNECTION (travel graph)
CREATE TABLE location_connection (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_location_id UUID NOT NULL REFERENCES entity(id),
    target_location_id UUID NOT NULL REFERENCES entity(id),
    connection_type VARCHAR NOT NULL DEFAULT 'path',
    travel_time VARCHAR,
    travel_description TEXT,
    is_bidirectional BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loc_conn_source ON location_connection(source_location_id);
CREATE INDEX idx_loc_conn_target ON location_connection(target_location_id);

-- NARRATIVE_BUDGET (word count allocation)
CREATE TABLE narrative_budget (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    narrative_node_id UUID NOT NULL,
    allocated_words INTEGER NOT NULL DEFAULT 0,
    used_words INTEGER NOT NULL DEFAULT 0,
    action_ratio DOUBLE PRECISION,
    dialogue_ratio DOUBLE PRECISION,
    description_ratio DOUBLE PRECISION,
    exposition_ratio DOUBLE PRECISION,
    internal_monologue_ratio DOUBLE PRECISION,
    pacing_warning_threshold DOUBLE PRECISION DEFAULT 0.9,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_narr_budget_project ON narrative_budget(project_id);
CREATE INDEX idx_narr_budget_node ON narrative_budget(narrative_node_id);

-- NOVEL_STATE_SNAPSHOT (full novel state at a point in time)
CREATE TABLE novel_state_snapshot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    scene_id UUID,
    story_time VARCHAR,
    world_summary TEXT,
    main_character_state TEXT,
    current_location VARCHAR,
    active_threads_count INTEGER DEFAULT 0,
    unresolved_foreshadows_count INTEGER DEFAULT 0,
    known_characters_count INTEGER DEFAULT 0,
    known_locations_count INTEGER DEFAULT 0,
    current_volume_id UUID,
    current_arc_id UUID,
    state_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_novel_snap_project ON novel_state_snapshot(project_id);
CREATE INDEX idx_novel_snap_scene ON novel_state_snapshot(scene_id);

-- NARRATIVE_THREAD (extended storyline with progression)
CREATE TABLE narrative_thread (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    storyline_id UUID,
    name VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL DEFAULT 'Active',
    importance VARCHAR NOT NULL DEFAULT 'Normal',
    current_stage TEXT,
    recent_progress TEXT,
    next_step TEXT,
    goal TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_narr_thread_project ON narrative_thread(project_id);
CREATE INDEX idx_narr_thread_storyline ON narrative_thread(storyline_id);

-- NARRATIVE_THREAD_PARTICIPANT
CREATE TABLE narrative_thread_participant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id UUID NOT NULL REFERENCES narrative_thread(id),
    entity_id UUID NOT NULL REFERENCES entity(id),
    role VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_narr_thread_part_thread ON narrative_thread_participant(thread_id);

-- ============================================================
-- 88-90. AGENT SYSTEM TABLES (008)
-- ============================================================

-- MEMORIES (cross-session memory)
CREATE TABLE memories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    memory_type VARCHAR NOT NULL DEFAULT 'general',
    content TEXT NOT NULL,
    importance DOUBLE PRECISION DEFAULT 0.5,
    source VARCHAR,
    embedding_vector_id VARCHAR,
    metadata JSONB DEFAULT '{}'::jsonb,
    access_count INTEGER DEFAULT 0,
    last_accessed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_memories_project ON memories(project_id);
CREATE INDEX idx_memories_type ON memories(project_id, memory_type);
CREATE INDEX idx_memories_importance ON memories(project_id, importance DESC);

-- AGENT_RUNS (agent execution history)
CREATE TABLE agent_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    agent_type VARCHAR NOT NULL,
    task_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    input JSONB DEFAULT '{}'::jsonb,
    output JSONB DEFAULT '{}'::jsonb,
    error_message TEXT,
    model VARCHAR,
    provider VARCHAR,
    context_snapshot_id UUID,
    tokens_used INTEGER,
    duration_ms BIGINT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_agent_runs_project ON agent_runs(project_id);
CREATE INDEX idx_agent_runs_status ON agent_runs(project_id, status);
CREATE INDEX idx_agent_runs_type ON agent_runs(project_id, agent_type);

-- SYSTEM_EVENT (singular, matching Rust code)
CREATE TABLE system_event (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID REFERENCES project(id),
    event_type VARCHAR NOT NULL,
    entity_type VARCHAR,
    entity_id UUID,
    actor VARCHAR NOT NULL DEFAULT 'system',
    description TEXT,
    data JSONB,
    source VARCHAR,
    old_value JSONB,
    new_value JSONB,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_system_event_project ON system_event(project_id);
CREATE INDEX idx_system_event_type ON system_event(project_id, event_type);
CREATE INDEX idx_system_event_entity ON system_event(entity_type, entity_id);
CREATE INDEX idx_system_event_time ON system_event(project_id, created_at DESC);

-- ============================================================
-- 91. EVENT_OUTBOX (011)
-- ============================================================
CREATE TABLE event_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES project(id),
    event_type VARCHAR NOT NULL,
    aggregate_type VARCHAR NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE INDEX idx_outbox_status ON event_outbox(status);
CREATE INDEX idx_outbox_project ON event_outbox(project_id);
CREATE INDEX idx_outbox_created ON event_outbox(created_at);

-- ============================================================
-- MIGRATION TRACKING
-- ============================================================
CREATE TABLE _migrations (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
