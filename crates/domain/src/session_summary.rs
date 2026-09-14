//! 对话滚动摘要（项目级，每个项目恒定一份）。
//!
//! 背景：`agent_memory` 里存的是**只增不减**的碎片结论，而整段会话历史又会被
//! 拍平成一次请求（见 `agent::runtime`）。两者都会无限膨胀，于是需要一份
//! 「当前故事进展到哪」的滚动摘要：会话收尾时更新，开新会话时自动读到。
//!
//! 三条硬约定：
//! 1. **每个项目恒定一份**——保存即覆盖（upsert），永不允许追加成多份，
//!    否则就从「历史膨胀」变成「摘要膨胀」。
//! 2. **生成时把旧摘要喂进去**——否则第二次收尾会丢掉前几轮谈定的内容。
//! 3. **只记对话层状态，不记世界事实**——premise / 人物 / 世界观必须经工具
//!    落进世界库（World Canon 是唯一真源），摘要里再写一遍就是双写。

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 滚动摘要的内容（固定字段，不用自由散文）。
///
/// 用固定字段而非自然语言，是为了让内容可校验、可展示、可局部修改；
/// 自由文本摘要在每次重新生成时都会随机丢细节。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionSummary {
    /// 故事现在推进到哪里（一到两句话）。
    #[serde(default)]
    pub story_state: String,
    /// 本次会话谈定的事（世界事实必须已落库，这里只记「决定了什么」）。
    #[serde(default)]
    pub confirmed: Vec<String>,
    /// 尚未收口的伏笔 / 待办 / 用户提过但还没处理的问题。
    #[serde(default)]
    pub open_threads: Vec<String>,
    /// 下次接着做什么。
    #[serde(default)]
    pub next_step: String,
}

/// 带元信息的滚动摘要（用于界面展示「上次收尾时间」）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSessionSummary {
    pub content: SessionSummary,
    pub updated_at: DateTime<Utc>,
}

/// 滚动摘要的存储端口。
///
/// 只做「取当前那一份」和「覆盖成新的一份」，没有追加语义。
#[async_trait]
pub trait SessionSummaryPort: Send + Sync {
    /// 读取项目当前的滚动摘要；从未收尾过时为 `None`。
    async fn load(&self, project_id: Uuid) -> Result<Option<StoredSessionSummary>>;

    /// 覆盖写入项目摘要（每个项目恒定一份），返回落库后的记录。
    async fn save(
        &self,
        project_id: Uuid,
        summary: &SessionSummary,
    ) -> Result<StoredSessionSummary>;
}
