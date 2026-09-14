//! 列表读取的「有界 + 可续读」零件（所有 `list_*` 工具共用）。
//!
//! 背景（实测，不是估算）：novel 的读工具原先没有上限——
//! `list_projects` 一次返回 1172 个项目的全字段 = **425,018 字符**；
//! `list_entities` 返回 33 个实体的全字段 = **36,555 字符**（其中 `description` 占 51%，
//! 外加 `id` / `world_id` 各 1254、`created_at` / `updated_at` 各 1020 字符的审计列）。
//!
//! 这些字符一旦进入上下文，**之后每一轮请求都要重发**，直接把模型拖进"思考 116 秒"
//! 并撞上网关的超时（`error decoding response body`）。
//!
//! 因此这里的约定：
//! 1. 列表默认只给 [`LIST_DEFAULT_LIMIT`] 条，硬上限 [`LIST_MAX_LIMIT`]，超上限**报错**
//!    （不静默夹紧——静默夹紧会让调用方以为「只有这么多」）；
//! 2. 返回信封必带 `total` / `returned` / `has_more` / `next_offset`，
//!    让模型先知道"有多少"，再决定要不要翻页；
//! 3. 列表只给「目录页」字段（id / 名称 / 类型 / 一句话摘要），**正文用 `get_*` 按需取**。

use anyhow::Result;
use serde_json::{json, Value};

/// 列表默认条数。
pub const LIST_DEFAULT_LIMIT: usize = 20;
/// 单次列表硬上限；超过直接报错，让调用方显式分批。
pub const LIST_MAX_LIMIT: usize = 100;

/// 从工具入参解析 `limit` / `offset`（缺省为 [`LIST_DEFAULT_LIMIT`] / 0）。
///
/// 非法值（负数、非数字、超过硬上限）一律报错，不做兜底。
pub fn parse_page_args(input: &Value) -> Result<(usize, usize)> {
    let limit = match input.get("limit") {
        None | Some(Value::Null) => LIST_DEFAULT_LIMIT,
        Some(v) => parse_usize(v, "limit")?,
    };
    let offset = match input.get("offset") {
        None | Some(Value::Null) => 0,
        Some(v) => parse_usize(v, "offset")?,
    };
    if limit == 0 {
        anyhow::bail!("limit 不能为 0：至少要读一条，否则这次调用没有意义");
    }
    if limit > LIST_MAX_LIMIT {
        anyhow::bail!(
            "limit={} 超过单次上限 {}：请分页读取（用 offset 继续），不要一次拉全量",
            limit,
            LIST_MAX_LIMIT
        );
    }
    Ok((limit, offset))
}

fn parse_usize(v: &Value, key: &str) -> Result<usize> {
    if let Some(n) = v.as_u64() {
        return Ok(n as usize);
    }
    if let Some(s) = v.as_str() {
        return s
            .trim()
            .parse::<usize>()
            .map_err(|_| anyhow::anyhow!("{} 应为非负整数，收到：{}", key, s));
    }
    anyhow::bail!("{} 应为非负整数，收到：{}", key, v)
}

/// 只保留 `keep` 里的字段（值为 null 的丢掉）——列表用的「目录页投影」。
pub fn pick_fields(item: &Value, keep: &[&str]) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(obj) = item.as_object() {
        for k in keep {
            if let Some(v) = obj.get(*k) {
                if !v.is_null() {
                    out.insert((*k).to_string(), v.clone());
                }
            }
        }
    }
    Value::Object(out)
}

/// 构造列表返回信封：`total` / `returned` / `has_more` / `next_offset` / `hint`。
///
/// `extras` 用来附加工具特有的说明性字段（如类型 id → 名称的映射表）。
pub fn list_envelope(
    action: &str,
    items: Vec<Value>,
    total: usize,
    limit: usize,
    offset: usize,
    noun: &str,
    extras: Vec<(&str, Value)>,
) -> Value {
    let returned = items.len();
    let has_more = offset + returned < total;

    let hint = if total == 0 {
        format!("该范围内没有{}。", noun)
    } else if has_more {
        format!(
            "共 {} 个{}，本次返回第 {}~{} 个；用 offset={} 继续。列表只给目录字段，正文请用 get_* 按需读取。",
            total,
            noun,
            offset + 1,
            offset + returned,
            offset + returned
        )
    } else {
        format!(
            "共 {} 个{}，已全部返回（第 {}~{} 个）。列表只给目录字段，正文请用 get_* 按需读取。",
            total,
            noun,
            offset + 1,
            offset + returned
        )
    };

    let mut map = serde_json::Map::new();
    map.insert("ok".into(), json!(true));
    map.insert("action".into(), json!(action));
    map.insert("data".into(), Value::Array(items));
    map.insert("total".into(), json!(total));
    map.insert("returned".into(), json!(returned));
    map.insert("limit".into(), json!(limit));
    map.insert("offset".into(), json!(offset));
    map.insert("has_more".into(), json!(has_more));
    if has_more {
        map.insert("next_offset".into(), json!(offset + returned));
    }
    for (k, v) in extras {
        map.insert(k.to_string(), v);
    }
    map.insert("hint".into(), json!(hint));
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_args_default_and_validation() {
        let (l, o) = parse_page_args(&json!({})).unwrap();
        assert_eq!((l, o), (LIST_DEFAULT_LIMIT, 0));

        let (l, o) = parse_page_args(&json!({ "limit": 5, "offset": 10 })).unwrap();
        assert_eq!((l, o), (5, 10));

        // 字符串数字也接受（模型常把数字写成字符串）
        let (l, _) = parse_page_args(&json!({ "limit": "7" })).unwrap();
        assert_eq!(l, 7);

        assert!(parse_page_args(&json!({ "limit": 0 })).is_err(), "limit=0 应报错");
        assert!(
            parse_page_args(&json!({ "limit": LIST_MAX_LIMIT + 1 })).is_err(),
            "超过硬上限应报错，不静默夹紧"
        );
        assert!(parse_page_args(&json!({ "limit": -1 })).is_err());
        assert!(parse_page_args(&json!({ "offset": "x" })).is_err());
    }

    #[test]
    fn envelope_reports_total_and_next_offset() {
        let items: Vec<Value> = (0..20).map(|i| json!({ "id": i })).collect();
        let v = list_envelope("list_entities", items, 33, 20, 0, "实体", vec![]);
        assert_eq!(v["total"], json!(33));
        assert_eq!(v["returned"], json!(20));
        assert_eq!(v["has_more"], json!(true));
        assert_eq!(v["next_offset"], json!(20));
        assert!(v["hint"].as_str().unwrap().contains("offset=20"));

        let last = list_envelope("list_entities", vec![json!({ "id": 1 })], 21, 20, 20, "实体", vec![]);
        assert_eq!(last["has_more"], json!(false));
        assert!(last.get("next_offset").is_none(), "已读完就不该给 next_offset");
    }

    #[test]
    fn pick_fields_drops_null_and_unknown() {
        let item = json!({ "id": "a", "name": "林夜", "description": "长正文", "summary": null });
        let p = pick_fields(&item, &["id", "name", "summary", "entity_type_id"]);
        assert_eq!(p, json!({ "id": "a", "name": "林夜" }));
    }
}
