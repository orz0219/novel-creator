//! Timeline Service - 时间线查询 + 冲突检测
//!
//! 通过 TimelineRepositoryPort 拉取事件；排序与冲突检测属于应用层逻辑，
//! 保留在 service 中。不直接依赖 db / sqlx。

use anyhow::Result;
use domain::entity::Event;
use domain::ports::TimelineRepositoryPort;
use std::sync::Arc;
use uuid::Uuid;

/// Timeline Service - 时间线服务
pub struct TimelineService {
    repo: Arc<dyn TimelineRepositoryPort>,
}

impl TimelineService {
    pub fn new(repo: Arc<dyn TimelineRepositoryPort>) -> Self {
        Self { repo }
    }

    /// 按时间顺序获取事件列表
    pub async fn get_timeline(&self, project_id: Uuid) -> Result<Vec<Event>> {
        let mut events = self.repo.list_events_by_project(project_id).await?;
        events.sort_by(|a, b| {
            let time_a = a.event_time.as_deref().unwrap_or("0");
            let time_b = b.event_time.as_deref().unwrap_or("0");
            time_a.cmp(time_b)
        });
        Ok(events)
    }

    /// 检测时间冲突
    pub async fn check_time_conflicts(&self, project_id: Uuid) -> Result<Vec<String>> {
        let events = self.get_timeline(project_id).await?;
        let mut conflicts = Vec::new();

        // 简单检查：同一时间段的事件
        for i in 0..events.len() {
            for j in (i + 1)..events.len() {
                if let (Some(ta), Some(tb)) = (&events[i].event_time, &events[j].event_time) {
                    if ta == tb && events[i].name != events[j].name {
                        conflicts.push(format!(
                            "事件 '{}' 和 '{}' 在同一时间点: {}",
                            events[i].name, events[j].name, ta
                        ));
                    }
                }
            }
        }

        Ok(conflicts)
    }
}
