//! Validation - 把"候选"裁决为"可提交"。
//!
//! 包含 Validator（领域不变量校验）与 ContractValidator（契约校验）。
//! 对应 ChatGPT 评审 P1 的运行时拆分。

pub mod validator;
pub mod contract_validator;

// 对外可达的常用类型
pub use validator::{Validator, ValidatorDeps};
pub use contract_validator::ContractValidator;
