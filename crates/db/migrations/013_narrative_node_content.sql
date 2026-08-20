-- 叙事节点正文（场景草稿内容）持久化
-- 编辑器 Save 现可写入真实正文，刷新/重开不再丢失。
ALTER TABLE narrative_node ADD COLUMN IF NOT EXISTS content TEXT;
