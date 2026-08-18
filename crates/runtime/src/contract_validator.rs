//! Contract Validator - Narrative Contract 验证

use anyhow::Result;
use db::connection::Database;
use domain::*;

/// 合约验证结果
#[derive(Debug)]
pub struct ContractValidationResult {
    pub passed: bool,
    pub required_events_met: bool,
    pub forbidden_events_violated: Vec<String>,
    pub required_characters_met: bool,
    pub required_facts_met: bool,
    pub reader_learns_met: bool,
    pub protagonist_learns_met: bool,
    pub world_changes_met: bool,
    pub issues: Vec<String>,
}

pub struct ContractValidator<'a> {
    db: &'a Database,
}

impl<'a> ContractValidator<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 验证场景是否满足契约
    pub fn validate(&self, contract: &SceneContract, draft_content: &str) -> Result<ContractValidationResult> {
        let mut issues = Vec::new();
        let mut forbidden_violated = Vec::new();

        // 检查禁止事件
        for forbidden in &contract.forbidden_events {
            if draft_content.contains(forbidden) {
                forbidden_violated.push(forbidden.clone());
                issues.push(format!("禁止事件发生: {}", forbidden));
            }
        }

        // 检查必需事件（简化：检查关键词）
        let required_events_met = contract.required_events.is_empty() || 
            contract.required_events.iter().any(|e| draft_content.contains(e));

        // 检查必需角色（简化：检查名称）
        let required_characters_met = true; // 需要角色名称才能检查

        // 检查必需事实
        let required_facts_met = contract.required_facts.is_empty() ||
            contract.required_facts.iter().any(|f| draft_content.contains(f));

        // 检查读者学习
        let reader_learns_met = contract.reader_learns.is_empty() ||
            contract.reader_learns.iter().any(|r| draft_content.contains(r));

        // 检查主角学习
        let protagonist_learns_met = contract.protagonist_learns.is_empty() ||
            contract.protagonist_learns.iter().any(|p| draft_content.contains(p));

        // 检查世界变化
        let world_changes_met = contract.world_changes.is_empty();

        let passed = required_events_met && forbidden_violated.is_empty() && required_characters_met && required_facts_met;

        Ok(ContractValidationResult {
            passed,
            required_events_met,
            forbidden_events_violated: forbidden_violated,
            required_characters_met,
            required_facts_met,
            reader_learns_met,
            protagonist_learns_met,
            world_changes_met,
            issues,
        })
    }
}
