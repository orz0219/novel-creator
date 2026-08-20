//! Validator - 变更验证
//!
//! 核心原则：AI 只能提出 ProposedChange，所有变更必须经过验证。
//! Validator 只负责验证，不负责提交。
//! 提交由 StateCommitter 在独立事务中完成。
//!
//! 性能要求：批量查询，避免 N+1 Query。

use anyhow::Result;
use chrono::Utc;
use domain::*;
use domain::ports::*;
use std::sync::Arc;
use uuid::Uuid;

/// Ports required by the Validator. Injected at the composition root.
pub struct ValidatorDeps {
    pub entity: Arc<dyn EntityPort>,
    pub validation: Arc<dyn ValidationPort>,
    pub approval: Arc<dyn ApprovalPort>,
    pub canon: Arc<dyn CanonRulePort>,
    pub proposed_change: Arc<dyn ProposedChangeQueryPort>,
}

/// Validator - 变更验证器。只负责验证 ProposedChange 的合规性。
/// 不负责 state commit - 那是 StateCommitter 的职责。
pub struct Validator {
    entity: Arc<dyn EntityPort>,
    validation: Arc<dyn ValidationPort>,
    approval: Arc<dyn ApprovalPort>,
    canon: Arc<dyn CanonRulePort>,
    proposed_change: Arc<dyn ProposedChangeQueryPort>,
}

impl Validator {
    pub fn new(deps: ValidatorDeps) -> Self {
        Self {
            entity: deps.entity,
            validation: deps.validation,
            approval: deps.approval,
            canon: deps.canon,
            proposed_change: deps.proposed_change,
        }
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
        // 批量查询：收集所有 entity_ids，一次加载
        let entity_ids: Vec<Uuid> = changes.iter().map(|c| c.target_entity_id).collect();
        let entities = self.entity.list_entities_by_ids(project_id, &entity_ids).await?;
        let entity_map: std::collections::HashMap<Uuid, Entity> = entities.into_iter().map(|e| (e.id, e)).collect();

        // 批量加载 Canon Rules
        let canon_rules = self.load_canon_rules(project_id).await?;

        let mut run = self.validation.create_validation_run(project_id, task_id).await?;
        let mut approved = 0;
        let mut rejected = 0;

        for change in changes {
            let entity = entity_map.get(&change.target_entity_id);
            let issues = self.validate_single_change(change, entity, &canon_rules).await?;

            let has_critical = issues.iter().any(|i| i.severity == IssueSeverity::Critical);
            let has_warning = issues.iter().any(|i| i.severity == IssueSeverity::Warning);

            if has_critical {
                self.validation.update_status(change.id, ProposedChangeStatus::Rejected).await?;
                rejected += 1;
                for issue in &issues {
                    self.validation.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref()).await?;
                }
            } else if has_warning {
                self.validation.update_status(change.id, ProposedChangeStatus::PendingApproval).await?;
                let _ = self.approval.create(
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
                    self.validation.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref()).await?;
                }
            } else {
                self.validation.update_status(change.id, ProposedChangeStatus::Approved).await?;
                approved += 1;
            }
        }

        run.changes_validated = changes.len() as i32;
        run.changes_approved = approved;
        run.changes_rejected = rejected;
        run.status = ValidationStatus::Completed;
        run.completed_at = Some(Utc::now());
        self.validation.update_validation_run(&run).await?;

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
        self.canon.list_canon_rules(project_id).await
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
        self.proposed_change.list_approved_changes(project_id, task_id).await
    }
}

// ============================================================
// Row types
// ============================================================