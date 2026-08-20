-- 006: generation_run 增加可复现性元数据（ChatGPT 评审 P1：AI Trace 必须可复现）
ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS reproducibility_meta jsonb NOT NULL DEFAULT '{}'::jsonb;
