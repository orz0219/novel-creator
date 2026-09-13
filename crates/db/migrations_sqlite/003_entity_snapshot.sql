-- 实体版本快照表（与 PG 侧 027_entity_snapshot.sql 对应）。
--
-- 每次实体被修改之前把当前这一版留档，供 /entities/{id}/versions 读真实历史。
-- 之前那三个接口是占位桩，返回的是编出来的数据（详见 PG 侧注释）。

CREATE TABLE IF NOT EXISTS "entity_snapshot" (
    "id" TEXT NOT NULL DEFAULT (randomblob(16)),
    "entity_id" TEXT NOT NULL,
    "project_id" TEXT NOT NULL,
    "version" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "summary" TEXT,
    "description" TEXT,
    "attributes" TEXT,
    "status" TEXT NOT NULL,
    "updated_by" TEXT,
    "replaced_at" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE ("entity_id", "version"),
    FOREIGN KEY ("entity_id") REFERENCES "entity"(id) ON DELETE CASCADE,
    FOREIGN KEY ("project_id") REFERENCES "project"(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS "idx_entity_snapshot_entity"
    ON "entity_snapshot" ("entity_id", "version" DESC);
