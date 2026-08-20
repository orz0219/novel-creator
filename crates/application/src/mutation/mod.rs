//! Mutation - World Canon 统一写入口（应用层编排）。

pub mod committer;
pub mod result;
pub mod validator;

pub use committer::MutationCommitter;
pub use domain::mutation::{
    MutationCommand, MutationCommitterPort, MutationError, MutationPayload, MutationSource,
    MutationTargetType,
};
pub use domain::mutation::MutationCommitResult;
pub use result::MutationResultExt;
pub use validator::validate_mutation;
