-- 005: context_snapshot 增加可复现性元数据（ChatGPT 评审 P1）
-- 记录 world_version / policy_version / model / temperature / retrieval / prompt_hash 等，
-- 使同一份 Context Snapshot 可被审计与重放。
ALTER TABLE context_snapshot ADD COLUMN IF NOT EXISTS reproducibility_meta jsonb NOT NULL DEFAULT '{}'::jsonb;
