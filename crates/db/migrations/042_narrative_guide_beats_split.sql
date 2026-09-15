-- 042: 引导流程拆步 —— 把存量项目的 current_step 从 `beats` 迁移到 `beats.chapters`
--
-- 背景：`beats`（细纲）原先是一步，而且**就是流程终点**（next 为空字符串）。
-- 结果是 AI 建完卷/弧就按宽松的完成判据宣布"细纲已就绪"，而用户认为细纲才刚开始；
-- 同时前端那个"确认推进"按钮在最后一步是禁用的「已是最后一步」，
-- AI 却被提示词要求请用户去点它 —— 三方直接矛盾。
--
-- 2026-09 拆成三步（见 crates/agent/src/guide.rs 的 STEPS）：
--     beats.chapters  卷 → 弧 → 章，每章一句话事件（description）
--     beats.scenes    把接下来要写的 3-5 章展开成带 attributes 的场景
--     writing         正文写作，真正的流程终点
--
-- 为什么统一映射到 `beats.chapters` 而不是 `beats.scenes`：
-- 存量项目已经建过卷/弧（部分还有章），正是 `beats.chapters` 的产物形态；
-- 但这些章普遍没有 description（空壳章），按新的完成判据（每章必须有非空 description、
-- 每个弧下至少 2 章、每条重要故事线都挂到节点）仍应停在 `beats.chapters` 补齐内容，
-- 跳到 beats.scenes 会让 AI 拿着"展开场景"的说明书去补章表。
--
-- 幂等：只更新 config->>'current_step' 恰好等于 'beats' 的行，重跑无副作用。
-- 找不到该键（config 为 NULL 或没有 current_step）的项目不动 —— 它们由代码按
-- 起始阶段（premise）处理，迁移不该替它们编造阶段。

UPDATE project
SET config = jsonb_set(
        COALESCE(config, '{}'::jsonb),
        '{current_step}',
        to_jsonb('beats.chapters'::text)
    ),
    updated_at = NOW()
WHERE config->>'current_step' = 'beats';

-- 校验（人工执行时可用来确认迁移结果）：
--   SELECT id, name, config->>'current_step' FROM project ORDER BY updated_at DESC;
