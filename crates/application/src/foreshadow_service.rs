//! Foreshadow Service - 伏笔管理的业务逻辑层。
//!
//! 通过 ForeshadowRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::ForeshadowRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Foreshadow Service - 伏笔服务
pub struct ForeshadowService {
    repo: Arc<dyn ForeshadowRepositoryPort>,
}

impl ForeshadowService {
    pub fn new(repo: Arc<dyn ForeshadowRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_foreshadows(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_foreshadows(project_id).await
    }

    /// 有界列表（目录页）：返回本页 + 总数。
    ///
    /// 过渡实现：repository 还没下推 LIMIT/OFFSET，先取回再切片——
    /// 目的是把返回给模型的体积有界化；等下推实现后替换。
    pub async fn list_foreshadows_page(
        &self,
        project_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Value>, usize)> {
        let all = self.repo.list_foreshadows(project_id).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }

    /// 创建伏笔。`status` / `importance` / `hint_level` 由工具层用
    /// `domain::foreshadowing` 的枚举解析器校验后传入（非法值在那里就报错）。
    pub async fn create_foreshadow(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        status: &str,
        importance: &str,
        hint_level: &str,
        hint_note: Option<&str>,
        introduced_at: Option<&str>,
        expected_reveal_at: Option<&str>,
        planted_node_id: Option<Uuid>,
        payoff_node_id: Option<Uuid>,
        parent_foreshadow_id: Option<Uuid>,
        storyline_id: Option<Uuid>,
    ) -> Result<Value> {
        self.repo
            .create_foreshadow(
                project_id,
                name,
                description,
                status,
                importance,
                hint_level,
                hint_note,
                introduced_at,
                expected_reveal_at,
                planted_node_id,
                payoff_node_id,
                parent_foreshadow_id,
                storyline_id,
            )
            .await
    }

    /// 修改伏笔。`storyline_id` 为 `None` 表示不动归属（见仓储端口说明：
    /// 用两层 Option 区分「不改」与「改成空」）；`name` / `description` 为
    /// `None` 同样表示不改——只挂线时不必把名字再抄一遍。
    pub async fn update_foreshadow(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        hint_note: Option<&str>,
        introduced_at: Option<&str>,
        expected_reveal_at: Option<&str>,
        actual_reveal_at: Option<&str>,
        planted_node_id: Option<Uuid>,
        payoff_node_id: Option<Uuid>,
        parent_foreshadow_id: Option<Uuid>,
        storyline_id: Option<Option<Uuid>>,
    ) -> Result<Value> {
        self.repo
            .update_foreshadow(
                id,
                name,
                description,
                status,
                hint_note,
                introduced_at,
                expected_reveal_at,
                actual_reveal_at,
                planted_node_id,
                payoff_node_id,
                parent_foreshadow_id,
                storyline_id,
            )
            .await
    }

    /// 删除伏笔（按 id）。
    pub async fn delete_foreshadow(&self, id: Uuid) -> Result<()> {
        self.repo.delete_foreshadow(id).await
    }
}
