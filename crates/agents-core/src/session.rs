use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::errors::Result;
use crate::items::InputItem;

#[async_trait]
pub trait Session: Send + Sync {
    fn session_id(&self) -> &str;
    async fn get_items(&self) -> Result<Vec<InputItem>>;
    async fn add_items(&self, items: Vec<InputItem>) -> Result<()>;
    async fn pop_item(&self) -> Result<Option<InputItem>>;
    async fn clear(&self) -> Result<()>;
}

/// In-memory session used by tests and local workflows.
#[derive(Clone, Debug)]
pub struct MemorySession {
    session_id: String,
    items: Arc<Mutex<Vec<InputItem>>>,
}

impl MemorySession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            items: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Session for MemorySession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    async fn get_items(&self) -> Result<Vec<InputItem>> {
        Ok(self.items.lock().await.clone())
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        self.items.lock().await.extend(items);
        Ok(())
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        Ok(self.items.lock().await.pop())
    }

    async fn clear(&self) -> Result<()> {
        self.items.lock().await.clear();
        Ok(())
    }
}
