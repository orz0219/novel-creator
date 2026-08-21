//! Skill 定义 - 13种完整 Skill 类型 + Input/Output Schema + ContextPolicy 绑定
//!
//! 每个 Skill 是一个独立的生成能力，包含：
//! - 名称和描述
//! - Input/Output Schema
//! - Context Policy（决定需要哪些上下文）
//! - 版本管理

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 上下文层类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextLayerType {
    L0Essential,
    L1SceneRelevant,
    L2RecentHistory,
    L3NarrativeContext,
    L4CharacterKnowledge,
    L5WorldBackground,
    L6OptionalSupplement,
}

/// 上下文策略 - 定义不同 Skill 需要哪些上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicy {
    pub name: String,
    pub required_layers: Vec<ContextLayerType>,
    pub optional_layers: Vec<ContextLayerType>,
    pub excluded_layers: Vec<ContextLayerType>,
    pub max_budget_ratio: f64,
}

impl ContextPolicy {
    pub fn location_designer() -> Self {
        Self {
            name: "location_designer".to_string(),
            required_layers: vec![ContextLayerType::L0Essential, ContextLayerType::L3NarrativeContext],
            optional_layers: vec![ContextLayerType::L1SceneRelevant, ContextLayerType::L5WorldBackground],
            excluded_layers: vec![ContextLayerType::L4CharacterKnowledge, ContextLayerType::L6OptionalSupplement],
            max_budget_ratio: 0.8,
        }
    }

    pub fn character_designer() -> Self {
        Self {
            name: "character_designer".to_string(),
            required_layers: vec![ContextLayerType::L0Essential, ContextLayerType::L3NarrativeContext, ContextLayerType::L5WorldBackground],
            optional_layers: vec![ContextLayerType::L1SceneRelevant],
            excluded_layers: vec![ContextLayerType::L4CharacterKnowledge, ContextLayerType::L6OptionalSupplement],
            max_budget_ratio: 0.8,
        }
    }

    pub fn scene_writer() -> Self {
        Self {
            name: "scene_writer".to_string(),
            required_layers: vec![ContextLayerType::L0Essential, ContextLayerType::L1SceneRelevant, ContextLayerType::L4CharacterKnowledge],
            optional_layers: vec![ContextLayerType::L2RecentHistory, ContextLayerType::L3NarrativeContext, ContextLayerType::L5WorldBackground],
            excluded_layers: vec![],
            max_budget_ratio: 1.0,
        }
    }

    pub fn continuity_validator() -> Self {
        Self {
            name: "continuity_validator".to_string(),
            required_layers: vec![ContextLayerType::L0Essential, ContextLayerType::L1SceneRelevant, ContextLayerType::L4CharacterKnowledge, ContextLayerType::L5WorldBackground],
            optional_layers: vec![ContextLayerType::L2RecentHistory, ContextLayerType::L3NarrativeContext],
            excluded_layers: vec![],
            max_budget_ratio: 1.0,
        }
    }
}

/// 完整的 Skill 定义（V2 增强版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub version: i32,
    /// 输入 Schema（JSON Schema 格式）
    pub input_schema: serde_json::Value,
    /// 输出 Schema（JSON Schema 格式）
    pub output_schema: serde_json::Value,
    /// 上下文策略
    pub context_policy: ContextPolicyConfig,
    /// 默认参数
    pub default_params: serde_json::Value,
    /// Prompt 模板
    pub prompt_template: String,
    pub status: SkillStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 上下文策略配置（可序列化版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicyConfig {
    pub name: String,
    pub required_layers: Vec<String>,
    pub optional_layers: Vec<String>,
    pub excluded_layers: Vec<String>,
    pub max_budget_ratio: f64,
}

impl ContextPolicyConfig {
    /// 从 ContextPolicy 转换
    pub fn from_policy(policy: &ContextPolicy) -> Self {
        Self {
            name: policy.name.clone(),
            required_layers: policy.required_layers.iter().map(|l| format!("{:?}", l)).collect(),
            optional_layers: policy.optional_layers.iter().map(|l| format!("{:?}", l)).collect(),
            excluded_layers: policy.excluded_layers.iter().map(|l| format!("{:?}", l)).collect(),
            max_budget_ratio: policy.max_budget_ratio,
        }
    }
}

/// Skill 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillStatus {
    Draft,
    Active,
    Deprecated,
}

/// Skill 类型（完整定义）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillType {
    // 规划类
    WorldPlanner,
    VolumePlanner,
    ArcPlanner,
    ScenePlanner,

    // 设计类
    CharacterDesigner,
    LocationDesigner,
    FactionDesigner,
    PlotDesigner,

    // 生成类
    Writer,
    Polisher,

    // 分析类
    Analyzer,
    ContinuityValidator,

    // 提取类
    KnowledgeExtractor,
    StateChangeExtractor,

    // 自定义
    Custom(String),
}

impl SkillType {
    /// 获取对应的 ContextPolicy
    pub fn context_policy(&self) -> ContextPolicy {
        match self {
            SkillType::LocationDesigner => ContextPolicy::location_designer(),
            SkillType::CharacterDesigner => ContextPolicy::character_designer(),
            SkillType::Writer => ContextPolicy::scene_writer(),
            SkillType::Analyzer | SkillType::ContinuityValidator => ContextPolicy::continuity_validator(),
            _ => ContextPolicy::scene_writer(),
        }
    }
}

/// Skill 版本历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: i32,
    pub prompt_template: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub default_params: Option<serde_json::Value>,
    pub changelog: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================
// 预定义的 13 种 Skill
// ============================================================

/// Skill 模板库
pub struct SkillTemplates;

impl SkillTemplates {
    /// 1. World Analyzer - 分析世界设定
    pub fn world_analyzer() -> SkillTemplate {
        SkillTemplate {
            name: "world_analyzer".to_string(),
            description: "分析世界设定，提取关键规则和约束".to_string(),
            skill_type: SkillType::Analyzer,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "world_rules": {"type": "string", "description": "世界规则文本"},
                    "existing_entities": {"type": "array", "description": "已存在的实体列表"}
                },
                "required": ["world_rules"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "rules": {"type": "array", "items": {"type": "string"}},
                    "constraints": {"type": "array", "items": {"type": "string"}},
                    "suggestions": {"type": "array", "items": {"type": "string"}}
                }
            }),
            prompt_template: "Analyze the world rules and extract key constraints and suggestions.".to_string(),
        }
    }

    /// 2. Volume Planner - 规划卷
    pub fn volume_planner() -> SkillTemplate {
        SkillTemplate {
            name: "volume_planner".to_string(),
            description: "规划一个卷的结构和目标".to_string(),
            skill_type: SkillType::VolumePlanner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "world_context": {"type": "string"},
                    "narrative_goals": {"type": "array", "items": {"type": "string"}},
                    "existing_volumes": {"type": "array"}
                },
                "required": ["world_context"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "mission": {"type": "string"},
                    "goal": {"type": "string"},
                    "conflict": {"type": "string"},
                    "start_state": {"type": "string"},
                    "target_state": {"type": "string"},
                    "theme": {"type": "string"}
                }
            }),
            prompt_template: "Plan a volume structure with mission, goal, conflict, and states.".to_string(),
        }
    }

    /// 3. Arc Planner - 规划弧线
    pub fn arc_planner() -> SkillTemplate {
        SkillTemplate {
            name: "arc_planner".to_string(),
            description: "规划一个故事弧线".to_string(),
            skill_type: SkillType::ArcPlanner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "volume_context": {"type": "string"},
                    "existing_arcs": {"type": "array"}
                },
                "required": ["volume_context"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "goal": {"type": "string"},
                    "conflict": {"type": "string"},
                    "participants": {"type": "array", "items": {"type": "string"}},
                    "start_condition": {"type": "string"},
                    "end_condition": {"type": "string"}
                }
            }),
            prompt_template: "Plan an arc with goal, conflict, participants, and conditions.".to_string(),
        }
    }

    /// 4. Scene Planner - 规划场景
    pub fn scene_planner() -> SkillTemplate {
        SkillTemplate {
            name: "scene_planner".to_string(),
            description: "规划一个场景的结构".to_string(),
            skill_type: SkillType::ScenePlanner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "arc_context": {"type": "string"},
                    "chapter_number": {"type": "integer"},
                    "existing_scenes": {"type": "array"}
                },
                "required": ["arc_context"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "objective": {"type": "string"},
                    "conflict": {"type": "string"},
                    "characters": {"type": "array", "items": {"type": "string"}},
                    "location": {"type": "string"},
                    "emotional_goal": {"type": "string"},
                    "information_goal": {"type": "string"}
                }
            }),
            prompt_template: "Plan a scene with objective, conflict, characters, and goals.".to_string(),
        }
    }

    /// 5. Character Designer - 设计角色（通用小说人物设定，服务于后续情节）
    pub fn character_designer() -> SkillTemplate {
        SkillTemplate {
            name: "character_designer".to_string(),
            description: "设计一个服务于后续情节的新角色（通用，不绑定单一题材）".to_string(),
            skill_type: SkillType::CharacterDesigner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "world_context": {"type": "string"},
                    "volume_context": {"type": "string"},
                    "arc_context": {"type": "string"},
                    "existing_characters": {"type": "array"},
                    "role_requirements": {"type": "string", "description": "该角色在故事中的功能位与需要承担的情节作用"}
                },
                "required": ["world_context", "role_requirements"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "aliases": {"type": "array", "items": {"type": "string"}},
                    "age_range": {"type": "string", "enum": ["Child","Teen","YoungAdult","Adult","MiddleAge","Elder","Unknown"]},
                    "gender": {"type": "string", "enum": ["Male","Female","NonBinary","Unknown","Other"]},
                    "identity": {"type": "string", "description": "身份定位，如'边境散修''王家家主'，不是真人档案"},
                    "appearance": {"type": "string"},
                    "background_origin": {"type": "string", "description": "导致现在状态的关键经历"},
                    "social_position": {
                        "type": "object",
                        "properties": {
                            "rank": {"type": "string"},
                            "authority_level": {"type": "integer"},
                            "social_access": {"type": "array", "items": {"type": "string"}}
                        }
                    },
                    "core_personality": {"type": "string"},
                    "values": {"type": "string"},
                    "role_in_story": {"type": "string", "enum": ["Protagonist","Antagonist","Mentor","Ally","Rival","Catalyst","Victim","Observer"]},
                    "narrative_necessity": {
                        "type": "object",
                        "properties": {
                            "importance": {"type": "integer"},
                            "irreplaceability": {"type": "integer"},
                            "absence_effect": {"type": "string"},
                            "replacement_cost": {"type": "string"}
                        }
                    },
                    "drive": {
                        "type": "object",
                        "properties": {
                            "primary_goal": {"type": "string"},
                            "motivation": {"type": "string"},
                            "urgency": {"type": "integer"},
                            "hidden_goal": {"type": "string"},
                            "fear": {"type": "string"},
                            "weakness": {"type": "string"},
                            "desire": {"type": "string"},
                            "contradiction": {"type": "string"}
                        }
                    },
                    "conflicts": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "conflict_type": {"type": "string", "enum": ["Internal","External","Relationship","Ideology"]},
                                "description": {"type": "string"},
                                "target": {"type": "string"}
                            }
                        }
                    },
                    "relationships": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "target": {"type": "string"},
                                "relationship_type": {"type": "string"},
                                "attitude": {"type": "string"},
                                "trust_level": {"type": "integer"}
                            }
                        }
                    },
                    "secrets": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": {"type": "string"},
                                "importance": {"type": "integer"},
                                "reveal_condition": {"type": "string"}
                            }
                        }
                    },
                    "capabilities": {
                        "type": "object",
                        "properties": {
                            "skills": {"type": "array", "items": {"type": "string"}},
                            "limitations": {"type": "array", "items": {"type": "string"}}
                        }
                    },
                    "arc_potential": {
                        "type": "object",
                        "properties": {
                            "starting_state": {"type": "string"},
                            "possible_change": {"type": "string"},
                            "resistance": {"type": "string"}
                        }
                    },
                    "extension": {
                        "type": "object",
                        "properties": {
                            "type": {"type": "string", "enum": ["Fantasy","Modern","SciFi","Custom"]},
                            "data": {"type": "object"}
                        },
                        "description": "题材相关扩展；通用字段不够时再填"
                    },
                    "traits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "trait_type": {"type": "string", "enum": ["Personality","Behavior","Value","Fear","Habit","Strength","Weakness"]},
                                "name": {"type": "string"},
                                "description": {"type": "string"},
                                "intensity": {"type": "integer"}
                            }
                        }
                    },
                    "state": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"},
                            "physical_state": {"type": "string"},
                            "mental_state": {"type": "string"},
                            "resource_state": {"type": "string"},
                            "social_state": {"type": "string"},
                            "flags": {"type": "array", "items": {"type": "string"}}
                        }
                    }
                }
            }),
            prompt_template:
                "你是一个通用小说人物设定助手。设计一个服务于后续情节的新角色——人物是剧情发动机，每个字段都要能驱动情节（冲突、关系、弧光、悬念），不要写真人档案或心理测评。\n\
                 \n\
                 严格遵守 ContextPolicy：\n\
                 - L1 场景相关：只生成当前场景/卷真正会用到的内容，不要堆砌无关细节。\n\
                 - L4 角色知识：仅当情节需要信息差时才给出角色隐藏知识，用 knowledge_scope 标出，不要默认全知。\n\
                 \n\
                 字段要求：\n\
                 - age_range / gender 是可选的身份约束，不是性格，不要据此预设行为。\n\
                 - social_position 用 rank/authority_level/social_access 抽象，不要写死'贵族/平民'。\n\
                 - narrative_necessity 说明这个角色对剧情的重要性与可替换性。\n\
                 - drive 给出 primary_goal + motivation + urgency，并尽量点出 hidden_goal / fear / weakness / contradiction（制造张力）。\n\
                 - conflicts 至少一条，标明类型与目标（人物参与剧情的接口）。\n\
                 - relationships / secrets / capabilities(限制比能力更重要) / arc_potential 都为后续情节服务。\n\
                 - extension 仅在玄幻/都市/科幻等题材需要专属字段时填写（如修炼等级、职业、义体），通用设定不要往里塞。\n\
                 \n\
                 输出严格符合给定 JSON schema，不要添加 schema 之外的顶层字段。".to_string(),
        }
    }

    /// 6. Location Designer - 设计地点
    pub fn location_designer() -> SkillTemplate {
        SkillTemplate {
            name: "location_designer".to_string(),
            description: "设计一个新地点".to_string(),
            skill_type: SkillType::LocationDesigner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "world_context": {"type": "string"},
                    "volume_context": {"type": "string"},
                    "arc_context": {"type": "string"},
                    "narrative_requirements": {"type": "string"},
                    "existing_locations": {"type": "array"},
                    "nearby_locations": {"type": "array"}
                },
                "required": ["world_context", "narrative_requirements"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "type": {"type": "string"},
                    "purpose": {"type": "string"},
                    "summary": {"type": "string"},
                    "geography": {"type": "object"},
                    "resources": {"type": "array"},
                    "facilities": {"type": "array"},
                    "factions": {"type": "array"},
                    "threats": {"type": "array"},
                    "secrets": {"type": "array"},
                    "connections": {"type": "array"},
                    "rules": {"type": "array"},
                    "narrative_hooks": {"type": "array"}
                }
            }),
            prompt_template: "Design a location with geography, resources, facilities, factions, threats, secrets, and narrative hooks.".to_string(),
        }
    }

    /// 7. Faction Designer - 设计势力
    pub fn faction_designer() -> SkillTemplate {
        SkillTemplate {
            name: "faction_designer".to_string(),
            description: "设计一个新势力".to_string(),
            skill_type: SkillType::FactionDesigner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "world_context": {"type": "string"},
                    "volume_context": {"type": "string"},
                    "existing_factions": {"type": "array"},
                    "role_requirements": {"type": "string"}
                },
                "required": ["world_context", "role_requirements"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "goals": {"type": "string"},
                    "leader": {"type": "string"},
                    "values": {"type": "string"},
                    "resources": {"type": "string"},
                    "territory": {"type": "string"},
                    "members": {"type": "string"},
                    "enemies": {"type": "string"},
                    "allies": {"type": "string"},
                    "internal_conflicts": {"type": "string"},
                    "secrets": {"type": "string"},
                    "modus_operandi": {"type": "string"}
                }
            }),
            prompt_template: "Design a faction with goals, leader, values, resources, and internal conflicts.".to_string(),
        }
    }

    /// 8. Plot Designer - 设计情节
    pub fn plot_designer() -> SkillTemplate {
        SkillTemplate {
            name: "plot_designer".to_string(),
            description: "设计情节线".to_string(),
            skill_type: SkillType::Custom("plot_designer".into()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "volume_context": {"type": "string"},
                    "arc_context": {"type": "string"},
                    "existing_plots": {"type": "array"}
                },
                "required": ["volume_context"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string"},
                    "key_events": {"type": "array", "items": {"type": "string"}},
                    "twists": {"type": "array", "items": {"type": "string"}}
                }
            }),
            prompt_template: "Design a plot with key events and twists.".to_string(),
        }
    }

    /// 9. Beat Planner - 规划节拍
    pub fn beat_planner() -> SkillTemplate {
        SkillTemplate {
            name: "beat_planner".to_string(),
            description: "规划场景内的节拍".to_string(),
            skill_type: SkillType::ScenePlanner,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "scene_context": {"type": "string"},
                    "scene_objective": {"type": "string"},
                    "scene_conflict": {"type": "string"}
                },
                "required": ["scene_context", "scene_objective"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "beats": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "action": {"type": "string"},
                                "emotion": {"type": "string"},
                                "dialogue_needed": {"type": "boolean"},
                                "word_count_target": {"type": "integer"}
                            }
                        }
                    }
                }
            }),
            prompt_template: "Plan beats for a scene with actions, emotions, and dialogue needs.".to_string(),
        }
    }

    /// 10. Scene Writer - 写正文
    pub fn writer() -> SkillTemplate {
        SkillTemplate {
            name: "scene_writer".to_string(),
            description: "生成场景正文".to_string(),
            skill_type: SkillType::Writer,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "context": {"type": "string", "description": "上下文包"},
                    "beats": {"type": "array", "items": {"type": "string"}},
                    "style": {"type": "string"},
                    "max_tokens": {"type": "integer"}
                },
                "required": ["context", "beats"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {"type": "string", "description": "生成的正文"},
                    "word_count": {"type": "integer"},
                    "model": {"type": "string"}
                }
            }),
            prompt_template: "Write prose for this scene following the beat plan.".to_string(),
        }
    }

    /// 11. Polisher - 润色
    pub fn polisher() -> SkillTemplate {
        SkillTemplate {
            name: "polisher".to_string(),
            description: "润色和优化文本".to_string(),
            skill_type: SkillType::Polisher,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "style": {"type": "string"},
                    "focus": {"type": "string"}
                },
                "required": ["text"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "polished_text": {"type": "string"},
                    "changes_made": {"type": "array", "items": {"type": "string"}}
                }
            }),
            prompt_template: "Polish and optimize the text for better readability and style.".to_string(),
        }
    }

    /// 12. Continuity Validator - 连续性验证
    pub fn continuity_validator() -> SkillTemplate {
        SkillTemplate {
            name: "continuity_validator".to_string(),
            description: "验证文本的连续性和一致性".to_string(),
            skill_type: SkillType::ContinuityValidator,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "draft": {"type": "string", "description": "待验证的文本"},
                    "world_truth": {"type": "string", "description": "世界真相"},
                    "timeline": {"type": "string", "description": "时间线"},
                    "state_history": {"type": "string", "description": "状态历史"}
                },
                "required": ["draft", "world_truth"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "is_consistent": {"type": "boolean"},
                    "issues": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string"},
                                "severity": {"type": "string"},
                                "description": {"type": "string"},
                                "suggestion": {"type": "string"}
                            }
                        }
                    }
                }
            }),
            prompt_template: "Validate the text for continuity and consistency with world truth.".to_string(),
        }
    }

    /// 13. Knowledge Extractor - 知识提取
    pub fn knowledge_extractor() -> SkillTemplate {
        SkillTemplate {
            name: "knowledge_extractor".to_string(),
            description: "从文本中提取角色获得的新知识".to_string(),
            skill_type: SkillType::KnowledgeExtractor,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "characters": {"type": "array", "items": {"type": "string"}},
                    "existing_knowledge": {"type": "object"}
                },
                "required": ["text", "characters"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "knowledge_gains": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "character": {"type": "string"},
                                "new_knowledge": {"type": "array", "items": {"type": "string"}},
                                "source": {"type": "string"}
                            }
                        }
                    }
                }
            }),
            prompt_template: "Extract knowledge gains from the text for each character.".to_string(),
        }
    }

    /// 获取所有预定义 Skill
    pub fn all() -> Vec<SkillTemplate> {
        vec![
            Self::world_analyzer(),
            Self::volume_planner(),
            Self::arc_planner(),
            Self::scene_planner(),
            Self::character_designer(),
            Self::location_designer(),
            Self::faction_designer(),
            Self::plot_designer(),
            Self::beat_planner(),
            Self::writer(),
            Self::polisher(),
            Self::continuity_validator(),
            Self::knowledge_extractor(),
        ]
    }
}

/// Skill 模板
#[derive(Debug, Clone)]
pub struct SkillTemplate {
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub prompt_template: String,
}

impl SkillTemplate {
    /// 转换为 SkillDefinition
    pub fn to_definition(self) -> SkillDefinition {
        let now = Utc::now();
        let policy = self.skill_type.context_policy();
        SkillDefinition {
            id: Uuid::new_v4(),
            name: self.name,
            description: self.description,
            skill_type: self.skill_type,
            version: 1,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            context_policy: ContextPolicyConfig::from_policy(&policy),
            default_params: serde_json::json!({}),
            prompt_template: self.prompt_template,
            status: SkillStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_templates_count() {
        let templates = SkillTemplates::all();
        assert_eq!(templates.len(), 13);
    }

    #[test]
    fn test_skill_type_context_policy() {
        let policy = SkillType::LocationDesigner.context_policy();
        assert_eq!(policy.name, "location_designer");

        let policy = SkillType::Writer.context_policy();
        assert_eq!(policy.name, "scene_writer");
    }

    #[test]
    fn test_skill_definition_conversion() {
        let template = SkillTemplates::writer();
        let def = template.to_definition();
        assert_eq!(def.name, "scene_writer");
        assert_eq!(def.version, 1);
        assert_eq!(def.status, SkillStatus::Active);
    }

    #[test]
    fn test_context_policy_config() {
        let policy = ContextPolicy::location_designer();
        let config = ContextPolicyConfig::from_policy(&policy);
        assert_eq!(config.name, "location_designer");
        assert!(config.required_layers.contains(&"L0Essential".to_string()));
    }
}
