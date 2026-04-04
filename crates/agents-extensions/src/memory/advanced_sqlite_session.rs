use std::path::Path;

use agents_core::{
    InputItem, OpenAIResponsesCompactionArgs, OpenAIResponsesCompactionAwareSession, Result,
    SQLiteSession, Session, SessionSettings,
};
use async_trait::async_trait;

/// Extended SQLite session with configurable table names and database URL selection.
#[derive(Clone, Debug)]
pub struct AdvancedSQLiteSession {
    pub database_url: String,
    pub sessions_table: String,
    pub messages_table: String,
    inner: SQLiteSession,
}

impl AdvancedSQLiteSession {
    pub async fn open(session_id: impl Into<String>, db_path: impl AsRef<Path>) -> Result<Self> {
        let db_url = format!("sqlite://{}", db_path.as_ref().display());
        Self::open_with_options(
            session_id,
            &db_url,
            "agent_sessions",
            "agent_messages",
            Some(SessionSettings::default()),
        )
        .await
    }

    pub async fn open_in_memory(session_id: impl Into<String>) -> Result<Self> {
        Self::open_with_options(
            session_id,
            "sqlite::memory:",
            "agent_sessions",
            "agent_messages",
            Some(SessionSettings::default()),
        )
        .await
    }

    pub async fn open_with_options(
        session_id: impl Into<String>,
        database_url: &str,
        sessions_table: impl Into<String>,
        messages_table: impl Into<String>,
        session_settings: Option<SessionSettings>,
    ) -> Result<Self> {
        let session_id = session_id.into();
        let sessions_table = sessions_table.into();
        let messages_table = messages_table.into();
        let inner = SQLiteSession::open_with_options(
            session_id.clone(),
            database_url,
            sessions_table.clone(),
            messages_table.clone(),
            session_settings,
        )
        .await?;

        Ok(Self {
            database_url: database_url.to_owned(),
            sessions_table,
            messages_table,
            inner,
        })
    }
}

#[async_trait]
impl Session for AdvancedSQLiteSession {
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
impl OpenAIResponsesCompactionAwareSession for AdvancedSQLiteSession {
    async fn run_compaction(&self, args: Option<OpenAIResponsesCompactionArgs>) -> Result<()> {
        self.inner.run_compaction(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn supports_configured_tables() {
        let session = AdvancedSQLiteSession::open_with_options(
            "session",
            "sqlite::memory:",
            "custom_sessions",
            "custom_messages",
            Some(SessionSettings::default()),
        )
        .await
        .expect("advanced sqlite should open");

        session
            .add_items(vec![InputItem::from("hello")])
            .await
            .expect("item should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(items.len(), 1);
        assert_eq!(session.sessions_table, "custom_sessions");
    }
}
