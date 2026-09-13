-- 把缺 ON DELETE CASCADE 的外键补齐（SQLite 只能重建表）。
--
-- 由 tmp/gen_cascade_migration.py 从两份 001 DDL 的差异自动生成，请勿手工编辑。
--
-- 为什么要做：早先生成的 001 丢掉了级联动作，PG 侧的 002 迁移又只改了「当时已存在」
-- 的外键，之后新建的表仍是 NO ACTION。结果 `DELETE FROM project` 会报
-- `FOREIGN KEY constraint failed` —— 手机上点删除项目直接失败。
--
-- 重建用「建新表 → 拷数据 → 删旧表 → 改名」，SQLite 的 RENAME 会自动把引用这张表的
-- 外键指向新表。整个过程在迁移事务内完成；`defer_foreign_keys` 让中间态的悬空引用
-- 推迟到 COMMIT 时再检查（`PRAGMA foreign_keys=OFF` 在事务里是不生效的）。

PRAGMA defer_foreign_keys = ON;

-- agent_messages
CREATE TABLE "agent_messages__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "session_id" TEXT NOT NULL,
    "role" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "seq" INTEGER NOT NULL,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (session_id) REFERENCES "agent_sessions"(id) ON DELETE CASCADE
);
INSERT INTO "agent_messages__rebuild" SELECT * FROM "agent_messages";
DROP TABLE "agent_messages";
ALTER TABLE "agent_messages__rebuild" RENAME TO "agent_messages";
CREATE INDEX IF NOT EXISTS "idx_agent_messages_session" ON "agent_messages" ("session_id", "seq");

-- agent_runs
CREATE TABLE "agent_runs__rebuild" (
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
INSERT INTO "agent_runs__rebuild" SELECT * FROM "agent_runs";
DROP TABLE "agent_runs";
ALTER TABLE "agent_runs__rebuild" RENAME TO "agent_runs";
CREATE INDEX IF NOT EXISTS "idx_agent_runs_project" ON "agent_runs" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_agent_runs_status" ON "agent_runs" ("project_id", "status");
CREATE INDEX IF NOT EXISTS "idx_agent_runs_type" ON "agent_runs" ("project_id", "agent_type");

-- approval_record
CREATE TABLE "approval_record__rebuild" (
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
INSERT INTO "approval_record__rebuild" SELECT * FROM "approval_record";
DROP TABLE "approval_record";
ALTER TABLE "approval_record__rebuild" RENAME TO "approval_record";
CREATE INDEX IF NOT EXISTS "idx_approval_content_hash" ON "approval_record" ("content_hash");
CREATE INDEX IF NOT EXISTS "idx_approval_project" ON "approval_record" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_approval_target" ON "approval_record" ("target_id");

-- arc_summary
CREATE TABLE "arc_summary__rebuild" (
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
INSERT INTO "arc_summary__rebuild" SELECT * FROM "arc_summary";
DROP TABLE "arc_summary";
ALTER TABLE "arc_summary__rebuild" RENAME TO "arc_summary";
CREATE INDEX IF NOT EXISTS "idx_arc_summary_project" ON "arc_summary" ("project_id");

-- authorial_intent
CREATE TABLE "authorial_intent__rebuild" (
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
INSERT INTO "authorial_intent__rebuild" SELECT * FROM "authorial_intent";
DROP TABLE "authorial_intent";
ALTER TABLE "authorial_intent__rebuild" RENAME TO "authorial_intent";
CREATE INDEX IF NOT EXISTS "idx_authorial_project" ON "authorial_intent" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_authorial_target" ON "authorial_intent" ("target_id");

-- belief
CREATE TABLE "belief__rebuild" (
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
INSERT INTO "belief__rebuild" SELECT * FROM "belief";
DROP TABLE "belief";
ALTER TABLE "belief__rebuild" RENAME TO "belief";
CREATE INDEX IF NOT EXISTS "idx_belief_character" ON "belief" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_belief_project" ON "belief" ("project_id");

-- canon_rule
CREATE TABLE "canon_rule__rebuild" (
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
INSERT INTO "canon_rule__rebuild" SELECT * FROM "canon_rule";
DROP TABLE "canon_rule";
ALTER TABLE "canon_rule__rebuild" RENAME TO "canon_rule";
CREATE INDEX IF NOT EXISTS "idx_canon_rule_level" ON "canon_rule" ("rule_level");
CREATE INDEX IF NOT EXISTS "idx_canon_rule_project" ON "canon_rule" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_canon_rule_world" ON "canon_rule" ("world_id");

-- causal_relation
CREATE TABLE "causal_relation__rebuild" (
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
INSERT INTO "causal_relation__rebuild" SELECT * FROM "causal_relation";
DROP TABLE "causal_relation";
ALTER TABLE "causal_relation__rebuild" RENAME TO "causal_relation";
CREATE INDEX IF NOT EXISTS "idx_causal_cause" ON "causal_relation" ("cause_event_id");
CREATE INDEX IF NOT EXISTS "idx_causal_effect" ON "causal_relation" ("effect_event_id");
CREATE INDEX IF NOT EXISTS "idx_causal_project" ON "causal_relation" ("project_id");

-- chapter_summary
CREATE TABLE "chapter_summary__rebuild" (
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
INSERT INTO "chapter_summary__rebuild" SELECT * FROM "chapter_summary";
DROP TABLE "chapter_summary";
ALTER TABLE "chapter_summary__rebuild" RENAME TO "chapter_summary";
CREATE INDEX IF NOT EXISTS "idx_chapter_summary_project" ON "chapter_summary" ("project_id");

-- character_arc
CREATE TABLE "character_arc__rebuild" (
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
INSERT INTO "character_arc__rebuild" SELECT * FROM "character_arc";
DROP TABLE "character_arc";
ALTER TABLE "character_arc__rebuild" RENAME TO "character_arc";
CREATE INDEX IF NOT EXISTS "idx_char_arc_character" ON "character_arc" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_char_arc_volume" ON "character_arc" ("volume_id");

-- character_arc_potential
CREATE TABLE "character_arc_potential__rebuild" (
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
INSERT INTO "character_arc_potential__rebuild" SELECT * FROM "character_arc_potential";
DROP TABLE "character_arc_potential";
ALTER TABLE "character_arc_potential__rebuild" RENAME TO "character_arc_potential";
CREATE INDEX IF NOT EXISTS "idx_char_arc_entity" ON "character_arc_potential" ("entity_id");

-- character_capability
CREATE TABLE "character_capability__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "skills" TEXT,
    "limitations" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);
INSERT INTO "character_capability__rebuild" SELECT * FROM "character_capability";
DROP TABLE "character_capability";
ALTER TABLE "character_capability__rebuild" RENAME TO "character_capability";
CREATE INDEX IF NOT EXISTS "idx_char_capability_entity" ON "character_capability" ("entity_id");

-- character_conflict
CREATE TABLE "character_conflict__rebuild" (
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
INSERT INTO "character_conflict__rebuild" SELECT * FROM "character_conflict";
DROP TABLE "character_conflict";
ALTER TABLE "character_conflict__rebuild" RENAME TO "character_conflict";
CREATE INDEX IF NOT EXISTS "idx_char_conflict_entity" ON "character_conflict" ("entity_id");

-- character_drive
CREATE TABLE "character_drive__rebuild" (
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
INSERT INTO "character_drive__rebuild" SELECT * FROM "character_drive";
DROP TABLE "character_drive";
ALTER TABLE "character_drive__rebuild" RENAME TO "character_drive";
CREATE INDEX IF NOT EXISTS "idx_char_drive_entity" ON "character_drive" ("entity_id");

-- character_extension
CREATE TABLE "character_extension__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "extension_type" TEXT,
    "data" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);
INSERT INTO "character_extension__rebuild" SELECT * FROM "character_extension";
DROP TABLE "character_extension";
ALTER TABLE "character_extension__rebuild" RENAME TO "character_extension";
CREATE INDEX IF NOT EXISTS "idx_char_extension_entity" ON "character_extension" ("entity_id");

-- character_fear
CREATE TABLE "character_fear__rebuild" (
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
INSERT INTO "character_fear__rebuild" SELECT * FROM "character_fear";
DROP TABLE "character_fear";
ALTER TABLE "character_fear__rebuild" RENAME TO "character_fear";
CREATE INDEX IF NOT EXISTS "idx_fear_character" ON "character_fear" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_fear_project" ON "character_fear" ("project_id");

-- character_goal_mind
CREATE TABLE "character_goal_mind__rebuild" (
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
INSERT INTO "character_goal_mind__rebuild" SELECT * FROM "character_goal_mind";
DROP TABLE "character_goal_mind";
ALTER TABLE "character_goal_mind__rebuild" RENAME TO "character_goal_mind";
CREATE INDEX IF NOT EXISTS "idx_goal_mind_character" ON "character_goal_mind" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_goal_mind_project" ON "character_goal_mind" ("project_id");

-- character_memory
CREATE TABLE "character_memory__rebuild" (
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
INSERT INTO "character_memory__rebuild" SELECT * FROM "character_memory";
DROP TABLE "character_memory";
ALTER TABLE "character_memory__rebuild" RENAME TO "character_memory";
CREATE INDEX IF NOT EXISTS "idx_memory_character" ON "character_memory" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_memory_project" ON "character_memory" ("project_id");

-- character_profile
CREATE TABLE "character_profile__rebuild" (
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
INSERT INTO "character_profile__rebuild" SELECT * FROM "character_profile";
DROP TABLE "character_profile";
ALTER TABLE "character_profile__rebuild" RENAME TO "character_profile";
CREATE INDEX IF NOT EXISTS "idx_char_profile_entity" ON "character_profile" ("entity_id");

-- character_relationship
CREATE TABLE "character_relationship__rebuild" (
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
INSERT INTO "character_relationship__rebuild" SELECT * FROM "character_relationship";
DROP TABLE "character_relationship";
ALTER TABLE "character_relationship__rebuild" RENAME TO "character_relationship";
CREATE INDEX IF NOT EXISTS "idx_char_relationship_entity" ON "character_relationship" ("entity_id");

-- character_secret
CREATE TABLE "character_secret__rebuild" (
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
INSERT INTO "character_secret__rebuild" SELECT * FROM "character_secret";
DROP TABLE "character_secret";
ALTER TABLE "character_secret__rebuild" RENAME TO "character_secret";
CREATE INDEX IF NOT EXISTS "idx_char_secret_entity" ON "character_secret" ("entity_id");

-- character_state
CREATE TABLE "character_state__rebuild" (
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
INSERT INTO "character_state__rebuild" SELECT * FROM "character_state";
DROP TABLE "character_state";
ALTER TABLE "character_state__rebuild" RENAME TO "character_state";
CREATE INDEX IF NOT EXISTS "idx_char_state_entity" ON "character_state" ("entity_id");

-- character_trait
CREATE TABLE "character_trait__rebuild" (
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
INSERT INTO "character_trait__rebuild" SELECT * FROM "character_trait";
DROP TABLE "character_trait";
ALTER TABLE "character_trait__rebuild" RENAME TO "character_trait";
CREATE INDEX IF NOT EXISTS "idx_char_trait_entity" ON "character_trait" ("entity_id");

-- current_state
CREATE TABLE "current_state__rebuild" (
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
INSERT INTO "current_state__rebuild" SELECT * FROM "current_state";
DROP TABLE "current_state";
ALTER TABLE "current_state__rebuild" RENAME TO "current_state";
CREATE INDEX IF NOT EXISTS "idx_current_state_entity" ON "current_state" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_current_state_project" ON "current_state" ("project_id", "state_key");
CREATE UNIQUE INDEX IF NOT EXISTS "idx_current_state_active_unique" ON "current_state" ("project_id", "entity_id", "state_key") WHERE ("effective_to" IS NULL);

-- decision_trace
CREATE TABLE "decision_trace__rebuild" (
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
INSERT INTO "decision_trace__rebuild" SELECT * FROM "decision_trace";
DROP TABLE "decision_trace";
ALTER TABLE "decision_trace__rebuild" RENAME TO "decision_trace";
CREATE INDEX IF NOT EXISTS "idx_decision_character" ON "decision_trace" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_decision_project" ON "decision_trace" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_decision_scene" ON "decision_trace" ("scene_id");

-- emotion_state
CREATE TABLE "emotion_state__rebuild" (
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
INSERT INTO "emotion_state__rebuild" SELECT * FROM "emotion_state";
DROP TABLE "emotion_state";
ALTER TABLE "emotion_state__rebuild" RENAME TO "emotion_state";
CREATE INDEX IF NOT EXISTS "idx_emotion_character" ON "emotion_state" ("character_id");
CREATE INDEX IF NOT EXISTS "idx_emotion_project" ON "emotion_state" ("project_id");

-- entity
CREATE TABLE "entity__rebuild" (
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
INSERT INTO "entity__rebuild" SELECT * FROM "entity";
DROP TABLE "entity";
ALTER TABLE "entity__rebuild" RENAME TO "entity";
CREATE INDEX IF NOT EXISTS "idx_entity_name" ON "entity" ("project_id", "name");
CREATE INDEX IF NOT EXISTS "idx_entity_project" ON "entity" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_entity_status" ON "entity" ("project_id", "status");
CREATE INDEX IF NOT EXISTS "idx_entity_type" ON "entity" ("entity_type_id");

-- entity_arc_stage
CREATE TABLE "entity_arc_stage__rebuild" (
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
INSERT INTO "entity_arc_stage__rebuild" SELECT * FROM "entity_arc_stage";
DROP TABLE "entity_arc_stage";
ALTER TABLE "entity_arc_stage__rebuild" RENAME TO "entity_arc_stage";
CREATE INDEX IF NOT EXISTS "idx_entity_arc_stage_entity" ON "entity_arc_stage" ("entity_id");

-- event
CREATE TABLE "event__rebuild" (
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
INSERT INTO "event__rebuild" SELECT * FROM "event";
DROP TABLE "event";
ALTER TABLE "event__rebuild" RENAME TO "event";
CREATE INDEX IF NOT EXISTS "idx_event_project" ON "event" ("project_id");

-- event_entity
CREATE TABLE "event_entity__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "event_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (event_id) REFERENCES "event"(id) ON DELETE CASCADE
);
INSERT INTO "event_entity__rebuild" SELECT * FROM "event_entity";
DROP TABLE "event_entity";
ALTER TABLE "event_entity__rebuild" RENAME TO "event_entity";
CREATE INDEX IF NOT EXISTS "idx_event_entity_event" ON "event_entity" ("event_id");

-- event_outbox
CREATE TABLE "event_outbox__rebuild" (
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
INSERT INTO "event_outbox__rebuild" SELECT * FROM "event_outbox";
DROP TABLE "event_outbox";
ALTER TABLE "event_outbox__rebuild" RENAME TO "event_outbox";
CREATE INDEX IF NOT EXISTS "idx_outbox_created" ON "event_outbox" ("created_at");
CREATE INDEX IF NOT EXISTS "idx_outbox_project" ON "event_outbox" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_outbox_status" ON "event_outbox" ("status");

-- fact
CREATE TABLE "fact__rebuild" (
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
INSERT INTO "fact__rebuild" SELECT * FROM "fact";
DROP TABLE "fact";
ALTER TABLE "fact__rebuild" RENAME TO "fact";
CREATE INDEX IF NOT EXISTS "idx_fact_certainty" ON "fact" ("project_id", "certainty");
CREATE INDEX IF NOT EXISTS "idx_fact_project" ON "fact" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_fact_project_status" ON "fact" ("project_id", "status");

-- fact_entity
CREATE TABLE "fact_entity__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "fact_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (fact_id) REFERENCES "fact"(id) ON DELETE CASCADE
);
INSERT INTO "fact_entity__rebuild" SELECT * FROM "fact_entity";
DROP TABLE "fact_entity";
ALTER TABLE "fact_entity__rebuild" RENAME TO "fact_entity";
CREATE INDEX IF NOT EXISTS "idx_fact_entity_entity" ON "fact_entity" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_fact_entity_fact" ON "fact_entity" ("fact_id");

-- fact_visibility
CREATE TABLE "fact_visibility__rebuild" (
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
INSERT INTO "fact_visibility__rebuild" SELECT * FROM "fact_visibility";
DROP TABLE "fact_visibility";
ALTER TABLE "fact_visibility__rebuild" RENAME TO "fact_visibility";
CREATE INDEX IF NOT EXISTS "idx_fact_visibility_fact" ON "fact_visibility" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_fact_visibility_project" ON "fact_visibility" ("project_id");

-- faction_profile
CREATE TABLE "faction_profile__rebuild" (
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
INSERT INTO "faction_profile__rebuild" SELECT * FROM "faction_profile";
DROP TABLE "faction_profile";
ALTER TABLE "faction_profile__rebuild" RENAME TO "faction_profile";
CREATE INDEX IF NOT EXISTS "idx_faction_profile_entity" ON "faction_profile" ("entity_id");

-- foreshadowing
CREATE TABLE "foreshadowing__rebuild" (
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
INSERT INTO "foreshadowing__rebuild" SELECT * FROM "foreshadowing";
DROP TABLE "foreshadowing";
ALTER TABLE "foreshadowing__rebuild" RENAME TO "foreshadowing";
CREATE INDEX IF NOT EXISTS "idx_foreshadowing_project" ON "foreshadowing" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_foreshadowing_status" ON "foreshadowing" ("status");

-- generation_run
CREATE TABLE "generation_run__rebuild" (
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
INSERT INTO "generation_run__rebuild" SELECT * FROM "generation_run";
DROP TABLE "generation_run";
ALTER TABLE "generation_run__rebuild" RENAME TO "generation_run";
CREATE INDEX IF NOT EXISTS "idx_gen_run_task" ON "generation_run" ("task_id");

-- generation_task
CREATE TABLE "generation_task__rebuild" (
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
INSERT INTO "generation_task__rebuild" SELECT * FROM "generation_task";
DROP TABLE "generation_task";
ALTER TABLE "generation_task__rebuild" RENAME TO "generation_task";
CREATE INDEX IF NOT EXISTS "idx_gen_task_project" ON "generation_task" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_gen_task_status" ON "generation_task" ("status");

-- global_story_state
CREATE TABLE "global_story_state__rebuild" (
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
INSERT INTO "global_story_state__rebuild" SELECT * FROM "global_story_state";
DROP TABLE "global_story_state";
ALTER TABLE "global_story_state__rebuild" RENAME TO "global_story_state";
CREATE INDEX IF NOT EXISTS "idx_global_state_project" ON "global_story_state" ("project_id");

-- knowledge_gap
CREATE TABLE "knowledge_gap__rebuild" (
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
INSERT INTO "knowledge_gap__rebuild" SELECT * FROM "knowledge_gap";
DROP TABLE "knowledge_gap";
ALTER TABLE "knowledge_gap__rebuild" RENAME TO "knowledge_gap";
CREATE INDEX IF NOT EXISTS "idx_gap_project" ON "knowledge_gap" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_gap_status" ON "knowledge_gap" ("status");

-- knowledge_state
CREATE TABLE "knowledge_state__rebuild" (
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
INSERT INTO "knowledge_state__rebuild" SELECT * FROM "knowledge_state";
DROP TABLE "knowledge_state";
ALTER TABLE "knowledge_state__rebuild" RENAME TO "knowledge_state";
CREATE INDEX IF NOT EXISTS "idx_knowledge_fact" ON "knowledge_state" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_knowledge_project" ON "knowledge_state" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_knowledge_subject" ON "knowledge_state" ("subject_type", "subject_id");

-- location_connection
CREATE TABLE "location_connection__rebuild" (
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
INSERT INTO "location_connection__rebuild" SELECT * FROM "location_connection";
DROP TABLE "location_connection";
ALTER TABLE "location_connection__rebuild" RENAME TO "location_connection";
CREATE INDEX IF NOT EXISTS "idx_loc_conn_source" ON "location_connection" ("source_location_id");
CREATE INDEX IF NOT EXISTS "idx_loc_conn_target" ON "location_connection" ("target_location_id");

-- location_facilities
CREATE TABLE "location_facilities__rebuild" (
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
INSERT INTO "location_facilities__rebuild" SELECT * FROM "location_facilities";
DROP TABLE "location_facilities";
ALTER TABLE "location_facilities__rebuild" RENAME TO "location_facilities";
CREATE INDEX IF NOT EXISTS "idx_loc_facilities_entity" ON "location_facilities" ("entity_id");

-- location_facility
CREATE TABLE "location_facility__rebuild" (
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
INSERT INTO "location_facility__rebuild" SELECT * FROM "location_facility";
DROP TABLE "location_facility";
ALTER TABLE "location_facility__rebuild" RENAME TO "location_facility";
CREATE INDEX IF NOT EXISTS "idx_loc_facility_entity" ON "location_facility" ("entity_id");

-- location_geography
CREATE TABLE "location_geography__rebuild" (
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
INSERT INTO "location_geography__rebuild" SELECT * FROM "location_geography";
DROP TABLE "location_geography";
ALTER TABLE "location_geography__rebuild" RENAME TO "location_geography";

-- location_identity
CREATE TABLE "location_identity__rebuild" (
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
INSERT INTO "location_identity__rebuild" SELECT * FROM "location_identity";
DROP TABLE "location_identity";
ALTER TABLE "location_identity__rebuild" RENAME TO "location_identity";
CREATE INDEX IF NOT EXISTS "idx_loc_identity_entity" ON "location_identity" ("entity_id");

-- location_narrative_hooks
CREATE TABLE "location_narrative_hooks__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "hook_text" TEXT NOT NULL,
    "hook_type" TEXT,
    "related_entities" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);
INSERT INTO "location_narrative_hooks__rebuild" SELECT * FROM "location_narrative_hooks";
DROP TABLE "location_narrative_hooks";
ALTER TABLE "location_narrative_hooks__rebuild" RENAME TO "location_narrative_hooks";
CREATE INDEX IF NOT EXISTS "idx_loc_hooks_entity" ON "location_narrative_hooks" ("entity_id");

-- location_profile
CREATE TABLE "location_profile__rebuild" (
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
INSERT INTO "location_profile__rebuild" SELECT * FROM "location_profile";
DROP TABLE "location_profile";
ALTER TABLE "location_profile__rebuild" RENAME TO "location_profile";
CREATE INDEX IF NOT EXISTS "idx_loc_profile_entity" ON "location_profile" ("entity_id");

-- location_rules
CREATE TABLE "location_rules__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "rule_text" TEXT NOT NULL,
    "rule_type" TEXT,
    "enforced_by" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);
INSERT INTO "location_rules__rebuild" SELECT * FROM "location_rules";
DROP TABLE "location_rules";
ALTER TABLE "location_rules__rebuild" RENAME TO "location_rules";
CREATE INDEX IF NOT EXISTS "idx_loc_rules_entity" ON "location_rules" ("entity_id");

-- location_secret
CREATE TABLE "location_secret__rebuild" (
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
INSERT INTO "location_secret__rebuild" SELECT * FROM "location_secret";
DROP TABLE "location_secret";
ALTER TABLE "location_secret__rebuild" RENAME TO "location_secret";
CREATE INDEX IF NOT EXISTS "idx_loc_secret_entity" ON "location_secret" ("entity_id");

-- location_secrets
CREATE TABLE "location_secrets__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "secret_text" TEXT NOT NULL,
    "discovered_by" TEXT,
    "narrative_importance" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE
);
INSERT INTO "location_secrets__rebuild" SELECT * FROM "location_secrets";
DROP TABLE "location_secrets";
ALTER TABLE "location_secrets__rebuild" RENAME TO "location_secrets";
CREATE INDEX IF NOT EXISTS "idx_loc_secrets_entity" ON "location_secrets" ("entity_id");

-- location_threat
CREATE TABLE "location_threat__rebuild" (
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
INSERT INTO "location_threat__rebuild" SELECT * FROM "location_threat";
DROP TABLE "location_threat";
ALTER TABLE "location_threat__rebuild" RENAME TO "location_threat";
CREATE INDEX IF NOT EXISTS "idx_loc_threat_entity" ON "location_threat" ("entity_id");

-- location_threats
CREATE TABLE "location_threats__rebuild" (
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
INSERT INTO "location_threats__rebuild" SELECT * FROM "location_threats";
DROP TABLE "location_threats";
ALTER TABLE "location_threats__rebuild" RENAME TO "location_threats";
CREATE INDEX IF NOT EXISTS "idx_loc_threats_entity" ON "location_threats" ("entity_id");

-- memories
CREATE TABLE "memories__rebuild" (
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
INSERT INTO "memories__rebuild" SELECT * FROM "memories";
DROP TABLE "memories";
ALTER TABLE "memories__rebuild" RENAME TO "memories";
CREATE INDEX IF NOT EXISTS "idx_memories_importance" ON "memories" ("project_id", "importance" DESC);
CREATE INDEX IF NOT EXISTS "idx_memories_project" ON "memories" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_memories_type" ON "memories" ("project_id", "memory_type");

-- mutation_ledger
CREATE TABLE "mutation_ledger__rebuild" (
    "command_id" TEXT NOT NULL,
    "project_id" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'committed',
    "result" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (command_id),
    FOREIGN KEY (project_id) REFERENCES "project"(id) ON DELETE CASCADE
);
INSERT INTO "mutation_ledger__rebuild" SELECT * FROM "mutation_ledger";
DROP TABLE "mutation_ledger";
ALTER TABLE "mutation_ledger__rebuild" RENAME TO "mutation_ledger";
CREATE INDEX IF NOT EXISTS "idx_mutation_ledger_project" ON "mutation_ledger" ("project_id");

-- narrative_branch
CREATE TABLE "narrative_branch__rebuild" (
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
INSERT INTO "narrative_branch__rebuild" SELECT * FROM "narrative_branch";
DROP TABLE "narrative_branch";
ALTER TABLE "narrative_branch__rebuild" RENAME TO "narrative_branch";
CREATE INDEX IF NOT EXISTS "idx_narrative_branch_project" ON "narrative_branch" ("project_id");

-- narrative_budget
CREATE TABLE "narrative_budget__rebuild" (
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
INSERT INTO "narrative_budget__rebuild" SELECT * FROM "narrative_budget";
DROP TABLE "narrative_budget";
ALTER TABLE "narrative_budget__rebuild" RENAME TO "narrative_budget";
CREATE INDEX IF NOT EXISTS "idx_narr_budget_node" ON "narrative_budget" ("narrative_node_id");
CREATE INDEX IF NOT EXISTS "idx_narr_budget_project" ON "narrative_budget" ("project_id");

-- narrative_node
CREATE TABLE "narrative_node__rebuild" (
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
INSERT INTO "narrative_node__rebuild" SELECT * FROM "narrative_node";
DROP TABLE "narrative_node";
ALTER TABLE "narrative_node__rebuild" RENAME TO "narrative_node";
CREATE INDEX IF NOT EXISTS "idx_narrative_parent" ON "narrative_node" ("parent_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_project" ON "narrative_node" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narrative_type" ON "narrative_node" ("project_id", "node_type");
CREATE UNIQUE INDEX IF NOT EXISTS "idx_narrative_sort_order_unique" ON "narrative_node" ("project_id", "parent_id", "sort_order");

-- narrative_state
CREATE TABLE "narrative_state__rebuild" (
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
INSERT INTO "narrative_state__rebuild" SELECT * FROM "narrative_state";
DROP TABLE "narrative_state";
ALTER TABLE "narrative_state__rebuild" RENAME TO "narrative_state";
CREATE INDEX IF NOT EXISTS "idx_narrative_state_dimension" ON "narrative_state" ("state_dimension");
CREATE INDEX IF NOT EXISTS "idx_narrative_state_key" ON "narrative_state" ("state_key");
CREATE INDEX IF NOT EXISTS "idx_narrative_state_project" ON "narrative_state" ("project_id");

-- narrative_thread
CREATE TABLE "narrative_thread__rebuild" (
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
INSERT INTO "narrative_thread__rebuild" SELECT * FROM "narrative_thread";
DROP TABLE "narrative_thread";
ALTER TABLE "narrative_thread__rebuild" RENAME TO "narrative_thread";
CREATE INDEX IF NOT EXISTS "idx_narr_thread_project" ON "narrative_thread" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_narr_thread_storyline" ON "narrative_thread" ("storyline_id");

-- narrative_thread_participant
CREATE TABLE "narrative_thread_participant__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "thread_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (thread_id) REFERENCES "narrative_thread"(id) ON DELETE CASCADE
);
INSERT INTO "narrative_thread_participant__rebuild" SELECT * FROM "narrative_thread_participant";
DROP TABLE "narrative_thread_participant";
ALTER TABLE "narrative_thread_participant__rebuild" RENAME TO "narrative_thread_participant";
CREATE INDEX IF NOT EXISTS "idx_narr_thread_part_thread" ON "narrative_thread_participant" ("thread_id");

-- novel_state_snapshot
CREATE TABLE "novel_state_snapshot__rebuild" (
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
INSERT INTO "novel_state_snapshot__rebuild" SELECT * FROM "novel_state_snapshot";
DROP TABLE "novel_state_snapshot";
ALTER TABLE "novel_state_snapshot__rebuild" RENAME TO "novel_state_snapshot";
CREATE INDEX IF NOT EXISTS "idx_novel_snap_project" ON "novel_state_snapshot" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_novel_snap_scene" ON "novel_state_snapshot" ("scene_id");

-- plot
CREATE TABLE "plot__rebuild" (
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
INSERT INTO "plot__rebuild" SELECT * FROM "plot";
DROP TABLE "plot";
ALTER TABLE "plot__rebuild" RENAME TO "plot";
CREATE INDEX IF NOT EXISTS "idx_plot_project" ON "plot" ("project_id");

-- plot_repair
CREATE TABLE "plot_repair__rebuild" (
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
INSERT INTO "plot_repair__rebuild" SELECT * FROM "plot_repair";
DROP TABLE "plot_repair";
ALTER TABLE "plot_repair__rebuild" RENAME TO "plot_repair";
CREATE INDEX IF NOT EXISTS "idx_plot_repair_project" ON "plot_repair" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_plot_repair_scene" ON "plot_repair" ("scene_id");

-- proposed_change
CREATE TABLE "proposed_change__rebuild" (
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
INSERT INTO "proposed_change__rebuild" SELECT * FROM "proposed_change";
DROP TABLE "proposed_change";
ALTER TABLE "proposed_change__rebuild" RENAME TO "proposed_change";
CREATE INDEX IF NOT EXISTS "idx_prop_change_project" ON "proposed_change" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_prop_change_status" ON "proposed_change" ("status");
CREATE INDEX IF NOT EXISTS "idx_prop_change_task" ON "proposed_change" ("task_id");

-- quality_score
CREATE TABLE "quality_score__rebuild" (
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
INSERT INTO "quality_score__rebuild" SELECT * FROM "quality_score";
DROP TABLE "quality_score";
ALTER TABLE "quality_score__rebuild" RENAME TO "quality_score";
CREATE INDEX IF NOT EXISTS "idx_quality_score_project" ON "quality_score" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_quality_score_scene" ON "quality_score" ("scene_id");

-- reader_knowledge
CREATE TABLE "reader_knowledge__rebuild" (
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
INSERT INTO "reader_knowledge__rebuild" SELECT * FROM "reader_knowledge";
DROP TABLE "reader_knowledge";
ALTER TABLE "reader_knowledge__rebuild" RENAME TO "reader_knowledge";
CREATE INDEX IF NOT EXISTS "idx_reader_knowledge_fact" ON "reader_knowledge" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_reader_knowledge_project" ON "reader_knowledge" ("project_id");

-- relation
CREATE TABLE "relation__rebuild" (
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
INSERT INTO "relation__rebuild" SELECT * FROM "relation";
DROP TABLE "relation";
ALTER TABLE "relation__rebuild" RENAME TO "relation";
CREATE INDEX IF NOT EXISTS "idx_relation_project" ON "relation" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_relation_source" ON "relation" ("source_entity_id");
CREATE INDEX IF NOT EXISTS "idx_relation_target" ON "relation" ("target_entity_id");
CREATE INDEX IF NOT EXISTS "idx_relation_type" ON "relation" ("project_id", "relation_type");

-- resource_state
CREATE TABLE "resource_state__rebuild" (
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
INSERT INTO "resource_state__rebuild" SELECT * FROM "resource_state";
DROP TABLE "resource_state";
ALTER TABLE "resource_state__rebuild" RENAME TO "resource_state";
CREATE INDEX IF NOT EXISTS "idx_resource_location" ON "resource_state" ("location_id");
CREATE UNIQUE INDEX IF NOT EXISTS "resource_state_project_id_location_id_resource_name_key" ON "resource_state" ("project_id", "location_id", "resource_name");

-- revelation
CREATE TABLE "revelation__rebuild" (
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
INSERT INTO "revelation__rebuild" SELECT * FROM "revelation";
DROP TABLE "revelation";
ALTER TABLE "revelation__rebuild" RENAME TO "revelation";
CREATE INDEX IF NOT EXISTS "idx_revelation_fact" ON "revelation" ("fact_id");
CREATE INDEX IF NOT EXISTS "idx_revelation_scene" ON "revelation" ("scene_id");

-- revelation_target
CREATE TABLE "revelation_target__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "revelation_id" TEXT NOT NULL,
    "subject_type" TEXT NOT NULL,
    "subject_id" TEXT,
    "knowledge_level" TEXT NOT NULL DEFAULT 'Complete',
    PRIMARY KEY (id),
    FOREIGN KEY (revelation_id) REFERENCES "revelation"(id) ON DELETE CASCADE
);
INSERT INTO "revelation_target__rebuild" SELECT * FROM "revelation_target";
DROP TABLE "revelation_target";
ALTER TABLE "revelation_target__rebuild" RENAME TO "revelation_target";
CREATE INDEX IF NOT EXISTS "idx_rev_target_revelation" ON "revelation_target" ("revelation_id");

-- revision_plan
CREATE TABLE "revision_plan__rebuild" (
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
INSERT INTO "revision_plan__rebuild" SELECT * FROM "revision_plan";
DROP TABLE "revision_plan";
ALTER TABLE "revision_plan__rebuild" RENAME TO "revision_plan";
CREATE INDEX IF NOT EXISTS "idx_revision_project" ON "revision_plan" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_revision_scene" ON "revision_plan" ("scene_id");

-- scene
CREATE TABLE "scene__rebuild" (
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
INSERT INTO "scene__rebuild" SELECT * FROM "scene";
DROP TABLE "scene";
ALTER TABLE "scene__rebuild" RENAME TO "scene";
CREATE INDEX IF NOT EXISTS "idx_scene_node" ON "scene" ("narrative_node_id");

-- scene_document
CREATE TABLE "scene_document__rebuild" (
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
INSERT INTO "scene_document__rebuild" SELECT * FROM "scene_document";
DROP TABLE "scene_document";
ALTER TABLE "scene_document__rebuild" RENAME TO "scene_document";
CREATE INDEX IF NOT EXISTS "idx_scene_doc_scene" ON "scene_document" ("scene_id");

-- scene_entity
CREATE TABLE "scene_entity__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scene_id" TEXT NOT NULL,
    "entity_id" TEXT NOT NULL,
    "role" TEXT,
    "notes" TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (entity_id) REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);
INSERT INTO "scene_entity__rebuild" SELECT * FROM "scene_entity";
DROP TABLE "scene_entity";
ALTER TABLE "scene_entity__rebuild" RENAME TO "scene_entity";
CREATE INDEX IF NOT EXISTS "idx_scene_entity_entity" ON "scene_entity" ("entity_id");
CREATE INDEX IF NOT EXISTS "idx_scene_entity_scene" ON "scene_entity" ("scene_id");

-- scene_ledger
CREATE TABLE "scene_ledger__rebuild" (
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
INSERT INTO "scene_ledger__rebuild" SELECT * FROM "scene_ledger";
DROP TABLE "scene_ledger";
ALTER TABLE "scene_ledger__rebuild" RENAME TO "scene_ledger";
CREATE INDEX IF NOT EXISTS "idx_ledger_project" ON "scene_ledger" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_ledger_scene" ON "scene_ledger" ("scene_id");

-- scene_requirement
CREATE TABLE "scene_requirement__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "scene_id" TEXT NOT NULL,
    "requirement_type" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "priority" TEXT NOT NULL DEFAULT 'Should',
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (scene_id) REFERENCES "scene"(id) ON DELETE CASCADE
);
INSERT INTO "scene_requirement__rebuild" SELECT * FROM "scene_requirement";
DROP TABLE "scene_requirement";
ALTER TABLE "scene_requirement__rebuild" RENAME TO "scene_requirement";
CREATE INDEX IF NOT EXISTS "idx_scene_req_scene" ON "scene_requirement" ("scene_id");

-- state_change
CREATE TABLE "state_change__rebuild" (
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
INSERT INTO "state_change__rebuild" SELECT * FROM "state_change";
DROP TABLE "state_change";
ALTER TABLE "state_change__rebuild" RENAME TO "state_change";
CREATE INDEX IF NOT EXISTS "idx_state_change_entity" ON "state_change" ("target_entity_id");
CREATE INDEX IF NOT EXISTS "idx_state_change_project" ON "state_change" ("project_id");

-- state_snapshot
CREATE TABLE "state_snapshot__rebuild" (
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
INSERT INTO "state_snapshot__rebuild" SELECT * FROM "state_snapshot";
DROP TABLE "state_snapshot";
ALTER TABLE "state_snapshot__rebuild" RENAME TO "state_snapshot";
CREATE INDEX IF NOT EXISTS "idx_snapshot_project" ON "state_snapshot" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_snapshot_scene" ON "state_snapshot" ("scene_id");

-- storyline
CREATE TABLE "storyline__rebuild" (
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
INSERT INTO "storyline__rebuild" SELECT * FROM "storyline";
DROP TABLE "storyline";
ALTER TABLE "storyline__rebuild" RENAME TO "storyline";
CREATE INDEX IF NOT EXISTS "idx_storyline_project" ON "storyline" ("project_id");

-- storyline_relation
CREATE TABLE "storyline_relation__rebuild" (
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
INSERT INTO "storyline_relation__rebuild" SELECT * FROM "storyline_relation";
DROP TABLE "storyline_relation";
ALTER TABLE "storyline_relation__rebuild" RENAME TO "storyline_relation";
CREATE INDEX IF NOT EXISTS "idx_storyline_rel_child" ON "storyline_relation" ("child_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_rel_parent" ON "storyline_relation" ("parent_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_rel_project" ON "storyline_relation" ("project_id");
CREATE UNIQUE INDEX IF NOT EXISTS "storyline_relation_parent_id_child_id_key" ON "storyline_relation" ("parent_id", "child_id");

-- storyline_scene
CREATE TABLE "storyline_scene__rebuild" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "storyline_id" TEXT NOT NULL,
    "scene_id" TEXT NOT NULL,
    "significance" TEXT,
    "created_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (storyline_id) REFERENCES "storyline"(id) ON DELETE CASCADE
);
INSERT INTO "storyline_scene__rebuild" SELECT * FROM "storyline_scene";
DROP TABLE "storyline_scene";
ALTER TABLE "storyline_scene__rebuild" RENAME TO "storyline_scene";
CREATE INDEX IF NOT EXISTS "idx_storyline_scene_scene" ON "storyline_scene" ("scene_id");
CREATE INDEX IF NOT EXISTS "idx_storyline_scene_storyline" ON "storyline_scene" ("storyline_id");

-- system_events
CREATE TABLE "system_events__rebuild" (
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
INSERT INTO "system_events__rebuild" SELECT * FROM "system_events";
DROP TABLE "system_events";
ALTER TABLE "system_events__rebuild" RENAME TO "system_events";
CREATE INDEX IF NOT EXISTS "idx_system_event_entity" ON "system_events" ("entity_type", "entity_id");
CREATE INDEX IF NOT EXISTS "idx_system_event_project" ON "system_events" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_system_event_time" ON "system_events" ("project_id", "created_at" DESC);
CREATE INDEX IF NOT EXISTS "idx_system_event_type" ON "system_events" ("project_id", "event_type");

-- test_case
CREATE TABLE "test_case__rebuild" (
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
INSERT INTO "test_case__rebuild" SELECT * FROM "test_case";
DROP TABLE "test_case";
ALTER TABLE "test_case__rebuild" RENAME TO "test_case";
CREATE INDEX IF NOT EXISTS "idx_test_project" ON "test_case" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_test_type" ON "test_case" ("test_type");

-- timeline_event
CREATE TABLE "timeline_event__rebuild" (
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
INSERT INTO "timeline_event__rebuild" SELECT * FROM "timeline_event";
DROP TABLE "timeline_event";
ALTER TABLE "timeline_event__rebuild" RENAME TO "timeline_event";
CREATE INDEX IF NOT EXISTS "idx_timeline_project" ON "timeline_event" ("project_id");
CREATE INDEX IF NOT EXISTS "idx_timeline_sort" ON "timeline_event" ("project_id", "sort_key");

-- validation_issue
CREATE TABLE "validation_issue__rebuild" (
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
INSERT INTO "validation_issue__rebuild" SELECT * FROM "validation_issue";
DROP TABLE "validation_issue";
ALTER TABLE "validation_issue__rebuild" RENAME TO "validation_issue";
CREATE INDEX IF NOT EXISTS "idx_val_issue_run" ON "validation_issue" ("validation_run_id");

-- validation_run
CREATE TABLE "validation_run__rebuild" (
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
INSERT INTO "validation_run__rebuild" SELECT * FROM "validation_run";
DROP TABLE "validation_run";
ALTER TABLE "validation_run__rebuild" RENAME TO "validation_run";
CREATE INDEX IF NOT EXISTS "idx_val_run_task" ON "validation_run" ("task_id");

-- volume_summary
CREATE TABLE "volume_summary__rebuild" (
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
INSERT INTO "volume_summary__rebuild" SELECT * FROM "volume_summary";
DROP TABLE "volume_summary";
ALTER TABLE "volume_summary__rebuild" RENAME TO "volume_summary";
CREATE INDEX IF NOT EXISTS "idx_volume_summary_project" ON "volume_summary" ("project_id");

-- world
CREATE TABLE "world__rebuild" (
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
INSERT INTO "world__rebuild" SELECT * FROM "world";
DROP TABLE "world";
ALTER TABLE "world__rebuild" RENAME TO "world";
CREATE INDEX IF NOT EXISTS "idx_world_project" ON "world" ("project_id");

-- world_branch
CREATE TABLE "world_branch__rebuild" (
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
INSERT INTO "world_branch__rebuild" SELECT * FROM "world_branch";
DROP TABLE "world_branch";
ALTER TABLE "world_branch__rebuild" RENAME TO "world_branch";
CREATE INDEX IF NOT EXISTS "idx_world_branch_project" ON "world_branch" ("project_id");

PRAGMA defer_foreign_keys = OFF;
