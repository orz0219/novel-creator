//! Extraction Executor — M1 文本→实体/关系抽取闭环
//!
//! 编排：组装抽取 Prompt → 调 LlmPort（结构化 JSON 输出）→
//! 解析 ExtractionResult → 为每个候选创建 ProposedChange 草稿
//! （EntityCreate / RelationCreate），经 ProposalRepositoryPort 落到库。
//!
//! 安全边界（呼应规划者裁决）：executor 绝不直写 Canon。
//! 抽取产物只是 Draft 态 Proposal；真正落库由既有
//! ProposalService.approve_proposal → MutationCommitter 在批准时完成。

use anyhow::{Context, Result};
use domain::extraction::ExtractionResult;
use domain::ports::{LlmPort, ProposalRepositoryPort};
use domain::validation::{ChangePayload, ProposedChangeType};
use std::sync::Arc;
use uuid::Uuid;

/// 抽取系统提示词：强制 JSON schema 输出，约束实体类型与关系可解析性。
pub const EXTRACTION_SYSTEM_PROMPT: &str = r#"你是一个小说世界观建模助手。请从给定散文中抽取结构化的实体与关系。
只输出一个 JSON 对象，不要输出任何解释或 markdown 代码块，schema 如下：

{
  "entities": [
    { "name": "实体名", "entity_type": "Character|Location|Organization|Event", "summary": "可选一句话简介" }
  ],
  "relations": [
    { "from": "源实体名", "to": "目标实体名", "relation_type": "如 Sibling/Friend/Enemy/LocatedAt/MemberOf", "description": "可选说明" }
  ]
}

规则：
- entity_type 只能取 Character / Location / Organization / Event 之一；不确定时取 Character。
- 只在文中明确出现或可合理推断时抽取，不要编造。
- relations 的 from / to 必须是 entities 中出现的名字。"#;

pub struct ExtractionExecutor {
    proposals: Arc<dyn ProposalRepositoryPort>,
    llm: Arc<dyn LlmPort>,
}

impl ExtractionExecutor {
    pub fn new(proposals: Arc<dyn ProposalRepositoryPort>, llm: Arc<dyn LlmPort>) -> Self {
        Self { proposals, llm }
    }

    /// 从一段自由文本抽取结构化世界模型候选，并为每个候选创建 ProposedChange 草稿。
    ///
    /// 返回解析后的 ExtractionResult（供前端预览）；草稿已写入库，待人工批准。
    pub async fn extract(&self, project_id: Uuid, text: &str) -> Result<ExtractionResult> {
        // 抽取产生的提案不依附于某个生成任务，task_id 为 None。
        let task_id = None;
        extract_into_proposals(self.proposals.clone(), self.llm.clone(), project_id, task_id, text)
            .await
    }
}

/// 抽取文本中的结构化世界模型候选，并为每个候选创建可提交的 ProposedChange 草稿
/// （EntityCreate / RelationCreate）。
///
/// 供 ExtractionExecutor 与 GenerationExecutor 复用，保证
/// “生成文本 → 抽取 → 合法 Canon 提案” 这条链路只有一套实现。
/// 抽取失败（模型未返回可解析 JSON）时返回错误，由调用方决定是否整体失败。
pub async fn extract_into_proposals(
    proposals: Arc<dyn ProposalRepositoryPort>,
    llm: Arc<dyn LlmPort>,
    project_id: Uuid,
    task_id: Option<Uuid>,
    text: &str,
) -> Result<ExtractionResult> {
    let user = format!("## 原文\n{}\n\n请只返回符合 schema 的 JSON。", text);
    let raw = llm
        .complete(EXTRACTION_SYSTEM_PROMPT, &user, "extraction")
        .await?;
    let result = parse_extraction_json(&raw).context("解析 LLM 抽取结果失败")?;

    // 为每个抽取出的实体分配稳定 id；关系提案据此引用其实体 id。
    // 实体提交时即以该 id 落库（见 MutationCommitter），因此只要先批准实体、
    // 再批准关系，关系即可解析到真实实体。
    let mut entity_ids: std::collections::HashMap<String, Uuid> =
        std::collections::HashMap::new();
    for e in &result.entities {
        let eid = Uuid::new_v4();
        entity_ids.insert(e.name.clone(), eid);
        let payload = serde_json::to_value(ChangePayload::EntityCreate {
            entity_type: e.entity_type.clone(),
            name: e.name.clone(),
            attributes: e.attributes.clone(),
        })?;
        proposals
            .create_proposal(
                project_id,
                task_id,
                ProposedChangeType::EntityCreate,
                eid,
                &e.description(),
                payload,
            )
            .await?;
    }
    for r in &result.relations {
        let (Some(&from), Some(&to)) =
            (entity_ids.get(&r.from), entity_ids.get(&r.to))
        else {
            // 关系端点未出现在抽取实体中，跳过该关系提案（仍保留在返回结果供预览）。
            continue;
        };
        let payload = serde_json::to_value(ChangePayload::RelationCreate {
            target_entity_id: to,
            relation_type: r.relation_type.clone(),
            attributes: serde_json::json!({}),
        })?;
        proposals
            .create_proposal(
                project_id,
                task_id,
                ProposedChangeType::RelationCreate,
                from,
                &r.description(),
                payload,
            )
            .await?;
    }
    Ok(result)
}

/// 从 LLM 原始输出中解析 ExtractionResult。
///
/// 容忍 ```json 代码块包裹与前后多余文本：去掉围栏后取第一个 `{`
/// 到最后一个 `}` 之间的子串再反序列化。
pub fn parse_extraction_json(raw: &str) -> Result<ExtractionResult> {
    let body = strip_code_fence(raw.trim());
    let start = body
        .find('{')
        .ok_or_else(|| anyhow::anyhow!("未找到 JSON 对象"))?;
    let end = body
        .rfind('}')
        .ok_or_else(|| anyhow::anyhow!("未找到 JSON 对象结尾"))?;
    if end <= start {
        return Err(anyhow::anyhow!("JSON 对象片段无效"));
    }
    let json = &body[start..=end];
    let parsed: ExtractionResult = serde_json::from_str(json).map_err(|e| {
        let snippet: String = json.chars().take(200).collect();
        anyhow::anyhow!("JSON 解析失败: {} | 片段: {}", e, snippet)
    })?;
    Ok(parsed)
}

fn strip_code_fence(s: &str) -> String {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```json").and_then(|r| r.strip_suffix("```")) {
        return rest.trim().to_string();
    }
    if let Some(rest) = s.strip_prefix("```").and_then(|r| r.strip_suffix("```")) {
        return rest.trim().to_string();
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::validation::{ProposedChange, ProposedChangeStatus};
    use std::sync::Mutex;

    struct MockLlm {
        reply: String,
    }
    #[async_trait]
    impl LlmPort for MockLlm {
        async fn complete(&self, _s: &str, _u: &str, _m: &str) -> Result<String> {
            Ok(self.reply.clone())
        }
    }

    #[derive(Default)]
    struct MockProposalRepo {
        created: Mutex<Vec<ProposedChange>>,
    }
    #[async_trait]
    impl ProposalRepositoryPort for MockProposalRepo {
        async fn list_proposals(
            &self,
            _project_id: Uuid,
        ) -> Result<Vec<ProposedChange>> {
            Ok(vec![])
        }
        async fn get_proposal(&self, _id: Uuid) -> Result<Option<ProposedChange>> {
            Ok(None)
        }
        async fn create_proposal(
            &self,
            project_id: Uuid,
            task_id: Option<Uuid>,
            change_type: ProposedChangeType,
            target_entity_id: Uuid,
            description: &str,
            payload: serde_json::Value,
        ) -> Result<ProposedChange> {
            let pc = ProposedChange {
                id: Uuid::new_v4(),
                project_id,
                task_id,
                change_type,
                target_entity_id,
                description: description.to_string(),
                payload,
                status: ProposedChangeStatus::Draft,
                created_at: chrono::Utc::now(),
                resolved_at: None,
            };
            self.created.lock().unwrap().push(pc.clone());
            Ok(pc)
        }
        async fn approve_proposal(&self, _id: Uuid) -> Result<ProposedChange> {
            anyhow::bail!("not implemented in mock")
        }
        async fn reject_proposal(&self, _id: Uuid) -> Result<ProposedChange> {
            anyhow::bail!("not implemented in mock")
        }
    }

    const SAMPLE: &str = r#"{
      "entities": [
        {"name":"林秋","entity_type":"Character","summary":"女主"},
        {"name":"北境城","entity_type":"Location"}
      ],
      "relations": [
        {"from":"林秋","to":"林寒","relation_type":"Sibling","description":"兄妹"}
      ]
    }"#;

    #[tokio::test]
    async fn extract_creates_proposal_drafts() {
        let repo = Arc::new(MockProposalRepo::default());
        let llm = Arc::new(MockLlm {
            reply: SAMPLE.to_string(),
        });
        let exec = ExtractionExecutor::new(repo.clone(), llm);
        let result = exec
            .extract(Uuid::new_v4(), "林秋第一次来到北境城，遇到兄长林寒。")
            .await
            .unwrap();

        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.entities[0].name, "林秋");
        assert_eq!(result.relations.len(), 1);

        let created = repo.created.lock().unwrap();
        assert_eq!(created.len(), 3);
        let entity_count = created
            .iter()
            .filter(|p| p.change_type == ProposedChangeType::EntityCreate)
            .count();
        let relation_count = created
            .iter()
            .filter(|p| p.change_type == ProposedChangeType::RelationCreate)
            .count();
        assert_eq!(entity_count, 2);
        assert_eq!(relation_count, 1);
    }

    #[test]
    fn parse_tolerates_fenced_and_prose() {
        let raw = "好的，这是结果：\n```json\n{\"entities\":[{\"name\":\"曹操\",\"entity_type\":\"Character\"}]}\n```\n希望对你有帮助。";
        let r = parse_extraction_json(raw).unwrap();
        assert_eq!(r.entities.len(), 1);
        assert_eq!(r.entities[0].name, "曹操");
        assert_eq!(r.entities[0].entity_type, "Character");
    }

    #[test]
    fn parse_empty_relations_ok() {
        let raw = "{\"entities\":[],\"relations\":[]}";
        let r = parse_extraction_json(raw).unwrap();
        assert!(r.is_empty());
    }
}
