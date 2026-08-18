-- Narrative Engine V2 Schema
-- DuckDB Migration
-- 扩展至约 50 张核心表

-- ============================================================
-- 1. PROJECT 项目
-- ============================================================
CREATE TABLE IF NOT EXISTS project (
    id VARCHAR PRIMARY KEY,
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- 2. WORLD 世界（V2 新增）
-- ============================================================
CREATE TABLE IF NOT EXISTS world (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    world_rules TEXT,
    config JSONB,
    is_main BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_world_project ON world(project_id);

-- ============================================================
-- 3. ENTITY_TYPE 实体类型
-- ============================================================
CREATE TABLE IF NOT EXISTS entity_type (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    schema JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- 3. ENTITY 实体
-- ============================================================
CREATE TABLE IF NOT EXISTS entity (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    world_id VARCHAR NOT NULL,
    entity_type_id VARCHAR NOT NULL REFERENCES entity_type(id),
    name VARCHAR NOT NULL,
    summary TEXT,
    description TEXT,
    attributes JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_entity_project ON entity(project_id);
CREATE INDEX IF NOT EXISTS idx_entity_type ON entity(entity_type_id);
CREATE INDEX IF NOT EXISTS idx_entity_name ON entity(project_id, name);

-- ============================================================
-- 4. RELATION 关系
-- ============================================================
CREATE TABLE IF NOT EXISTS relation (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    source_entity_id VARCHAR NOT NULL REFERENCES entity(id),
    target_entity_id VARCHAR NOT NULL REFERENCES entity(id),
    relation_type VARCHAR NOT NULL,
    description TEXT,
    attributes JSONB,
    valid_from VARCHAR,
    valid_until VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_relation_project ON relation(project_id);
CREATE INDEX IF NOT EXISTS idx_relation_source ON relation(source_entity_id);
CREATE INDEX IF NOT EXISTS idx_relation_target ON relation(target_entity_id);
CREATE INDEX IF NOT EXISTS idx_relation_type ON relation(project_id, relation_type);

-- ============================================================
-- 5. FACT 事实
-- ============================================================
CREATE TABLE IF NOT EXISTS fact (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    content TEXT NOT NULL,
    category VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_fact_project ON fact(project_id);

-- ============================================================
-- 6. FACT_ENTITY 事实-实体关联
-- ============================================================
CREATE TABLE IF NOT EXISTS fact_entity (
    id VARCHAR PRIMARY KEY,
    fact_id VARCHAR NOT NULL REFERENCES fact(id),
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    role VARCHAR
);

CREATE INDEX IF NOT EXISTS idx_fact_entity_fact ON fact_entity(fact_id);
CREATE INDEX IF NOT EXISTS idx_fact_entity_entity ON fact_entity(entity_id);

-- ============================================================
-- 7. EVENT 事件
-- ============================================================
CREATE TABLE IF NOT EXISTS event (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    event_type VARCHAR,
    timestamp VARCHAR,
    event_time VARCHAR,
    duration VARCHAR,
    timeline_id VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_event_project ON event(project_id);

-- ============================================================
-- 8. EVENT_ENTITY 事件-实体关联
-- ============================================================
CREATE TABLE IF NOT EXISTS event_entity (
    id VARCHAR PRIMARY KEY,
    event_id VARCHAR NOT NULL REFERENCES event(id),
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    role VARCHAR
);

CREATE INDEX IF NOT EXISTS idx_event_entity_event ON event_entity(event_id);

-- ============================================================
-- 9. STATE_CHANGE 状态变更
-- ============================================================
CREATE TABLE IF NOT EXISTS state_change (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    event_id VARCHAR REFERENCES event(id),
    change_type VARCHAR NOT NULL,
    target_entity_id VARCHAR NOT NULL REFERENCES entity(id),
    state_key VARCHAR NOT NULL,
    old_value JSONB,
    new_value JSONB NOT NULL,
    committed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    committed_by VARCHAR
);

CREATE INDEX IF NOT EXISTS idx_state_change_project ON state_change(project_id);
CREATE INDEX IF NOT EXISTS idx_state_change_entity ON state_change(target_entity_id);

-- ============================================================
-- 10. CURRENT_STATE 当前状态
-- ============================================================
CREATE TABLE IF NOT EXISTS current_state (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    state_key VARCHAR NOT NULL,
    state_value JSONB NOT NULL,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    effective_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_current_state_entity ON current_state(entity_id);
CREATE INDEX IF NOT EXISTS idx_current_state_project ON current_state(project_id, state_key);

-- ============================================================
-- 11. RESOURCE_STATE 资源状态
-- ============================================================
CREATE TABLE IF NOT EXISTS resource_state (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    location_id VARCHAR NOT NULL REFERENCES entity(id),
    resource_name VARCHAR NOT NULL,
    quantity DOUBLE PRECISION,
    production_rate DOUBLE PRECISION,
    controlled_by_entity_id VARCHAR REFERENCES entity(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_resource_location ON resource_state(location_id);

-- ============================================================
-- 12. NARRATIVE_NODE 叙事节点
-- ============================================================
CREATE TABLE IF NOT EXISTS narrative_node (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    world_id VARCHAR NOT NULL,
    node_type VARCHAR NOT NULL,
    parent_id VARCHAR REFERENCES narrative_node(id),
    title VARCHAR NOT NULL,
    description TEXT,
    attributes JSONB,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_narrative_project ON narrative_node(project_id);
CREATE INDEX IF NOT EXISTS idx_narrative_parent ON narrative_node(parent_id);
CREATE INDEX IF NOT EXISTS idx_narrative_type ON narrative_node(project_id, node_type);

-- ============================================================
-- 13. PLOT 情节（可选，用于更复杂的叙事结构）
-- ============================================================
CREATE TABLE IF NOT EXISTS plot (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    description TEXT,
    plot_type VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_plot_project ON plot(project_id);

-- ============================================================
-- 14. SCENE 场景（NARRATIVE_NODE 的具体化视图）
-- ============================================================
CREATE TABLE IF NOT EXISTS scene (
    id VARCHAR PRIMARY KEY,
    narrative_node_id VARCHAR NOT NULL REFERENCES narrative_node(id),
    objective TEXT,
    conflict TEXT,
    pov_character_id VARCHAR REFERENCES entity(id),
    location_id VARCHAR REFERENCES entity(id),
    time VARCHAR,
    scene_start_time VARCHAR,
    scene_end_time VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_scene_node ON scene(narrative_node_id);

-- ============================================================
-- 15. SCENE_ENTITY 场景-实体关联
-- ============================================================
CREATE TABLE IF NOT EXISTS scene_entity (
    id VARCHAR PRIMARY KEY,
    scene_id VARCHAR NOT NULL REFERENCES scene(id),
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    role VARCHAR,
    notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_scene_entity_scene ON scene_entity(scene_id);
CREATE INDEX IF NOT EXISTS idx_scene_entity_entity ON scene_entity(entity_id);

-- ============================================================
-- 16. SCENE_REQUIREMENT 场景需求
-- ============================================================
CREATE TABLE IF NOT EXISTS scene_requirement (
    id VARCHAR PRIMARY KEY,
    scene_id VARCHAR NOT NULL REFERENCES scene(id),
    requirement_type VARCHAR NOT NULL,
    content TEXT NOT NULL,
    priority VARCHAR NOT NULL DEFAULT 'Should',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_scene_req_scene ON scene_requirement(scene_id);

-- ============================================================
-- 17. CHARACTER_ARC 角色弧线
-- ============================================================
CREATE TABLE IF NOT EXISTS character_arc (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    character_id VARCHAR NOT NULL REFERENCES entity(id),
    volume_id VARCHAR REFERENCES narrative_node(id),
    arc_type VARCHAR NOT NULL,
    start_state TEXT,
    mid_state TEXT,
    end_state TEXT,
    key_moments JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_arc_character ON character_arc(character_id);
CREATE INDEX IF NOT EXISTS idx_char_arc_volume ON character_arc(volume_id);

-- ============================================================
-- 18. KNOWLEDGE_STATE 知识状态
-- ============================================================
CREATE TABLE IF NOT EXISTS knowledge_state (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    fact_id VARCHAR NOT NULL REFERENCES fact(id),
    subject_type VARCHAR NOT NULL,
    subject_id UUID,
    knows BOOLEAN NOT NULL DEFAULT FALSE,
    knowledge_level VARCHAR NOT NULL DEFAULT 'Unknown',
    source TEXT,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    effective_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_knowledge_fact ON knowledge_state(fact_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_subject ON knowledge_state(subject_type, subject_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_project ON knowledge_state(project_id);

-- ============================================================
-- 19. REVELATION 揭示
-- ============================================================
CREATE TABLE IF NOT EXISTS revelation (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    fact_id VARCHAR NOT NULL REFERENCES fact(id),
    scene_id VARCHAR NOT NULL REFERENCES scene(id),
    revelation_method VARCHAR,
    narrative_significance TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_revelation_scene ON revelation(scene_id);
CREATE INDEX IF NOT EXISTS idx_revelation_fact ON revelation(fact_id);

-- ============================================================
-- 20. REVELATION_TARGET 揭示目标
-- ============================================================
CREATE TABLE IF NOT EXISTS revelation_target (
    id VARCHAR PRIMARY KEY,
    revelation_id VARCHAR NOT NULL REFERENCES revelation(id),
    subject_type VARCHAR NOT NULL,
    subject_id UUID,
    knowledge_level VARCHAR NOT NULL DEFAULT 'Complete'
);

CREATE INDEX IF NOT EXISTS idx_rev_target_revelation ON revelation_target(revelation_id);

-- ============================================================
-- 21. SKILL 技能
-- ============================================================
CREATE TABLE IF NOT EXISTS skill (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    skill_type VARCHAR NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    prompt_template TEXT NOT NULL,
    input_schema JSONB,
    output_schema JSONB,
    default_params JSONB,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- 22. SKILL_VERSION 技能版本历史
-- ============================================================
CREATE TABLE IF NOT EXISTS skill_version (
    id VARCHAR PRIMARY KEY,
    skill_id VARCHAR NOT NULL REFERENCES skill(id),
    version INTEGER NOT NULL,
    prompt_template TEXT NOT NULL,
    input_schema JSONB,
    output_schema JSONB,
    default_params JSONB,
    changelog TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_skill_ver_skill ON skill_version(skill_id);

-- ============================================================
-- 23. GENERATION_TASK 生成任务
-- ============================================================
CREATE TABLE IF NOT EXISTS generation_task (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    skill_id VARCHAR NOT NULL REFERENCES skill(id),
    scene_id VARCHAR REFERENCES scene(id),
    input JSONB NOT NULL,
    output JSONB,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    token_usage JSONB,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gen_task_project ON generation_task(project_id);
CREATE INDEX IF NOT EXISTS idx_gen_task_scene ON generation_task(scene_id);
CREATE INDEX IF NOT EXISTS idx_gen_task_status ON generation_task(status);

-- ============================================================
-- 24. GENERATION_RUN 生成运行记录
-- ============================================================
CREATE TABLE IF NOT EXISTS generation_run (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    task_id VARCHAR NOT NULL REFERENCES generation_task(id),
    context_snapshot_id VARCHAR,
    llm_model VARCHAR NOT NULL,
    provider VARCHAR,
    prompt_sent TEXT NOT NULL,
    response_received TEXT NOT NULL,
    token_usage JSONB,
    latency_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_gen_run_task ON generation_run(task_id);

-- ============================================================
-- 25. CONTEXT_SNAPSHOT 上下文快照
-- ============================================================
CREATE TABLE IF NOT EXISTS context_snapshot (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL,
    scene_id VARCHAR NOT NULL,
    token_budget INTEGER NOT NULL,
    l0_essential JSONB,
    l1_scene_relevant JSONB,
    l2_recent_history JSONB,
    l3_narrative_context JSONB,
    l4_character_knowledge JSONB,
    l5_world_background JSONB,
    l6_optional_supplement JSONB,
    actual_tokens INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ctx_snap_scene ON context_snapshot(scene_id);

-- ============================================================
-- 26. PROPOSED_CHANGE 拟议变更
-- ============================================================
CREATE TABLE IF NOT EXISTS proposed_change (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    task_id VARCHAR NOT NULL REFERENCES generation_task(id),
    change_type VARCHAR NOT NULL,
    target_entity_id VARCHAR NOT NULL,
    description TEXT NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'Pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_prop_change_project ON proposed_change(project_id);
CREATE INDEX IF NOT EXISTS idx_prop_change_task ON proposed_change(task_id);
CREATE INDEX IF NOT EXISTS idx_prop_change_status ON proposed_change(status);

-- ============================================================
-- 27. VALIDATION_RUN 验证运行
-- ============================================================
CREATE TABLE IF NOT EXISTS validation_run (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    task_id VARCHAR NOT NULL REFERENCES generation_task(id),
    changes_validated INTEGER NOT NULL DEFAULT 0,
    changes_approved INTEGER NOT NULL DEFAULT 0,
    changes_rejected INTEGER NOT NULL DEFAULT 0,
    status VARCHAR NOT NULL DEFAULT 'Running',
    started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_val_run_task ON validation_run(task_id);

-- ============================================================
-- 28. VALIDATION_ISSUE 验证问题
-- ============================================================
CREATE TABLE IF NOT EXISTS validation_issue (
    id VARCHAR PRIMARY KEY,
    validation_run_id VARCHAR NOT NULL REFERENCES validation_run(id),
    proposed_change_id VARCHAR NOT NULL REFERENCES proposed_change(id),
    issue_type VARCHAR NOT NULL,
    severity VARCHAR NOT NULL DEFAULT 'Warning',
    message TEXT NOT NULL,
    suggestion TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_val_issue_run ON validation_issue(validation_run_id);

-- ============================================================
-- 29. SCENE_DOCUMENT 场景文档（生成的正文）
-- ============================================================
CREATE TABLE IF NOT EXISTS scene_document (
    id VARCHAR PRIMARY KEY,
    scene_id VARCHAR NOT NULL REFERENCES scene(id),
    generation_task_id VARCHAR REFERENCES generation_task(id),
    content TEXT NOT NULL,
    word_count INTEGER,
    version INTEGER NOT NULL DEFAULT 1,
    status VARCHAR NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_scene_doc_scene ON scene_document(scene_id);

-- ============================================================
-- 30. 通用时间线事件（用于全局时间线查询）
-- ============================================================
CREATE TABLE IF NOT EXISTS timeline_event (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    event_id VARCHAR REFERENCES event(id),
    scene_id VARCHAR REFERENCES scene(id),
    narrative_node_id VARCHAR REFERENCES narrative_node(id),
    sort_key VARCHAR NOT NULL,
    label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_timeline_project ON timeline_event(project_id);
CREATE INDEX IF NOT EXISTS idx_timeline_sort ON timeline_event(project_id, sort_key);

-- ============================================================
-- 31. CHARACTER_PROFILE 人物基础信息
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
    values TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_profile_entity ON character_profile(entity_id);

-- ============================================================
-- 32. CHARACTER_STATE 人物当前状态
-- ============================================================
CREATE TABLE IF NOT EXISTS character_state (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    location VARCHAR,
    health VARCHAR,
    cultivation VARCHAR,
    money VARCHAR,
    wanted BOOLEAN NOT NULL DEFAULT FALSE,
    extra JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_state_entity ON character_state(entity_id);

-- ============================================================
-- 33. CHARACTER_GOAL 人物目标
-- ============================================================
CREATE TABLE IF NOT EXISTS character_goal (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    long_term TEXT,
    current TEXT,
    immediate TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_goal_entity ON character_goal(entity_id);

-- ============================================================
-- 34. CHARACTER_TRAIT 人物特征
-- ============================================================
CREATE TABLE IF NOT EXISTS character_trait (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    trait_type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    parent_trait_id VARCHAR,
    intensity INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_char_trait_entity ON character_trait(entity_id);
CREATE INDEX IF NOT EXISTS idx_char_trait_parent ON character_trait(parent_trait_id);

-- ============================================================
-- 35. FACTION_PROFILE 势力详细信息
-- ============================================================
CREATE TABLE IF NOT EXISTS faction_profile (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_faction_profile_entity ON faction_profile(entity_id);

-- ============================================================
-- 36. LOCATION_IDENTITY 地点基本信息
-- ============================================================
CREATE TABLE IF NOT EXISTS location_identity (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    location_type VARCHAR,
    size VARCHAR,
    climate VARCHAR,
    era VARCHAR,
    accessibility TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_identity_entity ON location_identity(entity_id);

-- ============================================================
-- 37. LOCATION_GEOGRAPHY 地理信息
-- ============================================================
CREATE TABLE IF NOT EXISTS location_geography (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    terrain TEXT,
    climate TEXT,
    natural_resources TEXT,
    hazards TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- 38. LOCATION_FACILITIES 设施
-- ============================================================
CREATE TABLE IF NOT EXISTS location_facilities (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    facility_type VARCHAR,
    description TEXT,
    controlled_by VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_facilities_entity ON location_facilities(entity_id);

-- ============================================================
-- 39. LOCATION_RULES 规则
-- ============================================================
CREATE TABLE IF NOT EXISTS location_rules (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    rule_text TEXT NOT NULL,
    rule_type VARCHAR,
    enforced_by VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_rules_entity ON location_rules(entity_id);

-- ============================================================
-- 40. LOCATION_THREATS 威胁
-- ============================================================
CREATE TABLE IF NOT EXISTS location_threats (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    name VARCHAR NOT NULL,
    threat_type VARCHAR,
    severity VARCHAR,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_threats_entity ON location_threats(entity_id);

-- ============================================================
-- 41. LOCATION_SECRETS 秘密
-- ============================================================
CREATE TABLE IF NOT EXISTS location_secrets (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    secret_text TEXT NOT NULL,
    discovered_by VARCHAR,
    narrative_importance VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_secrets_entity ON location_secrets(entity_id);

-- ============================================================
-- 42. LOCATION_NARRATIVE_HOOKS 叙事钩子
-- ============================================================
CREATE TABLE IF NOT EXISTS location_narrative_hooks (
    id VARCHAR PRIMARY KEY,
    entity_id VARCHAR NOT NULL REFERENCES entity(id),
    hook_text TEXT NOT NULL,
    hook_type VARCHAR,
    related_entities JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_loc_hooks_entity ON location_narrative_hooks(entity_id);

-- ============================================================
-- 43. STANDARD_RELATION_TYPES 标准关系类型
-- ============================================================
CREATE TABLE IF NOT EXISTS standard_relation_type (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    description TEXT,
    category VARCHAR,
    is_symmetric BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 预置标准关系类型
INSERT OR IGNORE INTO standard_relation_type (id, name, description, category, is_symmetric) VALUES
    ('rt-001', 'MEMBER_OF', '是成员', '组织', FALSE),
    ('rt-002', 'CONTROLS', '控制', '权力', FALSE),
    ('rt-003', 'ENEMY_OF', '敌人', '关系', TRUE),
    ('rt-004', 'FRIEND_OF', '朋友', '关系', TRUE),
    ('rt-005', 'ALLY_OF', '盟友', '关系', TRUE),
    ('rt-006', 'PARENT_OF', '父母', '家族', FALSE),
    ('rt-007', 'CHILD_OF', '子女', '家族', FALSE),
    ('rt-008', 'SPOUSE_OF', '配偶', '家族', TRUE),
    ('rt-009', 'LOCATED_AT', '位于', '位置', FALSE),
    ('rt-010', 'CONNECTED_TO', '连接', '位置', TRUE),
    ('rt-011', 'CONTAINS', '包含', '位置', FALSE),
    ('rt-012', 'OWNS', '拥有', '资源', FALSE),
    ('rt-013', 'PRODUCES', '产出', '资源', FALSE),
    ('rt-014', 'TRAINS', '训练', '关系', FALSE),
    ('rt-015', 'MENTORS', '指导', '关系', FALSE),
    ('rt-016', 'SERVES', '服务', '关系', FALSE),
    ('rt-017', 'OPPOSES', '反对', '关系', FALSE),
    ('rt-018', 'PROTECTS', '保护', '关系', FALSE),
    ('rt-019', 'HUNTS', '追捕', '关系', FALSE),
    ('rt-020', 'LOVES', '爱慕', '关系', TRUE);

-- ============================================================
-- 44. AUTHORIAL_INTENT 作者意图
-- ============================================================
CREATE TABLE IF NOT EXISTS authorial_intent (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    target_id VARCHAR,
    target_type VARCHAR,
    pacing VARCHAR,
    emotional_tone VARCHAR,
    focus TEXT,
    avoid TEXT,
    perspective VARCHAR,
    narrative_distance VARCHAR,
    additional_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_authorial_project ON authorial_intent(project_id);
CREATE INDEX IF NOT EXISTS idx_authorial_target ON authorial_intent(target_id);
