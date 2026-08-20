//! ProjectResolverPort 的 PostgreSQL 实现。仅做低层 project_id 解析查询。

use anyhow::{Context, Result};
use async_trait::async_trait;
use domain::ports::ProjectResolverPort;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbProjectResolverPort {
    pool: PgPool,
}

impl DbProjectResolverPort {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectResolverPort for DbProjectResolverPort {
    async fn project_id_for_entity(&self, entity_id: Uuid) -> Result<Option<Uuid>> {
        let pid: Option<Uuid> = sqlx::query_scalar("SELECT project_id FROM entity WHERE id = $1")
            .bind(entity_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to resolve project_id for entity")?;
        Ok(pid)
    }

    async fn project_id_for_world(&self, world_id: Uuid) -> Result<Option<Uuid>> {
        let pid: Option<Uuid> = sqlx::query_scalar("SELECT project_id FROM world WHERE id = $1")
            .bind(world_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to resolve project_id for world")?;
        Ok(pid)
    }

    async fn project_id_for_relation(&self, relation_id: Uuid) -> Result<Option<Uuid>> {
        let pid: Option<Uuid> = sqlx::query_scalar("SELECT project_id FROM relation WHERE id = $1")
            .bind(relation_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to resolve project_id for relation")?;
        Ok(pid)
    }

    async fn project_id_for_narrative_node(&self, node_id: Uuid) -> Result<Option<Uuid>> {
        let pid: Option<Uuid> =
            sqlx::query_scalar("SELECT project_id FROM narrative_node WHERE id = $1")
                .bind(node_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to resolve project_id for narrative node")?;
        Ok(pid)
    }
}
