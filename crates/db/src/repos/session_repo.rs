//! Agent 会话持久化（agent_sessions / agent_messages 表）。

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use domain::agent_store::{AgentSession, ChatMessage, SessionStore};

pub struct SessionRepo {
    pool: PgPool,
}

impl SessionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn insert_messages(&self, sid: &Uuid, msgs: &[ChatMessage]) -> Result<()> {
        for (seq, m) in msgs.iter().enumerate() {
            sqlx::query(
                "INSERT INTO agent_messages (session_id, role, content, seq, created_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(*sid)
            .bind(&m.role)
            .bind(&m.content)
            .bind(seq as i32)
            .bind(m.created_at)
            .execute(&self.pool)
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
        .execute(&self.pool)
        .await
        .context("Failed to create agent session")?;
        self.insert_messages(&session.id, &session.messages).await?;
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
        sqlx::query(
            "UPDATE agent_sessions SET title = $2, current_step = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(session.id)
        .bind(&session.title)
        .bind(&session.current_step)
        .execute(&self.pool)
        .await
        .context("Failed to update agent session")?;
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(session.id)
            .execute(&self.pool)
            .await
            .context("Failed to clear agent messages")?;
        self.insert_messages(&session.id, &session.messages).await?;
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

    async fn list(&self) -> Result<Vec<AgentSession>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, project_id, title, current_step, created_at, updated_at \
             FROM agent_sessions ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list agent sessions")?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let msg_rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, session_id, role, content, seq, created_at FROM agent_messages \
             WHERE session_id = ANY($1) ORDER BY seq ASC",
        )
        .bind(&ids)
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
    project_id: Option<Uuid>,
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
