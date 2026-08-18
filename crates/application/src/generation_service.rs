//! Generation Runtime - 生成运行时
//!
//! 负责 Skill 管理、LLM 调用编排、生成任务管理。

use anyhow::{Context, Result};
use chrono::Utc;
use db::repos::generation_repo;
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

/// Generation Runtime - 生成运行时
pub struct GenerationRuntime {
    pool: PgPool,
}

impl GenerationRuntime {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ============================================================
    // Skill 管理
    // ============================================================

    /// 注册一个新 Skill
    pub async fn register_skill(
        &self,
        name: &str,
        description: Option<&str>,
        skill_type: SkillType,
        prompt_template: &str,
        input_schema: Option<serde_json::Value>,
        output_schema: Option<serde_json::Value>,
        default_params: serde_json::Value,
    ) -> Result<Skill> {
        let repo = generation_repo::SkillRepo::new(self.pool.clone());
        let skill = repo
            .create(name, description, skill_type, prompt_template, input_schema, output_schema, default_params)
            .await?;
        tracing::info!("Registered skill: {} (v{})", skill.name, skill.version);
        Ok(skill)
    }

    /// 获取 Skill
    pub async fn get_skill(&self, name: &str) -> Result<Option<Skill>> {
        let repo = generation_repo::SkillRepo::new(self.pool.clone());
        repo.get_by_name(name).await
    }

    /// 列出所有 Skill
    pub async fn list_skills(&self) -> Result<Vec<Skill>> {
        let repo = generation_repo::SkillRepo::new(self.pool.clone());
        repo.list_all().await
    }

    // ============================================================
    // 生成任务管理
    // ============================================================

    /// 创建生成任务
    pub async fn create_task(
        &self,
        project_id: Uuid,
        skill_name: &str,
        scene_id: Option<Uuid>,
        input: serde_json::Value,
    ) -> Result<GenerationTask> {
        let skill = self
            .get_skill(skill_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_name))?;

        let repo = generation_repo::TaskRepo::new(self.pool.clone());
        let task = repo.create(project_id, skill.id, scene_id, input).await?;

        tracing::info!("Created generation task: {} for skill {}", task.id, skill_name);
        Ok(task)
    }

    /// 执行生成任务（模拟 LLM 调用）
    ///
    /// TODO: 实际实现需要调用 LLM API
    pub async fn execute_task(&self, task_id: Uuid) -> Result<GenerationTask> {
        let task_repo = generation_repo::TaskRepo::new(self.pool.clone());
        let task = task_repo
            .get_by_id(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

        // 标记为运行中
        task_repo.update_status(task_id, TaskStatus::Running, None, None).await?;

        // TODO: 这里应该调用实际的 LLM API
        // 现在用模拟输出
        let mock_output = serde_json::json!({
            "content": "Generated text placeholder",
            "word_count": 0,
            "model": "mock"
        });

        // 标记为完成
        task_repo
            .update_status(task_id, TaskStatus::Completed, Some(mock_output.clone()), None)
            .await?;

        tracing::info!("Completed generation task: {}", task_id);

        Ok(GenerationTask {
            id: task_id,
            project_id: task.project_id,
            skill_id: task.skill_id,
            scene_id: task.scene_id,
            input: task.input,
            output: Some(mock_output),
            status: TaskStatus::Completed,
            token_usage: None,
            error: None,
            created_at: task.created_at,
            completed_at: Some(Utc::now()),
        })
    }

    /// 获取任务状态
    pub async fn get_task(&self, task_id: Uuid) -> Result<Option<GenerationTask>> {
        let repo = generation_repo::TaskRepo::new(self.pool.clone());
        repo.get_by_id(task_id).await
    }

    // ============================================================
    // 生成运行记录
    // ============================================================

    /// 记录一次生成运行
    pub async fn record_run(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        llm_model: &str,
        provider: Option<&str>,
        prompt_sent: &str,
        response_received: &str,
        token_usage: Option<TokenUsage>,
        latency_ms: Option<i64>,
    ) -> Result<GenerationRun> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO generation_run (id, project_id, task_id, llm_model, provider, prompt_sent, response_received, token_usage, latency_ms, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(project_id)
        .bind(task_id)
        .bind(llm_model)
        .bind(provider.unwrap_or(""))
        .bind(prompt_sent)
        .bind(response_received)
        .bind(token_usage.as_ref().map(|t| serde_json::to_value(t).unwrap_or_default()))
        .bind(latency_ms)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to record generation run")?;

        Ok(GenerationRun {
            id,
            project_id,
            task_id,
            context_snapshot_id: None,
            llm_model: llm_model.to_string(),
            provider: provider.map(|s| s.to_string()),
            prompt_sent: prompt_sent.to_string(),
            response_received: response_received.to_string(),
            token_usage,
            latency_ms,
            skill_version: None,
            prompt_version: None,
            schema_version: None,
            context_policy_version: None,
            retry_count: 0,
            max_retries: 3,
            error: None,
            output_artifact_id: None,
            created_at: now,
        })
    }
}

/// 预注册的 Skill 模板
pub struct SkillTemplates;

impl SkillTemplates {
    pub fn location_designer() -> (&'static str, &'static str, SkillType) {
        ("location_designer", r#"You are a location designer for a novel. Given the world context, volume context, arc context, and narrative requirements, design a detailed location package.

Input: World context, Volume context, Arc context, Narrative requirements, Existing locations
Output: Location Design Package with name, type, purpose, geography, resources, facilities, factions, threats, secrets, connections, plot hooks"#, SkillType::LocationDesigner)
    }

    pub fn character_designer() -> (&'static str, &'static str, SkillType) {
        ("character_designer", r#"You are a character designer for a novel. Given the world context, volume context, arc context, existing characters, and plot requirements, design a detailed character package.

Input: World, Volume, Arc, Existing characters, Plot requirements, Location
Output: Character Design Package with identity, role, appearance, personality, motivation, goal, fear, conflict, ability, weakness, background, relationships, secrets, knowledge, arc"#, SkillType::CharacterDesigner)
    }

    pub fn writer() -> (&'static str, &'static str, SkillType) {
        ("writer", r#"You are a novelist. Given the scene context, beat plan, and character knowledge, write the prose for this scene.

Input: Scene context, Beat plan, Character states, Location state
Output: Written prose in Chinese"#, SkillType::Writer)
    }

    pub fn planner() -> (&'static str, &'static str, SkillType) {
        ("planner", r#"You are a narrative planner. Given the volume context, arc context, and world state, plan the next sequence of scenes.

Input: Volume context, Arc context, World state, Character states
Output: Scene plan with objectives, conflicts, required events, required facts"#, SkillType::ScenePlanner)
    }
}