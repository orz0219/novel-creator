//! Project - 项目顶层模型
//!
//! 一个 Project 代表一部完整的小说作品。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 项目 - 一部小说的顶层容器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// 项目语言（如 "zh-CN", "en"）
    pub language: Option<String>,
    /// 世界观设定（自由文本）
    pub world_setting: Option<String>,
    /// 金手指/系统设定
    pub system_setting: Option<String>,
    /// 故事脑洞/前提（一句话核心反常设定，如"我捡了一块钱怎么也花不完"）
    ///
    /// 与 genre 同层级：项目级元数据，世界观展开前由 Agent 引导用户与 AI
    /// 持续沟通打磨后确认落库。后续所有世界观/角色/叙事生成都受 premise 约束，
    /// 作为 prompt 注入的最强变量。
    pub premise: Option<String>,
    /// 默认 LLM 模型
    pub default_model: Option<String>,
    /// 默认写作风格
    pub default_style: Option<String>,
    /// 默认生成参数（JSON）
    pub default_params: serde_json::Value,
    /// 项目配置（JSON）
    pub config: serde_json::Value,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 项目状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectStatus {
    /// 概念阶段
    Concept,
    /// 规划中
    Planning,
    /// 创作中
    Writing,
    /// 暂停
    Paused,
    /// 已完成
    Completed,
    /// 已归档
    Archived,
}
