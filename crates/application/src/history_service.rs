//! History Service - event / fact 的业务逻辑层。
//!
//! 通过 HistoryRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! version 相关为占位，仍由 host 层返回 stub。

use anyhow::Result;

/// 过渡实现用的取数上限：repository 尚未支持 LIMIT/OFFSET 下推，
/// 先按这个大上限取回再切片；下推实现落地后本常量随之删除。
const LIST_FETCH_ALL: i64 = 100_000;
use domain::ports::HistoryRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// History Service - 历史服务
pub struct HistoryService {
    repo: Arc<dyn HistoryRepositoryPort>,
}

impl HistoryService {
    pub fn new(repo: Arc<dyn HistoryRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_events(&self, project_id: Uuid, limit: i64) -> Result<Vec<Value>> {
        self.repo.list_events(project_id, limit, "recent").await
    }

    /// 按历史轴（`era_order`）排序列出事件——「这条时间线按发生顺序长什么样」。
    pub async fn list_events_by_era(&self, project_id: Uuid, limit: i64) -> Result<Vec<Value>> {
        self.repo.list_events(project_id, limit, "era").await
    }

    /// 有界列表（目录页）：返回本页 + 总数。（历史事件）
    ///
    /// 过渡实现：repository 还没下推 LIMIT/OFFSET，先取回再切片——
    /// 目的是**把返回给模型的体积有界化**（实测无界列表一次能到 42 万字符）；
    /// 等下推实现后就替换成真正的分页查询。
    pub async fn list_events_page(
        &self,
        project_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<serde_json::Value>, usize)> {
        let all = self.list_events(project_id, LIST_FETCH_ALL).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }


    /// 按指定排序锚分页列出事件（`order_by`：recent / era）。
    pub async fn list_events_page_ordered(
        &self,
        project_id: Uuid,
        limit: usize,
        offset: usize,
        order_by: &str,
    ) -> Result<(Vec<Value>, usize)> {
        let all = self.repo.list_events(project_id, LIST_FETCH_ALL, order_by).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }

    /// 创建事件（含结构化字段：类型 / 发生时间 / 时长 / attributes）。
    pub async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        event_time: Option<&str>,
        duration: Option<&str>,
        attributes: &Value,
        era_order: Option<i64>,
        narrative_node_id: Option<Uuid>,
    ) -> Result<Value> {
        self.repo
            .create_event(
                project_id,
                name,
                description,
                event_type,
                event_time,
                duration,
                attributes,
                era_order,
                narrative_node_id,
            )
            .await
    }

    /// 语义化结束事件（逻辑删除，不物理删除）。
    pub async fn delete_event(&self, id: Uuid) -> Result<()> {
        self.repo.delete_event(id).await
    }

    /// 语义化结束事实（逻辑删除）。
    pub async fn delete_fact(&self, id: Uuid) -> Result<()> {
        self.repo.delete_fact(id).await
    }

    /// 修改事件。各字段为 `None` 时保持原值。
    pub async fn update_event(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        event_type: Option<&str>,
        event_time: Option<&str>,
        duration: Option<&str>,
        attributes: Option<&Value>,
        era_order: Option<i64>,
        narrative_node_id: Option<Uuid>,
        clear_narrative_node: bool,
    ) -> Result<Value> {
        self.repo
            .update_event(
                id,
                name,
                description,
                event_type,
                event_time,
                duration,
                attributes,
                era_order,
                narrative_node_id,
                clear_narrative_node,
            )
            .await
    }

    pub async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_facts(project_id).await
    }

    /// 有界列表（目录页）：返回本页 + 总数。（事实）
    ///
    /// 过渡实现：repository 还没下推 LIMIT/OFFSET，先取回再切片——
    /// 目的是**把返回给模型的体积有界化**（实测无界列表一次能到 42 万字符）；
    /// 等下推实现后就替换成真正的分页查询。
    pub async fn list_facts_page(
        &self,
        project_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<serde_json::Value>, usize)> {
        let all = self.list_facts(project_id).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }


    pub async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
    ) -> Result<Value> {
        self.repo
            .create_fact(project_id, content, category, certainty)
            .await
    }
}

/// 收集事件的叙述性结构化字段到 `attributes`。
///
/// 放在 application 层是为了让 **agent 工具与 HTTP 接口共用同一套口径**：
/// 两边各写一份校验必然漂移（键名、空值语义、UUID 校验松紧）。
///
/// 三条约定：
/// 1. 只装入**确实传了**的键——`revise_event` 若把缺失键写成 null，
///    会把库里已有的 where / participants 清掉；
/// 2. 空串视为未提供；
/// 3. `participants[].entity_id` 必须是合法 UUID，非法值直接报错：
///    静默接受会让「从事件反查实体」永久失效，且事后完全看不出原因。
pub fn collect_event_attributes(input: &Value) -> Result<Value> {
    let mut attrs = serde_json::Map::new();

    for key in ["where", "consequences", "reveal_at"] {
        if let Some(v) = input.get(key) {
            if v.is_null() {
                continue;
            }
            let s = v
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("{} 应为字符串，收到：{}", key, v))?;
            if !s.trim().is_empty() {
                attrs.insert(key.to_string(), serde_json::json!(s.trim()));
            }
        }
    }

    if let Some(v) = input.get("participants") {
        if !v.is_null() {
            let arr = v
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("participants 应为数组"))?;
            let mut items = Vec::with_capacity(arr.len());
            for (i, item) in arr.iter().enumerate() {
                let entity_id = item
                    .get("entity_id")
                    .and_then(|x| x.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "participants[{}] 缺少 entity_id（先用 list_entities 取得实体 id）",
                            i
                        )
                    })?;
                let parsed = Uuid::parse_str(entity_id).map_err(|_| {
                    anyhow::anyhow!("participants[{}].entity_id 不是合法 UUID：{}", i, entity_id)
                })?;
                items.push(serde_json::json!({
                    "entity_id": parsed.to_string(),
                    "name": item.get("name").and_then(|x| x.as_str()),
                    "role": item.get("role").and_then(|x| x.as_str()),
                }));
            }
            attrs.insert("participants".to_string(), serde_json::json!(items));
        }
    }

    Ok(Value::Object(attrs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collect_event_attributes_takes_only_provided_keys() {
        // 只传 where：不应凭空造出 consequences / participants 键——
        // 否则 revise 时会把库里已有内容覆盖成空
        let attrs = collect_event_attributes(&json!({"where": "阴面·轮回渡"})).unwrap();
        assert_eq!(attrs["where"], json!("阴面·轮回渡"));
        assert!(attrs.get("consequences").is_none());
        assert!(attrs.get("participants").is_none());
        assert!(attrs.get("reveal_at").is_none());
    }

    #[test]
    fn collect_event_attributes_skips_blank_and_null() {
        let attrs = collect_event_attributes(&json!({
            "where": "   ",
            "consequences": null,
            "reveal_at": "第三卷"
        }))
        .unwrap();
        assert!(attrs.get("where").is_none(), "空白视为未提供");
        assert!(attrs.get("consequences").is_none(), "null 视为未提供");
        assert_eq!(attrs["reveal_at"], json!("第三卷"));
    }

    #[test]
    fn collect_event_attributes_validates_participant_uuid() {
        let id = Uuid::new_v4();
        let attrs = collect_event_attributes(&json!({
            "participants": [{"entity_id": id.to_string(), "role": "失踪者"}]
        }))
        .unwrap();
        assert_eq!(attrs["participants"][0]["entity_id"], json!(id.to_string()));
        assert_eq!(attrs["participants"][0]["role"], json!("失踪者"));

        // 非法 UUID 必须报错：静默接受会让「从事件反查实体」永久失效且事后无迹可查
        let err = collect_event_attributes(&json!({
            "participants": [{"entity_id": "not-a-uuid"}]
        }))
        .unwrap_err();
        assert!(err.to_string().contains("不是合法 UUID"), "{}", err);

        // 缺 entity_id 同样报错，并指明是第几项
        let err2 = collect_event_attributes(&json!({"participants": [{}]})).unwrap_err();
        assert!(err2.to_string().contains("participants[0]"), "{}", err2);
    }

    #[test]
    fn collect_event_attributes_rejects_wrong_types() {
        let err = collect_event_attributes(&json!({"where": 123})).unwrap_err();
        assert!(err.to_string().contains("应为字符串"), "{}", err);

        let err2 = collect_event_attributes(&json!({"participants": "王久财"})).unwrap_err();
        assert!(err2.to_string().contains("应为数组"), "{}", err2);
    }
}
