//! 统一命令类型（再导出 domain 定义，保持提案的目录结构）。
//!
//! 所有 Canon mutation 都通过 [`MutationCommand`] 表达，由
//! `MutationCommitter` 统一提交。这里只提供便捷构造器。

pub mod command;

pub use domain::mutation::{
    MutationCommand, MutationPayload, MutationSource, MutationTargetType,
};
