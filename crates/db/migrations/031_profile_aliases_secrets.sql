-- ============================================================
-- 地点 / 势力档案补 aliases；地点补 secrets
--
-- 背景（来自 AI 使用方的实际反馈，已核实）：
-- 1. 别名此前只能硬塞进 name，例如「天柱山脉（盘脊）」「无尽之海（死亡之海）· 盘沿」。
--    后果是 name 越写越长，而且系统里「盘脊」与「天柱山脉」是两个不同的串——
--    按别名检索、按别名去重全都做不到。角色（character_profile.aliases）已有该列，
--    地点与势力照抄即可。
-- 2. 地点有秘密但无处安放：势力（faction_profile.secrets）与角色都有 secrets 列，
--    唯独地点没有，只能把它写进 description —— 而 description 不参与「揭示状态」
--    管理，暗线（例如「那尊无名神像原本属于谁」）因此无法被前端按揭示程度跟踪。
--
-- 列形态与 character_profile.aliases 保持一致（JSONB NOT NULL DEFAULT '[]'），
-- 这样读取路径不必处处处理 NULL。
--
-- 幂等，可重复执行。
-- ============================================================

-- 地点档案：别名
ALTER TABLE location_profile
    ADD COLUMN IF NOT EXISTS aliases JSONB NOT NULL DEFAULT '[]'::jsonb;

-- 地点档案：秘密（不为外人所知的事）
ALTER TABLE location_profile
    ADD COLUMN IF NOT EXISTS secrets TEXT;

-- 势力档案：别名
ALTER TABLE faction_profile
    ADD COLUMN IF NOT EXISTS aliases JSONB NOT NULL DEFAULT '[]'::jsonb;
