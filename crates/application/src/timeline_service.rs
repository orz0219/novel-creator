//! Timeline Service - 时间线查询 + 冲突检测

use anyhow::Result;
use db::repos::event_repo::EventRepo;
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TimelineService {
    pool: PgPool,
}

impl TimelineService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按时间顺序获取事件列表
    pub async fn get_timeline(&self, project_id: Uuid) -> Result<Vec<Event>> {
        let repo = EventRepo::new(self.pool.clone());
        let mut events = repo.list_by_project(project_id).await?;
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
            for j in (i+1)..events.len() {
                if let (Some(ta), Some(tb)) = (&events[i].event_time, &events[j].event_time) {
                    if ta == tb && events[i].name != events[j].name {
                        conflicts.push(format!("事件 '{}' 和 '{}' 在同一时间点: {}", events[i].name, events[j].name, ta));
                    }
                }
            }
        }

        Ok(conflicts)
    }
}
