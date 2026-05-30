use agents_core::{
    AgentsError, InputItem, Result, Session, SessionSettings, memory::resolve_session_limit,
};
use async_trait::async_trait;
use reqwest::StatusCode;
use serde_json::{Value, json};

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

    async fn read_all_values(&self) -> Result<Vec<Value>> {
        self.read_all_values_with_strictness(false).await
    }

    async fn read_all_values_for_update(&self) -> Result<Vec<Value>> {
        self.read_all_values_with_strictness(true).await
    }

    async fn read_all_values_with_strictness(&self, strict: bool) -> Result<Vec<Value>> {
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

        let value: serde_json::Value = match response.json().await {
            Ok(value) => value,
            Err(_) if !strict => return Ok(Vec::new()),
            Err(error) => {
                return Err(AgentsError::message(format!(
                    "stored Dapr session messages are not valid JSON and cannot be safely updated: {error}"
                )));
            }
        };
        decode_dapr_state_values(value, strict)
    }

    async fn read_all_items_for_update(&self) -> Result<Vec<InputItem>> {
        self.read_all_values_for_update()
            .await?
            .into_iter()
            .map(parse_dapr_input_item)
            .collect::<Result<Vec<_>>>()
    }

    async fn write_all_values(&self, values: &[Value]) -> Result<()> {
        let mut state = json!([{
            "key": self.key(),
            "value": values,
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

    async fn write_all_items(&self, items: &[InputItem]) -> Result<()> {
        let values = items
            .iter()
            .map(|item| {
                serde_json::to_value(item).map_err(|error| AgentsError::message(error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        self.write_all_values(&values).await
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
        let resolved_limit = resolve_session_limit(limit, self.session_settings());
        if matches!(resolved_limit, Some(0)) {
            return Ok(Vec::new());
        }
        let mut values = self.read_all_values().await?;
        if let Some(limit) = resolved_limit {
            let start = values.len().saturating_sub(limit);
            values = values.split_off(start);
        }
        Ok(values
            .into_iter()
            .filter_map(|value| parse_dapr_input_item(value).ok())
            .collect())
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let mut existing = self.read_all_items_for_update().await?;
        existing.extend(items);
        self.write_all_items(&existing).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        loop {
            let mut values = self.read_all_values_for_update().await?;
            let Some(popped) = values.pop() else {
                return Ok(None);
            };
            self.write_all_values(&values).await?;
            if let Ok(item) = parse_dapr_input_item(popped) {
                return Ok(Some(item));
            }
        }
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

fn parse_dapr_input_item(value: Value) -> Result<InputItem> {
    match value {
        Value::String(raw) => serde_json::from_str::<InputItem>(&raw)
            .map_err(|error| AgentsError::message(error.to_string())),
        other => serde_json::from_value::<InputItem>(other)
            .map_err(|error| AgentsError::message(error.to_string())),
    }
}

fn decode_dapr_state_values(value: Value, strict: bool) -> Result<Vec<Value>> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    match value {
        Value::Array(values) => Ok(values),
        _ if strict => Err(AgentsError::message(
            "stored Dapr session messages must be a JSON list and cannot be safely updated",
        )),
        _ => Ok(Vec::new()),
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

    #[test]
    fn dapr_item_parser_accepts_direct_and_encoded_items() {
        let item = InputItem::from("valid");
        let direct = serde_json::to_value(&item).expect("item should serialize");
        let encoded = Value::String(serde_json::to_string(&item).expect("item should serialize"));

        assert_eq!(parse_dapr_input_item(direct).ok(), Some(item.clone()));
        assert_eq!(parse_dapr_input_item(encoded).ok(), Some(item));
        assert!(parse_dapr_input_item(Value::String("not json".to_owned())).is_err());
    }

    #[test]
    fn dapr_state_decoder_rejects_corrupt_aggregate_state_for_updates() {
        assert_eq!(
            decode_dapr_state_values(json!({"unexpected": "object"}), false)
                .expect("non-strict reads should tolerate corrupt aggregate state"),
            Vec::<Value>::new()
        );

        let error = decode_dapr_state_values(json!({"unexpected": "object"}), true)
            .expect_err("strict updates should reject corrupt aggregate state");

        assert!(
            error
                .to_string()
                .contains("stored Dapr session messages must be a JSON list")
        );
    }
}
