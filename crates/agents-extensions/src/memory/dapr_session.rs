use agents_core::{
    AgentsError, InputItem, Result, Session, SessionSettings, memory::util::apply_session_limit,
};
use async_trait::async_trait;
use reqwest::StatusCode;
use serde_json::json;

/// Dapr state-store backed session using the HTTP state API.
#[derive(Clone, Debug)]
pub struct DaprSession {
    session_id: String,
    pub address: String,
    pub state_store_name: String,
    pub key_prefix: String,
    pub ttl_seconds: Option<u64>,
    session_settings: Option<SessionSettings>,
    client: reqwest::Client,
}

impl DaprSession {
    pub fn new(
        session_id: impl Into<String>,
        address: impl Into<String>,
        state_store_name: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            address: normalize_dapr_address(&address.into()),
            state_store_name: state_store_name.into(),
            key_prefix: "agents-session-".to_owned(),
            ttl_seconds: None,
            session_settings: Some(SessionSettings::default()),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = key_prefix.into();
        self
    }

    pub fn with_ttl_seconds(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self
    }

    pub fn from_address(
        session_id: impl Into<String>,
        state_store_name: impl Into<String>,
        address: impl Into<String>,
    ) -> Self {
        Self::new(session_id, address, state_store_name)
    }

    fn key(&self) -> String {
        format!("{}{}", self.key_prefix, self.session_id)
    }

    fn state_url(&self) -> String {
        format!(
            "{}/v1.0/state/{}",
            self.address.trim_end_matches('/'),
            self.state_store_name
        )
    }

    async fn read_all_items(&self) -> Result<Vec<InputItem>> {
        let response = self
            .client
            .get(format!("{}/{}", self.state_url(), self.key()))
            .send()
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;

        if matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::NO_CONTENT
        ) {
            return Ok(Vec::new());
        }
        if !response.status().is_success() {
            return Err(AgentsError::message(format!(
                "dapr state read failed with status {}",
                response.status()
            )));
        }

        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        if value.is_null() {
            return Ok(Vec::new());
        }
        serde_json::from_value::<Vec<InputItem>>(value)
            .map_err(|error| AgentsError::message(error.to_string()))
    }

    async fn write_all_items(&self, items: &[InputItem]) -> Result<()> {
        let mut state = json!([{
            "key": self.key(),
            "value": items,
        }]);
        if let Some(ttl_seconds) = self.ttl_seconds {
            state[0]["metadata"] = json!({ "ttlInSeconds": ttl_seconds.to_string() });
        }

        let response = self
            .client
            .post(self.state_url())
            .json(&state)
            .send()
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;

        if !response.status().is_success() {
            return Err(AgentsError::message(format!(
                "dapr state write failed with status {}",
                response.status()
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl Session for DaprSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn session_settings(&self) -> Option<&SessionSettings> {
        self.session_settings.as_ref()
    }

    async fn get_items_with_limit(&self, limit: Option<usize>) -> Result<Vec<InputItem>> {
        let items = self.read_all_items().await?;
        Ok(apply_session_limit(
            &items,
            limit.or_else(|| self.session_settings.as_ref().and_then(|s| s.limit)),
        ))
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let mut existing = self.read_all_items().await?;
        existing.extend(items);
        self.write_all_items(&existing).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        let mut items = self.read_all_items().await?;
        let popped = items.pop();
        self.write_all_items(&items).await?;
        Ok(popped)
    }

    async fn clear_session(&self) -> Result<()> {
        let response = self
            .client
            .delete(format!("{}/{}", self.state_url(), self.key()))
            .send()
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;

        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            return Err(AgentsError::message(format!(
                "dapr state delete failed with status {}",
                response.status()
            )));
        }
        Ok(())
    }
}

fn normalize_dapr_address(address: &str) -> String {
    if address.starts_with("http://") || address.starts_with("https://") {
        address.to_owned()
    } else {
        format!("http://{address}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_address() {
        assert_eq!(
            normalize_dapr_address("localhost:3500"),
            "http://localhost:3500"
        );
        assert_eq!(
            normalize_dapr_address("https://example.com"),
            "https://example.com"
        );
    }
}
