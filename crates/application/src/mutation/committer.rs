//! MutationCommitter - 应用层 Canon 写入唯一入口（编排层）。
//!
//! 职责（提案 十二 / 二十四）：
//! - 校验命令（schema 层；domain 层后续接入 ValidationService）
//! - 调用底层 [`MutationCommitterPort`] 在事务中落实
//! - 只有此处能把变更落到 Canon；Repository 不直接被 Application 用于 mutation

use std::sync::Arc;

use crate::mutation::validator::validate_mutation;
use domain::mutation::*;

pub struct MutationCommitter {
    port: Arc<dyn MutationCommitterPort>,
}

impl MutationCommitter {
    pub fn new(port: Arc<dyn MutationCommitterPort>) -> Self {
        Self { port }
    }

    pub async fn commit(&self, cmd: MutationCommand) -> Result<MutationCommitResult, MutationError> {
        validate_mutation(&cmd)?;
        self.port.commit(cmd).await
    }

    pub async fn commit_batch(
        &self,
        cmds: Vec<MutationCommand>,
    ) -> Result<Vec<MutationCommitResult>, MutationError> {
        for c in &cmds {
            validate_mutation(c)?;
        }
        self.port.commit_batch(cmds).await
    }
}
