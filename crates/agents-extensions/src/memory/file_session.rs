use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use agents_core::{
    AgentsError, InputItem, Result, RunState, Session, SessionSettings,
    memory::{resolve_session_limit, util::apply_session_limit},
};
use async_trait::async_trait;
use tokio::fs;
use uuid::Uuid;

/// File-backed session storage for local workflows and examples.
#[derive(Clone, Debug)]
pub struct FileSession {
    session_id: String,
    pub dir: PathBuf,
    session_settings: Option<SessionSettings>,
}

impl FileSession {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::with_session_id(dir, format!("session-{}", Uuid::new_v4()))
    }

    pub fn with_session_id(dir: impl Into<PathBuf>, session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            dir: dir.into(),
            session_settings: Some(SessionSettings::default()),
        }
    }

    pub fn with_session_settings(mut self, session_settings: SessionSettings) -> Self {
        self.session_settings = Some(session_settings);
        self
    }

    pub fn items_path(&self) -> PathBuf {
        self.dir.join(format!("{}.json", self.session_id))
    }

    pub fn state_path(&self) -> PathBuf {
        self.dir.join(format!("{}-state.json", self.session_id))
    }

    pub async fn load_state_json(&self) -> Result<Option<String>> {
        read_optional_string(&self.state_path()).await
    }

    pub async fn save_state_json(&self, state_json: &str) -> Result<()> {
        write_string(&self.state_path(), state_json).await
    }

    pub async fn load_run_state(&self) -> Result<Option<RunState>> {
        self.load_state_json()
            .await?
            .map(|value| RunState::from_json_str(&value))
            .transpose()
    }

    pub async fn save_run_state(&self, state: &RunState) -> Result<()> {
        self.save_state_json(&state.to_json_string()?).await
    }

    async fn read_items(&self) -> Result<Vec<InputItem>> {
        let Some(raw) = read_optional_string(&self.items_path()).await? else {
            return Ok(Vec::new());
        };
        serde_json::from_str::<Vec<InputItem>>(&raw)
            .map_err(|error| AgentsError::message(error.to_string()))
    }

    async fn write_items(&self, items: &[InputItem]) -> Result<()> {
        let payload = serde_json::to_string_pretty(items)
            .map_err(|error| AgentsError::message(error.to_string()))?;
        write_string(&self.items_path(), &payload).await
    }
}

#[async_trait]
impl Session for FileSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn session_settings(&self) -> Option<&SessionSettings> {
        self.session_settings.as_ref()
    }

    async fn get_items_with_limit(&self, limit: Option<usize>) -> Result<Vec<InputItem>> {
        let items = self.read_items().await?;
        let resolved_limit = resolve_session_limit(limit, self.session_settings());
        Ok(apply_session_limit(&items, resolved_limit))
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let mut current = self.read_items().await?;
        current.extend(items);
        self.write_items(&current).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        let mut current = self.read_items().await?;
        let popped = current.pop();
        self.write_items(&current).await?;
        Ok(popped)
    }

    async fn clear_session(&self) -> Result<()> {
        remove_optional_file(&self.items_path()).await?;
        remove_optional_file(&self.state_path()).await
    }
}

async fn read_optional_string(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path).await {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AgentsError::message(error.to_string())),
    }
}

async fn write_string(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
    }
    fs::write(path, value)
        .await
        .map_err(|error| AgentsError::message(error.to_string()))
}

async fn remove_optional_file(path: &Path) -> Result<()> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AgentsError::message(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_session_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "openai-agents-file-session-{name}-{}",
            Uuid::new_v4()
        ))
    }

    #[tokio::test]
    async fn file_session_rehydrates_items_from_disk() {
        let dir = temp_session_dir("rehydrate");
        let session = FileSession::with_session_id(&dir, "conversation");
        session
            .add_items(vec![InputItem::from("one"), InputItem::from("two")])
            .await
            .expect("items should store");

        let rehydrated = FileSession::with_session_id(&dir, "conversation");
        let items = rehydrated.get_items().await.expect("items should load");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_text(), Some("one"));
        assert_eq!(items[1].as_text(), Some("two"));

        rehydrated.clear_session().await.expect("session clears");
        let _ = fs::remove_dir_all(dir).await;
    }

    #[tokio::test]
    async fn file_session_saves_and_loads_state_json() {
        let dir = temp_session_dir("state");
        let session = FileSession::with_session_id(&dir, "conversation");
        session
            .save_state_json(r#"{"schema_version":"test"}"#)
            .await
            .expect("state should store");

        let loaded = session
            .load_state_json()
            .await
            .expect("state should load")
            .expect("state should exist");
        assert_eq!(loaded, r#"{"schema_version":"test"}"#);

        session.clear_session().await.expect("session clears");
        let _ = fs::remove_dir_all(dir).await;
    }
}
