//! ⚠️ 本文件由 tmp/gen_sqlite_backend.py 自动生成，请勿手工编辑。
//! 如需修改逻辑，请改 PG 侧的对应文件后重新生成。

//! Agent 会话持久化（agent_sessions / agent_messages 表）。

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use domain::agent_store::{AgentSession, ChatMessage, SessionStore};

pub struct SessionRepo {
    pool: SqlitePool,
}

impl SessionRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 在一个已开启的事务里批量写入消息（seq 按数组下标）。
    ///
    /// 必须与「清空消息」处在同一事务：本项目采用「删除后重写」的全量落库方式，
    /// 若两条语句分开提交，中间会出现「消息被删光」的空窗期，
    /// 任何并发读取（例如前端在收到 done 事件后立刻拉取会话）都会读到 0 条，
    /// 表现为「对话被截断」。
    async fn insert_messages(
        conn: &mut sqlx::SqliteConnection,
        sid: Uuid,
        msgs: &[ChatMessage],
    ) -> Result<()> {
        for (seq, m) in msgs.iter().enumerate() {
            sqlx::query(
                "INSERT INTO agent_messages (session_id, role, content, seq, created_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(sid)
            .bind(&m.role)
            .bind(&m.content)
            .bind(seq as i32)
            .bind(m.created_at)
            .execute(&mut *conn)
            .await
            .context("Failed to insert agent message")?;
        }
        Ok(())
    }

    async fn load_messages(&self, sid: Uuid) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, session_id, role, content, seq, created_at FROM agent_messages \
             WHERE session_id = $1 ORDER BY seq ASC",
        )
        .bind(sid)
        .fetch_all(&self.pool)
        .await
        .context("Failed to load agent messages")?;
        Ok(rows.into_iter().map(|r| r.into_message()).collect())
    }
}

#[async_trait]
impl SessionStore for SessionRepo {
    async fn create(&self, session: AgentSession) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin create-session tx")?;

        sqlx::query(
            "INSERT INTO agent_sessions (id, project_id, title, current_step, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session.id)
        .bind(session.project_id)
        .bind(&session.title)
        .bind(&session.current_step)
        .bind(session.created_at)
        .bind(session.updated_at)
        .execute(&mut *tx)
        .await
        .context("Failed to create agent session")?;

        Self::insert_messages(&mut tx, session.id, &session.messages).await?;

        tx.commit().await.context("Failed to commit create-session")?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<AgentSession>> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT id, project_id, title, current_step, created_at, updated_at \
             FROM agent_sessions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get agent session")?;
        let Some(row) = row else { return Ok(None) };
        let messages = self.load_messages(id).await?;
        Ok(Some(row.into_session(messages)))
    }

    async fn update(&self, session: AgentSession) -> Result<()> {
        // 「清空 + 重写」必须在同一事务内完成：否则 DELETE 与 INSERT 之间存在空窗期，
        // 并发读取（如前端在收到 done 事件后立刻拉取会话）会读到 0 条消息，表现为「对话被截断」。
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin update-session tx")?;

        sqlx::query(
            "UPDATE agent_sessions SET title = $2, current_step = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(session.id)
        .bind(&session.title)
        .bind(&session.current_step)
        .execute(&mut *tx)
        .await
        .context("Failed to update agent session")?;

        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(session.id)
            .execute(&mut *tx)
            .await
            .context("Failed to clear agent messages")?;

        Self::insert_messages(&mut tx, session.id, &session.messages).await?;

        tx.commit().await.context("Failed to commit update-session")?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete agent session")?;
        Ok(())
    }

    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<AgentSession>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, project_id, title, current_step, created_at, updated_at \
             FROM agent_sessions WHERE project_id = $1 ORDER BY updated_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list agent sessions")?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let msg_rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, session_id, role, content, seq, created_at FROM agent_messages \
             WHERE upper(replace(hex(session_id), '-', '')) IN (SELECT upper(replace(value, '-', '')) FROM json_each($1)) ORDER BY seq ASC",
        )
        .bind(serde_json::to_string(&ids).context("序列化 id 数组失败")?)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list agent messages")?;
        let mut by_session: HashMap<Uuid, Vec<ChatMessage>> = HashMap::new();
        for m in msg_rows {
            by_session
                .entry(m.session_id)
                .or_default()
                .push(m.into_message());
        }
        Ok(rows
            .into_iter()
            .map(|r| {
                let id = r.id;
                r.into_session(by_session.remove(&id).unwrap_or_default())
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    project_id: Uuid,
    title: Option<String>,
    current_step: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl SessionRow {
    fn into_session(self, messages: Vec<ChatMessage>) -> AgentSession {
        AgentSession {
            id: self.id,
            project_id: self.project_id,
            title: self.title,
            messages,
            current_step: self.current_step,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)] // id/seq 暂时未读，保留以便后续 audit
struct MessageRow {
    id: Uuid,
    session_id: Uuid,
    role: String,
    content: String,
    seq: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl MessageRow {
    fn into_message(self) -> ChatMessage {
        ChatMessage {
            role: self.role,
            content: self.content,
            created_at: self.created_at,
        }
    }
}
