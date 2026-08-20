//! Commit - 把"裁决通过的计划"落到 World Canon。
//!
//! 包含 StateCommitter（状态提交协调）。
//! 对应 ChatGPT 评审 P1 的运行时拆分：commit 边界只负责"提交"，
//! 不负责生成、不负责校验。

pub mod state_committer;

// 对外可达的常用类型
pub use state_committer::DbStateCommitter;
