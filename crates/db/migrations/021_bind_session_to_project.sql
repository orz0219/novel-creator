-- 将 Agent 会话与项目强绑定（对话-项目绑定，P2）。
--
-- 背景：019 中 agent_sessions.project_id 为可空；020 又把 agent_memory 改成
-- 「会话级」（新增 session_id 列、project_id 改为可空）。本次反向 020 的会话级
-- 改动，使记忆回到「项目级」，并让会话必须归属一个项目。
--
-- 1) 清理 project_id 为 NULL 的孤儿会话及其消息（测试期遗留数据）。
-- 2) agent_sessions.project_id 改为 NOT NULL（从结构上杜绝新的孤儿对话）。
-- 3) agent_memory 回到「项目级」作用域：删除 session_id 列，project_id 恢复
--    NOT NULL，并重建按项目的索引。

DELETE FROM agent_messages
WHERE session_id IN (SELECT id FROM agent_sessions WHERE project_id IS NULL);

DELETE FROM agent_sessions WHERE project_id IS NULL;

ALTER TABLE agent_sessions ALTER COLUMN project_id SET NOT NULL;

DELETE FROM agent_memory WHERE project_id IS NULL;
ALTER TABLE agent_memory DROP COLUMN IF EXISTS session_id;
ALTER TABLE agent_memory ALTER COLUMN project_id SET NOT NULL;
DROP INDEX IF EXISTS idx_agent_memory_session;
CREATE INDEX IF NOT EXISTS idx_agent_memory_project ON agent_memory(project_id);
