//! ⚠️ 本文件由 tmp/gen_sqlite_backend.py 自动生成，请勿手工编辑。
//! 如需修改逻辑，请改 PG 侧的对应文件后重新生成。

//! 实体版本历史查询。
//!
//! 数据来源是 `entity_snapshot`：每次实体被修改**之前**，`EntityRepo::snapshot_tx`
//! 会把当时那一版留档。所以：
//!   - 快照 = 历史上的某一版（已被取代的那一版）
//!   - `entity` 表里的当前行 = 最新一版
//!
//! 这里把两者拼成一条时间线返回，并为每一版算出**相对上一版的字段级差异**
//! ——电脑端的 `VersionDiff.vue` 就是按 `changes` 渲染的。

use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::{SqlitePool, Row};
use uuid::Uuid;

pub struct EntityVersionRepo;

/// 一版的完整内容（快照或当前状态）
#[derive(Debug, Clone)]
struct Revision {
    version: i32,
    name: String,
    summary: Option<String>,
    description: Option<String>,
    attributes: Value,
    status: String,
    actor: Option<String>,
    /// 这一版"成为当前"的时间点
    at: chrono::DateTime<chrono::Utc>,
}

impl Revision {
    fn content(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("name", json!(self.name)),
            ("summary", json!(self.summary)),
            ("description", json!(self.description)),
            ("status", json!(self.status)),
            ("attributes", self.attributes.clone()),
        ]
    }
}

/// 算出 b 相对 a 的字段差异（只列出真正变了的字段）
fn diff(a: &Revision, b: &Revision) -> Value {
    let mut changes = serde_json::Map::new();
    for ((key, old), (_, new)) in a.content().into_iter().zip(b.content().into_iter()) {
        // 空值与 null 视为同一件事：库里 summary 可能是 NULL，也可能是空串
        let norm = |v: &Value| {
            if v.is_null() || v.as_str() == Some("") {
                Value::Null
            } else {
                v.clone()
            }
        };
        if norm(&old) != norm(&new) {
            changes.insert(key.to_string(), json!({ "old": old, "new": new }));
        }
    }
    Value::Object(changes)
}

fn entry(rev: &Revision, changes: Value, entity_id: Uuid) -> Value {
    let who = rev.actor.clone().unwrap_or_else(|| "unknown".to_string());
    json!({
        // 前端把 id 当 key 用，必须唯一
        "id": format!("v{}-{}", rev.version, entity_id),
        "entity_id": entity_id.to_string(),
        "version": rev.version,
        "description": format!("第 {} 版 · 由 {} 修改", rev.version, who),
        "actor": who,
        "created_at": rev.at.to_rfc3339(),
        "changes": changes,
    })
}

impl EntityVersionRepo {
    /// 取某实体的全部版本时间线（旧 → 新）。
    ///
    /// 实体不存在时返回空列表（调用方原样展示即可）。
    pub async fn list(pool: &SqlitePool, entity_id: Uuid) -> Result<Vec<Revision>> {
        let rows = sqlx::query(
            "SELECT version, name, summary, description, attributes, status, updated_by, replaced_at \
             FROM entity_snapshot WHERE entity_id = $1 ORDER BY version ASC",
        )
        .bind(entity_id)
        .fetch_all(pool)
        .await
        .context("读取实体历史快照失败")?;

        let mut out: Vec<Revision> = rows
            .into_iter()
            .map(|r| Revision {
                version: r.get("version"),
                name: r.get("name"),
                summary: r.get("summary"),
                description: r.get("description"),
                attributes: r
                    .try_get::<Option<Value>, _>("attributes")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| json!({})),
                status: r.get("status"),
                actor: r.get("updated_by"),
                at: r.get("replaced_at"),
            })
            .collect();

        // 当前这一版（entity 表里还没被取代的那一版）
        let current = sqlx::query(
            "SELECT version, name, summary, description, attributes, status, updated_by, updated_at \
             FROM entity WHERE id = $1",
        )
        .bind(entity_id)
        .fetch_optional(pool)
        .await
        .context("读取实体当前版本失败")?;

        if let Some(r) = current {
            out.push(Revision {
                version: r.get("version"),
                name: r.get("name"),
                summary: r.get("summary"),
                description: r.get("description"),
                attributes: r
                    .try_get::<Option<Value>, _>("attributes")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| json!({})),
                status: r.get("status"),
                actor: r.get("updated_by"),
                at: r.get("updated_at"),
            });
        }

        Ok(out)
    }

    /// 版本列表（含每一版相对上一版的差异）
    pub async fn list_entries(pool: &SqlitePool, entity_id: Uuid) -> Result<Vec<Value>> {
        let revs = Self::list(pool, entity_id).await?;
        let mut out = Vec::with_capacity(revs.len());
        for (i, rev) in revs.iter().enumerate() {
            // 第一版没有上一版可比，changes 为空
            let changes = if i == 0 {
                json!({})
            } else {
                diff(&revs[i - 1], rev)
            };
            out.push(entry(rev, changes, entity_id));
        }
        Ok(out)
    }

    /// 单版详情
    pub async fn get_entry(pool: &SqlitePool, entity_id: Uuid, version: i32) -> Result<Option<Value>> {
        let revs = Self::list(pool, entity_id).await?;
        let Some(idx) = revs.iter().position(|r| r.version == version) else {
            return Ok(None);
        };
        let changes = if idx == 0 {
            json!({})
        } else {
            diff(&revs[idx - 1], &revs[idx])
        };
        Ok(Some(entry(&revs[idx], changes, entity_id)))
    }

    /// 比较任意两个版本（from 旧 / to 新）
    pub async fn compare(
        pool: &SqlitePool,
        entity_id: Uuid,
        from: i32,
        to: i32,
    ) -> Result<Option<Value>> {
        let revs = Self::list(pool, entity_id).await?;
        let (Some(a), Some(b)) = (
            revs.iter().find(|r| r.version == from),
            revs.iter().find(|r| r.version == to),
        ) else {
            return Ok(None);
        };
        Ok(Some(diff(a, b)))
    }
}

/// 档案（角色档案 / 当前状态 / 地点档案 / 势力档案）的改动历史。
///
/// 快照里存的是**改动前**那一份完整档案，所以：
///   - 第 N 条的「改成了什么」= 第 N+1 条的内容
///   - 最后一条的「改成了什么」= 当前档案（由调用方传进来）
///
/// `current` 允许为 None（档案被清空或尚未建立），此时最后一条只标注改动前的样子。
pub async fn list_profile_entries(
    pool: &SqlitePool,
    entity_id: Uuid,
    current: Option<Value>,
) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        "SELECT kind, payload, replaced_at, actor FROM entity_profile_snapshot \
         WHERE entity_id = $1 ORDER BY replaced_at ASC",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await
    .context("读取档案历史快照失败")?;

    let snaps: Vec<(String, Value, chrono::DateTime<chrono::Utc>, Option<String>)> = rows
        .into_iter()
        .map(|r| {
            (
                r.get::<String, _>("kind"),
                r.get::<Value, _>("payload"),
                r.get::<chrono::DateTime<chrono::Utc>, _>("replaced_at"),
                r.get::<Option<String>, _>("actor"),
            )
        })
        .collect();

    let mut out: Vec<Value> = Vec::with_capacity(snaps.len());
    for i in 0..snaps.len() {
        let (kind, before, at, actor) = &snaps[i];
        // 「改后」优先取下一条快照；最后一条则用调用方给的当前档案
        let after = snaps
            .get(i + 1)
            .map(|(_, v, _, _)| v.clone())
            .or_else(|| current.clone());
        let changes = match after {
            Some(a) => diff_values(before, &a),
            None => json!({}),
        };
        out.push(json!({
            "id": format!("p{}-{}", i, entity_id),
            "kind": kind,
            // 谁改的：user / ai / system；早期没有这一列，可能是 null
            "actor": actor,
            "created_at": at.to_rfc3339(),
            "changes": changes,
        }));
    }

    // 最近的排前面
    out.reverse();
    Ok(out)
}

/// 两份档案 JSON 的字段级差异（只列出真正变了的顶层字段）。
///
/// 与 `diff` 的区别：这里比较的是任意两份 JSON，字段集合可能不同
/// （档案是 JSON，条目增删都要反映出来）。
fn diff_values(a: &Value, b: &Value) -> Value {
    let empty = serde_json::Map::new();
    let a_obj = a.as_object().unwrap_or(&empty);
    let b_obj = b.as_object().unwrap_or(&empty);

    let norm = |v: &Value| {
        if v.is_null() || v.as_str() == Some("") {
            Value::Null
        } else {
            v.clone()
        }
    };

    let mut keys: Vec<&String> = a_obj.keys().chain(b_obj.keys()).collect();
    keys.sort();
    keys.dedup();

    let mut changes = serde_json::Map::new();
    for k in keys {
        // 内部字段不展示：
        //   entity_id / id / created_at / updated_at 是记账用的，不是用户改的内容
        //   arc_stages 这类嵌套结构体在手机上一行读不了，且它由另一条链路维护
        if matches!(k.as_str(), "entity_id" | "id" | "created_at" | "updated_at" | "arc_stages") {
            continue;
        }
        let old = norm(a_obj.get(k).unwrap_or(&Value::Null));
        let new = norm(b_obj.get(k).unwrap_or(&Value::Null));
        if old != new && !(old.is_null() && new.is_null()) {
            changes.insert(k.clone(), json!({ "old": old, "new": new }));
        }
    }
    Value::Object(changes)
}

/// 某个地点的「相关人物」与「相关事件」。
///
/// 这两个查询此前是**写死的空数组**（`Ok(Json(json!([])))`），
/// 于是手机端详情页永远显示「暂无相关人物 / 暂无相关事件」——
/// 而实际数据是有的，用户看到的结论是错的。
///
/// 数据来源：
///   - 相关实体：`relation` 表里与该地点有关系的实体（两侧都可能是地点，取另一端）
///   - 相关事件：`event_entity` 关联到该地点的事件
pub struct LocationLinksRepo;

impl LocationLinksRepo {
    /// 与该地点有关系的实体（排除已删除的、以及地点自己）
    pub async fn entities(pool: &SqlitePool, location_id: Uuid) -> Result<Vec<Value>> {
        let rows = sqlx::query(
            "SELECT e.id, e.name, e.summary, et.name AS entity_type, r.relation_type \
             FROM relation r \
             JOIN entity e ON e.id = CASE \
                 WHEN r.source_entity_id = $1 THEN r.target_entity_id \
                 ELSE r.source_entity_id END \
             LEFT JOIN entity_type et ON et.id = e.entity_type_id \
             WHERE (r.source_entity_id = $1 OR r.target_entity_id = $1) \
               AND e.status != 'Deleted' \
             ORDER BY e.name",
        )
        .bind(location_id)
        .fetch_all(pool)
        .await
        .context("查询地点相关实体失败")?;

        Ok(rows
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id").to_string(),
                    "name": r.get::<String, _>("name"),
                    "summary": r.get::<Option<String>, _>("summary"),
                    "entity_type": r.get::<Option<String>, _>("entity_type"),
                    "relation_type": r.get::<String, _>("relation_type"),
                })
            })
            .collect())
    }

    /// 与该地点相关联的事件
    pub async fn events(pool: &SqlitePool, location_id: Uuid) -> Result<Vec<Value>> {
        let rows = sqlx::query(
            "SELECT ev.id, ev.name, ev.description, ev.event_time \
             FROM event_entity ee JOIN event ev ON ev.id = ee.event_id \
             WHERE ee.entity_id = $1 \
             ORDER BY ev.event_time NULLS LAST, ev.created_at",
        )
        .bind(location_id)
        .fetch_all(pool)
        .await
        .context("查询地点相关事件失败")?;

        Ok(rows
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id").to_string(),
                    "name": r.get::<String, _>("name"),
                    "description": r.get::<String, _>("description"),
                    "event_time": r.get::<Option<String>, _>("event_time"),
                })
            })
            .collect())
    }
}
