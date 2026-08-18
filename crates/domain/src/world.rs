//! World - 世界模型的顶层容器
//!
//! 一个 Project 可以拥有多个 World（V1 默认只有一个主 World）。
//! World 是客观存在的世界，不关心谁什么时候知道什么。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 世界 - 客观存在的世界
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// 世界规则（自由文本，如"灵气复苏"、"魔法体系"等）
    pub world_rules: Option<String>,
    /// 世界配置（JSON，用于存储自定义设置）
    pub config: serde_json::Value,
    pub is_main: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            project_id: Uuid::nil(),
            name: String::new(),
            description: None,
            world_rules: None,
            config: serde_json::json!({}),
            is_main: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
