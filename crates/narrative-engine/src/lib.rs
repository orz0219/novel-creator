//! AI Narrative Engine - 通用小说生成框架
//!
//! 重新导出 application 和 runtime 的公共接口。

// 重新导出 application 服务
pub use application::world_service;
pub use application::narrative_service;
pub use application::generation_service;

// 重新导出 runtime 组件
pub use runtime::context_engine;
pub use runtime::validator;
pub use runtime::state_committer;
