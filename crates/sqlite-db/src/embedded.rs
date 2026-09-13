//! 编译期内嵌的 SQLite 迁移（手工维护）
//!
//! 手机端运行时没有迁移文件目录，因此把迁移 SQL 直接编进二进制。
//! 新增 PG 迁移后需要重新生成 crates/db/migrations_sqlite 并更新下面的引用。
//!
//! 单个文件通常远小于 1MB，用 include_str! 嵌入不会带来明显体积负担。

/// 导出的 JSON 格式版本（与 crates/db/src/export.rs 的常量保持一致）。
/// 手机端据此判断能否读入电脑端导出的文件。
pub const EXPORT_FORMAT_VERSION: i32 = 2;

/// 数据库结构代号。
///
/// **每次改动 migrations_sqlite/001_init.sql 的内容都必须把这个值加一。**
///
/// 为什么需要它：迁移是按文件名记录在 `_migrations` 表里的，
/// 文件名不变时改了内容会被判定为「已应用」而跳过，旧数据库会停留在旧结构上，
/// 表现为各种字段缺失的怪错误。
///
/// 版本不一致时的行为：**清空并重建数据库**（手机端是单机数据，
/// 且结构尚未稳定，这比带着半个旧结构跑更安全）。重建会丢数据，但会在
/// 日志与 App 内明确告知，不做静默处理。
pub const SCHEMA_GENERATION: i32 = 1;

/// 记录 schema 代号的表名
pub const SCHEMA_VERSION_TABLE: &str = "schema_version";

/// 内嵌迁移列表：(文件名, SQL 内容)，按文件名顺序执行
pub const EMBEDDED_MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_init.sql",
        include_str!("../../db/migrations_sqlite/001_init.sql"),
    ),
    (
        "002_fix_cascade_fks.sql",
        include_str!("../../db/migrations_sqlite/002_fix_cascade_fks.sql"),
    ),
    (
        "003_entity_snapshot.sql",
        include_str!("../../db/migrations_sqlite/003_entity_snapshot.sql"),
    ),
    (
        "004_entity_profile_snapshot.sql",
        include_str!("../../db/migrations_sqlite/004_entity_profile_snapshot.sql"),
    ),
    (
        "005_profile_snapshot_actor.sql",
        include_str!("../../db/migrations_sqlite/005_profile_snapshot_actor.sql"),
    ),
];
