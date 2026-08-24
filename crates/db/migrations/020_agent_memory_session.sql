-- 记忆改为会话级：支持当前会话内跨刷新保留偏好/设定，
-- 不再强制依赖 project（当前 Agent 会话 project_id 为 NULL，原 NOT NULL 导致无法落库）。
ALTER TABLE agent_memory ALTER COLUMN project_id DROP NOT NULL;
ALTER TABLE agent_memory ADD COLUMN IF NOT EXISTS session_id UUID REFERENCES agent_sessions(id) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_agent_memory_project;
CREATE INDEX IF NOT EXISTS idx_agent_memory_session ON agent_memory(session_id);
