//! Runtime Components - 运行时层
//!
//! 负责 Context Engine、Validator、Extractor 等运行时组件。
//! 依赖 domain，不依赖 infrastructure (db)。
//! 这确保了将来换 LLM provider 或换数据库时，runtime 层不受影响。

pub mod context_engine;
pub mod validator;
pub mod state_committer;
pub mod contract_validator;
