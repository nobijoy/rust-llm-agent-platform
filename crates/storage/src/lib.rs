use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Row};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: Uuid,
    pub session_id: String,
    pub user_prompt: String,
    pub response: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Storage {
    pub pool: SqlitePool,
}

impl Storage {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::migrate!("../../migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn save_run(
        &self,
        session_id: &str,
        user_prompt: &str,
        response: &str,
    ) -> Result<AgentRun, StorageError> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();

        sqlx::query(
            "INSERT INTO agent_runs (id, session_id, user_prompt, response, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(session_id)
        .bind(user_prompt)
        .bind(response)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(AgentRun {
            id,
            session_id: session_id.to_string(),
            user_prompt: user_prompt.to_string(),
            response: response.to_string(),
            created_at,
        })
    }

    pub async fn recent_runs(&self, limit: i64) -> Result<Vec<AgentRun>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, user_prompt, response, created_at FROM agent_runs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let created_at: String = row.get("created_at");
                Ok(AgentRun {
                    id: Uuid::parse_str(&id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
                    session_id: row.get("session_id"),
                    user_prompt: row.get("user_prompt"),
                    response: row.get("response"),
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                        .with_timezone(&Utc),
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(StorageError::Database)
    }
}
