use std::path::Path;

use agents_core::{
    InputItem, OpenAIResponsesCompactionArgs, OpenAIResponsesCompactionAwareSession, Result,
    SQLiteSession, Session, SessionSettings,
};
use async_trait::async_trait;

/// Async-friendly wrapper around the shared SQLite session implementation.
#[derive(Clone, Debug)]
pub struct AsyncSQLiteSession {
    pub database_url: String,
    inner: SQLiteSession,
}

impl AsyncSQLiteSession {
    pub async fn open(session_id: impl Into<String>, db_path: impl AsRef<Path>) -> Result<Self> {
        let db_url = format!("sqlite://{}", db_path.as_ref().display());
        Self::open_with_url(session_id, &db_url).await
    }

    pub async fn open_in_memory(session_id: impl Into<String>) -> Result<Self> {
        Self::open_with_url(session_id, "sqlite::memory:").await
    }

    pub async fn open_with_url(session_id: impl Into<String>, database_url: &str) -> Result<Self> {
        let inner = SQLiteSession::open_with_url(session_id, database_url).await?;
        Ok(Self {
            database_url: database_url.to_owned(),
            inner,
        })
    }
}

#[async_trait]
impl Session for AsyncSQLiteSession {
    fn session_id(&self) -> &str {
        self.inner.session_id()
    }

    fn session_settings(&self) -> Option<&SessionSettings> {
        self.inner.session_settings()
    }

    async fn get_items_with_limit(&self, limit: Option<usize>) -> Result<Vec<InputItem>> {
        self.inner.get_items_with_limit(limit).await
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        self.inner.add_items(items).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        self.inner.pop_item().await
    }

    async fn clear_session(&self) -> Result<()> {
        self.inner.clear_session().await
    }

    fn compaction_session(&self) -> Option<&dyn OpenAIResponsesCompactionAwareSession> {
        Some(self)
    }
}

#[async_trait]
impl OpenAIResponsesCompactionAwareSession for AsyncSQLiteSession {
    async fn run_compaction(&self, args: Option<OpenAIResponsesCompactionArgs>) -> Result<()> {
        self.inner.run_compaction(args).await
    }
}
