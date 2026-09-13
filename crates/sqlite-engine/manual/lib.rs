//! sqlite-engine：手机单机版引擎
//!
//! 提供与电脑端 `narrative-engine` 完全一致的 HTTP API（同一套路由与 handler），
//! 但底层数据来自 SQLite，服务层（application / agent / runtime）完全复用。
//!
//! 模块来源：
//!   - 自动生成：api/* / state.rs / agent_tools.rs（由 tmp/gen_sqlite_backend.py 转换）
//!   - 手工维护：engine.rs（组装与启动，见 manual/）

pub mod agent_tools;
pub mod api;
pub mod engine;
pub mod state;
