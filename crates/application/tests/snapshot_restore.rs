//! SnapshotService::restore_snapshot 契约测试。
//!
//! 锁定恢复语义：快照的宏观状态（story_time / world_summary / main_character_state /
//! current_location / state_data）幂等回写到 narrative_state（World 维度），
//! 不存在则插入、已存在则更新，且不删除任何既有数据。
//!
//! 需要 PostgreSQL 环境：设置 DATABASE_URL（默认 localhost/novel_engine）。

use sqlx::PgPool;
use uuid::Uuid;

use application::snapshot_service::SnapshotService;
use db::application_ports::{DbNarrativeStateWritePort, DbSnapshotRepositoryPort};
use domain::narrative::StateDimension;
use domain::ports::NarrativeStateWritePort;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://novel:novel_pass@localhost:5432/novel_engine".to_string()
    });
    sqlx::PgPool::connect(&url).await.expect("connect database")
}

fn service(pool: PgPool) -> SnapshotService {
    SnapshotService::new(
        std::sync::Arc::new(DbSnapshotRepositoryPort::new(pool.clone())),
        std::sync::Arc::new(DbNarrativeStateWritePort::new(pool)),
    )
}

async fn read_world_state(pool: &PgPool, project_id: Uuid, key: &str) -> Option<serde_json::Value> {
    sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT state_value FROM narrative_state \
         WHERE project_id = $1 AND state_dimension = $2 AND state_key = $3",
    )
    .bind(project_id)
    .bind(StateDimension::World.as_str())
    .bind(key)
    .fetch_optional(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn restore_writes_macro_state_into_narrative_state() {
    let pool = pool().await;

    // 独立项目，避免污染共享库
    let project_id = Uuid::new_v4();
    sqlx::query("INSERT INTO project (id, name, status) VALUES ($1, 'restore-test', 'active')")
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();

    // 预置一个将被覆盖的旧值，验证「已存在则更新」分支
    let writer: std::sync::Arc<dyn NarrativeStateWritePort> =
        std::sync::Arc::new(DbNarrativeStateWritePort::new(pool.clone()));
    writer
        .upsert_state(
            project_id,
            StateDimension::World,
            "story_time",
            serde_json::json!("旧时间线"),
        )
        .await
        .unwrap();

    // 创建快照（走 create_snapshot 后直接补齐 find_snapshot 所需字段）
    let svc = service(pool.clone());
    let created = svc
        .create_snapshot(project_id, Some("恢复测试快照"), Some("天玄历381年"), Some("大陆一统"))
        .await
        .unwrap();
    let snapshot_id: Uuid = created["id"].as_str().unwrap().parse().unwrap();
    sqlx::query(
        "UPDATE novel_state_snapshot SET main_character_state = $1, current_location = $2, \
         state_data = $3 WHERE id = $4",
    )
    .bind("林惊羽 · 金丹初期")
    .bind("青云山")
    .bind(serde_json::json!({"progress": "第3卷", "custom": 42}))
    .bind(snapshot_id)
    .execute(&pool)
    .await
    .unwrap();

    // 执行恢复
    let result = svc.restore_snapshot(snapshot_id).await.unwrap();
    assert_eq!(result["restored"], serde_json::json!(true));
    assert_eq!(result["project_id"], serde_json::json!(project_id.to_string()));

    // 断言：五个键全部落库，且值正确（story_time 覆盖了旧值）
    assert_eq!(
        read_world_state(&pool, project_id, "story_time").await,
        Some(serde_json::json!("天玄历381年"))
    );
    assert_eq!(
        read_world_state(&pool, project_id, "world_summary").await,
        Some(serde_json::json!("大陆一统"))
    );
    assert_eq!(
        read_world_state(&pool, project_id, "main_character_state").await,
        Some(serde_json::json!("林惊羽 · 金丹初期"))
    );
    assert_eq!(
        read_world_state(&pool, project_id, "current_location").await,
        Some(serde_json::json!("青云山"))
    );
    assert_eq!(
        read_world_state(&pool, project_id, "snapshot_state_data").await,
        Some(serde_json::json!({"progress": "第3卷", "custom": 42}))
    );

    // 幂等：再恢复一次不报错、行数不翻倍
    svc.restore_snapshot(snapshot_id).await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM narrative_state WHERE project_id = $1 AND state_dimension = $2",
    )
    .bind(project_id)
    .bind(StateDimension::World.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 5, "重复恢复不得产生重复行");

    // 清理
    for k in [
        "story_time",
        "world_summary",
        "main_character_state",
        "current_location",
        "snapshot_state_data",
    ] {
        sqlx::query("DELETE FROM narrative_state WHERE project_id = $1 AND state_key = $2")
            .bind(project_id)
            .bind(k)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM novel_state_snapshot WHERE id = $1")
        .bind(snapshot_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM project WHERE id = $1")
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn restore_missing_snapshot_errors() {
    let pool = pool().await;
    let svc = service(pool.clone());
    let err = svc.restore_snapshot(Uuid::new_v4()).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("Snapshot not found"));
}
