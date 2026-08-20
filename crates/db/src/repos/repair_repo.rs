//! PlotRepair Repository - CRUD operations for PlotRepair

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ai::{PlotRepair, RepairStatus, RepairType};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PlotRepairRepo {
    pool: PgPool,
}

impl PlotRepairRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建剧情修复记录
    pub async fn create(
        &self,
        project_id: Uuid,
        scene_id: Uuid,
        issue_description: &str,
        repair_suggestion: &str,
        repair_type: RepairType,
    ) -> Result<PlotRepair> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let rt_str = match repair_type {
            RepairType::Automatic => "Automatic",
            RepairType::Suggested => "Suggested",
            RepairType::Manual => "Manual",
        };

        sqlx::query(
            "INSERT INTO plot_repair (id, project_id, scene_id, issue_description, repair_suggestion, repair_type, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'Pending', $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(scene_id)
        .bind(issue_description)
        .bind(repair_suggestion)
        .bind(rt_str)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create plot repair")?;

        Ok(PlotRepair {
            id,
            project_id,
            scene_id,
            issue_description: issue_description.to_string(),
            repair_suggestion: repair_suggestion.to_string(),
            repair_type,
            status: RepairStatus::Pending,
            applied_at: None,
            created_at: now,
        })
    }

    /// 列出项目中的所有剧情修复
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<PlotRepair>> {
        let rows = sqlx::query_as::<_, PlotRepairRow>(
            "SELECT id, project_id, scene_id, issue_description, repair_suggestion, repair_type, status, applied_at, created_at \
             FROM plot_repair WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query plot repairs")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct PlotRepairRow {
    id: Uuid,
    project_id: Uuid,
    scene_id: Uuid,
    issue_description: String,
    repair_suggestion: String,
    repair_type: String,
    status: String,
    applied_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<PlotRepairRow> for PlotRepair {
    fn from(r: PlotRepairRow) -> Self {
        let repair_type = match r.repair_type.as_str() {
            "Automatic" => RepairType::Automatic,
            "Suggested" => RepairType::Suggested,
            "Manual" => RepairType::Manual,
            _ => RepairType::Suggested,
        };
        let status = match r.status.as_str() {
            "Pending" => RepairStatus::Pending,
            "Applied" => RepairStatus::Applied,
            "Rejected" => RepairStatus::Rejected,
            _ => RepairStatus::Pending,
        };
        PlotRepair {
            id: r.id,
            project_id: r.project_id,
            scene_id: r.scene_id,
            issue_description: r.issue_description,
            repair_suggestion: r.repair_suggestion,
            repair_type,
            status,
            applied_at: r.applied_at,
            created_at: r.created_at,
        }
    }
}
