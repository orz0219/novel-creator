//! Extraction — 从自由文本抽取结构化世界模型候选（M1）。
//!
//! 这是 AI → Domain 的第一座桥：LLM 输出被解析成 EntityCandidate /
//! RelationCandidate，再经 ProposalService 落成 ProposedChange 草稿，
//! 最终由人工批准经 MutationCommitter 落到 World Canon。
//!
//! 设计要点（呼应规划者裁决）：
//! - AI 只"提议"，绝不直接改 Canon；抽取结果先成为 Draft 态 Proposal。
//! - entity_type 收敛为 Character / Location / Organization / Event 四种，
//!   与已 seed 的 entity_type 表对齐，降低后续 commit 阻力。

use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_entity_type() -> String {
    "Character".to_string()
}

/// 单个实体候选
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCandidate {
    pub name: String,
    #[serde(default = "default_entity_type")]
    pub entity_type: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub attributes: Value,
}

impl EntityCandidate {
    pub fn description(&self) -> String {
        format!("抽取实体 {}（类型 {}）", self.name, self.entity_type)
    }
}

/// 单个关系候选（from/to 为实体名，待批准时再解析为实体 id）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationCandidate {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub relation_type: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl RelationCandidate {
    pub fn description(&self) -> String {
        let rt = if self.relation_type.is_empty() {
            "未知"
        } else {
            &self.relation_type
        };
        format!("抽取关系 {} → {}（类型 {}）", self.from, self.to, rt)
    }
}

/// 一次抽取的整体结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractionResult {
    #[serde(default)]
    pub entities: Vec<EntityCandidate>,
    #[serde(default)]
    pub relations: Vec<RelationCandidate>,
}

impl ExtractionResult {
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.relations.is_empty()
    }
}
