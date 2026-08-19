//! Extractor - 从 LLM 输出中提取结构化变更
//!
//! StateChangeExtractor 从 Writer 的输出中提取状态变更建议。
//! KnowledgeExtractor 从 Writer 的输出中提取知识变更建议。
//! 这些是 runtime 层的核心接口。

use anyhow::Result;
use crate::*;
use uuid::Uuid;

/// StateChangeExtractor - 从 LLM 输出中提取状态变更
///
/// Writer 生成的文本中可能隐含了世界状态的变化。
/// 这个 trait 负责识别并提取这些变化。
pub trait StateChangeExtractor {
    /// 从 LLM 输出中提取状态变更
    ///
    /// # Arguments
    /// * `scene` - 当前场景信息
    /// * `output` - LLM 生成的文本
    /// * `entities` - 当前场景涉及的实体
    ///
    /// # Returns
    /// 提取到的状态变更列表
    fn extract_state_changes(
        &self,
        scene: &SceneAttributes,
        output: &str,
        entities: &[Entity],
    ) -> Result<Vec<StateChange>>;
}

/// KnowledgeExtractor - 从 LLM 输出中提取知识变更
///
/// Writer 生成的文本中可能隐含了角色获得新知识。
/// 这个 trait 负责识别并提取这些知识变更。
pub trait KnowledgeExtractor {
    /// 从 LLM 输出中提取知识变更
    ///
    /// # Arguments
    /// * `scene` - 当前场景信息
    /// * `output` - LLM 生成的文本
    /// * `characters` - 当前场景涉及的角色
    ///
    /// # Returns
    /// 提取到的知识变更列表（角色ID -> 新知列表）
    fn extract_knowledge_changes(
        &self,
        scene: &SceneAttributes,
        output: &str,
        characters: &[Entity],
    ) -> Result<Vec<(Uuid, Vec<String>)>>;
}

/// Mock StateChangeExtractor - 用于测试
pub struct MockStateChangeExtractor;

impl StateChangeExtractor for MockStateChangeExtractor {
    fn extract_state_changes(
        &self,
        _scene: &SceneAttributes,
        _output: &str,
        _entities: &[Entity],
    ) -> Result<Vec<StateChange>> {
        Ok(vec![])
    }
}

/// Mock KnowledgeExtractor - 用于测试
pub struct MockKnowledgeExtractor;

impl KnowledgeExtractor for MockKnowledgeExtractor {
    fn extract_knowledge_changes(
        &self,
        _scene: &SceneAttributes,
        _output: &str,
        _characters: &[Entity],
    ) -> Result<Vec<(Uuid, Vec<String>)>> {
        Ok(vec![])
    }
}

/// StateCommitter - 事务化提交状态变更
///
/// 将 ProposedChange 列表事务化提交到世界状态。
/// P1-2: commit 接受 change_ids 而非 ProposedChange 快照，
/// 内部会从 DB 重新加载权威版本，防止并发修改导致的数据不一致。
pub trait StateCommitter {
    /// 提交一批已验证的变更（通过 ID）
    /// 内部会从 DB 重新加载 proposal，不依赖外部传入的快照。
    fn commit(
        &self,
        project_id: Uuid,
        change_ids: &[Uuid],
    ) -> Result<Vec<StateChangeRecord>>;
}
