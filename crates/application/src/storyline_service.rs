//! Storyline Service - 跨卷剧情线管理
//!
//! 通过 StorylineRepositoryPort 拉取；过滤属于应用层逻辑，保留在 service。
//! 不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::StorylineRepositoryPort;
use domain::storyline::{Storyline, StorylineStatus};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Storyline Service - 剧情线服务
pub struct StorylineService {
    repo: Arc<dyn StorylineRepositoryPort>,
}

impl StorylineService {
    pub fn new(repo: Arc<dyn StorylineRepositoryPort>) -> Self {
        Self { repo }
    }

    /// 获取所有活跃的剧情线
    pub async fn get_active_storylines(&self, project_id: Uuid) -> Result<Vec<Storyline>> {
        let all = self.repo.list_by_project(project_id).await?;
        Ok(all
            .into_iter()
            .filter(|s| s.status == StorylineStatus::Active)
            .collect())
    }

    /// 获取需要在 Volume 中推进的剧情线
    pub async fn get_storylines_for_volume(
        &self,
        project_id: Uuid,
        volume_id: Uuid,
    ) -> Result<Vec<Storyline>> {
        let active = self.get_active_storylines(project_id).await?;
        // 过滤出在该 Volume 中出现的剧情线
        Ok(active
            .into_iter()
            .filter(|s| s.created_volume_id.map(|v| v == volume_id).unwrap_or(true))
            .collect())
    }

    /// 列出项目全部剧情线（P7：替代 host 层原始 SQL）。
    pub async fn list_storylines(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_storylines(project_id).await
    }

    /// 创建剧情线（含可选挂载：parent_id 不为空则建挂载关系）
    pub async fn create_storyline(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: &str,
        tone: &str,
        visibility: &str,
        parent_id: Option<Uuid>,
    ) -> Result<Value> {
        // 业务规则：每项目 1 条 Main 主线（强约束，DB 不加 UNIQUE 保留灵活）
        if importance == "Main" {
            let existing = self
                .repo
                .list_storylines(project_id)
                .await?
                .into_iter()
                .any(|s| {
                    s.get("importance")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "Main")
                        .unwrap_or(false)
                });
            if existing {
                anyhow::bail!("该项目已存在 Main 主线，每项目只能有 1 条主线");
            }
        }
        self.repo
            .create_storyline(project_id, name, description, importance, tone, visibility, parent_id)
            .await
    }

    /// 更新剧情线（含 tone/visibility）
    pub async fn update_storyline(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
        tone: Option<&str>,
        visibility: Option<&str>,
    ) -> Result<Value> {
        self.repo
            .update_storyline(id, name, description, tone, visibility)
            .await
    }

    /// 删除剧情线（按 id）。
    pub async fn delete_storyline(&self, id: Uuid) -> Result<()> {
        self.repo.delete_storyline(id).await
    }

    /// 列出所有挂载关系
    pub async fn list_storyline_relations(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<Value>> {
        self.repo.list_storyline_relations(project_id).await
    }
}
