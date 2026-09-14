//! Narrative API handlers
//!
//! 所有 mutation 通过 application service (NarrativeService)。
//! 删除使用软删除（status=Deleted）。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;
use application::narrative_service::NarrativeService;
use application::storyline_service::StorylineService;
use application::foreshadow_service::ForeshadowService;
use application::mutation::MutationCommitter;
use db::application_ports::{DbStorylineRepositoryPort, DbForeshadowRepositoryPort};
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateNodeInput { pub node_type: String, pub parent_id: Option<String>, pub title: String, pub description: Option<String>, pub attributes: Option<serde_json::Value> }
#[derive(Deserialize)]
pub struct UpdateNodeInput { pub title: Option<String>, pub description: Option<String>, pub content: Option<String>, pub status: Option<String> }
#[derive(Deserialize)]
pub struct CreateStorylineInput {
    pub name: String,
    pub description: Option<String>,
    /// 状态（默认 Planned）。取值：Planned / Active / Resolved / Abandoned，或中文「计划中 / 进行中 / 已解决 / 已放弃」
    pub status: Option<String>,
    pub importance: Option<String>,
    /// 明/暗线（默认 light）
    pub tone: Option<String>,
    /// 可见性（默认 visible；暗线一般 hidden）
    pub visibility: Option<String>,
    /// 挂载到哪条 story line 下（None 表示独立 / 主线）
    pub parent_id: Option<String>,
}
#[derive(Deserialize)]
pub struct CreateForeshadowInput {
    pub name: String,
    pub description: Option<String>,
    /// 状态（默认 Planned）。取值：Planned / Introduced / Active / Revealed / Abandoned，或对应中文
    pub status: Option<String>,
    pub importance: Option<String>,
    pub hint_level: Option<String>,
    /// 所属剧情线。**双层 Option 区分三态**（REST 惯例）：
    /// 字段省略 = 不动归属；显式传 null = 解除挂载；传 uuid = 改挂到该线。
    /// 这样「只改名字」的请求不会顺手把伏笔从暗线上摘下来。
    #[serde(default, deserialize_with = "double_option")]
    pub storyline_id: Option<Option<Uuid>>,
}

/// 把 JSON 字段的「缺失」与「显式 null」区分开：
/// 缺失 → `None`；null → `Some(None)`；有值 → `Some(Some(v))`。
fn double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<T>::deserialize(de).map(Some)
}

/// 构建 NarrativeService：repo + MutationCommitter（统一写入口）+ ProjectResolver。
fn narrative_service(state: &AppState) -> NarrativeService {
    let pool = state.pool.clone();
    let committer = Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(
        pool.clone(),
    ))));
    let resolver = Arc::new(DbProjectResolverPort::new(pool.clone()));
    NarrativeService::new(
        Arc::new(db::application_ports::DbNarrativeRepositoryPort::new(pool)),
        committer,
        resolver,
    )
}

pub async fn list_nodes(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = narrative_service(&state);
    let nodes = service.list_nodes(project_id).await?;
    Ok(Json(serde_json::json!(nodes)))
}

pub async fn get_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = narrative_service(&state);
    let node = service.get_node(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Narrative node not found")))?;
    Ok(Json(node))
}

pub async fn create_node(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let parent_id = input.parent_id.map(|p| Uuid::parse_str(&p)).transpose()
        .map_err(|_| AppError(anyhow::anyhow!("Invalid parent ID")))?;

    let service = narrative_service(&state);
    let node = service.create_node(
        project_id,
        &input.node_type,
        parent_id,
        &input.title,
        input.description.as_deref(),
        input.attributes.unwrap_or(serde_json::json!({})),
    ).await?;

    Ok(Json(node))
}

pub async fn update_node(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = narrative_service(&state);
    let node = service.update_node(
        id,
        input.title.as_deref(),
        input.description.as_deref(),
        input.content.as_deref(),
        input.status.as_deref(),
    ).await?;

    Ok(Json(node))
}

pub async fn delete_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = narrative_service(&state);
    service.delete_node(id).await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

pub async fn list_storylines(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let storylines = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .list_storylines(project_id)
        .await?;
    Ok(Json(serde_json::json!(storylines)))
}

pub async fn list_storyline_relations(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let relations = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .list_storyline_relations(project_id)
        .await?;
    Ok(Json(serde_json::json!(relations)))
}

pub async fn create_storyline(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let parent_uuid = match &input.parent_id {
        Some(s) if !s.is_empty() => Some(Uuid::parse_str(s).map_err(|_| AppError(anyhow::anyhow!("Invalid parent_id")))?),
        _ => None,
    };
    // 状态经 domain 枚举校验：非法取值在这里就报错，不把脏字符串写进库
    let status = match input.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => domain::storyline::StorylineStatus::parse(raw)
            .ok_or_else(|| AppError(anyhow::anyhow!(
                "status 无法识别：{}（合法取值：{}）",
                raw,
                domain::storyline::STORYLINE_STATUSES.join(" / ")
            )))?
            .as_str(),
        None => "Planned",
    };
    let storyline = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .create_storyline(
            project_id,
            &input.name,
            input.description.as_deref(),
            status,
            input.importance.as_deref().unwrap_or("Normal"),
            input.tone.as_deref().unwrap_or("light"),
            input.visibility.as_deref().unwrap_or("visible"),
            parent_uuid,
            // 阶段弧线：HTTP 侧暂不暴露（前端编辑框没有这个字段），走工具层写
            None,
        )
        .await?;
    Ok(Json(storyline))
}

pub async fn update_storyline(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid storyline ID")))?;
    let status = match input.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(
            domain::storyline::StorylineStatus::parse(raw)
                .ok_or_else(|| AppError(anyhow::anyhow!(
                    "status 无法识别：{}（合法取值：{}）",
                    raw,
                    domain::storyline::STORYLINE_STATUSES.join(" / ")
                )))?
                .as_str(),
        ),
        None => None,
    };
    let storyline = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .update_storyline(
            id,
            // HTTP 侧 name 仍是必填（前端编辑框总会带上），显式转成 Some
            Some(&input.name),
            input.description.as_deref(),
            status,
            input.tone.as_deref(),
            input.visibility.as_deref(),
            // 阶段弧线：HTTP 侧暂不暴露
            None,
        )
        .await?;
    Ok(Json(storyline))
}

pub async fn delete_storyline(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid storyline ID")))?;
    StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .delete_storyline(id)
        .await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id}))
    )
}

pub async fn list_foreshadows(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let foreshadows = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .list_foreshadows(project_id)
        .await?;
    Ok(Json(serde_json::json!(foreshadows)))
}

/// 把可选的枚举入参解析成规范值：**非法值报错**，中文写法照常归一后落库。
///
/// 为什么必须有这个函数（实测事故）：这里原先写的是
/// `input.importance.as_deref().unwrap_or("Normal")` —— 字符串直接透传，
/// 于是库里混进了 `重要` / `Main` / `Major` / `低（前期只透风，不揭示）` / `中期显形`
/// 这类自由文本，前端再也没法按等级排序 / 过滤（见迁移 034 的清洗）。
/// 工具层一直有 `opt_enum_arg` 严格校验，**只有 HTTP 这条路是漏的**。
fn resolve_enum<'a>(
    raw: Option<&'a str>,
    key: &str,
    parse: impl Fn(&str) -> Option<&'static str>,
    allowed: &[&str],
) -> Result<Option<&'static str>, AppError> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    parse(s).map(Some).ok_or_else(|| {
        AppError(anyhow::anyhow!(
            "{} 无法识别：{}（合法取值：{}）",
            key,
            s,
            allowed.join(" / ")
        ))
    })
}

pub async fn create_foreshadow(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let foreshadow = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .create_foreshadow(
            project_id,
            &input.name,
            input.description.as_deref(),
            match input.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(raw) => domain::foreshadowing::ForeshadowingStatus::parse(raw)
                    .ok_or_else(|| AppError(anyhow::anyhow!(
                        "status 无法识别：{}（合法取值：{}）",
                        raw,
                        domain::foreshadowing::FORESHADOWING_STATUSES.join(" / ")
                    )))?
                    .as_str(),
                None => "Planned",
            },
            resolve_enum(
                input.importance.as_deref(),
                "importance",
                |s| domain::foreshadowing::ForeshadowingImportance::parse(s).map(|v| v.as_str()),
                domain::foreshadowing::FORESHADOWING_IMPORTANCES,
            )?
            .unwrap_or("Normal"),
            resolve_enum(
                input.hint_level.as_deref(),
                "hint_level",
                |s| domain::foreshadowing::HintLevel::parse(s).map(|v| v.as_str()),
                domain::foreshadowing::HINT_LEVELS,
            )?
            .unwrap_or("Subtle"),
            // HTTP（前端编辑框）暂不暴露备注与时间锚，传 None 表示"用默认/不改"。
            // 需要写这几个字段时走工具层（AI 路径），两边共用同一套仓储校验。
            None,
            None,
            None,
            // 节点锚与伏笔树：HTTP 侧暂不暴露
            None,
            None,
            None,
            match input.storyline_id {
                Some(v) => v,
                None => None,
            },
        )
        .await?;
    Ok(Json(foreshadow))
}

pub async fn update_foreshadow(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid foreshadow ID")))?;
    let foreshadow = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .update_foreshadow(
        id,
        // HTTP 侧 name 仍是必填（前端编辑框总会带上它），这里显式转成 Some
        Some(&input.name),
        input.description.as_deref(),
        match input.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(raw) => Some(
                domain::foreshadowing::ForeshadowingStatus::parse(raw)
                    .ok_or_else(|| AppError(anyhow::anyhow!(
                        "status 无法识别：{}（合法取值：{}）",
                        raw,
                        domain::foreshadowing::FORESHADOWING_STATUSES.join(" / ")
                    )))?
                    .as_str(),
            ),
            None => None,
        },
        // 备注与三个时间锚：HTTP 侧暂不暴露（前端编辑框没有这些字段）
        None,
        None,
        None,
        None,
        // 节点锚与伏笔树：HTTP 侧暂不暴露
        None,
        None,
        None,
        input.storyline_id,
    )
        .await?;
    Ok(Json(foreshadow))
}

pub async fn delete_foreshadow(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid foreshadow ID")))?;
    ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .delete_foreshadow(id)
        .await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}