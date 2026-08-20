//! AI 执行与上下文模型层。
//!
//! 这一层承载 AI 执行 / 上下文建模的"过程"模块：
//! retrieval（检索体系）、job（异步任务状态机）。
//!
//! 结构化抽取（LLM 输出 → 世界变更）已在 application::extraction_executor
//! 中以可复用函数实现，并由 GenerationExecutor / ExtractionExecutor 共享，
//! 故 ai::extractor 这一套未接线的 trait 已移除（见铁三角评审 P2 清理）。
//!
//! 原位于本 crate 的"领域数据"模块（character_mind / state_mgmt / repair）
//! 已被下沉到 `domain`（它们描述的是"世界事实"，不是 AI 推理过程）。
//! 本 crate 仅保留向后兼容的 re-export 垫片，规范归属是 `domain`。
//!
//! 依赖方向（修正后，对应 ARCHITECTURE 评审 P0）：
//!   db  → domain        （不再依赖 ai）
//!   ai  → domain        （ai 读取/产生领域类型）
//!   application / runtime / narrative-engine → domain (+ ai)
//!
//! 注意：generation（含 ContextPackage / ContextLayer）仍留在 domain，
//! 因为 domain::ports 直接引用 ContextPackage，domain 不能反过来依赖 ai。

pub mod retrieval;
pub mod job;

// 领域数据已下沉到 domain；保留 re-export 垫片以兼容旧引用。
pub use domain::character_mind;
pub use domain::state_mgmt;
pub use domain::repair;

// Re-export commonly used AI-layer types (mirrors domain's glob re-exports).
pub use retrieval::*;
pub use job::*;
