-- 生成任务补充 skill / scene 关联列（此前代码已引用但表缺少这两列）
ALTER TABLE generation_task ADD COLUMN IF NOT EXISTS skill_id uuid NULL;
ALTER TABLE generation_task ADD COLUMN IF NOT EXISTS scene_id uuid NULL;
