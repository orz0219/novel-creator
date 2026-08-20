//! MutationCommand 便捷构造器（再导出）。
//!
//! 实际的构造方法定义在 `domain::mutation::MutationCommand` 上（在同一 crate 内，
//! 满足孤儿规则）；此处仅做再导出以保持提案要求的 `command/command.rs` 目录结构。

pub use domain::mutation::{
    MutationCommand, MutationPayload, MutationSource, MutationTargetType,
};
