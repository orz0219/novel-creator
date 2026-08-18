//! Storyline Service - 跨卷剧情线管理

use anyhow::Result;
use db::connection::Database;
use db::repos::storyline_repo::StorylineRepo;
use domain::*;
use uuid::Uuid;

pub struct StorylineService<'a> {
    db: &'a Database,
}

impl<'a> StorylineService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 获取所有活跃的剧情线
    pub fn get_active_storylines(&self, project_id: Uuid) -> Result<Vec<Storyline>> {
        let repo = StorylineRepo::new(self.db);
        let all = repo.list_by_project(project_id)?;
        Ok(all.into_iter().filter(|s| s.status == StorylineStatus::Active).collect())
    }

    /// 获取需要在 Volume 中推进的剧情线
    pub fn get_storylines_for_volume(&self, project_id: Uuid, volume_id: Uuid) -> Result<Vec<Storyline>> {
        let active = self.get_active_storylines(project_id)?;
        // 过滤出在该 Volume 中出现的剧情线
        Ok(active.into_iter().filter(|s| {
            s.created_volume_id.map(|v| v == volume_id).unwrap_or(true)
        }).collect())
    }
}
