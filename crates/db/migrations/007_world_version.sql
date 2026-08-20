-- 007: world_version 表（ChatGPT 评审 P2：世界版本边界，类比 git commit）
-- 每次 AI 提案被提交，世界前进一个版本（World v100 -> v101），支撑多人 /
-- 多 Agent 协同与回滚，也是"这次生成为何与上次不同"的可解释性基础之一。
CREATE TABLE IF NOT EXISTS world_version (
    id uuid PRIMARY KEY,
    world_id uuid NOT NULL,
    version integer NOT NULL,
    kind varchar NOT NULL,
    trigger_id uuid,
    summary text,
    parent_version_id uuid,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    UNIQUE (world_id, version)
);
