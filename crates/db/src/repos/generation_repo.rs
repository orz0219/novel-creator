//! Generation Repository

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{GenerationTask, Skill, SkillStatus, SkillType, TaskStatus};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ser;

// ============= SkillRepo =============

pub struct SkillRepo {
    pool: PgPool,
}

impl SkillRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        skill_type: SkillType,
        prompt_template: &str,
        input_schema: Option<serde_json::Value>,
        output_schema: Option<serde_json::Value>,
        default_params: serde_json::Value,
    ) -> Result<Skill> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let type_str = ser::skill_type_str(&skill_type);

        sqlx::query(
            "INSERT INTO skill (id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 1, $5, $6, $7, $8, 'Draft', $9, $10)",
        )
        .bind(id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(&type_str)
        .bind(prompt_template)
        .bind(input_schema.as_ref())
        .bind(output_schema.as_ref())
        .bind(&default_params)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create skill")?;

        Ok(Skill {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            skill_type,
            version: 1,
            prompt_template: prompt_template.to_string(),
            input_schema,
            output_schema,
            default_params,
            status: SkillStatus::Draft,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Skill>> {
        let row = sqlx::query_as::<_, SkillRow>(
            "SELECT id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at \
             FROM skill WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query skill")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_all(&self) -> Result<Vec<Skill>> {
        let rows = sqlx::query_as::<_, SkillRow>(
            "SELECT id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at \
             FROM skill ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query skills")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Skill>> {
        let row = sqlx::query_as::<_, SkillRow>(
            "SELECT id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at \
             FROM skill WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query skill")?;
        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct SkillRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    skill_type: String,
    version: i32,
    prompt_template: String,
    input_schema: Option<serde_json::Value>,
    output_schema: Option<serde_json::Value>,
    default_params: Option<serde_json::Value>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SkillRow> for Skill {
    fn from(r: SkillRow) -> Self {
        Skill {
            id: r.id,
            name: r.name,
            description: r.description,
            skill_type: ser::parse_skill_type(&r.skill_type),
            version: r.version,
            prompt_template: r.prompt_template,
            input_schema: r.input_schema,
            output_schema: r.output_schema,
            default_params: r.default_params.unwrap_or_default(),
            status: ser::parse_skill_status(&r.status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= TaskRepo =============

pub struct TaskRepo {
    pool: PgPool,
}

impl TaskRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        skill_id: Uuid,
        scene_id: Option<Uuid>,
        input: serde_json::Value,
    ) -> Result<GenerationTask> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO generation_task (id, project_id, skill_id, scene_id, input, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, 'Pending', $6)",
        )
        .bind(id)
        .bind(project_id)
        .bind(skill_id)
        .bind(scene_id)
        .bind(&input)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create task")?;

        Ok(GenerationTask {
            id,
            project_id,
            skill_id,
            scene_id,
            input,
            output: None,
            status: TaskStatus::Pending,
            token_usage: None,
            error: None,
            created_at: now,
            completed_at: None,
        })
    }

    pub async fn update_status(
        &self,
        task_id: Uuid,
        status: TaskStatus,
        output: Option<serde_json::Value>,
        error: Option<&str>,
    ) -> Result<()> {
        let status_str = ser::task_status_str(&status);

        sqlx::query(
            "UPDATE generation_task SET status = $1, output = $2, error = $3, completed_at = $4 WHERE id = $5",
        )
        .bind(&status_str)
        .bind(output.as_ref())
        .bind(error.unwrap_or(""))
        .bind(Utc::now())
        .bind(task_id)
        .execute(&self.pool)
        .await
        .context("Failed to update task")?;
        Ok(())
    }

    pub async fn get_by_id(&self, task_id: Uuid) -> Result<Option<GenerationTask>> {
        let row = sqlx::query_as::<_, GenerationTaskRow>(
            "SELECT id, project_id, skill_id, scene_id, input, output, status, error, created_at, completed_at \
             FROM generation_task WHERE id = $1",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query task")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn update_output(&self, task_id: Uuid, output: serde_json::Value) -> Result<()> {
        sqlx::query(
            "UPDATE generation_task SET output = $1, status = 'Completed', completed_at = NOW() WHERE id = $2",
        )
        .bind(&output)
        .bind(task_id)
        .execute(&self.pool)
        .await
        .context("Failed to update task output")?;
        Ok(())
    }
}

// ============= RunRepo =============

pub struct RunRepo {
    pool: PgPool,
}

impl RunRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 记录一次 GenerationRun（提案 十 / 十一）。token_usage 与 reproducibility_meta 以 JSONB 存储。
    pub async fn create(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        context_snapshot_id: Option<Uuid>,
        llm_model: &str,
        provider: Option<&str>,
        prompt_sent: &str,
        response_received: &str,
        token_usage: Option<serde_json::Value>,
        latency_ms: Option<i64>,
        reproducibility: &domain::generation::ReproducibilityMeta,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO generation_run (id, project_id, task_id, context_snapshot_id, llm_model, provider, prompt_sent, response_received, token_usage, latency_ms, reproducibility_meta, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(project_id)
        .bind(task_id)
        .bind(context_snapshot_id)
        .bind(llm_model)
        .bind(provider)
        .bind(prompt_sent)
        .bind(response_received)
        .bind(token_usage.as_ref())
        .bind(latency_ms)
        .bind(serde_json::to_value(reproducibility).unwrap_or_default())
        .execute(&self.pool)
        .await
        .context("Failed to create generation run")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct GenerationTaskRow {
    id: Uuid,
    project_id: Uuid,
    skill_id: Uuid,
    scene_id: Option<Uuid>,
    input: Option<serde_json::Value>,
    output: Option<serde_json::Value>,
    status: String,
    error: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<GenerationTaskRow> for GenerationTask {
    fn from(r: GenerationTaskRow) -> Self {
        GenerationTask {
            id: r.id,
            project_id: r.project_id,
            skill_id: r.skill_id,
            scene_id: r.scene_id,
            input: r.input.unwrap_or_default(),
            output: r.output,
            status: ser::parse_task_status(&r.status),
            token_usage: None,
            error: r.error,
            created_at: r.created_at,
            completed_at: r.completed_at,
        }
    }
}
