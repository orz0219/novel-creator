-- Agent 系统提示词自定义基座（落库，支持全局与按项目覆盖）。
-- scope 唯一：'global' 表示全局默认；按项目覆盖时用 'project:<uuid>'。
CREATE TABLE IF NOT EXISTS agent_prompts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope TEXT NOT NULL UNIQUE,
    project_id UUID NULL,
    system_prompt TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
