//! 记忆端口 `replace_by_type` 的数据库集成测试。
//!
//! 为什么必须有这层测试：滚动摘要靠它保证「项目里恒定只有一条摘要记忆」。
//! 若它退化成追加，每次收尾都会往系统提示词里多塞一份（可能互相矛盾的）摘要，
//! 而 agent 单元测试用的是内存实现，察觉不到 SQL 写错。
//!
//! 需要 `DATABASE_URL`（与其它 db 集成测试一致）；未设置时直接失败，不做跳过。

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use db::repos::memory_repo::MemoryRepo;
use domain::agent_store::AgentMemory;

/// 连接测试库（未配置 DATABASE_URL 时明确报错）。
async fn test_pool() -> Result<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").context("需要 DATABASE_URL（db 集成测试）")?;
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .context("连接测试数据库失败")
}

/// 造一个真实项目行（agent_memory.project_id 有外键约束）。
async fn seed_project(pool: &sqlx::PgPool) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO project (id, name, description) VALUES ($1, $2, $3)")
        .bind(id)
        .bind("记忆替换测试-临时项目")
        .bind("自动化测试创建，测试结束即删除")
        .execute(pool)
        .await
        .context("创建临时项目失败")?;
    Ok(id)
}

/// 删除临时项目（agent_memory 随项目外键级联清理）。
async fn cleanup(pool: &sqlx::PgPool, project_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM project WHERE id = $1")
        .bind(project_id)
        .execute(pool)
        .await
        .context("清理临时项目失败")?;
    Ok(())
}

#[tokio::test]
async fn replace_by_type_keeps_single_latest_entry() -> Result<()> {
    let pool = test_pool().await?;
    let repo = MemoryRepo::new(pool.clone());
    let project = seed_project(&pool).await?;

    let result = async {
        // 连续三次「收尾」：每次都只应留下最新那一份
        repo.replace_by_type(project, "session_summary", "第一版摘要").await?;
        repo.replace_by_type(project, "session_summary", "第二版摘要").await?;
        repo.replace_by_type(project, "session_summary", "第三版摘要").await?;

        let items = repo.list(project).await?;
        let summaries: Vec<_> = items
            .iter()
            .filter(|m| m.memory_type == "session_summary")
            .collect();

        assert_eq!(summaries.len(), 1, "同类记忆应恒定一条：{:?}", items);
        assert_eq!(summaries[0].content, "第三版摘要", "留下的必须是最新版");
        Ok(())
    }
    .await;

    cleanup(&pool, project).await?;
    result
}

#[tokio::test]
async fn replace_by_type_leaves_other_memory_types_untouched() -> Result<()> {
    let pool = test_pool().await?;
    let repo = MemoryRepo::new(pool.clone());
    let project = seed_project(&pool).await?;

    let result = async {
        repo.save(project, "用户偏好", "偏好黑暗奇幻").await?;
        repo.save(project, "session_summary", "旧摘要").await?;

        repo.replace_by_type(project, "session_summary", "新摘要").await?;

        let items = repo.list(project).await?;
        assert!(
            items.iter().any(|m| m.memory_type == "用户偏好"),
            "替换摘要不应动到别的记忆类型：{:?}",
            items
        );
        assert_eq!(
            items.iter().filter(|m| m.memory_type == "session_summary").count(),
            1,
            "摘要仍应只有一条：{:?}",
            items
        );
        Ok(())
    }
    .await;

    cleanup(&pool, project).await?;
    result
}
