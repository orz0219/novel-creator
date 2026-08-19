//! Validator - 变更验证
//!
//! 核心原则：AI 只能提出 ProposedChange，所有变更必须经过验证。
//! Validator 只负责验证，不负责提交。
//! 提交由 StateCommitter 在独立事务中完成。
//!
//! 性能要求：批量查询，避免 N+1 Query。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use db::repos::{approval_repo, entity_repo, validation_repo};
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

/// Validator - 变更验证器
///
/// 只负责验证 ProposedChange 的合规性。
/// 不负责 state commit - 那是 StateCommitter 的职责。
pub struct Validator {
    pool: PgPool,
}

impl Validator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 验证一批 ProposedChange
    ///
    /// 使用批量查询避免 N+1：先收集所有 entity_ids，批量加载，然后从内存验证。
    pub async fn validate_changes(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        changes: &[ProposedChange],
    ) -> Result<ValidationRun> {
        let val_repo = validation_repo::ValidationRepo::new(self.pool.clone());
        let entity_repo = entity_repo::EntityRepo::new(self.pool.clone());

        // ============================================================
        // 批量查询：收集所有 entity_ids，一次加载
        // ============================================================
        let entity_ids: Vec<Uuid> = changes.iter().map(|c| c.target_entity_id).collect();
        let entities = entity_repo.list_by_ids(project_id, &entity_ids).await?;
        let entity_map: std::collections::HashMap<Uuid, Entity> = entities.into_iter().map(|e| (e.id, e)).collect();

        // 批量加载 Canon Rules
        let canon_rules = self.load_canon_rules(project_id).await?;

        let mut run = val_repo.create_validation_run(project_id, task_id).await?;
        let mut approved = 0;
        let mut rejected = 0;

        for change in changes {
            // 从内存 map 查找 entity，避免数据库查询
            let entity = entity_map.get(&change.target_entity_id);
            let issues = self.validate_single_change(change, entity, &canon_rules).await?;

            let has_critical = issues.iter().any(|i| i.severity == IssueSeverity::Critical);
            let has_warning = issues.iter().any(|i| i.severity == IssueSeverity::Warning);

            if has_critical {
                val_repo.update_status(change.id, ProposedChangeStatus::Rejected).await?;
                rejected += 1;
                for issue in &issues {
                    val_repo.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref()).await?;
                }
            } else if has_warning {
                val_repo.update_status(change.id, ProposedChangeStatus::PendingApproval).await?;
                let app_repo = approval_repo::ApprovalRepo::new(self.pool.clone());
                let _ = app_repo.create(
                    change.project_id,
                    ApprovalTargetType::Entity,
                    change.target_entity_id,
                    "ai_validator",
                    serde_json::json!({
                        "proposed_change_id": change.id,
                        "change_type": format!("{:?}", change.change_type),
                        "description": change.description,
                        "payload": change.payload,
                    }),
                ).await;
                for issue in &issues {
                    val_repo.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref()).await?;
                }
            } else {
                val_repo.update_status(change.id, ProposedChangeStatus::Approved).await?;
                approved += 1;
            }
        }

        run.changes_validated = changes.len() as i32;
        run.changes_approved = approved;
        run.changes_rejected = rejected;
        run.status = ValidationStatus::Completed;
        run.completed_at = Some(Utc::now());
        val_repo.update_validation_run(&run).await?;

        tracing::info!("Validation complete: {} validated, {} approved, {} rejected", changes.len(), approved, rejected);
        Ok(run)
    }

    /// 从内存中的 entity map 验证单个 change
    async fn validate_single_change(
        &self,
        change: &ProposedChange,
        entity: Option<&Entity>,
        canon_rules: &[CanonRule],
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // Project scope 验证
        match entity {
            None => {
                issues.push(ValidationIssue {
                    id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                    issue_type: ValidationIssueType::EntityNotFound, severity: IssueSeverity::Critical,
                    message: format!("Target entity {} not found", change.target_entity_id),
                    suggestion: Some("Ensure the entity exists before proposing changes".to_string()),
                    created_at: Utc::now(),
                });
                return Ok(issues);
            }
            Some(entity) => {
                if entity.project_id != change.project_id {
                    issues.push(ValidationIssue {
                        id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                        issue_type: ValidationIssueType::RuleViolation, severity: IssueSeverity::Critical,
                        message: format!("Cross-project pollution: entity {} belongs to project {}, but change targets project {}", change.target_entity_id, entity.project_id, change.project_id),
                        suggestion: Some("Ensure the target entity belongs to the same project".to_string()),
                        created_at: Utc::now(),
                    });
                    return Ok(issues);
                }
            }
        }

        // Payload 验证
        match &change.change_type {
            ProposedChangeType::StateChange => {
                if change.payload.get("state_key").is_none() {
                    issues.push(ValidationIssue {
                        id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                        issue_type: ValidationIssueType::TypeMismatch, severity: IssueSeverity::Critical,
                        message: "StateChange payload missing 'state_key'".to_string(),
                        suggestion: Some("Include state_key in payload".to_string()),
                        created_at: Utc::now(),
                    });
                }
                if change.payload.get("new_value").is_none() {
                    issues.push(ValidationIssue {
                        id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                        issue_type: ValidationIssueType::TypeMismatch, severity: IssueSeverity::Critical,
                        message: "StateChange payload missing 'new_value'".to_string(),
                        suggestion: Some("Include new_value in payload".to_string()),
                        created_at: Utc::now(),
                    });
                }
            }
            _ => {}
        }

        // 结构化 Canon Rule 验证
        let canon_issues = self.check_canon_rules_structured(change, canon_rules).await?;
        issues.extend(canon_issues);

        Ok(issues)
    }

    /// 批量加载 Canon Rules
    async fn load_canon_rules(&self, project_id: Uuid) -> Result<Vec<CanonRule>> {
        let rows = sqlx::query_as::<_, CanonRuleRow>(
            "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at \
             FROM canon_rule WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 结构化 Canon Rule 验证
    ///
    /// 使用 constraints JSON 进行结构化比较，而非字符串 contains。
    /// 支持的 operator: EQUAL, NOT_EQUAL, IN, NOT_IN
    async fn check_canon_rules_structured(
        &self,
        change: &ProposedChange,
        canon_rules: &[CanonRule],
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // 只验证 StateChange 类型
        let (state_key, new_value) = match &change.change_type {
            ProposedChangeType::StateChange => {
                let key = change.payload.get("state_key").and_then(|v| v.as_str()).unwrap_or("");
                let val = change.payload.get("new_value").cloned().unwrap_or(serde_json::Value::Null);
                (key, val)
            }
            _ => return Ok(issues),
        };

        for rule in canon_rules {
            // 解析 constraints JSON
            let constraints = if let Some(obj) = rule.constraints.as_object() {
                obj
            } else {
                // 没有结构化约束，回退到字符串匹配
                let is_relevant = change.description.to_lowercase().contains(&rule.affected_scope.to_lowercase())
                    || rule.rule_content.to_lowercase().contains(&rule.affected_scope.to_lowercase());
                if is_relevant && rule.enforcement == EnforcementAction::Reject {
                    issues.push(ValidationIssue {
                        id: Uuid::new_v4(),
                        validation_run_id: Uuid::nil(),
                        proposed_change_id: change.id,
                        issue_type: ValidationIssueType::RuleViolation,
                        severity: IssueSeverity::Critical,
                        message: format!("RULE-0 violation: {}", rule.rule_content),
                        suggestion: Some(format!("This change conflicts with absolute rule [{}]: {}", rule.affected_scope, rule.rule_content)),
                        created_at: Utc::now(),
                    });
                }
                continue;
            };

            // 结构化验证：检查 state_key 是否匹配
            let rule_state_key = constraints.get("state_key").and_then(|v| v.as_str());
            if let Some(rk) = rule_state_key {
                if rk != state_key {
                    continue; // 不是这个规则关心的 state_key
                }
            } else {
                continue; // 没有 state_key 约束
            }

            // 获取 operator 和 expected_value
            let operator = constraints.get("operator").and_then(|v| v.as_str()).unwrap_or("EQUAL");
            let expected = constraints.get("value");

            let violated = match operator {
                "EQUAL" => {
                    if let Some(expected) = expected {
                        new_value == *expected
                    } else {
                        false
                    }
                }
                "NOT_EQUAL" => {
                    if let Some(expected) = expected {
                        new_value == *expected
                    } else {
                        false
                    }
                }
                "IN" => {
                    if let Some(expected_arr) = expected.and_then(|v| v.as_array()) {
                        expected_arr.contains(&new_value)
                    } else {
                        false
                    }
                }
                "NOT_IN" => {
                    if let Some(expected_arr) = expected.and_then(|v| v.as_array()) {
                        expected_arr.contains(&new_value)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if violated {
                let severity = match rule.rule_level {
                    RuleLevel::Rule0 => IssueSeverity::Critical,
                    RuleLevel::Rule1 => IssueSeverity::Warning,
                    RuleLevel::Rule2 => IssueSeverity::Warning,
                    RuleLevel::Rule3 => IssueSeverity::Warning,
                };

                issues.push(ValidationIssue {
                    id: Uuid::new_v4(),
                    validation_run_id: Uuid::nil(),
                    proposed_change_id: change.id,
                    issue_type: ValidationIssueType::RuleViolation,
                    severity,
                    message: format!("Canon rule violation: {} ({} {} {:?})", rule.rule_content, state_key, operator, expected),
                    suggestion: Some(format!("This change violates rule [{}]: {}", rule.affected_scope, rule.rule_content)),
                    created_at: Utc::now(),
                });
            }
        }

        Ok(issues)
    }

    /// 列出已批准的变更
    pub async fn list_approved_changes(&self, project_id: Uuid, task_id: Uuid) -> Result<Vec<ProposedChange>> {
        let rows = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at \
             FROM proposed_change WHERE project_id = $1 AND task_id = $2 AND status = 'Approved' ORDER BY created_at",
        )
        .bind(project_id)
        .bind(task_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query approved changes")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

// ============================================================
// Row types
// ============================================================

#[derive(sqlx::FromRow)]
struct ProposedChangeRow {
    id: Uuid,
    project_id: Uuid,
    task_id: Uuid,
    change_type: String,
    target_entity_id: Uuid,
    description: String,
    payload: Option<serde_json::Value>,
    status: String,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl From<ProposedChangeRow> for ProposedChange {
    fn from(r: ProposedChangeRow) -> Self {
        ProposedChange {
            id: r.id,
            project_id: r.project_id,
            task_id: r.task_id,
            change_type: db::ser::parse_proposed_change_type(&r.change_type),
            target_entity_id: r.target_entity_id,
            description: r.description,
            payload: r.payload.unwrap_or_default(),
            status: db::ser::parse_proposed_change_status(&r.status),
            created_at: r.created_at,
            resolved_at: r.resolved_at,
        }
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
            constraints: r.constraints.unwrap_or_default(),
            source: r.source,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
