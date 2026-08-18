//! Job System - async task management with state machine

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Job 状态机
///
/// 状态转换:
/// PENDING -> RUNNING -> WAITING_INPUT -> COMPLETED
/// 异常: FAILED, CANCELLED, TIMEOUT
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 等待输入
    WaitingInput,
    /// 已完成
    Completed,
    /// 执行失败
    Failed,
    /// 已取消
    Cancelled,
    /// 执行超时
    Timeout,
}

impl JobStatus {
    /// 检查状态转换是否合法
    pub fn can_transition_to(&self, new_status: &JobStatus) -> bool {
        matches!(
            (self, new_status),
            (JobStatus::Pending, JobStatus::Running)
                | (JobStatus::Pending, JobStatus::Cancelled)
                | (JobStatus::Running, JobStatus::WaitingInput)
                | (JobStatus::Running, JobStatus::Completed)
                | (JobStatus::Running, JobStatus::Failed)
                | (JobStatus::Running, JobStatus::Timeout)
                | (JobStatus::WaitingInput, JobStatus::Running)
                | (JobStatus::WaitingInput, JobStatus::Cancelled)
        )
    }
}

/// Job - 后台任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub project_id: Uuid,
    pub job_type: String,
    pub name: String,
    pub description: Option<String>,
    pub status: JobStatus,
    pub priority: i32,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Job 创建请求
#[derive(Debug, Clone)]
pub struct CreateJobRequest {
    pub project_id: Uuid,
    pub job_type: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: i32,
    pub input: serde_json::Value,
}

impl Job {
    /// 创建新 Job
    pub fn new(request: CreateJobRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id: request.project_id,
            job_type: request.job_type,
            name: request.name,
            description: request.description,
            status: JobStatus::Pending,
            priority: request.priority,
            input: request.input,
            output: None,
            error: None,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// 开始执行
    pub fn start(&mut self) -> Result<()> {
        if self.status.can_transition_to(&JobStatus::Running) {
            self.status = JobStatus::Running;
            self.started_at = Some(Utc::now());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cannot start job in status: {:?}", self.status))
        }
    }

    /// 完成执行
    pub fn complete(&mut self, output: serde_json::Value) -> Result<()> {
        if self.status.can_transition_to(&JobStatus::Completed) {
            self.status = JobStatus::Completed;
            self.output = Some(output);
            self.progress = 1.0;
            self.completed_at = Some(Utc::now());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cannot complete job in status: {:?}", self.status))
        }
    }

    /// 标记失败
    pub fn fail(&mut self, error: String) -> Result<()> {
        if self.status.can_transition_to(&JobStatus::Failed) {
            self.status = JobStatus::Failed;
            self.error = Some(error);
            self.completed_at = Some(Utc::now());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cannot fail job in status: {:?}", self.status))
        }
    }

    /// 取消执行
    pub fn cancel(&mut self) -> Result<()> {
        if self.status.can_transition_to(&JobStatus::Cancelled) {
            self.status = JobStatus::Cancelled;
            self.completed_at = Some(Utc::now());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cannot cancel job in status: {:?}", self.status))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_lifecycle() {
        let mut job = Job::new(CreateJobRequest {
            project_id: Uuid::new_v4(),
            job_type: "generation".to_string(),
            name: "Generate Chapter 1".to_string(),
            description: None,
            priority: 1,
            input: serde_json::json!({}),
        });

        assert_eq!(job.status, JobStatus::Pending);

        job.start().unwrap();
        assert_eq!(job.status, JobStatus::Running);

        job.complete(serde_json::json!({"result": "success"})).unwrap();
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_failure() {
        let mut job = Job::new(CreateJobRequest {
            project_id: Uuid::new_v4(),
            job_type: "generation".to_string(),
            name: "Generate Chapter 1".to_string(),
            description: None,
            priority: 1,
            input: serde_json::json!({}),
        });

        job.start().unwrap();
        job.fail("LLM timeout".to_string()).unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error.unwrap(), "LLM timeout");
    }

    #[test]
    fn test_job_cancellation() {
        let mut job = Job::new(CreateJobRequest {
            project_id: Uuid::new_v4(),
            job_type: "generation".to_string(),
            name: "Generate Chapter 1".to_string(),
            description: None,
            priority: 1,
            input: serde_json::json!({}),
        });

        job.start().unwrap();
        job.cancel().unwrap();
        assert_eq!(job.status, JobStatus::Cancelled);
    }

    #[test]
    fn test_invalid_state_transition() {
        let mut job = Job::new(CreateJobRequest {
            project_id: Uuid::new_v4(),
            job_type: "generation".to_string(),
            name: "Generate Chapter 1".to_string(),
            description: None,
            priority: 1,
            input: serde_json::json!({}),
        });

        job.start().unwrap();
        job.complete(serde_json::json!({})).unwrap();

        // Cannot complete again
        assert!(job.complete(serde_json::json!({})).is_err());
    }
}
