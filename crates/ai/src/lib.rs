//! AI 执行与上下文模型层。
//!
//! 这一层承载原本位于 domain 的"AI 执行 / 上下文建模"模块：
//! skill（技能与上下文策略）、extractor（LLM 输出结构化提取）、
//! retrieval（检索体系）、repair（剧情修复）、job（异步任务状态机）、
//! character_mind（角色认知模型）、state_mgmt（状态管理 / 知识缺口 / 多级记忆）。
//!
//! 依赖倒置：ai 依赖 domain（读取领域类型），但 domain 不依赖 ai，
//! 因此不会出现 domain -> ai 的循环依赖。runtime / application / db
//! 等上层在需要这些 AI 概念时依赖 ai，而非把它们塞回 domain。
//!
//! 注意：generation（含 ContextPackage / ContextLayer）仍留在 domain，
//! 因为 domain::ports 直接引用 ContextPackage，domain 不能反过来依赖 ai。

pub mod extractor;
pub mod retrieval;
pub mod repair;
pub mod job;
pub mod character_mind;
pub mod state_mgmt;

// Re-export commonly used AI-layer types (mirrors domain's glob re-exports).
pub use extractor::*;
pub use retrieval::*;
pub use repair::*;
pub use job::*;
pub use character_mind::*;
pub use state_mgmt::*;
