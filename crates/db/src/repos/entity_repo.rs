//! Entity Repository - CRUD operations for Entity, EntityType, Relation, Fact

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Entity, EntityType, Fact, Relation};
use sqlx::PgPool;
use uuid::Uuid;


// ============= EntityTypeRepo =============

pub struct EntityTypeRepo {
    pool: PgPool,
}

impl EntityTypeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, description: Option<&str>) -> Result<EntityType> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO entity_type (id, name, description, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create entity type")?;

        Ok(EntityType {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            schema: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<EntityType>> {
        let row = sqlx::query_as::<_, EntityTypeRow>(
            "SELECT id, name, description, schema_json, created_at, updated_at FROM entity_type WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query entity type")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_all(&self) -> Result<Vec<EntityType>> {
        let rows = sqlx::query_as::<_, EntityTypeRow>(
            "SELECT id, name, description, schema_json, created_at, updated_at FROM entity_type ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query entity types")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn ensure(&self, name: &str, description: Option<&str>) -> Result<EntityType> {
        if let Some(existing) = self.get_by_name(name).await? {
            return Ok(existing);
        }
        self.create(name, description).await
    }

    /// Transactional ensure: atomically insert-or-return using ON CONFLICT.
    /// Safe for concurrent callers -- no check-then-insert race.
    pub async fn ensure_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        name: &str,
        description: Option<&str>,
    ) -> Result<EntityType> {
        let row = sqlx::query_as::<_, EntityTypeRow>(
            "INSERT INTO entity_type (id, name, description, created_at, updated_at)
             VALUES (gen_random_uuid(), $1, $2, NOW(), NOW())
             ON CONFLICT (name) DO UPDATE SET updated_at = NOW()
             RETURNING id, name, description, schema_json, created_at, updated_at",
        )
        .bind(name)
        .bind(description.unwrap_or(""))
        .fetch_one(executor)
        .await
        .context("Failed to ensure entity type")?;

        Ok(row.into())
    }
}

#[derive(sqlx::FromRow)]
struct EntityTypeRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    schema_json: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EntityTypeRow> for EntityType {
    fn from(r: EntityTypeRow) -> Self {
        EntityType {
            id: r.id,
            name: r.name,
            description: r.description,
            schema: r.schema_json,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= EntityRepo =============

pub struct EntityRepo {
    pool: PgPool,
}

impl EntityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        entity_type_id: Uuid,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Entity> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'system', $9, $10)",
        )
        .bind(id)
        .bind(project_id)
        .bind(world_id)
        .bind(entity_type_id)
        .bind(name)
        .bind(summary.unwrap_or(""))
        .bind(description.unwrap_or(""))
        .bind(&attributes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create entity")?;

        Ok(Entity {
            id,
            project_id,
            world_id,
            entity_type_id,
            name: name.to_string(),
            summary: summary.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            attributes,
            version: 1,
            created_by: "system".to_string(),
            updated_by: None,
            source_generation_id: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get entity by ID without project scope.
    ///
    /// **WARNING**: This method does NOT enforce project isolation.
    /// For production code, use get_by_id_with_project instead.
    /// This method should only be used for:
    /// - Global entity lookups (e.g., entity types)
    /// - Testing
    /// - Admin operations
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Entity>> {
        tracing::warn!(
            "EntityRepo::get_by_id called without project scope.              Consider using get_by_id_with_project for project isolation.",
        );
        let row = sqlx::query_as::<_, EntityRow>(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, \
             attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at \
             FROM entity WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query entity")?;

        Ok(row.map(|r| r.into()))
    }

    /// Project-scoped entity query. Ensures entity belongs to the specified project.
    ///
    /// Use this method instead of get_by_id when you need project isolation.
    /// Returns None if entity doesn't exist OR doesn't belong to the project.
    pub async fn get_by_id_with_project(&self, project_id: Uuid, id: Uuid) -> Result<Option<Entity>> {
        let row = sqlx::query_as::<_, EntityRow>(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description,              attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at              FROM entity WHERE id = $1 AND project_id = $2 AND status != 'Deleted'",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query entity")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Entity>> {
        let rows = sqlx::query_as::<_, EntityRow>(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, \
             attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at \
             FROM entity WHERE project_id = $1 AND status != 'Deleted' ORDER BY name",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query entities")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_by_type(&self, project_id: Uuid, entity_type_id: Uuid) -> Result<Vec<Entity>> {
        let rows = sqlx::query_as::<_, EntityRow>(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, \
             attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at \
             FROM entity WHERE project_id = $1 AND entity_type_id = $2 AND status != 'Deleted' ORDER BY name",
        )
        .bind(project_id)
        .bind(entity_type_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query entities")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 批量获取 entities，避免 N+1 query。
    pub async fn list_by_ids(&self, project_id: Uuid, ids: &[Uuid]) -> Result<Vec<Entity>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, EntityRow>(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, \
             attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at \
             FROM entity WHERE project_id = $1 AND id = ANY($2) ORDER BY name",
        )
        .bind(project_id)
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .context("Failed to batch query entities")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(&self, entity: &Entity) -> Result<()> {
        let result = sqlx::query(
            "UPDATE entity SET name = $1, summary = $2, attributes = $3, \
             version = version + 1, updated_at = NOW() \
             WHERE id = $4 AND project_id = $5 AND version = $6",
        )
        .bind(&entity.name)
        .bind(&entity.summary)
        .bind(&entity.attributes)
        .bind(entity.id)
        .bind(entity.project_id)
        .bind(entity.version)
        .execute(&self.pool)
        .await
        .context("Failed to update entity")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Concurrent modification detected for entity {} (expected version {})",
                entity.id, entity.version
            ));
        }
        Ok(())
    }

    pub async fn delete(&self, project_id: Uuid, id: Uuid, expected_version: i32) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE entity SET status = 'Deleted', version = version + 1, updated_at = NOW() \
             WHERE id = $1 AND project_id = $2 AND version = $3 AND status = 'Active'",
        )
        .bind(id)
        .bind(project_id)
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .context("Failed to soft-delete entity")?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================================
    // Transactional methods (accept any Executor -- pool or &mut Tx)
    // ============================================================

    /// Transactional create entity.
    pub async fn create_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        world_id: Uuid,
        entity_type_id: Uuid,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Entity> {
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO entity (id, project_id, world_id, entity_type_id, name, summary, description, attributes, version, created_by, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'system', NOW(), NOW())",
        )
        .bind(id)
        .bind(project_id)
        .bind(world_id)
        .bind(entity_type_id)
        .bind(name)
        .bind(summary.unwrap_or(""))
        .bind(description.unwrap_or(""))
        .bind(&attributes)
        .execute(executor)
        .await
        .context("Failed to create entity")?;

        Ok(Entity {
            id,
            project_id,
            world_id,
            entity_type_id,
            name: name.to_string(),
            summary: summary.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            attributes,
            version: 1,
            created_by: "system".to_string(),
            updated_by: None,
            source_generation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Transactional project-scoped get.
    pub async fn get_by_id_with_project_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Entity>> {
        let row = sqlx::query_as::<_, EntityRow>(
            "SELECT id, project_id, world_id, entity_type_id, name, summary, description, \
             attributes, version, created_by, updated_by, source_generation_id, created_at, updated_at \
             FROM entity WHERE id = $1 AND project_id = $2 AND status != 'Deleted'",
        )
        .bind(id)
        .bind(project_id)
        .fetch_optional(executor)
        .await
        .context("Failed to query entity")?;

        Ok(row.map(|r| r.into()))
    }

    /// Transactional update with version CAS.
    ///
    /// Returns the number of rows affected. If 0, a ConcurrentModification occurred.
    pub async fn update_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        entity: &Entity,
    ) -> Result<usize> {
        let result = sqlx::query(
            "UPDATE entity SET name = $1, summary = $2, attributes = $3, \
             version = version + 1, updated_at = NOW() \
             WHERE id = $4 AND project_id = $5 AND version = $6",
        )
        .bind(&entity.name)
        .bind(&entity.summary)
        .bind(&entity.attributes)
        .bind(entity.id)
        .bind(entity.project_id)
        .bind(entity.version)
        .execute(executor)
        .await
        .context("Failed to update entity")?;

        Ok(result.rows_affected() as usize)
    }

    /// Transactional soft delete with project isolation.
    pub async fn delete_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        id: Uuid,
        expected_version: i32,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE entity SET status = 'Deleted', version = version + 1, updated_at = NOW() \
             WHERE id = $1 AND project_id = $2 AND version = $3 AND status = 'Active'",
        )
        .bind(id)
        .bind(project_id)
        .bind(expected_version)
        .execute(executor)
        .await
        .context("Failed to soft-delete entity")?;

        Ok(result.rows_affected() > 0)
    }
}

#[derive(sqlx::FromRow)]
struct EntityRow {
    id: Uuid,
    project_id: Uuid,
    world_id: Uuid,
    entity_type_id: Uuid,
    name: String,
    summary: Option<String>,
    description: Option<String>,
    attributes: Option<serde_json::Value>,
    version: i32,
    created_by: String,
    updated_by: Option<String>,
    source_generation_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EntityRow> for Entity {
    fn from(r: EntityRow) -> Self {
        Entity {
            id: r.id,
            project_id: r.project_id,
            world_id: r.world_id,
            entity_type_id: r.entity_type_id,
            name: r.name,
            summary: r.summary,
            description: r.description,
            attributes: r.attributes.unwrap_or_default(),
            version: r.version,
            created_by: r.created_by,
            updated_by: r.updated_by,
            source_generation_id: r.source_generation_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= RelationRepo =============

pub struct RelationRepo {
    pool: PgPool,
}

impl RelationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Relation> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO relation (id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(project_id)
        .bind(source_entity_id)
        .bind(target_entity_id)
        .bind(relation_type)
        .bind(description.unwrap_or(""))
        .bind(&attributes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create relation")?;

        Ok(Relation {
            id,
            project_id,
            source_entity_id,
            target_entity_id,
            relation_type: relation_type.to_string(),
            description: description.map(|s| s.to_string()),
            attributes,
            valid_from: None,
            valid_until: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_by_entity(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<Relation>> {
        let rows = sqlx::query_as::<_, RelationRow>(
            "SELECT id, project_id, source_entity_id, target_entity_id, relation_type, description, \
             attributes, valid_from, valid_until, created_at, updated_at \
             FROM relation WHERE project_id = $1 AND (source_entity_id = $2 OR target_entity_id = $2) ORDER BY created_at",
        )
        .bind(project_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query relations")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn delete(&self, project_id: Uuid, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM relation WHERE id = $1 AND project_id = $2")
            .bind(id)
            .bind(project_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete relation")?;
        Ok(result.rows_affected() > 0)
    }

    // ============================================================
    // Transactional methods
    // ============================================================

    /// Transactional delete relation with project isolation.
    pub async fn delete_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        id: Uuid,
    ) -> Result<bool> {
        let result = sqlx::query("DELETE FROM relation WHERE id = $1 AND project_id = $2")
            .bind(id)
            .bind(project_id)
            .execute(executor)
            .await
            .context("Failed to delete relation")?;
        Ok(result.rows_affected() > 0)
    }

    /// Transactional create relation.
    pub async fn create_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Relation> {
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO relation (id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
        )
        .bind(id)
        .bind(project_id)
        .bind(source_entity_id)
        .bind(target_entity_id)
        .bind(relation_type)
        .bind(description.unwrap_or(""))
        .bind(&attributes)
        .execute(executor)
        .await
        .context("Failed to create relation")?;

        Ok(Relation {
            id,
            project_id,
            source_entity_id,
            target_entity_id,
            relation_type: relation_type.to_string(),
            description: description.map(|s| s.to_string()),
            attributes,
            valid_from: None,
            valid_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Transactional project-scoped list by entity.
    pub async fn list_by_entity_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<Relation>> {
        let rows = sqlx::query_as::<_, RelationRow>(
            "SELECT id, project_id, source_entity_id, target_entity_id, relation_type, description, \
             attributes, valid_from, valid_until, created_at, updated_at \
             FROM relation WHERE project_id = $1 AND (source_entity_id = $2 OR target_entity_id = $2) ORDER BY created_at",
        )
        .bind(project_id)
        .bind(entity_id)
        .fetch_all(executor)
        .await
        .context("Failed to query relations")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct RelationRow {
    id: Uuid,
    project_id: Uuid,
    source_entity_id: Uuid,
    target_entity_id: Uuid,
    relation_type: String,
    description: Option<String>,
    attributes: Option<serde_json::Value>,
    valid_from: Option<String>,
    valid_until: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<RelationRow> for Relation {
    fn from(r: RelationRow) -> Self {
        Relation {
            id: r.id,
            project_id: r.project_id,
            source_entity_id: r.source_entity_id,
            target_entity_id: r.target_entity_id,
            relation_type: r.relation_type,
            description: r.description,
            attributes: r.attributes.unwrap_or_default(),
            valid_from: r.valid_from,
            valid_until: r.valid_until,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= FactRepo =============

pub struct FactRepo {
    pool: PgPool,
}

impl FactRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO fact (id, project_id, content, category, certainty, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(content)
        .bind(category.unwrap_or(""))
        .bind(certainty)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create fact")?;

        for entity_id in related_entity_ids {
            sqlx::query("INSERT INTO fact_entity (id, fact_id, entity_id) VALUES ($1, $2, $3)")
                .bind(Uuid::new_v4())
                .bind(id)
                .bind(entity_id)
                .execute(&self.pool)
                .await
                .context("Failed to create fact_entity")?;
        }

        Ok(Fact {
            id,
            project_id,
            content: content.to_string(),
            category: category.map(|s| s.to_string()),
            certainty: domain::canon::FactCertainty::from_str(certainty),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Fact>> {
        let row = sqlx::query_as::<_, FactRow>(
            "SELECT id, project_id, content, category, COALESCE(certainty, 'CANON') as certainty, created_at, updated_at \
             FROM fact WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query fact")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Fact>> {
        let rows = sqlx::query_as::<_, FactRow>(
            "SELECT id, project_id, content, category, COALESCE(certainty, 'CANON') as certainty, created_at, updated_at \
             FROM fact WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query facts")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM fact_entity WHERE fact_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete fact_entity")?;
        sqlx::query("DELETE FROM fact WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete fact")?;
        Ok(())
    }

    /// Transactional create fact with all fact_entity associations atomically.
    pub async fn create_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres> + Copy,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact> {
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO fact (id, project_id, content, category, certainty, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
        )
        .bind(id)
        .bind(project_id)
        .bind(content)
        .bind(category.unwrap_or(""))
        .bind(certainty)
        .execute(executor)
        .await
        .context("Failed to create fact")?;

        for entity_id in related_entity_ids {
            sqlx::query("INSERT INTO fact_entity (id, fact_id, entity_id) VALUES (gen_random_uuid(), $1, $2)")
                .bind(id)
                .bind(entity_id)
                .execute(executor)
                .await
                .context("Failed to create fact_entity")?;
        }

        Ok(Fact {
            id,
            project_id,
            content: content.to_string(),
            category: category.map(|s| s.to_string()),
            certainty: domain::canon::FactCertainty::from_str(certainty),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct FactRow {
    id: Uuid,
    project_id: Uuid,
    content: String,
    category: Option<String>,
    certainty: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FactRow> for Fact {
    fn from(r: FactRow) -> Self {
        Fact {
            id: r.id,
            project_id: r.project_id,
            content: r.content,
            category: r.category,
            certainty: domain::canon::FactCertainty::from_str(&r.certainty),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}