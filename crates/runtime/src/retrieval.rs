//! Retrieval - 数据库特定的上下文素材检索。
//!
//! 这一层是 ContextEngine 与"从存储读取关系 / 事件 / 知识 / 规则"之间的边界。
//! 原本散布在 ContextEngine 中的检索逻辑在此集中，使 ContextEngine 只负责
//! 可见性、排序、预算与组装（策略），而不关心数据具体来自哪个端口。
//!
//! 检索本身仍通过 domain::ports 抽象进行（Retriever 持有端口），
//! 因此 ContextEngine 不直接依赖 db / sqlx。

use std::sync::Arc;

use anyhow::Result;
use domain::ports::*;
use domain::*;
use uuid::Uuid;

/// 聚合"场景上下文素材"的检索器。
///
/// 持有 Relation / Event / Knowledge / CanonRule 四个读取端口。
/// 这些是 ContextEngine 在 build_context 中需要"按实体集合拉取"的素材。
pub struct Retriever {
    relation: Arc<dyn RelationPort>,
    event: Arc<dyn EventPort>,
    knowledge: Arc<dyn KnowledgePort>,
    canon: Arc<dyn CanonRulePort>,
}

impl Retriever {
    pub fn new(
        relation: Arc<dyn RelationPort>,
        event: Arc<dyn EventPort>,
        knowledge: Arc<dyn KnowledgePort>,
        canon: Arc<dyn CanonRulePort>,
    ) -> Self {
        Self {
            relation,
            event,
            knowledge,
            canon,
        }
    }

    /// 检索与场景相关实体有关的所有关系。
    pub async fn get_relevant_relations(
        &self,
        project_id: Uuid,
        scene_attrs: &SceneAttributes,
    ) -> Result<Vec<Relation>> {
        let mut entity_ids: Vec<Uuid> = scene_attrs.characters_present.clone();
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !entity_ids.contains(&pov_id) {
                entity_ids.push(pov_id);
            }
        }
        if let Some(loc_id) = scene_attrs.location_id {
            if !entity_ids.contains(&loc_id) {
                entity_ids.push(loc_id);
            }
        }

        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        self.relation
            .find_relations_by_entities(project_id, &entity_ids)
            .await
    }

    /// 检索与场景相关实体有关的最近事件。
    pub async fn get_recent_events(
        &self,
        project_id: Uuid,
        scene_attrs: &SceneAttributes,
    ) -> Result<Vec<Event>> {
        let mut entity_ids: Vec<Uuid> = scene_attrs.characters_present.clone();
        if let Some(pov_id) = scene_attrs.pov_character_id {
            if !entity_ids.contains(&pov_id) {
                entity_ids.push(pov_id);
            }
        }
        if let Some(loc_id) = scene_attrs.location_id {
            if !entity_ids.contains(&loc_id) {
                entity_ids.push(loc_id);
            }
        }

        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        self.event
            .find_events_by_entities(project_id, &entity_ids)
            .await
    }

    /// 检索某角色实际掌握的知识，并格式化为可读文本。
    pub async fn get_character_knowledge(
        &self,
        project_id: Uuid,
        character_id: Uuid,
    ) -> Result<String> {
        let items = self
            .knowledge
            .get_character_known_facts(character_id, project_id)
            .await?;
        if items.is_empty() {
            return Ok(String::new());
        }

        let mut content =
            String::from("## Character Knowledge (what this character actually knows)
");
        for item in &items {
            let level_str = match item.knowledge_level {
                KnowledgeLevel::Unknown => "unknown",
                KnowledgeLevel::Hearsay => "hearsay",
                KnowledgeLevel::Partial => "partial",
                KnowledgeLevel::Complete => "complete",
                KnowledgeLevel::Misunderstood => "misunderstood",
            };
            let certainty_tag = if item.fact_certainty != "CANON" {
                format!(" [{}]", item.fact_certainty)
            } else {
                String::new()
            };
            let source_info = item.source.as_deref().unwrap_or("unknown");
            content.push_str(&format!(
                "- {} ({}){}: {}
",
                level_str, source_info, certainty_tag, item.fact_content
            ));
        }
        Ok(content)
    }

    /// 检索世界规则（Canon 宪法）文本。
    pub async fn get_world_rules_summary(&self, project_id: Uuid) -> Result<String> {
        let mut content = String::new();

        let rules = self.canon.list_canon_rules(project_id).await?;
        if !rules.is_empty() {
            content.push_str("## World Rules (Canon Constitution)
");
            for rule in rules {
                content.push_str(&format!(
                    "- [{:?}] {}: {}
",
                    rule.rule_level, rule.affected_scope, rule.rule_content
                ));
            }
        }

        if let Some(world_rules) = self.canon.get_main_world_rules_text(project_id).await? {
            if !world_rules.is_empty() {
                if content.is_empty() {
                    content.push_str("## World Rules
");
                }
                content.push_str(&format!("{}
", world_rules));
            }
        }

        Ok(content)
    }
}
