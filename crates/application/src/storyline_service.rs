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

    /// 有界列表（目录页）：返回本页 + 总数。（剧情线）
    ///
    /// 过渡实现：repository 还没下推 LIMIT/OFFSET，先取回再切片——
    /// 目的是**把返回给模型的体积有界化**（实测无界列表一次能到 42 万字符）；
    /// 等下推实现后就替换成真正的分页查询。
    pub async fn list_storylines_page(
        &self,
        project_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<serde_json::Value>, usize)> {
        let all = self.list_storylines(project_id).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }


    /// 创建剧情线（含可选挂载：parent_id 不为空则建挂载关系）。
    ///
    /// `status` 由调用方给出（已由工具层用 `StorylineStatus::parse` 校验过），
    /// 因此创建时就能直接落成「进行中」等状态，而不是一律停在 Planned。
    pub async fn create_storyline(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        status: &str,
        importance: &str,
        tone: &str,
        visibility: &str,
        parent_id: Option<Uuid>,
        arc_stages: Option<&Value>,
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
            .create_storyline(
                project_id,
                name,
                description,
                status,
                importance,
                tone,
                visibility,
                parent_id,
                arc_stages,
            )
            .await
    }

    /// 更新剧情线（含 tone/visibility）
    /// 更新剧情线。`status` 为 `None` 表示保持原状态。
    pub async fn update_storyline(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        tone: Option<&str>,
        visibility: Option<&str>,
        arc_stages: Option<&Value>,
    ) -> Result<Value> {
        self.repo
            .update_storyline(id, name, description, status, tone, visibility, arc_stages)
            .await
    }

    /// 按 id 读取单条剧情线（revise 的 arc_stages 合并需要旧值）。
    pub async fn get_storyline(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_storyline(id).await
    }

    /// 删除剧情线（按 id）。
    pub async fn delete_storyline(&self, id: Uuid) -> Result<()> {
        self.repo.delete_storyline(id).await
    }

    /// 列出所有剧情线关系（含 relation_type）
    pub async fn list_storyline_relations(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<Value>> {
        self.repo.list_storyline_relations(project_id).await
    }

    /// 建立（或更新）一条剧情线关系：驱动 / 依赖 / 交汇 / 对冲 / 包含。
    pub async fn relate_storylines(
        &self,
        project_id: Uuid,
        from_storyline_id: Uuid,
        to_storyline_id: Uuid,
        relation_type: &str,
    ) -> Result<Value> {
        self.repo
            .relate_storylines(project_id, from_storyline_id, to_storyline_id, relation_type)
            .await
    }

    /// 删除一条剧情线关系。
    pub async fn unrelate_storylines(&self, id: Uuid) -> Result<()> {
        self.repo.unrelate_storylines(id).await
    }
}
