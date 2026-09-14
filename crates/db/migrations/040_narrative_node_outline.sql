-- ============================================================
-- 细纲（narrative_node）补齐「可移动、可重排、可挂载」
--
-- 背景（来自 AI 使用方的实测反馈，已核实）：节点树能建，但不能动树。
--   1. revise_node 改不了 parent_id / sort_order —— 卷第 3 章建错了位置，
--      只能 retire + 重建；重建后节点 id 变了，而伏笔的 planted_node_id /
--      payoff_node_id 还指着旧 id，暗线直接脱钩，逻辑删除的老人节点还留在库里。
--   2. 节点「服务哪条故事线、推进到哪个阶段」没有字段，只能靠 attributes 约定
--      （实测能塞进去、也能读回来，但等于没有护栏，也没有结构可比对）。
--   3. 场景级挂载（谁在场 / 在哪 / 用了什么道具）同样只能塞 attributes。
--   4. 元数据（预计章数 / 预计字数 / 故事时间跨度）只能塞 attributes。
--
-- 本迁移把这些从「约定」升级成列，并让兄弟重排成为可能。
--
-- 为什么 stage_refs 用 JSONB 而不是建关联表：
--   与 035_storyline_arc_stages.sql 同一判断——节点对「线 × 阶段」的引用不需要
--   按列过滤或参与外键级联，前端只读展示、AI 整体读写；建表要新增 repo + 端口 +
--   两套实现 + 两套迁移，收益不匹配。主挂载（storyline_id / arc_stage）仍然是
--   正式列，便于按线检索。
--
-- 幂等，可重复执行。
-- ============================================================

-- 主挂载：这条节点服务哪条故事线、推进到哪个阶段（阶段名与 storyline.arc_stages[].stage 对齐）
ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS storyline_id UUID REFERENCES storyline(id);

ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS arc_stage VARCHAR;

-- 附加挂载：一个节点可能同时服务多条线 / 多个阶段（实测里是「五条线的 stage_refs」）。
-- 形状：数组，元素为对象 { storyline_id, arc_stage }。
ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS stage_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

-- 场景级挂载：在场角色 / 地点 / 道具
ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS participant_entity_ids JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS location_id UUID REFERENCES entity(id);

ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS item_ids JSONB NOT NULL DEFAULT '[]'::jsonb;

-- 元数据：预计章数 / 预计字数 / 故事时间跨度
ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS estimated_chapters INTEGER;

ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS estimated_words INTEGER;

ALTER TABLE narrative_node
    ADD COLUMN IF NOT EXISTS story_time VARCHAR;

CREATE INDEX IF NOT EXISTS idx_narrative_storyline ON narrative_node(storyline_id);
CREATE INDEX IF NOT EXISTS idx_narrative_location_node ON narrative_node(location_id);

-- 兄弟重排：原来的唯一索引是**不可延迟**的，事务内交换两个兄弟的 sort_order
-- 会在第二条 UPDATE 时撞上第一条尚未提交的旧值（唯一冲突），于是「上移/下移」
-- 这种最基本的操作在当前结构下必然失败。
-- 换成 DEFERRABLE INITIALLY DEFERRED 的约束：约束在事务提交时才检查，
-- 事务内可以自由地把一组兄弟整体重排，唯一性保障不丢。
ALTER TABLE narrative_node
    DROP CONSTRAINT IF EXISTS narrative_node_sort_order_unique;

DROP INDEX IF EXISTS idx_narrative_sort_order_unique;

ALTER TABLE narrative_node
    ADD CONSTRAINT narrative_node_sort_order_unique
    UNIQUE (project_id, parent_id, sort_order) DEFERRABLE INITIALLY DEFERRED;

-- 事件 ↔ 节点：让「这一章发生了什么」和事件表对上。
-- 原来只能靠事件名 / description 里的文字描述，检索不到、也串联不起来。
ALTER TABLE event
    ADD COLUMN IF NOT EXISTS narrative_node_id UUID REFERENCES narrative_node(id);

CREATE INDEX IF NOT EXISTS idx_event_narrative_node ON event(narrative_node_id);

-- ------------------------------------------------------------
-- 历史脏数据归一：把不认识的 node_type 转成显式自定义类型
--
-- 此前写入路径没有任何枚举校验（实测把 "foobar_test" 原样写进了库），
-- 而读取路径会把不认识的值**静默当成 Scene**。现在读取是严格的，
-- 若不先归一，这些历史行会让整棵细纲读不出来。
-- 归一成 `custom:原词` 而不是删改：原词是作者写的，保留下来仍可用。
-- ------------------------------------------------------------
UPDATE narrative_node
SET node_type = 'custom:' || node_type
WHERE node_type NOT IN (
        'Volume', 'Arc', 'Sequence', 'Chapter', 'Scene',
        'Beat', 'Storyline', 'SubArc', 'Special'
    )
  AND node_type NOT LIKE 'custom:%'
  AND node_type NOT LIKE '自定义:%';
