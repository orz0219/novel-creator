//! Validator - 变更验证
//!
//! 核心原则：AI 只能提出 ProposedChange，所有变更必须经过验证。
//! Validator 只负责验证，不负责提交。
//! 提交由 StateCommitter 在独立事务中完成。

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
    pub async fn validate_changes(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        changes: &[ProposedChange],
    ) -> Result<ValidationRun> {
        let val_repo = validation_repo::ValidationRepo::new(self.pool.clone());
        let entity_repo = entity_repo::EntityRepo::new(self.pool.clone());

        let mut run = val_repo.create_validation_run(project_id, task_id).await?;
        let mut approved = 0;
        let mut rejected = 0;

        for change in changes {
            let issues = self.validate_single_change(change, &entity_repo).await?;

            let has_critical = issues.iter().any(|i| i.severity == IssueSeverity::Critical);
            let has_warning = issues.iter().any(|i| i.severity == IssueSeverity::Warning);

            if has_critical {
                val_repo.update_status(change.id, ProposedChangeStatus::Rejected).await?;
                rejected += 1;
                for issue in &issues {
                    val_repo.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref()).await?;
                }
            } else if has_warning {
                // RULE-1/RULE-2 warnings require human approval
                val_repo.update_status(change.id, ProposedChangeStatus::PendingApproval).await?;
                // Create an ApprovalRecord for the human review gate
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

    async fn validate_single_change(
        &self,
        change: &ProposedChange,
        entity_repo: &entity_repo::EntityRepo,
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // P0-3: 验证 target entity 属于当前 project
        match entity_repo.get_by_id(change.target_entity_id).await? {
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

        // Check Canon Rules
        let canon_issues = self.check_canon_rules(change).await?;
        issues.extend(canon_issues);

        Ok(issues)
    }

    /// Check ProposedChange against Canon Constitution (RULE-0 ~ RULE-3)
    async fn check_canon_rules(&self, change: &ProposedChange) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, rule_level, rule_content, affected_scope, enforcement FROM canon_rule WHERE project_id = $1"
        )
        .bind(change.project_id)
        .fetch_all(&self.pool)
        .await;

        let rows = match rows {
            Ok(r) => r,
            Err(_) => return Ok(issues), // No canon rules table or no rules
        };

        for (_rule_id, rule_level, rule_content, scope, enforcement) in rows {
            let change_desc = change.description.to_lowercase();
            let rule_lower = rule_content.to_lowercase();
            let scope_lower = scope.to_lowercase();

            let is_relevant = change_desc.contains(&scope_lower)
                || change_desc.contains(&rule_lower)
                || change.payload.to_string().to_lowercase().contains(&scope_lower);

            if is_relevant {
                match rule_level.as_str() {
                    "RULE-0" => {
                        if enforcement == "Reject" {
                            issues.push(ValidationIssue {
                                id: Uuid::new_v4(),
                                validation_run_id: Uuid::nil(),
                                proposed_change_id: change.id,
                                issue_type: ValidationIssueType::RuleViolation,
                                severity: IssueSeverity::Critical,
                                message: format!("RULE-0 violation: {}", rule_content),
                                suggestion: Some(format!("This change conflicts with absolute rule [{}]: {}", scope, rule_content)),
                                created_at: Utc::now(),
                            });
                        }
                    }
                    "RULE-1" => {
                        issues.push(ValidationIssue {
                            id: Uuid::new_v4(),
                            validation_run_id: Uuid::nil(),
                            proposed_change_id: change.id,
                            issue_type: ValidationIssueType::RuleViolation,
                            severity: IssueSeverity::Warning,
                            message: format!("RULE-1 conflict: {}", rule_content),
                            suggestion: Some(format!("This change may conflict with world rule [{}]. Requires author approval.", scope)),
                            created_at: Utc::now(),
                        });
                    }
                    "RULE-2" => {
                        issues.push(ValidationIssue {
                            id: Uuid::new_v4(),
                            validation_run_id: Uuid::nil(),
                            proposed_change_id: change.id,
                            issue_type: ValidationIssueType::RuleViolation,
                            severity: IssueSeverity::Warning,
                            message: format!("RULE-2 conflict: {}", rule_content),
                            suggestion: Some(format!("This change may conflict with established fact [{}]. Requires author approval.", scope)),
                            created_at: Utc::now(),
                        });
                    }
                    _ => {}
                }
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
