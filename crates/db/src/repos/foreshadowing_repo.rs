//! Foreshadowing Repository - CRUD operations for Foreshadowing

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Foreshadowing, ForeshadowingImportance, ForeshadowingStatus, HintLevel};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ForeshadowingRepo {
    pool: PgPool,
}

impl ForeshadowingRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建伏笔
    pub async fn create(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: ForeshadowingImportance,
        hint_level: HintLevel,
    ) -> Result<Foreshadowing> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let imp_str = match importance {
            ForeshadowingImportance::Core => "Core",
            ForeshadowingImportance::Important => "Important",
            ForeshadowingImportance::Normal => "Normal",
            ForeshadowingImportance::Minor => "Minor",
        };
        let hl_str = match hint_level {
            HintLevel::Explicit => "Explicit",
            HintLevel::Direct => "Direct",
            HintLevel::Subtle => "Subtle",
            HintLevel::Hidden => "Hidden",
        };

        sqlx::query(
            "INSERT INTO foreshadowing (id, project_id, name, description, status, importance, hint_level, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'Planned', $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(imp_str)
        .bind(hl_str)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create foreshadowing")?;

        Ok(Foreshadowing {
            id,
            project_id,
            storyline_id: None,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            status: ForeshadowingStatus::Planned,
            importance,
            hint_level,
            introduced_at: None,
            expected_reveal_at: None,
            actual_reveal_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// 更新伏笔状态
    pub async fn update_status(&self, id: Uuid, status: ForeshadowingStatus) -> Result<()> {
        let status_str = match status {
            ForeshadowingStatus::Planned => "Planned",
            ForeshadowingStatus::Introduced => "Introduced",
            ForeshadowingStatus::Active => "Active",
            ForeshadowingStatus::Revealed => "Revealed",
            ForeshadowingStatus::Abandoned => "Abandoned",
        };

        sqlx::query("UPDATE foreshadowing SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status_str)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update foreshadowing status")?;
        Ok(())
    }

    /// 列出项目中的所有伏笔
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Foreshadowing>> {
        let rows = sqlx::query_as::<_, ForeshadowingRow>(
            "SELECT id, project_id, storyline_id, name, description, status, importance, hint_level, introduced_at, expected_reveal_at, actual_reveal_at, created_at, updated_at \
             FROM foreshadowing WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query foreshadowings")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 伏笔更新（提案 六）：COALESCE 部分更新；status 直接存 VARCHAR。
    pub async fn update_tx(
        executor: &mut sqlx::PgConnection,
        id: Uuid,
        project_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE foreshadowing SET name = COALESCE($1, name), description = COALESCE($2, description), status = COALESCE($3, status), updated_at = NOW() \
             WHERE id = $4 AND project_id = $5",
        )
        .bind(name)
        .bind(description)
        .bind(status)
        .bind(id)
        .bind(project_id)
        .execute(executor)
        .await
        .context("Failed to update foreshadowing")?;
        Ok(result.rows_affected() > 0)
    }
}

#[derive(sqlx::FromRow)]
struct ForeshadowingRow {
    id: Uuid,
    project_id: Uuid,
    storyline_id: Option<Uuid>,
    name: String,
    description: Option<String>,
    status: String,
    importance: String,
    hint_level: String,
    introduced_at: Option<String>,
    expected_reveal_at: Option<String>,
    actual_reveal_at: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ForeshadowingRow> for Foreshadowing {
    fn from(r: ForeshadowingRow) -> Self {
        let status = match r.status.as_str() {
            "Planned" => ForeshadowingStatus::Planned,
            "Introduced" => ForeshadowingStatus::Introduced,
            "Active" => ForeshadowingStatus::Active,
            "Revealed" => ForeshadowingStatus::Revealed,
            "Abandoned" => ForeshadowingStatus::Abandoned,
            _ => ForeshadowingStatus::Planned,
        };
        let importance = match r.importance.as_str() {
            "Core" => ForeshadowingImportance::Core,
            "Important" => ForeshadowingImportance::Important,
            "Normal" => ForeshadowingImportance::Normal,
            "Minor" => ForeshadowingImportance::Minor,
            _ => ForeshadowingImportance::Normal,
        };
        let hint_level = match r.hint_level.as_str() {
            "Explicit" => HintLevel::Explicit,
            "Direct" => HintLevel::Direct,
            "Subtle" => HintLevel::Subtle,
            "Hidden" => HintLevel::Hidden,
            _ => HintLevel::Subtle,
        };
        Foreshadowing {
            id: r.id,
            project_id: r.project_id,
            storyline_id: r.storyline_id,
            name: r.name,
            description: r.description,
            status,
            importance,
            hint_level,
            introduced_at: r.introduced_at,
            expected_reveal_at: r.expected_reveal_at,
            actual_reveal_at: r.actual_reveal_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
