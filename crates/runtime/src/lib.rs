//! Runtime Components - 运行时层
//!
//! 负责 Context Engine、Validator、Extractor 等运行时组件。
//! 依赖 domain，不依赖 infrastructure (db)。
//! 这确保了将来换 LLM provider 或换数据库时，runtime 层不受影响。
//!
//! 运行时按职责拆分为三个子层（对应 ChatGPT 评审 P1）：
//!
//! ```text
//! execution/   生成候选（Context Engine + Retrieval）
//! validation/  裁决候选（Validator + ContractValidator）
//! commit/      落库候选（StateCommitter）
//! ```
//!
//! 这一拆分避免 Context Engine 膨胀成"第二个 application"。

pub mod context;
pub mod execution;
pub mod validation;
pub mod commit;

// 保留历史顶层路径，供 application / narrative-engine / 测试引用，
// 避免在评审重构中破坏既有 `runtime::context_engine` 等外部引用。
pub use execution::context_engine;
pub use execution::retrieval;
pub use validation::validator;
pub use validation::contract_validator;
pub use commit::state_committer;