//! 应用层 commit 契约测试（对应 docs/contracts/commit.md）
//!
//! 锁定 application::mutation::MutationCommitter（路径二：MutationCommand DSL +
//! DbMutationCommitter）的不变式：
//!   - 每次通过 affected_worlds 显式声明的世界，commit 必须推进其 world_version
//!   - command_id 幂等键：重复提交同一 command_id 不得重复创建实体
//!     （mutation_ledger 幂等护栏）
//!
//! 需要 PostgreSQL 环境：设置 DATABASE_URL。

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use application::mutation::MutationCommitter;
use db::connection::Database;
use db::migration;
use db::mutation_committer::DbMutationCommitter;
use domain::mutation::{MutationCommand, MutationPayload, MutationSource, MutationTargetType};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string()
    });
    let database = Database::open(&url).await.expect("open database");
    let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../db/migrations");
    migration::run_migrations(database.pool(), migrations_dir)
        .await
        .expect("run migrations");
    database.pool().clone()
}

async fn setup(pool: &PgPool) -> (Uuid, Uuid) {
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO project (id, name, created_at, updated_at) VALUES ($1, 'p', NOW(), NOW())")
        .bind(project_id)
        .execute(pool)
        .await
        .unwrap();
    let world_id = Uuid::new_v4();
    sqlx::query("INSERT INTO world (id, project_id, name, is_main, created_at, updated_at) VALUES ($1, $2, 'w', TRUE, NOW(), NOW())")
        .bind(world_id)
        .bind(project_id)
        .execute(pool)
        .await
        .unwrap();
    (project_id, world_id)
}

fn create_entity_cmd(project_id: Uuid, world_id: Uuid, command_id: Uuid, name: &str) -> MutationCommand {
    MutationCommand {
        command_id,
        project_id,
        target: Uuid::new_v4(),
        target_type: MutationTargetType::Entity,
        expected_version: None,
        source: MutationSource::AI,
        payload: MutationPayload::CreateEntity {
            world_id,
            entity_type: "Character".to_string(),
            name: name.to_string(),
            summary: None,
            description: None,
            attributes: serde_json::json!({}),
        },
    }
}

// 不变量：每次 commit 必须为 affected_worlds 中的每个世界推进 world_version
#[tokio::test]
async fn app_commit_advances_world_version_per_affected_world() {
    let pool = pool().await;
    let (project_id, world_id) = setup(&pool).await;
    let committer = MutationCommitter::new(Arc::new(DbMutationCommitter::new(pool.clone())));

    let r1 = committer
        .commit_with_worlds(create_entity_cmd(project_id, world_id, Uuid::new_v4(), "Hero"), vec![world_id])
        .await
        .expect("first commit");
    assert_eq!(r1.len(), 1);

    let entities = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entity WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(entities, 1);
    let wv1 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM world_version WHERE world_id = $1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(wv1, 1, "首个 commit 必须推进 world_version");
    let v1 = sqlx::query_scalar::<_, i32>("SELECT version FROM world_version WHERE world_id = $1 ORDER BY version DESC LIMIT 1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(v1, 1);

    let r2 = committer
        .commit_with_worlds(create_entity_cmd(project_id, world_id, Uuid::new_v4(), "Villain"), vec![world_id])
        .await
        .expect("second commit");
    assert_eq!(r2.len(), 1);

    let entities2 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entity WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(entities2, 2);
    let wv2 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM world_version WHERE world_id = $1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(wv2, 2, "第二个 commit 必须再推进 world_version");
    let v2 = sqlx::query_scalar::<_, i32>("SELECT version FROM world_version WHERE world_id = $1 ORDER BY version DESC LIMIT 1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(v2, 2);
}

// 不变量：command_id 幂等键 —— 同一 command_id 重复提交不得重复创建实体
#[tokio::test]
async fn app_command_idempotency_no_duplicate_entity() {
    let pool = pool().await;
    let (project_id, world_id) = setup(&pool).await;
    let committer = MutationCommitter::new(Arc::new(DbMutationCommitter::new(pool.clone())));
    let cmd = create_entity_cmd(project_id, world_id, Uuid::new_v4(), "Hero");

    let _ = committer
        .commit_with_worlds(cmd.clone(), vec![world_id])
        .await
        .expect("first");
    let _ = committer
        .commit_with_worlds(cmd, vec![world_id])
        .await
        .expect("second (idempotent)");

    let entities = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entity WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(entities, 1, "相同 command_id 重复提交不得重复创建实体（mutation_ledger 幂等）");
}
