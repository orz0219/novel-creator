//! CanonRule Repository - 世界规则 CRUD

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::canon::{CanonRule, EnforcementAction, RuleLevel};
use sqlx::PgPool;
use uuid::Uuid;

pub struct CanonRuleRepo {
    pool: PgPool,
}

impl CanonRuleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建规则
    pub async fn create(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        rule_level: RuleLevel,
        rule_content: &str,
        affected_scope: &str,
        enforcement: EnforcementAction,
        constraints: serde_json::Value,
        source: Option<&str>,
    ) -> Result<CanonRule> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO canon_rule (id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(project_id)
        .bind(world_id)
        .bind(rule_level.as_str())
        .bind(rule_content)
        .bind(affected_scope)
        .bind(enforcement.as_str())
        .bind(&constraints)
        .bind(source.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert canon_rule")?;

        Ok(CanonRule {
            id,
            project_id,
            world_id,
            rule_level,
            rule_content: rule_content.to_string(),
            affected_scope: affected_scope.to_string(),
            enforcement,
            constraints,
            source: source.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    /// 查询项目的所有规则
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CanonRule>> {
        let rows = sqlx::query_as::<_, CanonRuleRow>(
            "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at \
             FROM canon_rule WHERE project_id = $1 ORDER BY rule_level, created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query canon_rules")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 查询指定世界的所有规则
    pub async fn list_by_world(&self, world_id: Uuid) -> Result<Vec<CanonRule>> {
        let rows = sqlx::query_as::<_, CanonRuleRow>(
            "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at \
             FROM canon_rule WHERE world_id = $1 ORDER BY rule_level, created_at",
        )
        .bind(world_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query canon_rules")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 删除规则
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM canon_rule WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete canon_rule")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct CanonRuleRow {
    id: Uuid,
    project_id: Uuid,
    world_id: Uuid,
    rule_level: String,
    rule_content: String,
    affected_scope: String,
    enforcement: String,
    constraints: Option<serde_json::Value>,
    source: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CanonRuleRow> for CanonRule {
    fn from(r: CanonRuleRow) -> Self {
        CanonRule {
            id: r.id,
            project_id: r.project_id,
            world_id: r.world_id,
            rule_level: RuleLevel::from_str(&r.rule_level),
            rule_content: r.rule_content,
            affected_scope: r.affected_scope,
            enforcement: EnforcementAction::from_str(&r.enforcement),
            constraints: r.constraints.unwrap_or(serde_json::json!({})),
            source: r.source,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
