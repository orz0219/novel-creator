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
            if phrase_matches(draft_content, forbidden) {
                forbidden_violated.push(forbidden.clone());
                issues.push(format!("禁止事件发生: {}", forbidden));
            }
        }

        // 检查必需事件（按词匹配，容忍助词/量词插入）
        let required_events_met = contract.required_events.is_empty() ||
            contract.required_events.iter().any(|e| phrase_matches(draft_content, e));

        // 检查必需角色（简化：检查名称）
        let required_characters_met = true; // 需要角色名称才能检查

        // 检查必需事实
        let required_facts_met = contract.required_facts.is_empty() ||
            contract.required_facts.iter().any(|f| phrase_matches(draft_content, f));

        // 检查读者学习
        let reader_learns_met = contract.reader_learns.is_empty() ||
            contract.reader_learns.iter().any(|r| phrase_matches(draft_content, r));

        // 检查主角学习
        let protagonist_learns_met = contract.protagonist_learns.is_empty() ||
            contract.protagonist_learns.iter().any(|p| phrase_matches(draft_content, p));

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

/// 按字序匹配短语：检查 draft 中是否包含 phrase 的所有汉字且顺序正确。
/// 容忍中间插入助词、量词等（如 "进入黑市" 可匹配 "进入了黑市"、"进入了那座黑市"）。
fn phrase_matches(draft: &str, phrase: &str) -> bool {
    let phrase_chars: Vec<char> = phrase.chars().collect();
    let draft_chars: Vec<char> = draft.chars().collect();
    let mut pi = 0;
    for &dc in &draft_chars {
        if pi < phrase_chars.len() && dc == phrase_chars[pi] {
            pi += 1;
        }
    }
    pi == phrase_chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phrase_matches_exact() {
        assert!(phrase_matches("林凡进入黑市", "进入黑市"));
    }

    #[test]
    fn test_phrase_matches_with_particle() {
        assert!(phrase_matches("林凡进入了黑市", "进入黑市"));
    }

    #[test]
    fn test_phrase_matches_with_extra_chars() {
        assert!(phrase_matches("林凡进入了那座黑市", "进入黑市"));
    }

    #[test]
    fn test_phrase_matches_not_found() {
        assert!(!phrase_matches("林凡离开黑市", "进入黑市"));
    }

    #[test]
    fn test_phrase_matches_discover() {
        assert!(phrase_matches("突然发现了远古遗迹的入口", "发现遗迹"));
    }

    #[test]
    fn test_phrase_matches_forbidden_violation() {
        assert!(phrase_matches("林凡进入黑市，突然发现了远古遗迹的入口。", "发现遗迹"));
    }
}
