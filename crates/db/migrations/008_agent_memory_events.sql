-- Migration 008: Agent Runs, Memory, and System Events
-- Required by 数据库重构.txt §14, §15

-- ============================================================
-- 1. MEMORIES - Cross-session memory system (§14)
-- ============================================================
CREATE TABLE IF NOT EXISTS memories (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    memory_type VARCHAR NOT NULL DEFAULT 'general',
    content TEXT NOT NULL,
    importance DOUBLE PRECISION DEFAULT 0.5,
    source VARCHAR,
    embedding_vector_id VARCHAR,
    metadata JSONB DEFAULT '{}'::jsonb,
    access_count INTEGER DEFAULT 0,
    last_accessed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id);
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(project_id, memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(project_id, importance DESC);

-- ============================================================
-- 2. AGENT_RUNS - Agent execution history (§15)
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_runs (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL REFERENCES project(id),
    agent_type VARCHAR NOT NULL,
    task_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    input JSONB DEFAULT '{}'::jsonb,
    output JSONB DEFAULT '{}'::jsonb,
    error_message TEXT,
    model VARCHAR,
    provider VARCHAR,
    context_snapshot_id VARCHAR,
    tokens_used INTEGER,
    duration_ms BIGINT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_project ON agent_runs(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(project_id, status);
CREATE INDEX IF NOT EXISTS idx_agent_runs_type ON agent_runs(project_id, agent_type);

-- ============================================================
-- 3. SYSTEM_EVENTS - System event log (§15)
-- ============================================================
CREATE TABLE IF NOT EXISTS system_events (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR REFERENCES project(id),
    event_type VARCHAR NOT NULL,
    entity_type VARCHAR,
    entity_id VARCHAR,
    actor VARCHAR NOT NULL DEFAULT 'system',
    description TEXT NOT NULL,
    old_value JSONB,
    new_value JSONB,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_system_events_project ON system_events(project_id);
CREATE INDEX IF NOT EXISTS idx_system_events_type ON system_events(project_id, event_type);
CREATE INDEX IF NOT EXISTS idx_system_events_entity ON system_events(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_system_events_time ON system_events(project_id, created_at DESC);
