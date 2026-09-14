-- ============================================================
-- 对话滚动摘要（session_summary）
--
-- 背景：会话收尾时要把「故事现在到哪了」沉淀下来，供之后新开的会话自动读到。
-- 之前只有 agent_memory（碎片结论，只增不减）和会话消息（整段拍平后塞进请求），
-- 两者都会无限膨胀，没有一份「当前状态」的滚动快照。
--
-- 关键约束：**每个项目恒定一份**。因此 project_id 直接做主键，
-- 写入走 ON CONFLICT DO UPDATE（覆盖），从表结构上杜绝出现多份摘要 ——
-- 若允许追加，就会从「历史膨胀」变成「摘要膨胀」。
-- ============================================================

CREATE TABLE IF NOT EXISTS session_summary (
    -- 项目 id 即主键：一个项目只能有一份摘要
    project_id UUID PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE,
    -- 固定字段的结构化摘要（story_state / confirmed / open_threads / next_step）
    content JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
