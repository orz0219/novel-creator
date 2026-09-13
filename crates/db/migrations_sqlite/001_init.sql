-- 由 tmp/gen_sqlite_schema.py 从 PostgreSQL 权威 schema 自动生成，请勿手工编辑
-- 源：探针库 ne_schema_probe 执行全部 25 个 PG 迁移后的最终结构
-- 类型映射：uuid->TEXT  timestamptz->DATETIME  jsonb->TEXT

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS "agent_memory" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "memory_type" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "agent_prompts" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scope" TEXT NOT NULL,
    "project_id" TEXT,
    "system_prompt" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE (scope)
);

CREATE TABLE IF NOT EXISTS "agent_sessions" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "title" TEXT,
    "current_step" TEXT NOT NULL DEFAULT '项目初始化',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "app_settings" (
    "id" TEXT NOT NULL DEFAULT 'default',
    "settings" TEXT NOT NULL DEFAULT '{}',
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "context_snapshot" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "token_budget" INTEGER NOT NULL,
    "l0_essential" TEXT,
    "l1_scene_relevant" TEXT,
    "l2_recent_history" TEXT,
    "l3_narrative_context" TEXT,
    "l4_character_knowledge" TEXT,
    "l5_world_background" TEXT,
    "l6_optional_supplement" TEXT,
    "actual_tokens" INTEGER,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "reproducibility_meta" TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "entity_alias" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "alias_type" TEXT NOT NULL,
    "alias" TEXT NOT NULL,
    "valid_from_scene_id" TEXT,
    "valid_until_scene_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "entity_type" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "name" TEXT NOT NULL,
    "description" TEXT,
    "schema_json" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS "identity_timeline" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "identity" TEXT NOT NULL,
    "start_scene_id" TEXT NOT NULL,
    "end_scene_id" TEXT,
    "change_reason" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "project" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "name" TEXT NOT NULL,
    "description" TEXT,
    "language" TEXT,
    "world_setting" TEXT,
    "system_setting" TEXT,
    "default_model" TEXT,
    "default_style" TEXT,
    "default_params" TEXT,
    "config" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Concept',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "premise" TEXT,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "quality_score" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "run_id" TEXT,
    "continuity_score" INTEGER,
    "character_score" INTEGER,
    "plot_score" INTEGER,
    "knowledge_score" INTEGER,
    "world_score" INTEGER,
    "style_score" INTEGER,
    "overall_score" INTEGER,
    "issues" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "reader_knowledge" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "fact_id" TEXT NOT NULL,
    "knowledge_level" TEXT NOT NULL DEFAULT 'Unknown',
    "source_scene_id" TEXT,
    "confidence" TEXT NOT NULL DEFAULT 'Certain',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "revision_plan" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "original_draft_id" TEXT NOT NULL,
    "issues" TEXT NOT NULL,
    "revision_strategy" TEXT NOT NULL,
    "revision_prompt" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "scene_contract" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scene_id" TEXT NOT NULL,
    "required_events" TEXT,
    "forbidden_events" TEXT,
    "required_characters" TEXT,
    "required_facts" TEXT,
    "reader_learns" TEXT,
    "protagonist_learns" TEXT,
    "world_changes" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "scene_ledger" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "events" TEXT,
    "gains" TEXT,
    "losses" TEXT,
    "relationship_changes" TEXT,
    "knowledge_changes" TEXT,
    "world_changes" TEXT,
    "foreshadowing_mentions" TEXT,
    "storyline_progress" TEXT,
    "character_growth" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "skill" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "name" TEXT NOT NULL,
    "description" TEXT,
    "skill_type" TEXT NOT NULL,
    "version" INTEGER NOT NULL DEFAULT 1,
    "prompt_template" TEXT NOT NULL,
    "input_schema" TEXT,
    "output_schema" TEXT,
    "default_params" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Draft',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "skill_version" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "skill_id" TEXT NOT NULL,
    "version" INTEGER NOT NULL,
    "prompt_template" TEXT NOT NULL,
    "input_schema" TEXT,
    "output_schema" TEXT,
    "default_params" TEXT,
    "changelog" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (skill_id) REFERENCES "skill"(id)
);

CREATE TABLE IF NOT EXISTS "standard_relation_type" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "name" TEXT NOT NULL,
    "description" TEXT,
    "category" TEXT,
    "is_symmetric" INTEGER NOT NULL DEFAULT 0,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS "state_snapshot" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "state_before" TEXT NOT NULL,
    "changes" TEXT NOT NULL,
    "state_after" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "story_contract" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "narrative_node_id" TEXT NOT NULL,
    "mission" TEXT,
    "objectives" TEXT,
    "required_events" TEXT,
    "required_revelations" TEXT,
    "required_character_changes" TEXT,
    "required_world_changes" TEXT,
    "forbidden_events" TEXT,
    "exit_conditions" TEXT,
    "completion_progress" REAL NOT NULL DEFAULT 0.0,
    "completed_events" TEXT,
    "completed_character_changes" TEXT,
    "completed_world_changes" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS "storyline" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "importance" TEXT NOT NULL DEFAULT 'Normal',
    "created_volume_id" TEXT,
    "resolved_volume_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "tone" TEXT NOT NULL DEFAULT 'light',
    "visibility" TEXT NOT NULL DEFAULT 'visible',
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "storyline_relation" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "parent_id" TEXT NOT NULL,
    "child_id" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE (parent_id, child_id),
    FOREIGN KEY (child_id) REFERENCES "storyline"(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES "storyline"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "storyline_scene" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "storyline_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "significance" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (storyline_id) REFERENCES "storyline"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "system_events" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT,
    "event_type" TEXT NOT NULL,
    "entity_type" TEXT,
    "entity_id" TEXT,
    "actor" TEXT NOT NULL DEFAULT 'system',
    "description" TEXT,
    "data" TEXT,
    "source" TEXT,
    "old_value" TEXT,
    "new_value" TEXT,
    "metadata" TEXT DEFAULT '{}',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "test_case" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "test_type" TEXT NOT NULL,
    "preconditions" TEXT NOT NULL,
    "expected_result" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'Pending',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "test_result" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "test_case_id" TEXT NOT NULL,
    "passed" INTEGER NOT NULL,
    "actual_result" TEXT NOT NULL,
    "issues" TEXT,
    "model_version" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (test_case_id) REFERENCES "test_case"(id)
);

CREATE TABLE IF NOT EXISTS "volume_summary" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "volume_id" TEXT NOT NULL,
    "summary" TEXT NOT NULL,
    "character_changes" TEXT,
    "world_changes" TEXT,
    "foreshadowing_progress" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "world" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "world_rules" TEXT,
    "config" TEXT,
    "is_main" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "world_branch" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "parent_branch_id" TEXT,
    "is_main" INTEGER NOT NULL DEFAULT 0,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "world_version" (
    "id" TEXT NOT NULL,
    "world_id" TEXT NOT NULL,
    "version" INTEGER NOT NULL,
    "kind" TEXT NOT NULL,
    "trigger_id" TEXT,
    "summary" TEXT,
    "parent_version_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE (world_id, version)
);

CREATE TABLE IF NOT EXISTS "agent_messages" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "session_id" TEXT NOT NULL,
    "role" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "seq" INTEGER NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (session_id) REFERENCES "agent_sessions"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "agent_runs" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "agent_type" TEXT NOT NULL,
    "task_type" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'pending',
    "input" TEXT DEFAULT '{}',
    "output" TEXT DEFAULT '{}',
    "error_message" TEXT,
    "model" TEXT,
    "provider" TEXT,
    "context_snapshot_id" TEXT,
    "tokens_used" INTEGER,
    "duration_ms" INTEGER,
    "started_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at" DATETIME,
    "metadata" TEXT DEFAULT '{}',
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "approval_record" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "target_type" TEXT NOT NULL,
    "target_id" TEXT NOT NULL,
    "proposed_by" TEXT NOT NULL,
    "proposal_content" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Pending',
    "reviewer_id" TEXT,
    "reviewer_comment" TEXT,
    "content_hash" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "reviewed_at" DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "arc_summary" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "arc_id" TEXT NOT NULL,
    "summary" TEXT NOT NULL,
    "key_turning_points" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "authorial_intent" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "target_id" TEXT,
    "target_type" TEXT,
    "pacing" TEXT,
    "emotional_tone" TEXT,
    "focus" TEXT,
    "avoid" TEXT,
    "perspective" TEXT,
    "narrative_distance" TEXT,
    "additional_notes" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "belief" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "belief_content" TEXT NOT NULL,
    "confidence" REAL NOT NULL DEFAULT 0.5,
    "source" TEXT,
    "source_scene_id" TEXT,
    "is_active" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "canon_rule" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "world_id" TEXT NOT NULL,
    "rule_level" TEXT NOT NULL,
    "rule_content" TEXT NOT NULL,
    "affected_scope" TEXT NOT NULL,
    "enforcement" TEXT NOT NULL,
    "constraints" TEXT,
    "source" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "causal_relation" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "cause_event_id" TEXT NOT NULL,
    "effect_event_id" TEXT NOT NULL,
    "relation_type" TEXT NOT NULL DEFAULT 'DirectCause',
    "strength" TEXT NOT NULL DEFAULT 'Strong',
    "description" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "chapter_summary" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "chapter_id" TEXT NOT NULL,
    "summary" TEXT NOT NULL,
    "key_events" TEXT,
    "involved_characters" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_fear" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "fear_content" TEXT NOT NULL,
    "intensity" INTEGER NOT NULL DEFAULT 5,
    "source" TEXT,
    "is_active" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_goal_mind" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "goal_content" TEXT NOT NULL,
    "priority" INTEGER NOT NULL DEFAULT 5,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "source" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_memory" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "memory_content" TEXT NOT NULL,
    "emotional_impact" TEXT,
    "scene_id" TEXT,
    "importance" INTEGER NOT NULL DEFAULT 5,
    "is_active" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "memory_type" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "decision_trace" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "decision" TEXT NOT NULL,
    "factors" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "emotion_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "emotion_type" TEXT NOT NULL,
    "intensity" INTEGER NOT NULL DEFAULT 50,
    "decay_rate" REAL NOT NULL DEFAULT 0.1,
    "trigger_scene_id" TEXT,
    "trigger_description" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "entity" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "world_id" TEXT NOT NULL,
    "entity_type_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "summary" TEXT,
    "description" TEXT,
    "attributes" TEXT,
    "version" INTEGER NOT NULL DEFAULT 1,
    "created_by" TEXT NOT NULL DEFAULT 'system',
    "updated_by" TEXT,
    "source_generation_id" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_type_id) REFERENCES "entity_type"(id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (world_id) REFERENCES "world"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "entity_arc_stage" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "stage" TEXT NOT NULL,
    "order_index" INTEGER NOT NULL DEFAULT 0,
    "role" TEXT,
    "screen_weight" TEXT,
    "goal" TEXT,
    "function" TEXT,
    "entry_trigger" TEXT,
    "status" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "event" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "event_type" TEXT,
    "timestamp" TEXT,
    "event_time" TEXT,
    "duration" TEXT,
    "timeline_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "event_entity" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "event_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (event_id) REFERENCES "event"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "event_outbox" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "event_type" TEXT NOT NULL,
    "aggregate_type" TEXT NOT NULL,
    "aggregate_id" TEXT NOT NULL,
    "payload" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'Pending',
    "retry_count" INTEGER NOT NULL DEFAULT 0,
    "max_retries" INTEGER NOT NULL DEFAULT 3,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "delivered_at" DATETIME,
    "error_message" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "fact" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "category" TEXT,
    "certainty" TEXT NOT NULL DEFAULT 'CANON',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "superseded_by" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (superseded_by) REFERENCES "fact"(id)
);

CREATE TABLE IF NOT EXISTS "fact_entity" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "fact_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (fact_id) REFERENCES "fact"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "fact_visibility" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "fact_id" TEXT NOT NULL,
    "subject_type" TEXT NOT NULL,
    "subject_id" TEXT,
    "visibility_level" TEXT NOT NULL DEFAULT 'Hidden',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "faction_profile" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "goals" TEXT,
    "leader" TEXT,
    "values" TEXT,
    "resources" TEXT,
    "territory" TEXT,
    "members" TEXT,
    "enemies" TEXT,
    "allies" TEXT,
    "internal_conflicts" TEXT,
    "secrets" TEXT,
    "modus_operandi" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "foreshadowing" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "storyline_id" TEXT,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Planned',
    "importance" TEXT NOT NULL DEFAULT 'Normal',
    "hint_level" TEXT NOT NULL DEFAULT 'Subtle',
    "introduced_at" TEXT,
    "expected_reveal_at" TEXT,
    "actual_reveal_at" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "generation_task" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "task_type" TEXT NOT NULL,
    "target_id" TEXT,
    "model" TEXT,
    "parameters" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Pending',
    "result" TEXT,
    "context_tokens" INTEGER,
    "error" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "skill_id" TEXT,
    "scene_id" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "global_story_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "current_progress" TEXT NOT NULL,
    "open_foreshadowing" TEXT,
    "open_storylines" TEXT,
    "world_state_summary" TEXT NOT NULL,
    "character_state_summary" TEXT NOT NULL,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "knowledge_gap" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "gap_type" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "importance" TEXT NOT NULL DEFAULT 'MEDIUM',
    "required_by_scene_id" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Open',
    "designer_skill_hint" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "knowledge_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "fact_id" TEXT NOT NULL,
    "subject_type" TEXT NOT NULL,
    "subject_id" TEXT,
    "knows" INTEGER NOT NULL DEFAULT 0,
    "knowledge_level" TEXT NOT NULL DEFAULT 'Unknown',
    "source" TEXT,
    "effective_from" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "effective_to" DATETIME,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (fact_id) REFERENCES "fact"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_connection" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "source_location_id" TEXT NOT NULL,
    "target_location_id" TEXT NOT NULL,
    "connection_type" TEXT NOT NULL DEFAULT 'path',
    "travel_time" TEXT,
    "travel_description" TEXT,
    "is_bidirectional" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (source_location_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (target_location_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_facilities" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "facility_type" TEXT,
    "description" TEXT,
    "controlled_by" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_facility" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "facility_type" TEXT,
    "description" TEXT,
    "controlled_by_entity_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_geography" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "terrain" TEXT,
    "climate" TEXT,
    "natural_resources" TEXT,
    "hazards" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_identity" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "location_type" TEXT,
    "size" TEXT,
    "climate" TEXT,
    "era" TEXT,
    "accessibility" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_narrative_hooks" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "hook_text" TEXT NOT NULL,
    "hook_type" TEXT,
    "related_entities" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_profile" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "geography" TEXT,
    "appearance" TEXT,
    "population" TEXT,
    "economy" TEXT,
    "rules" TEXT,
    "history" TEXT,
    "narrative_usage" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_rules" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "rule_text" TEXT NOT NULL,
    "rule_type" TEXT,
    "enforced_by" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_secret" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "discovered" INTEGER NOT NULL DEFAULT 0,
    "discovered_by_entity_id" TEXT,
    "discovered_at_scene_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_secrets" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "secret_text" TEXT NOT NULL,
    "discovered_by" TEXT,
    "narrative_importance" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_threat" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "threat_type" TEXT,
    "description" TEXT,
    "severity" TEXT DEFAULT 'Normal',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "location_threats" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "threat_type" TEXT,
    "severity" TEXT,
    "description" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "memories" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "memory_type" TEXT NOT NULL DEFAULT 'general',
    "content" TEXT NOT NULL,
    "importance" REAL DEFAULT 0.5,
    "source" TEXT,
    "embedding_vector_id" TEXT,
    "metadata" TEXT DEFAULT '{}',
    "access_count" INTEGER DEFAULT 0,
    "last_accessed_at" DATETIME,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "mutation_ledger" (
    "command_id" TEXT NOT NULL,
    "project_id" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'committed',
    "result" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (command_id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "narrative_branch" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "parent_branch_id" TEXT,
    "fork_point_scene_id" TEXT,
    "is_main" INTEGER NOT NULL DEFAULT 0,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "narrative_budget" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "narrative_node_id" TEXT NOT NULL,
    "allocated_words" INTEGER NOT NULL DEFAULT 0,
    "used_words" INTEGER NOT NULL DEFAULT 0,
    "action_ratio" REAL,
    "dialogue_ratio" REAL,
    "description_ratio" REAL,
    "exposition_ratio" REAL,
    "internal_monologue_ratio" REAL,
    "pacing_warning_threshold" REAL DEFAULT 0.9,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "narrative_node" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "world_id" TEXT NOT NULL,
    "node_type" TEXT NOT NULL,
    "parent_id" TEXT,
    "title" TEXT NOT NULL,
    "description" TEXT,
    "attributes" TEXT,
    "sort_order" INTEGER NOT NULL DEFAULT 0,
    "version" INTEGER NOT NULL DEFAULT 1,
    "status" TEXT NOT NULL DEFAULT 'Draft',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "content" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (parent_id) REFERENCES "narrative_node"(id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "narrative_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "state_dimension" TEXT NOT NULL,
    "state_key" TEXT NOT NULL,
    "state_value" TEXT NOT NULL,
    "scene_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "narrative_thread" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "storyline_id" TEXT,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Active',
    "importance" TEXT NOT NULL DEFAULT 'Normal',
    "current_stage" TEXT,
    "recent_progress" TEXT,
    "next_step" TEXT,
    "goal" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "narrative_thread_participant" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "thread_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (thread_id) REFERENCES "narrative_thread"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "novel_state_snapshot" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT,
    "story_time" TEXT,
    "world_summary" TEXT,
    "main_character_state" TEXT,
    "current_location" TEXT,
    "active_threads_count" INTEGER DEFAULT 0,
    "unresolved_foreshadows_count" INTEGER DEFAULT 0,
    "known_characters_count" INTEGER DEFAULT 0,
    "known_locations_count" INTEGER DEFAULT 0,
    "current_volume_id" TEXT,
    "current_arc_id" TEXT,
    "state_data" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "plot" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "plot_type" TEXT,
    "status" TEXT NOT NULL DEFAULT 'Draft',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "plot_repair" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "issue_description" TEXT NOT NULL,
    "repair_suggestion" TEXT NOT NULL,
    "repair_type" TEXT NOT NULL DEFAULT 'Automatic',
    "status" TEXT NOT NULL DEFAULT 'Pending',
    "applied_at" DATETIME,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "proposed_change" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "task_id" TEXT,
    "change_type" TEXT NOT NULL,
    "target_entity_id" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "payload" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'Pending',
    "content_hash" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "resolved_at" DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES "generation_task"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "relation" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "source_entity_id" TEXT NOT NULL,
    "target_entity_id" TEXT NOT NULL,
    "relation_type" TEXT NOT NULL,
    "description" TEXT,
    "attributes" TEXT,
    "valid_from" TEXT,
    "valid_until" TEXT,
    "version" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (source_entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (target_entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "resource_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "location_id" TEXT NOT NULL,
    "resource_name" TEXT NOT NULL,
    "quantity" REAL,
    "production_rate" REAL,
    "controlled_by_entity_id" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE (project_id, location_id, resource_name),
    FOREIGN KEY (controlled_by_entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (location_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "scene" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "narrative_node_id" TEXT NOT NULL,
    "objective" TEXT,
    "conflict" TEXT,
    "pov_character_id" TEXT,
    "location_id" TEXT,
    "time" TEXT,
    "scene_start_time" TEXT,
    "scene_end_time" TEXT,
    "version" INTEGER NOT NULL DEFAULT 1,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (location_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (narrative_node_id) REFERENCES "narrative_node"(id) ON DELETE CASCADE,
    FOREIGN KEY (pov_character_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "scene_document" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scene_id" TEXT NOT NULL,
    "generation_task_id" TEXT,
    "content" TEXT NOT NULL,
    "word_count" INTEGER,
    "version" INTEGER NOT NULL DEFAULT 1,
    "status" TEXT NOT NULL DEFAULT 'Draft',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (generation_task_id) REFERENCES "generation_task"(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "scene_entity" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scene_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    "notes" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "scene_requirement" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scene_id" TEXT NOT NULL,
    "requirement_type" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "priority" TEXT NOT NULL DEFAULT 'Should',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "state_change" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "event_id" TEXT,
    "change_type" TEXT NOT NULL,
    "target_entity_id" TEXT NOT NULL,
    "state_key" TEXT NOT NULL,
    "old_value" TEXT,
    "new_value" TEXT NOT NULL,
    "committed_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "committed_by" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (event_id) REFERENCES "system_events"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (target_entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "timeline_event" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "event_id" TEXT,
    "scene_id" TEXT,
    "narrative_node_id" TEXT,
    "sort_key" TEXT NOT NULL,
    "label" TEXT NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (event_id) REFERENCES "event"(id) ON DELETE CASCADE,
    FOREIGN KEY (narrative_node_id) REFERENCES "narrative_node"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "validation_run" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "task_id" TEXT NOT NULL,
    "changes_validated" INTEGER NOT NULL DEFAULT 0,
    "changes_approved" INTEGER NOT NULL DEFAULT 0,
    "changes_rejected" INTEGER NOT NULL DEFAULT 0,
    "status" TEXT NOT NULL DEFAULT 'Running',
    "started_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at" DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES "generation_task"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_arc" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "character_id" TEXT NOT NULL,
    "volume_id" TEXT,
    "arc_type" TEXT NOT NULL,
    "start_state" TEXT,
    "mid_state" TEXT,
    "end_state" TEXT,
    "key_moments" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (character_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (volume_id) REFERENCES "narrative_node"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_arc_potential" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "starting_state" TEXT,
    "possible_change" TEXT,
    "resistance" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_capability" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "skills" TEXT,
    "limitations" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_conflict" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "conflict_type" TEXT,
    "description" TEXT,
    "target_entity_id" TEXT,
    "resolution_status" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "phase" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_drive" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "primary_goal" TEXT,
    "motivation" TEXT,
    "urgency" INTEGER NOT NULL DEFAULT 5,
    "long_term" TEXT,
    "current_goal" TEXT,
    "immediate" TEXT,
    "hidden_goal" TEXT,
    "fear" TEXT,
    "weakness" TEXT,
    "desire" TEXT,
    "contradiction" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_extension" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "extension_type" TEXT,
    "data" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_profile" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "name" TEXT,
    "age_range" TEXT,
    "gender" TEXT,
    "identity" TEXT,
    "appearance" TEXT,
    "background_origin" TEXT,
    "core_personality" TEXT,
    "values" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "aliases" TEXT NOT NULL DEFAULT '[]',
    "social_position" TEXT,
    "role_in_story" TEXT,
    "narrative_necessity" TEXT,
    "extra" TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_relationship" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "target_entity_id" TEXT,
    "relationship_type" TEXT,
    "attitude" TEXT,
    "trust_level" INTEGER,
    "secret_knowledge" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_secret" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "content" TEXT,
    "importance" INTEGER,
    "reveal_condition" TEXT,
    "related_entities" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "location" TEXT,
    "physical_state" TEXT,
    "extra" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "resources" TEXT,
    "current_status" TEXT,
    "emotion" TEXT,
    "mental_state" TEXT,
    "resource_state" TEXT,
    "social_state" TEXT,
    "flags" TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "character_trait" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "trait_type" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "intensity" INTEGER,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "current_state" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "state_key" TEXT NOT NULL,
    "state_value" TEXT NOT NULL,
    "version" INTEGER NOT NULL DEFAULT 1,
    "effective_from" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "effective_to" DATETIME,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "generation_run" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "task_id" TEXT NOT NULL,
    "context_snapshot_id" TEXT,
    "llm_model" TEXT NOT NULL,
    "provider" TEXT,
    "prompt_sent" TEXT NOT NULL,
    "response_received" TEXT NOT NULL,
    "token_usage" TEXT,
    "latency_ms" INTEGER,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "reproducibility_meta" TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES "generation_task"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "revelation" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "project_id" TEXT NOT NULL,
    "fact_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "revelation_method" TEXT,
    "narrative_significance" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (fact_id) REFERENCES "fact"(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "revelation_target" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "revelation_id" TEXT NOT NULL,
    "subject_type" TEXT NOT NULL,
    "subject_id" TEXT,
    "knowledge_level" TEXT NOT NULL DEFAULT 'Complete',
    PRIMARY KEY (id),
    FOREIGN KEY (revelation_id) REFERENCES "revelation"(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "validation_issue" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "validation_run_id" TEXT NOT NULL,
    "proposed_change_id" TEXT NOT NULL,
    "issue_type" TEXT NOT NULL,
    "severity" TEXT NOT NULL DEFAULT 'Warning',
    "message" TEXT NOT NULL,
    "suggestion" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (proposed_change_id) REFERENCES "proposed_change"(id) ON DELETE CASCADE,
    FOREIGN KEY (validation_run_id) REFERENCES "validation_run"(id) ON DELETE CASCADE
);


CREATE INDEX IF NOT EXISTS "idx_agent_memory_project" ON "agent_memory" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_agent_messages_session" ON "agent_messages" ("session_id", "seq");
CREATE INDEX IF NOT EXISTS "idx_agent_runs_project" ON "agent_runs" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_agent_runs_status" ON "agent_runs" ("project_id", "status");
CREATE INDEX IF NOT EXISTS "idx_agent_runs_type" ON "agent_runs" ("project_id", "agent_type");
CREATE INDEX IF NOT EXISTS "idx_alias_entity" ON "entity_alias" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_approval_content_hash" ON "approval_record" ("content_hash");
CREATE INDEX IF NOT EXISTS "idx_approval_project" ON "approval_record" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_approval_target" ON "approval_record" ("target_id");
CREATE INDEX IF NOT EXISTS "idx_arc_summary_project" ON "arc_summary" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_authorial_project" ON "authorial_intent" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_authorial_target" ON "authorial_intent" ("target_id");
CREATE INDEX IF NOT EXISTS "idx_belief_character" ON "belief" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_belief_project" ON "belief" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_canon_rule_level" ON "canon_rule" ("rule_level");
CREATE INDEX IF NOT EXISTS "idx_canon_rule_project" ON "canon_rule" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_canon_rule_world" ON "canon_rule" ("world_id");
CREATE INDEX IF NOT EXISTS "idx_causal_cause" ON "causal_relation" ("cause_event_id");
CREATE INDEX IF NOT EXISTS "idx_causal_effect" ON "causal_relation" ("effect_event_id");
CREATE INDEX IF NOT EXISTS "idx_causal_project" ON "causal_relation" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_chapter_summary_project" ON "chapter_summary" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_char_arc_character" ON "character_arc" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_char_arc_entity" ON "character_arc_potential" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_arc_volume" ON "character_arc" ("volume_id");
CREATE INDEX IF NOT EXISTS "idx_char_capability_entity" ON "character_capability" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_conflict_entity" ON "character_conflict" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_drive_entity" ON "character_drive" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_extension_entity" ON "character_extension" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_profile_entity" ON "character_profile" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_relationship_entity" ON "character_relationship" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_secret_entity" ON "character_secret" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_state_entity" ON "character_state" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_char_trait_entity" ON "character_trait" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_ctx_snap_scene" ON "context_snapshot" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_current_state_entity" ON "current_state" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_current_state_project" ON "current_state" ("project_id", "state_key");
CREATE INDEX IF NOT EXISTS "idx_decision_character" ON "decision_trace" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_decision_project" ON "decision_trace" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_decision_scene" ON "decision_trace" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_emotion_character" ON "emotion_state" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_emotion_project" ON "emotion_state" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_entity_arc_stage_entity" ON "entity_arc_stage" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_entity_name" ON "entity" ("project_id", "name");
CREATE INDEX IF NOT EXISTS "idx_entity_project" ON "entity" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_entity_status" ON "entity" ("project_id", "status");
CREATE INDEX IF NOT EXISTS "idx_entity_type" ON "entity" ("entity_type_id");
CREATE INDEX IF NOT EXISTS "idx_event_entity_event" ON "event_entity" ("event_id");
CREATE INDEX IF NOT EXISTS "idx_event_project" ON "event" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_fact_certainty" ON "fact" ("project_id", "certainty");
CREATE INDEX IF NOT EXISTS "idx_fact_entity_entity" ON "fact_entity" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_fact_entity_fact" ON "fact_entity" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_fact_project" ON "fact" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_fact_project_status" ON "fact" ("project_id", "status");
CREATE INDEX IF NOT EXISTS "idx_fact_visibility_fact" ON "fact_visibility" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_fact_visibility_project" ON "fact_visibility" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_faction_profile_entity" ON "faction_profile" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_fear_character" ON "character_fear" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_fear_project" ON "character_fear" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_foreshadowing_project" ON "foreshadowing" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_foreshadowing_status" ON "foreshadowing" ("status");
CREATE INDEX IF NOT EXISTS "idx_gap_project" ON "knowledge_gap" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_gap_status" ON "knowledge_gap" ("status");
CREATE INDEX IF NOT EXISTS "idx_gen_run_task" ON "generation_run" ("task_id");
CREATE INDEX IF NOT EXISTS "idx_gen_task_project" ON "generation_task" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_gen_task_status" ON "generation_task" ("status");
CREATE INDEX IF NOT EXISTS "idx_global_state_project" ON "global_story_state" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_goal_mind_character" ON "character_goal_mind" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_goal_mind_project" ON "character_goal_mind" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_identity_entity" ON "identity_timeline" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_knowledge_fact" ON "knowledge_state" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_knowledge_project" ON "knowledge_state" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_knowledge_subject" ON "knowledge_state" ("subject_type", "subject_id");
CREATE INDEX IF NOT EXISTS "idx_ledger_project" ON "scene_ledger" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_ledger_scene" ON "scene_ledger" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_loc_conn_source" ON "location_connection" ("source_location_id");
CREATE INDEX IF NOT EXISTS "idx_loc_conn_target" ON "location_connection" ("target_location_id");
CREATE INDEX IF NOT EXISTS "idx_loc_facilities_entity" ON "location_facilities" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_facility_entity" ON "location_facility" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_hooks_entity" ON "location_narrative_hooks" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_identity_entity" ON "location_identity" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_profile_entity" ON "location_profile" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_rules_entity" ON "location_rules" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_secret_entity" ON "location_secret" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_secrets_entity" ON "location_secrets" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_threat_entity" ON "location_threat" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_loc_threats_entity" ON "location_threats" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_memories_importance" ON "memories" ("project_id", "importance" DESC);
CREATE INDEX IF NOT EXISTS "idx_memories_project" ON "memories" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_memories_type" ON "memories" ("project_id", "memory_type");
CREATE INDEX IF NOT EXISTS "idx_memory_character" ON "character_memory" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_memory_project" ON "character_memory" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_mutation_ledger_project" ON "mutation_ledger" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narr_budget_node" ON "narrative_budget" ("narrative_node_id");
CREATE INDEX IF NOT EXISTS "idx_narr_budget_project" ON "narrative_budget" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narr_thread_part_thread" ON "narrative_thread_participant" ("thread_id");
CREATE INDEX IF NOT EXISTS "idx_narr_thread_project" ON "narrative_thread" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narr_thread_storyline" ON "narrative_thread" ("storyline_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_branch_project" ON "narrative_branch" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_parent" ON "narrative_node" ("parent_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_project" ON "narrative_node" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_state_dimension" ON "narrative_state" ("state_dimension");
CREATE INDEX IF NOT EXISTS "idx_narrative_state_key" ON "narrative_state" ("state_key");
CREATE INDEX IF NOT EXISTS "idx_narrative_state_project" ON "narrative_state" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_type" ON "narrative_node" ("project_id", "node_type");
CREATE INDEX IF NOT EXISTS "idx_novel_snap_project" ON "novel_state_snapshot" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_novel_snap_scene" ON "novel_state_snapshot" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_outbox_created" ON "event_outbox" ("created_at");
CREATE INDEX IF NOT EXISTS "idx_outbox_project" ON "event_outbox" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_outbox_status" ON "event_outbox" ("status");
CREATE INDEX IF NOT EXISTS "idx_plot_project" ON "plot" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_plot_repair_project" ON "plot_repair" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_plot_repair_scene" ON "plot_repair" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_prop_change_project" ON "proposed_change" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_prop_change_status" ON "proposed_change" ("status");
CREATE INDEX IF NOT EXISTS "idx_prop_change_task" ON "proposed_change" ("task_id");
CREATE INDEX IF NOT EXISTS "idx_quality_score_project" ON "quality_score" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_quality_score_scene" ON "quality_score" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_reader_knowledge_fact" ON "reader_knowledge" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_reader_knowledge_project" ON "reader_knowledge" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_relation_project" ON "relation" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_relation_source" ON "relation" ("source_entity_id");
CREATE INDEX IF NOT EXISTS "idx_relation_target" ON "relation" ("target_entity_id");
CREATE INDEX IF NOT EXISTS "idx_relation_type" ON "relation" ("project_id", "relation_type");
CREATE INDEX IF NOT EXISTS "idx_resource_location" ON "resource_state" ("location_id");
CREATE INDEX IF NOT EXISTS "idx_rev_target_revelation" ON "revelation_target" ("revelation_id");
CREATE INDEX IF NOT EXISTS "idx_revelation_fact" ON "revelation" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_revelation_scene" ON "revelation" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_revision_project" ON "revision_plan" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_revision_scene" ON "revision_plan" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_scene_contract_scene" ON "scene_contract" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_scene_doc_scene" ON "scene_document" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_scene_entity_entity" ON "scene_entity" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_scene_entity_scene" ON "scene_entity" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_scene_node" ON "scene" ("narrative_node_id");
CREATE INDEX IF NOT EXISTS "idx_scene_req_scene" ON "scene_requirement" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_skill_ver_skill" ON "skill_version" ("skill_id");
CREATE INDEX IF NOT EXISTS "idx_snapshot_project" ON "state_snapshot" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_snapshot_scene" ON "state_snapshot" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_state_change_entity" ON "state_change" ("target_entity_id");
CREATE INDEX IF NOT EXISTS "idx_state_change_project" ON "state_change" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_story_contract_narrative" ON "story_contract" ("narrative_node_id");
CREATE INDEX IF NOT EXISTS "idx_story_contract_project" ON "story_contract" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_project" ON "storyline" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_rel_child" ON "storyline_relation" ("child_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_rel_parent" ON "storyline_relation" ("parent_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_rel_project" ON "storyline_relation" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_scene_scene" ON "storyline_scene" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_scene_storyline" ON "storyline_scene" ("storyline_id");
CREATE INDEX IF NOT EXISTS "idx_system_event_entity" ON "system_events" ("entity_type", "entity_id");
CREATE INDEX IF NOT EXISTS "idx_system_event_project" ON "system_events" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_system_event_time" ON "system_events" ("project_id", "created_at" DESC);
CREATE INDEX IF NOT EXISTS "idx_system_event_type" ON "system_events" ("project_id", "event_type");
CREATE INDEX IF NOT EXISTS "idx_test_project" ON "test_case" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_test_result_case" ON "test_result" ("test_case_id");
CREATE INDEX IF NOT EXISTS "idx_test_type" ON "test_case" ("test_type");
CREATE INDEX IF NOT EXISTS "idx_timeline_project" ON "timeline_event" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_timeline_sort" ON "timeline_event" ("project_id", "sort_key");
CREATE INDEX IF NOT EXISTS "idx_val_issue_run" ON "validation_issue" ("validation_run_id");
CREATE INDEX IF NOT EXISTS "idx_val_run_task" ON "validation_run" ("task_id");
CREATE INDEX IF NOT EXISTS "idx_volume_summary_project" ON "volume_summary" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_world_branch_project" ON "world_branch" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_world_project" ON "world" ("project_id");
CREATE UNIQUE INDEX IF NOT EXISTS "agent_prompts_scope_key" ON "agent_prompts" ("scope");
CREATE UNIQUE INDEX IF NOT EXISTS "entity_type_name_key" ON "entity_type" ("name");
CREATE UNIQUE INDEX IF NOT EXISTS "idx_current_state_active_unique" ON "current_state" ("project_id", "entity_id", "state_key") WHERE ("effective_to" IS NULL);
CREATE UNIQUE INDEX IF NOT EXISTS "idx_narrative_sort_order_unique" ON "narrative_node" ("project_id", "parent_id", "sort_order");
CREATE UNIQUE INDEX IF NOT EXISTS "resource_state_project_id_location_id_resource_name_key" ON "resource_state" ("project_id", "location_id", "resource_name");
CREATE UNIQUE INDEX IF NOT EXISTS "standard_relation_type_name_key" ON "standard_relation_type" ("name");
CREATE UNIQUE INDEX IF NOT EXISTS "storyline_relation_parent_id_child_id_key" ON "storyline_relation" ("parent_id", "child_id");
CREATE UNIQUE INDEX IF NOT EXISTS "world_version_world_id_version_key" ON "world_version" ("world_id", "version");

PRAGMA foreign_keys = ON;
