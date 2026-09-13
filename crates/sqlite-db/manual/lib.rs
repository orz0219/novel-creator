//! sqlite-db：手机单机版 SQLite 持久层
//!
//! 本 crate 复用与 PostgreSQL 完全相同的 `domain` 端口 trait，
//! 因此 `application` / `agent` / `runtime` 服务层无需任何改动即可跑在 SQLite 上。
//!
//! 模块来源分两类：
//!   - 自动生成：由 tmp/gen_sqlite_backend.py 从 crates/db 机械转换（勿手工编辑）
//!   - 手工维护：migration / schema / seed / lib（见 crates/sqlite-db/manual/）

pub mod ai_settings;
pub mod application_ports;
pub mod connection;
pub mod embedded;
pub mod export;
pub mod guide_progress;
pub mod import;
pub mod migration;
pub mod mutation_committer;
pub mod project_resolver;
pub mod repos;
pub mod runtime_ports;
pub mod schema;
pub mod seed;
pub mod ser;
pub mod time_utils;
