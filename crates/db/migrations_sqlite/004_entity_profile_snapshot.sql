-- 实体档案的历史快照（与 PG 侧 028 对应）。
--
-- entity_snapshot 只覆盖 entity 表的字段；档案类改动（角色档案 / 当前状态 /
-- 地点档案 / 势力档案）存在别的表里，此前不留痕，这里补上。

CREATE TABLE IF NOT EXISTS "entity_profile_snapshot" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "project_id" TEXT NOT NULL,
    "kind" TEXT NOT NULL,
    "payload" TEXT NOT NULL,
    "replaced_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY ("entity_id") REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY ("project_id") REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS "idx_entity_profile_snapshot_entity"
    ON "entity_profile_snapshot" ("entity_id", "replaced_at" DESC);
