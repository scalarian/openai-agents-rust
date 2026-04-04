use agents_core::{InputItem, MemorySession, Result, Session};
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct OpenAIConversationsSession {
    inner: MemorySession,
}

impl OpenAIConversationsSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            inner: MemorySession::new(session_id),
        }
    }
}

#[async_trait]
impl Session for OpenAIConversationsSession {
    fn session_id(&self) -> &str {
        self.inner.session_id()
    }

    async fn get_items(&self) -> Result<Vec<InputItem>> {
        self.inner.get_items().await
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        self.inner.add_items(items).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        self.inner.pop_item().await
    }

    async fn clear(&self) -> Result<()> {
        self.inner.clear().await
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpenAIResponsesCompactionMode {
    PreviousResponseId,
    Input,
    #[default]
    Auto,
}

#[derive(Clone, Debug)]
pub struct OpenAIResponsesCompactionSession {
    inner: MemorySession,
    pub mode: OpenAIResponsesCompactionMode,
}

impl OpenAIResponsesCompactionSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            inner: MemorySession::new(session_id),
            mode: OpenAIResponsesCompactionMode::Auto,
        }
    }

    pub fn with_mode(mut self, mode: OpenAIResponsesCompactionMode) -> Self {
        self.mode = mode;
        self
    }
}

#[async_trait]
impl Session for OpenAIResponsesCompactionSession {
    fn session_id(&self) -> &str {
        self.inner.session_id()
    }

    async fn get_items(&self) -> Result<Vec<InputItem>> {
        self.inner.get_items().await
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        self.inner.add_items(items).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        self.inner.pop_item().await
    }

    async fn clear(&self) -> Result<()> {
        self.inner.clear().await
    }
}
