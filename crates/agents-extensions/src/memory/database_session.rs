use agents_core::{
    AgentsError, InputItem, OpenAIResponsesCompactionArgs, OpenAIResponsesCompactionAwareSession,
    Result, SQLiteSession, Session, SessionSettings,
};
use async_trait::async_trait;

/// Database-backed session adapter. The current implementation supports SQLite connection strings.
#[derive(Clone, Debug)]
pub struct DatabaseSession {
    pub connection_string: String,
    inner: SQLiteSession,
}

impl DatabaseSession {
    pub async fn open(
        session_id: impl Into<String>,
        connection_string: impl Into<String>,
    ) -> Result<Self> {
        let connection_string = connection_string.into();
        if !connection_string.starts_with("sqlite:") {
            return Err(AgentsError::message(
                "DatabaseSession currently supports sqlite connection strings only",
            ));
        }
        let inner = SQLiteSession::open_with_url(session_id, &connection_string).await?;
        Ok(Self {
            connection_string,
            inner,
        })
    }
}

#[async_trait]
impl Session for DatabaseSession {
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
impl OpenAIResponsesCompactionAwareSession for DatabaseSession {
    async fn run_compaction(&self, args: Option<OpenAIResponsesCompactionArgs>) -> Result<()> {
        self.inner.run_compaction(args).await
    }
}
