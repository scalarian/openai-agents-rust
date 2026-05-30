use std::sync::Arc;

use agents_core::{
    AgentsError, InputItem, Result, Session, SessionSettings, memory::resolve_session_limit,
};
use async_trait::async_trait;
use mongodb::{
    Client, Collection, IndexModel,
    bson::{Bson, DateTime, Document, doc},
    options::{IndexOptions, ReturnDocument},
};
use tokio::sync::OnceCell;

/// MongoDB-backed session storage using ordered message documents per session.
#[derive(Clone, Debug)]
pub struct MongoDBSession {
    session_id: String,
    pub database: String,
    pub sessions_collection: String,
    pub messages_collection: String,
    session_settings: Option<SessionSettings>,
    client: Client,
    indexes_ready: Arc<OnceCell<()>>,
}

impl MongoDBSession {
    pub fn new(session_id: impl Into<String>, client: Client, database: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            database: database.into(),
            sessions_collection: "agent_sessions".to_owned(),
            messages_collection: "agent_messages".to_owned(),
            session_settings: Some(SessionSettings::default()),
            client,
            indexes_ready: Arc::new(OnceCell::new()),
        }
    }

    pub async fn from_uri(
        session_id: impl Into<String>,
        uri: impl AsRef<str>,
        database: impl Into<String>,
    ) -> Result<Self> {
        let client = Client::with_uri_str(uri)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        Ok(Self::new(session_id, client, database))
    }

    pub fn with_collections(
        mut self,
        sessions_collection: impl Into<String>,
        messages_collection: impl Into<String>,
    ) -> Self {
        self.sessions_collection = sessions_collection.into();
        self.messages_collection = messages_collection.into();
        self.indexes_ready = Arc::new(OnceCell::new());
        self
    }

    pub fn with_session_settings(mut self, session_settings: SessionSettings) -> Self {
        self.session_settings = Some(session_settings);
        self
    }

    pub async fn ping(&self) -> bool {
        self.client
            .database("admin")
            .run_command(doc! { "ping": 1 })
            .await
            .is_ok()
    }

    fn sessions(&self) -> Collection<Document> {
        self.client
            .database(&self.database)
            .collection::<Document>(&self.sessions_collection)
    }

    fn messages(&self) -> Collection<Document> {
        self.client
            .database(&self.database)
            .collection::<Document>(&self.messages_collection)
    }

    async fn ensure_indexes(&self) -> Result<()> {
        self.indexes_ready
            .get_or_try_init(|| async {
                self.sessions()
                    .create_index(
                        IndexModel::builder()
                            .keys(doc! { "session_id": 1 })
                            .options(IndexOptions::builder().unique(true).build())
                            .build(),
                    )
                    .await
                    .map_err(|error| AgentsError::message(error.to_string()))?;

                self.messages()
                    .create_index(
                        IndexModel::builder()
                            .keys(doc! { "session_id": 1, "seq": 1 })
                            .build(),
                    )
                    .await
                    .map_err(|error| AgentsError::message(error.to_string()))?;
                Ok::<(), AgentsError>(())
            })
            .await?;
        Ok(())
    }

    async fn reserve_sequence_block(&self, len: usize) -> Result<i64> {
        let now = DateTime::now();
        let len = len as i64;
        let doc = self
            .sessions()
            .find_one_and_update(
                doc! { "session_id": &self.session_id },
                doc! {
                    "$setOnInsert": {
                        "session_id": &self.session_id,
                        "created_at": now,
                    },
                    "$set": { "updated_at": now },
                    "$inc": { "_seq": len },
                },
            )
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        let next_seq = doc.as_ref().and_then(sequence_from_document).unwrap_or(len);
        Ok(next_seq - len)
    }
}

#[async_trait]
impl Session for MongoDBSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn session_settings(&self) -> Option<&SessionSettings> {
        self.session_settings.as_ref()
    }

    async fn get_items_with_limit(&self, limit: Option<usize>) -> Result<Vec<InputItem>> {
        self.ensure_indexes().await?;
        let resolved_limit = resolve_session_limit(limit, self.session_settings());
        if matches!(resolved_limit, Some(0)) {
            return Ok(Vec::new());
        }

        let mut cursor = match resolved_limit {
            Some(limit) => {
                self.messages()
                    .find(doc! { "session_id": &self.session_id })
                    .sort(doc! { "seq": -1 })
                    .limit(limit as i64)
                    .await
            }
            None => {
                self.messages()
                    .find(doc! { "session_id": &self.session_id })
                    .sort(doc! { "seq": 1 })
                    .await
            }
        }
        .map_err(|error| AgentsError::message(error.to_string()))?;

        let mut items = Vec::new();
        while cursor
            .advance()
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?
        {
            let doc = cursor
                .deserialize_current()
                .map_err(|error| AgentsError::message(error.to_string()))?;
            if let Some(item) = parse_mongodb_input_item(&doc) {
                items.push(item);
            }
        }
        if resolved_limit.is_some() {
            items.reverse();
        }
        Ok(items)
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        self.ensure_indexes().await?;
        let start_seq = self.reserve_sequence_block(items.len()).await?;
        let docs = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let message_data = serde_json::to_string(item)
                    .map_err(|error| AgentsError::message(error.to_string()))?;
                Ok(doc! {
                    "session_id": &self.session_id,
                    "seq": start_seq + index as i64,
                    "message_data": message_data,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        self.messages()
            .insert_many(docs)
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        Ok(())
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        self.ensure_indexes().await?;
        loop {
            let doc = self
                .messages()
                .find_one_and_delete(doc! { "session_id": &self.session_id })
                .sort(doc! { "seq": -1 })
                .await
                .map_err(|error| AgentsError::message(error.to_string()))?;
            let Some(doc) = doc else {
                return Ok(None);
            };
            if let Some(item) = parse_mongodb_input_item(&doc) {
                return Ok(Some(item));
            }
        }
    }

    async fn clear_session(&self) -> Result<()> {
        self.ensure_indexes().await?;
        self.messages()
            .delete_many(doc! { "session_id": &self.session_id })
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        self.sessions()
            .delete_one(doc! { "session_id": &self.session_id })
            .await
            .map_err(|error| AgentsError::message(error.to_string()))?;
        Ok(())
    }
}

fn parse_mongodb_input_item(doc: &Document) -> Option<InputItem> {
    let raw = doc.get_str("message_data").ok()?;
    serde_json::from_str::<InputItem>(raw).ok()
}

fn sequence_from_document(doc: &Document) -> Option<i64> {
    match doc.get("_seq") {
        Some(Bson::Int32(value)) => Some(i64::from(*value)),
        Some(Bson::Int64(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mongodb_item_parser_drops_corrupt_documents() {
        let item = InputItem::from("valid");
        let raw = serde_json::to_string(&item).expect("item should serialize");

        assert_eq!(
            parse_mongodb_input_item(&doc! { "message_data": raw }),
            Some(item)
        );
        assert_eq!(
            parse_mongodb_input_item(&doc! { "message_data": "not json" }),
            None
        );
        assert_eq!(parse_mongodb_input_item(&doc! { "other": "value" }), None);
    }

    #[test]
    fn reads_sequence_from_mongodb_numeric_types() {
        assert_eq!(sequence_from_document(&doc! { "_seq": 3 }), Some(3));
        assert_eq!(sequence_from_document(&doc! { "_seq": 4_i64 }), Some(4));
        assert_eq!(sequence_from_document(&doc! { "_seq": "5" }), None);
    }
}
