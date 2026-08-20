//! Execution - 把"候选"变成"进入世界的动作"。
//!
//! 包含 Context Engine（上下文组装）与 Retrieval（检索体系）。
//! 对应 ChatGPT 评审 P1 的运行时拆分：execution / validation / commit 三分。

pub mod context_engine;
pub mod retrieval;

// 对外可达的常用类型
pub use context_engine::{ContextEngine, ContextEngineDeps};
pub use retrieval::Retriever;
