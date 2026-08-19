-- P1-5: current_state 增加 active-row 唯一约束
-- 防止并发首次写入时产生多个 active state
-- WHERE effective_to IS NULL 确保只对 active 行生效

-- 先清理可能存在的重复 active 行（如果有）
-- 保留最新的那条，标记其他为 expired
WITH ranked AS (
    SELECT id, project_id, entity_id, state_key,
           ROW_NUMBER() OVER (
               PARTITION BY project_id, entity_id, state_key
               ORDER BY created_at DESC
           ) AS rn
    FROM current_state
    WHERE effective_to IS NULL
)
UPDATE current_state
SET effective_to = NOW()
FROM ranked
WHERE current_state.id = ranked.id
  AND ranked.rn > 1;

-- 创建 partial unique index：每个 (project_id, entity_id, state_key) 只能有一条 active 行
CREATE UNIQUE INDEX IF NOT EXISTS idx_current_state_active_unique
ON current_state (project_id, entity_id, state_key)
WHERE effective_to IS NULL;
