-- ============================================================
-- 伏笔等级字段：归一 + 备注分离 + 数据库层约束
--
-- 背景（实测，已核实）：
--   importance 同时存在 Important(47) / 重要(2) / Main(1) / Major(1)；
--   hint_level 同时存在 Hidden(47) / Low(1) / 低（前期只透风，不揭示）(1) /
--                            低（前期只作背景异象，不点破）(1) / 中期显形(1)。
--
-- 根因：工具层（create_foreshadow / revise_foreshadow）用 `opt_enum_arg` 严格校验，
-- 但 **HTTP API 直接透传字符串**（`input.importance.as_deref().unwrap_or("Normal")`），
-- 于是任何自由文本都能落库。后果是前端无法按等级排序 / 过滤 / 画热力图——
-- 同一个字段既是标签又是段落。
--
-- 本迁移做三件事：
--   1. 加 `hint_note`：把"等级之外的那半句话"保留下来（**不丢信息**）；
--   2. 把已有取值归一成枚举（先精确映射，再兜底归一并把原文存进 hint_note）；
--   3. 加 CHECK 约束，让数据库层再也写不进非法值。
--
-- 幂等，可重复执行。
-- ============================================================

ALTER TABLE foreshadowing ADD COLUMN IF NOT EXISTS hint_note text;

-- 1) importance：精确映射已知的中英写法
UPDATE foreshadowing SET importance = 'Core'      WHERE importance IN ('Main', 'main', '主线', '核心');
UPDATE foreshadowing SET importance = 'Important' WHERE importance IN ('重要', 'Major', 'major');
UPDATE foreshadowing SET importance = 'Minor'     WHERE importance IN ('次要', '次');
UPDATE foreshadowing SET importance = 'Normal'    WHERE importance IN ('普通', '一般');

-- 2) hint_level：先把「低（…）」里的括号内容抽到 hint_note，再归一到 Subtle
UPDATE foreshadowing
   SET hint_note = COALESCE(hint_note, NULLIF(substring(hint_level from '（([^）]*)）'), '')),
       hint_level = 'Subtle'
 WHERE hint_level LIKE '低（%';

UPDATE foreshadowing SET hint_level = 'Subtle'   WHERE hint_level IN ('低', 'Low', 'low');
UPDATE foreshadowing SET hint_level = 'Hidden'   WHERE hint_level IN ('隐藏');
UPDATE foreshadowing SET hint_level = 'Explicit' WHERE hint_level IN ('明示');
UPDATE foreshadowing SET hint_level = 'Direct'   WHERE hint_level IN ('直接');
-- 「中期显形」这类描述性取值：整句留作备注，等级取 Direct（逐步显现 ≈ 直接暗示）
UPDATE foreshadowing
   SET hint_note = COALESCE(hint_note, hint_level),
       hint_level = 'Direct'
 WHERE hint_level LIKE '中期%';

-- 3) 兜底归一：任何仍不在枚举里的取值都归到默认值，**原文完整保留在 hint_note**
--    （否则下面的 CHECK 会因残留非法值而失败，导致迁移中断、服务起不来）
UPDATE foreshadowing
   SET hint_note = COALESCE(hint_note, importance),
       importance = 'Normal'
 WHERE importance NOT IN ('Core', 'Important', 'Normal', 'Minor');

UPDATE foreshadowing
   SET hint_note = COALESCE(hint_note, hint_level),
       hint_level = 'Subtle'
 WHERE hint_level NOT IN ('Explicit', 'Direct', 'Subtle', 'Hidden');

-- 4) 约束：数据库层保证这两个字段此后只有枚举值
ALTER TABLE foreshadowing DROP CONSTRAINT IF EXISTS foreshadowing_importance_check;
ALTER TABLE foreshadowing
    ADD CONSTRAINT foreshadowing_importance_check
    CHECK (importance IN ('Core', 'Important', 'Normal', 'Minor'));

ALTER TABLE foreshadowing DROP CONSTRAINT IF EXISTS foreshadowing_hint_level_check;
ALTER TABLE foreshadowing
    ADD CONSTRAINT foreshadowing_hint_level_check
    CHECK (hint_level IN ('Explicit', 'Direct', 'Subtle', 'Hidden'));
