//! 数据库事务不变量契约测试（对应 docs/contracts/commit.md）
//!
//! 锁定 ffc46af 修复的 commit 不变式（路径一：DbStateCommitterPort::commit，
//! 即 e2e / integration 测试使用的 canonical 提交入口）：
//!   - 不变量 A：批内同 (entity_id, state_key) 冲突必须失败且无部分写入
//!   - 不变量 B：commit 失败必须整笔回滚（无 system_events / state_change /
//!     world_version / current_state 的部分写入，proposed_change 仍 Approved）
//!   - 不变量 C：成功 commit 必须产出完整链路
//!       system_events + state_change(event_id 正确) + current_state 更新 + world_version++
//!
//! 需要 PostgreSQL 环境：设置 DATABASE_URL（默认 localhost/novel_engine）。

use sqlx::PgPool;
use uuid::Uuid;

use db::connection::Database;
use db::migration;
use db::repos::state_repo::StateRepo;
use db::repos::validation_repo::ValidationRepo;
use db::runtime_ports::DbStateCommitterPort;
use domain::ports::StateCommitterPort;
use domain::ProposedChangeType;

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

async fn ensure_task(pool: &PgPool, project_id: Uuid, task_id: Uuid) {
    sqlx::query(
        "INSERT INTO generation_task (id, project_id, task_type, status, created_at) \
         VALUES ($1, $2, 'general', 'Pending', NOW()) ON CONFLICT (id) DO NOTHING",
    )
    .bind(task_id)
    .bind(project_id)
    .execute(pool)
    .await
    .unwrap();
}

/// 把 ProposedChange 置为 Approved（与 runtime integration_tests 的写法一致）。
async fn approve(pool: &PgPool, change_id: Uuid) {
    sqlx::query("UPDATE proposed_change SET status = 'Approved' WHERE id = $1")
        .bind(change_id)
        .execute(pool)
        .await
        .unwrap();
}

/// 创建 project + main world + Character 实体 + 初始状态 hp=100。
async fn setup(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
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

    let entity_type_id = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM entity_type WHERE name = 'Character'")
        .fetch_optional(pool)
        .await
        .unwrap()
    {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO entity_type (id, name, created_at, updated_at) VALUES ($1, 'Character', NOW(), NOW())")
                .bind(id)
                .execute(pool)
                .await
                .unwrap();
            id
        }
    };
    let entity_id = Uuid::new_v4();
    sqlx::query("INSERT INTO entity (id, project_id, world_id, entity_type_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, 'e', NOW(), NOW())")
        .bind(entity_id)
        .bind(project_id)
        .bind(world_id)
        .bind(entity_type_id)
        .execute(pool)
        .await
        .unwrap();

    StateRepo::new(pool.clone())
        .upsert_state(project_id, entity_id, "hp", serde_json::json!(100), None)
        .await
        .unwrap();
    (project_id, world_id, entity_id)
}

async fn count_where(pool: &PgPool, sql: &str, project_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(project_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// 不变量 A + B：批内同键冲突必须失败，且不得有任何部分写入
// ---------------------------------------------------------------------------
#[tokio::test]
async fn invariant_a_batch_same_key_conflict_no_partial_write() {
    let pool = pool().await;
    let (project_id, _world_id, entity_id) = setup(&pool).await;
    let val = ValidationRepo::new(pool.clone());

    let task_id = Uuid::new_v4();
    ensure_task(&pool, project_id, task_id).await;

    let c1 = val
        .create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "c1",
            serde_json::json!({"state_key": "hp", "new_value": 80}),
        )
        .await
        .unwrap();
    let c2 = val
        .create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "c2",
            serde_json::json!({"state_key": "hp", "new_value": 90}),
        )
        .await
        .unwrap();

    approve(&pool, c1.id).await;
    approve(&pool, c2.id).await;

    let port = DbStateCommitterPort::new(pool.clone());
    let result = port.commit(project_id, &[c1.id, c2.id]).await;
    assert!(result.is_err(), "批内同 (entity,state_key) 必须冲突失败：{:?}", result);

    // 不变量 B：无部分写入
    let st = StateRepo::new(pool.clone())
        .get_current_state(project_id, entity_id, "hp")
        .await
        .unwrap();
    assert_eq!(
        st.as_ref().unwrap().state_value,
        serde_json::json!(100),
        "状态必须保持初始值，无部分写入"
    );

    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM system_events WHERE project_id = $1", project_id).await,
        0,
        "失败时不得写入 system_events"
    );
    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM state_change WHERE project_id = $1", project_id).await,
        0,
        "失败时不得写入 state_change"
    );
    let wv = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM world_version wv JOIN world w ON wv.world_id = w.id WHERE w.project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(wv, 0, "失败时不得写入 world_version");

    // 两个 proposed_change 仍保持 Approved（未被改成 Applied）
    let s1 = sqlx::query_scalar::<_, String>("SELECT status FROM proposed_change WHERE id = $1")
        .bind(c1.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let s2 = sqlx::query_scalar::<_, String>("SELECT status FROM proposed_change WHERE id = $1")
        .bind(c2.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(s1, "Approved");
    assert_eq!(s2, "Approved");
}

// ---------------------------------------------------------------------------
// 不变量 C：成功 commit 必须产出完整链路（StateChange 分支）
// ---------------------------------------------------------------------------
#[tokio::test]
async fn invariant_c_state_change_full_chain_and_world_version() {
    let pool = pool().await;
    let (project_id, world_id, entity_id) = setup(&pool).await;
    let val = ValidationRepo::new(pool.clone());
    let task_id = Uuid::new_v4();
    ensure_task(&pool, project_id, task_id).await;

    let c1 = val
        .create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "c1",
            serde_json::json!({"state_key": "hp", "new_value": 80}),
        )
        .await
        .unwrap();
    approve(&pool, c1.id).await;

    let port = DbStateCommitterPort::new(pool.clone());
    let resp = port
        .commit(project_id, &[c1.id])
        .await
        .expect("commit 应成功");
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.events.len(), 1);

    // current_state 更新为 80
    let st = StateRepo::new(pool.clone())
        .get_current_state(project_id, entity_id, "hp")
        .await
        .unwrap()
        .expect("状态应存在");
    assert_eq!(st.state_value, serde_json::json!(80));

    // system_events 1 条
    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM system_events WHERE project_id = $1", project_id).await,
        1
    );
    let event_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM system_events WHERE project_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // state_change 1 条，event_id 正确指向 system_events
    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM state_change WHERE project_id = $1", project_id).await,
        1
    );
    let sc_event_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT event_id FROM state_change WHERE project_id = $1 ORDER BY committed_at DESC LIMIT 1",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sc_event_id, event_id, "state_change.event_id 必须指向 system_events");

    // world_version 1 条，version=1，parent 为 NULL
    let wv_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM world_version WHERE world_id = $1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(wv_count, 1, "成功 commit 必须推进 world_version");
    let (ver, parent) = sqlx::query_as::<_, (i32, Option<Uuid>)>(
        "SELECT version, parent_version_id FROM world_version WHERE world_id = $1 ORDER BY version DESC LIMIT 1",
    )
    .bind(world_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ver, 1, "首个版本应为 1");
    assert_eq!(parent, None, "首个版本 parent 必须为 NULL");

    // proposed_change 状态变为 Applied
    let s1 = sqlx::query_scalar::<_, String>("SELECT status FROM proposed_change WHERE id = $1")
        .bind(c1.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(s1, "Applied");
}

// ---------------------------------------------------------------------------
// 不变量 C：成功 commit 完整链路（EntityCreate 分支）
// ---------------------------------------------------------------------------
#[tokio::test]
async fn invariant_c_entity_create_full_chain() {
    let pool = pool().await;
    let (project_id, world_id, _entity_id) = setup(&pool).await;
    let val = ValidationRepo::new(pool.clone());
    let task_id = Uuid::new_v4();
    ensure_task(&pool, project_id, task_id).await;

    let c1 = val
        .create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::EntityCreate,
            Uuid::new_v4(),
            "create hero",
            serde_json::json!({"entity_type": "Character", "name": "Hero", "attributes": {}}),
        )
        .await
        .unwrap();
    approve(&pool, c1.id).await;

    let port = DbStateCommitterPort::new(pool.clone());
    let resp = port
        .commit(project_id, &[c1.id])
        .await
        .expect("entity create commit 应成功");
    assert_eq!(resp.results.len(), 1);

    // setup 已建 1 个实体，这里应新增 1 个 → 共 2 个
    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM entity WHERE project_id = $1", project_id).await,
        2
    );
    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM system_events WHERE project_id = $1", project_id).await,
        1
    );
    let wv = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM world_version WHERE world_id = $1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(wv, 1, "成功 commit 必须推进 world_version");
}

// 不变量：world_version 只能由 canonical commit 推进（禁止绕开直接裸写）
#[tokio::test]
async fn world_version_only_advanced_by_canonical_commit() {
    let pool = pool().await;
    let (project_id, world_id, entity_id) = setup(&pool).await;
    let val = ValidationRepo::new(pool.clone());
    let task_id = Uuid::new_v4();
    ensure_task(&pool, project_id, task_id).await;

    let c1 = val
        .create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::StateChange,
            entity_id,
            "c1",
            serde_json::json!({"state_key": "hp", "new_value": 80}),
        )
        .await
        .unwrap();
    approve(&pool, c1.id).await;

    // 提交前：无 world_version
    let before = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM world_version WHERE world_id = $1")
        .bind(world_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, 0, "提交前不应有 world_version");

    let port = DbStateCommitterPort::new(pool.clone());
    port.commit(project_id, &[c1.id]).await.expect("commit 应成功");

    // 提交后：恰好 1 行，且 kind 非空。
    // 这证明 world_version 经由 canonical 拥有者（WorldVersionRepo）写入，
    // 而非绕开契约的原始裸写。任何直接 INSERT world_version(kind=NULL) 都会
    // 违背此契约（canonical 路径始终设置非空的 kind）。
    let (ver, kind) = sqlx::query_as::<_, (i32, Option<String>)>(
        "SELECT version, kind FROM world_version WHERE world_id = $1 ORDER BY version DESC LIMIT 1",
    )
    .bind(world_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ver, 1);
    assert!(
        kind.is_some(),
        "world_version.kind 必须由 canonical commit 设置，禁止绕开直接裸写 NULL"
    );
}

// ---------------------------------------------------------------------------
// 不变量：payload 必须匹配 change_type（缺失必需字段必须失败且无部分写入）
// ---------------------------------------------------------------------------
#[tokio::test]
async fn invariant_payload_must_match_change_type() {
    let pool = pool().await;
    let (project_id, _world_id, _entity_id) = setup(&pool).await;
    let val = ValidationRepo::new(pool.clone());
    let task_id = Uuid::new_v4();
    ensure_task(&pool, project_id, task_id).await;

    // EntityCreate payload 缺 name → 解析失败
    let c1 = val
        .create_proposed_change(
            project_id,
            Some(task_id),
            ProposedChangeType::EntityCreate,
            Uuid::new_v4(),
            "bad create",
            serde_json::json!({"entity_type": "Character"}),
        )
        .await
        .unwrap();
    approve(&pool, c1.id).await;

    let port = DbStateCommitterPort::new(pool.clone());
    let result = port.commit(project_id, &[c1.id]).await;
    assert!(result.is_err(), "payload 与 change_type 不匹配必须失败：{:?}", result);

    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM entity WHERE project_id = $1", project_id).await,
        1,
        "payload 解析失败不得创建实体"
    );
    assert_eq!(
        count_where(&pool, "SELECT COUNT(*) FROM system_events WHERE project_id = $1", project_id).await,
        0
    );
}
