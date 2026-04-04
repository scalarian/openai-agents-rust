use agents_core::{
    AgentsError, InputItem, Result, Session, SessionSettings, memory::util::apply_session_limit,
};
use async_trait::async_trait;
use redis::AsyncCommands;

/// Redis-backed session storage using a list per session.
#[derive(Clone, Debug)]
pub struct RedisSession {
    session_id: String,
    pub url: String,
    pub key_prefix: String,
    pub ttl_seconds: Option<u64>,
    session_settings: Option<SessionSettings>,
    client: redis::Client,
}

impl RedisSession {
    pub fn new(
        session_id: impl Into<String>,
        client: redis::Client,
        url: impl Into<String>,
        key_prefix: impl Into<String>,
        ttl_seconds: Option<u64>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            url: url.into(),
            key_prefix: key_prefix.into(),
            ttl_seconds,
            session_settings: Some(SessionSettings::default()),
            client,
        }
    }

    pub fn from_url(session_id: impl Into<String>, url: impl Into<String>) -> Result<Self> {
        let url = url.into();
        let client = redis::Client::open(url.clone())
            .map_err(|error| AgentsError::message(error.to_string()))?;
        Ok(Self::new(session_id, client, url, "agents:session:", None))
    }

    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = key_prefix.into();
        self
    }

    pub fn with_ttl_seconds(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self
    }

    fn key(&self) -> String {
        format!("{}{}", self.key_prefix, self.session_id)
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AgentsError::message(error.to_string()))
    }
}

#[async_trait]
impl Session for RedisSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn session_settings(&self) -> Option<&SessionSettings> {
        self.session_settings.as_ref()
    }

    async fn get_items_with_limit(&self, limit: Option<usize>) -> Result<Vec<InputItem>> {
        let mut conn = self.connection().await?;
        let values: Vec<String> = conn
            .lrange(self.key(), 0, -1)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        let items = values
            .into_iter()
            .map(|value| {
                serde_json::from_str::<InputItem>(&value)
                    .map_err(|error| AgentsError::message(error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(apply_session_limit(
            &items,
            limit.or_else(|| self.session_settings.as_ref().and_then(|s| s.limit)),
        ))
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut conn = self.connection().await?;
        let key = self.key();
        let values = items
            .into_iter()
            .map(|item| {
                serde_json::to_string(&item)
                    .map_err(|error| AgentsError::message(error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        let _: usize = conn
            .rpush(&key, values)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        if let Some(ttl_seconds) = self.ttl_seconds {
            let _: bool = conn
                .expire(&key, ttl_seconds as i64)
                .await
                .map_err(|error| AgentsError::message(error.to_string()))?;
        }
        Ok(())
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        let mut conn = self.connection().await?;
        let value: Option<String> = conn
            .rpop(self.key(), None)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        value
            .map(|value| {
                serde_json::from_str::<InputItem>(&value)
                    .map_err(|error| AgentsError::message(error.to_string()))
            })
            .transpose()
    }

    async fn clear_session(&self) -> Result<()> {
        let mut conn = self.connection().await?;
        let _: usize = conn
            .del(self.key())
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_session_key() {
        let client = redis::Client::open("redis://127.0.0.1/").expect("redis client should parse");
        let session = RedisSession::new("session", client, "redis://127.0.0.1/", "test:", None);
        assert_eq!(session.key(), "test:session");
    }
}
