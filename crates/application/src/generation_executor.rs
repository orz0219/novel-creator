//! Generation Executor - 完整生成执行链（提案 十 / 十一 / 十二）
//!
//! 编排：加载生成任务与 Skill -> 组装 Prompt -> 调用 LLM（LlmPort）->
//! 保存 ContextSnapshot（提案 十二）-> 记录 GenerationRun（关联 snapshot id）
//! -> 写回任务产出 -> 基于产出创建 ProposedChange
//! （经由 ProposalService，批准时由 MutationCommitter 落到 World Canon）。
//!
//! 这样 Generation 不再直接写 Canon；所有落库变更都通过统一的提交者。

use anyhow::Result;
use chrono::Utc;
use domain::generation::{ContextLayer, ContextPackage, ReproducibilityMeta};
use domain::sha256_hex;
use domain::ports::{
    ContextSnapshotRepositoryPort, GenerationRepositoryPort, LlmPort, ProposalRepositoryPort,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::warn;
use uuid::Uuid;

use crate::extraction_executor::extract_into_proposals;

pub struct GenerationExecutor {
    repo: Arc<dyn GenerationRepositoryPort>,
    snapshots: Arc<dyn ContextSnapshotRepositoryPort>,
    proposals: Arc<dyn ProposalRepositoryPort>,
    llm: Arc<dyn LlmPort>,
}

impl GenerationExecutor {
    pub fn new(
        repo: Arc<dyn GenerationRepositoryPort>,
        snapshots: Arc<dyn ContextSnapshotRepositoryPort>,
        proposals: Arc<dyn ProposalRepositoryPort>,
        llm: Arc<dyn LlmPort>,
    ) -> Self {
        Self {
            repo,
            snapshots,
            proposals,
            llm,
        }
    }

    fn empty_layer() -> ContextLayer {
        ContextLayer {
            content: String::new(),
            token_estimate: 0,
            included: false,
        }
    }

    /// 执行一次完整的生成任务，返回模型产出文本。
    pub async fn execute(&self, task_id: Uuid) -> Result<String> {
        let task = self
            .repo
            .get_task_struct(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("generation task {} not found", task_id))?;
        // skill 为可选：未关联 Skill 时使用默认提示词，使无 Skill 的生成任务亦可用。
        let (system_prompt, skill_prompt_template) = match task.skill_id {
            Some(sid) => {
                let skill = self
                    .repo
                    .get_skill_by_id(sid)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("skill {} not found", sid))?;
                let sp = skill
                    .description
                    .clone()
                    .unwrap_or_else(|| "You are a professional novel writing assistant.".to_string());
                (sp, skill.prompt_template)
            }
            None => (
                "You are a professional novel writing assistant.".to_string(),
                String::new(),
            ),
        };
        let user_prompt = format!(
            "{}\n\n## 输入\n{}",
            skill_prompt_template,
            serde_json::to_string_pretty(&task.input).unwrap_or_default()
        );
        // 模型来源：统一配置透传（当前阶段以 OPENCODE_MODEL 为准；task 级覆盖为 P1）。
        let model = std::env::var("OPENCODE_MODEL")
            .unwrap_or_else(|_| "mimo-v2.5".to_string());

        let start = Instant::now();
        let output = self.llm.complete(&system_prompt, &user_prompt, &model).await?;
        let latency_ms = start.elapsed().as_millis() as i64;

        // 保存 ContextSnapshot 并关联本次 Run（提案 十二）
        let snapshot_id = {
            let pkg = ContextPackage {
                id: Uuid::nil(),
                project_id: task.project_id,
                scene_id: task.scene_id.unwrap_or(Uuid::nil()),
                token_budget: 0,
                l0_essential: ContextLayer {
                    content: user_prompt.clone(),
                    token_estimate: 0,
                    included: true,
                },
                l1_scene_relevant: Self::empty_layer(),
                l2_recent_history: Self::empty_layer(),
                l3_narrative_context: Self::empty_layer(),
                l4_character_knowledge: Self::empty_layer(),
                l5_world_background: Self::empty_layer(),
                l6_optional_supplement: Self::empty_layer(),
                actual_tokens: 0,
                reproducibility: ReproducibilityMeta::default(),
                created_at: Utc::now(),
            };
            Some(self.snapshots.save(&pkg).await?)
        };

        // 写回任务产出
        self.repo
            .update_task_output(task_id, serde_json::json!({ "content": output }))
            .await?;

        // 记录 GenerationRun（提案 十 / 十一，关联 ContextSnapshot 提案 十二）
        // 生成请求参数在此固定：prompt_hash 取最终拼装 prompt 的 sha256，
        // 而非 ContextPackage（上下文是输入材料，不是最终 prompt）——符合 ChatGPT 评审 P1。
        let reproducibility = ReproducibilityMeta {
            model: Some(model.to_string()),
            temperature: None,
            prompt_hash: Some(sha256_hex(&format!("{}\n{}", system_prompt, user_prompt))),
            ..Default::default()
        };
        self.repo
            .create_run(
                task.project_id,
                task_id,
                snapshot_id,
                &model,
                Some("infra"),
                &user_prompt,
                &output,
                None,
                Some(latency_ms),
                reproducibility,
            )
            .await?;

        // 生成文本进入既有抽取链路 → 产出可提交的 Entity/Relation 提案
        // （批准时由 MutationCommitter 落到 World Canon）。原始全文仍作为 Run /
        // Task output / Snapshot 留存，作为审计与回溯，不再伪装成 Canon 变更。
        // 抽取失败不阻断整个生成任务：产出已持久化，只是本次没有新提案。
        if let Err(e) = extract_into_proposals(
            self.proposals.clone(),
            self.llm.clone(),
            task.project_id,
            Some(task_id),
            &output,
        )
        .await
        {
            warn!(error = ?e, "生成文本抽取为提案失败（生成结果仍保留）");
        }

        Ok(output)
    }
}
