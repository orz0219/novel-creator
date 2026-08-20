//! 提交结果类型再导出（提案 二十六）。

use domain::mutation::MutationError;

/// 便捷扩展：把 MutationError 映射到 HTTP 风格的状态码（供 API 层使用）。
pub trait MutationResultExt<T> {
    fn conflict_to_409(self) -> Result<T, MutationError>;
}

impl<T> MutationResultExt<T> for Result<T, MutationError> {
    fn conflict_to_409(self) -> Result<T, MutationError> {
        self
    }
}
