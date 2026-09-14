//! 细纲（叙事节点）写入时的**共享前置校验**。
//!
//! 为什么单独一个模块：新建（`DbNarrativeRepositoryPort::create_node_full`）与
//! 修改（`mutation_committer::apply_node_outline_tx`）两条写路径都要校验
//! 「故事线属于本项目 / 阶段名对得上 / 实体存在」。校验规则只写一份——
//! 两份实现迟早会分叉，而分叉之后没人知道哪份才是对的。
//!
//! 校验一律「缺前置条件就报错」，不做猜测、不写悬空引用。

use anyhow::{Context, Result};
use sqlx::PgConnection;
use uuid::Uuid;

/// 故事线必须存在且属于本项目。
pub async fn ensure_storyline_in_project(
    conn: &mut PgConnection,
    project_id: Uuid,
    storyline_id: Uuid,
) -> Result<()> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT project_id FROM storyline WHERE id=$1")
        .bind(storyline_id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to read storyline")?;
    match found {
        None => anyhow::bail!("故事线不存在：{}", storyline_id),
        Some(pid) if pid != project_id => {
            anyhow::bail!("故事线 {} 不属于本项目", storyline_id)
        }
        Some(_) => Ok(()),
    }
}

/// 阶段名必须在该故事线的 `arc_stages` 里。
///
/// 阶段表为空时不拦——那时无从判断，而「先建树、后补阶段表」是正常顺序；
/// 硬拦会把这条正常路径堵死。一旦该线已经有了阶段表，写错的阶段名就是错误。
pub async fn ensure_stage_exists(
    conn: &mut PgConnection,
    storyline_id: Uuid,
    stage: &str,
) -> Result<()> {
    let stages: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT arc_stages FROM storyline WHERE id=$1")
            .bind(storyline_id)
            .fetch_optional(&mut *conn)
            .await
            .context("Failed to read storyline arc_stages")?;
    let stages = stages.unwrap_or_else(|| serde_json::json!([]));
    let known: Vec<String> = stages
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    s.get("stage")
                        .or_else(|| s.get("stage_role"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    if known.is_empty() {
        return Ok(());
    }
    if known.iter().any(|k| k == stage) {
        return Ok(());
    }
    anyhow::bail!(
        "故事线 {} 上没有这个阶段：{}。它已有的阶段是：{}",
        storyline_id,
        stage,
        known.join(" / ")
    )
}

/// 实体 id 必须存在、属于本项目、且未被逻辑删除。
pub async fn ensure_entities_in_project(
    conn: &mut PgConnection,
    project_id: Uuid,
    ids: &[Uuid],
    field: &str,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let found: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM entity WHERE project_id=$1 AND id = ANY($2) AND status != 'Deleted'",
    )
    .bind(project_id)
    .bind(ids)
    .fetch_all(&mut *conn)
    .await
    .context("Failed to read entities for narrative node")?;
    let found: Vec<Uuid> = found.into_iter().map(|(id,)| id).collect();
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !found.contains(id))
        .map(|id| id.to_string())
        .collect();
    if !missing.is_empty() {
        anyhow::bail!(
            "{} 里的实体不存在 / 不属于本项目 / 已删除：{}",
            field,
            missing.join(" / ")
        );
    }
    Ok(())
}

/// 故事线 + 阶段一起校验（主挂载与 stage_refs 共用一条路径）。
pub async fn ensure_storyline_stage(
    conn: &mut PgConnection,
    project_id: Uuid,
    storyline_id: Uuid,
    arc_stage: Option<&str>,
) -> Result<()> {
    ensure_storyline_in_project(conn, project_id, storyline_id).await?;
    if let Some(stage) = arc_stage {
        ensure_stage_exists(conn, storyline_id, stage).await?;
    }
    Ok(())
}
