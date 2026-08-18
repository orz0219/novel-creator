//! Timeline Service - 时间线查询 + 冲突检测

use anyhow::Result;
use db::connection::Database;
use db::repos::event_repo::EventRepo;
use domain::*;
use uuid::Uuid;

pub struct TimelineService<'a> {
    db: &'a Database,
}

impl<'a> TimelineService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 按时间顺序获取事件列表
    pub fn get_timeline(&self, project_id: Uuid) -> Result<Vec<Event>> {
        let repo = EventRepo::new(self.db);
        let mut events = repo.list_by_project(project_id)?;
        events.sort_by(|a, b| {
            let time_a = a.event_time.as_deref().unwrap_or("0");
            let time_b = b.event_time.as_deref().unwrap_or("0");
            time_a.cmp(time_b)
        });
        Ok(events)
    }

    /// 检测时间冲突
    pub fn check_time_conflicts(&self, project_id: Uuid) -> Result<Vec<String>> {
        let events = self.get_timeline(project_id)?;
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
