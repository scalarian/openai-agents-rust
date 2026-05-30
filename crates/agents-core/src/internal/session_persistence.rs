use crate::errors::Result;
use crate::exceptions::UserError;
use crate::items::{InputItem, RunItem};
use crate::memory::resolve_session_limit;
use crate::run_config::ReasoningItemIdPolicy;
use crate::run_config::RunConfig;
use crate::session::Session;
use crate::tracing::{custom_span, get_trace_provider};
use std::collections::HashMap;
use uuid::Uuid;

const EMPTY_JSON_OBJECT_SIDECAR_PREFIX: &str = "__agents_internal_empty_object_identity_";

pub(crate) async fn prepare_input_with_session(
    input: &[InputItem],
    config: &RunConfig,
    session: &(dyn Session + Sync),
) -> Result<(Vec<InputItem>, Vec<InputItem>, Vec<InputItem>)> {
    let provider = get_trace_provider();
    let mut span = custom_span(
        "session.prepare_input",
        std::collections::BTreeMap::from([(
            "session_id".to_owned(),
            serde_json::Value::String(session.session_id().to_owned()),
        )]),
    );
    provider.start_span(&mut span, true);
    let resolved_settings = session
        .session_settings()
        .cloned()
        .unwrap_or_default()
        .resolve(config.session_settings.as_ref());
    let history = session
        .get_items_with_limit(resolve_session_limit(None, Some(&resolved_settings)))
        .await?;
    let original_input = input.to_vec();
    let (mut prepared, mut session_input_items) = if let Some(callback) =
        &config.session_input_callback
    {
        let mut history_for_callback = history.clone();
        let mut new_items_for_callback = original_input.clone();
        let mut generated_empty_json_sidecars = HashMap::new();
        seed_empty_json_object_identity(
            &mut history_for_callback,
            &mut generated_empty_json_sidecars,
        );
        seed_empty_json_object_identity(
            &mut new_items_for_callback,
            &mut generated_empty_json_sidecars,
        );
        let mut history_refs =
            build_reference_map(&history_for_callback, &generated_empty_json_sidecars);
        let mut new_refs =
            build_reference_map(&new_items_for_callback, &generated_empty_json_sidecars);
        let mut history_counts =
            build_frequency_map(&history_for_callback, &generated_empty_json_sidecars);
        let mut new_counts =
            build_frequency_map(&new_items_for_callback, &generated_empty_json_sidecars);
        let combined = callback(history_for_callback, new_items_for_callback).await?;
        let mut session_input_items = Vec::new();

        for item in &combined {
            let key = session_item_key(item, &generated_empty_json_sidecars);
            let normalized_item =
                strip_empty_json_object_identity(item.clone(), &generated_empty_json_sidecars);
            if consume_reference(&mut new_refs, item, &generated_empty_json_sidecars) {
                decrement_count(&mut new_counts, &key);
                session_input_items.push(normalized_item.clone());
                continue;
            }
            if consume_reference(&mut history_refs, item, &generated_empty_json_sidecars) {
                decrement_count(&mut history_counts, &key);
                continue;
            }
            if prefers_new_frequency_match(item) {
                if new_counts.get(&key).copied().unwrap_or_default() > 0 {
                    decrement_count(&mut new_counts, &key);
                    session_input_items.push(normalized_item.clone());
                    continue;
                }
                if history_counts.get(&key).copied().unwrap_or_default() > 0 {
                    decrement_count(&mut history_counts, &key);
                    continue;
                }
            } else {
                if history_counts.get(&key).copied().unwrap_or_default() > 0 {
                    decrement_count(&mut history_counts, &key);
                    continue;
                }
                if new_counts.get(&key).copied().unwrap_or_default() > 0 {
                    decrement_count(&mut new_counts, &key);
                    session_input_items.push(normalized_item.clone());
                    continue;
                }
            }

            session_input_items.push(normalized_item);
        }

        (
            combined
                .into_iter()
                .map(|item| strip_empty_json_object_identity(item, &generated_empty_json_sidecars))
                .collect(),
            session_input_items,
        )
    } else {
        let mut prepared = history;
        prepared.extend(original_input.clone());
        (prepared, original_input.clone())
    };
    if prepared.is_empty() && config.session_input_callback.is_none() {
        prepared = original_input.clone();
        session_input_items = original_input.clone();
    }
    provider.finish_span(&mut span, true);
    Ok((prepared, original_input, session_input_items))
}

pub(crate) async fn save_result_to_session(
    session: &(dyn Session + Sync),
    original_input: &[InputItem],
    new_items: &[RunItem],
    reasoning_item_id_policy: ReasoningItemIdPolicy,
) -> Result<usize> {
    let provider = get_trace_provider();
    let mut span = custom_span(
        "session.save_result",
        std::collections::BTreeMap::from([(
            "session_id".to_owned(),
            serde_json::Value::String(session.session_id().to_owned()),
        )]),
    );
    provider.start_span(&mut span, true);
    let preserve_openai_conversation_items = session.preserves_openai_conversation_item_identity();
    let persistence_reasoning_policy = if preserve_openai_conversation_items {
        ReasoningItemIdPolicy::Preserve
    } else {
        reasoning_item_id_policy
    };
    let mut items = original_input.to_vec();
    items.extend(new_items.iter().filter_map(|run_item| {
        crate::internal::items::run_item_to_input_item(run_item, persistence_reasoning_policy)
    }));
    let count = items.len();
    if preserve_openai_conversation_items {
        items = items
            .into_iter()
            .filter(|item| !is_unpersistable_openai_conversation_reasoning_item(item))
            .map(sanitize_openai_conversation_item)
            .collect();
    }
    if !items.is_empty() {
        session.add_items(items).await?;
    }
    provider.finish_span(&mut span, true);
    Ok(count)
}

fn sanitize_openai_conversation_item(item: InputItem) -> InputItem {
    let InputItem::Json {
        value: serde_json::Value::Object(mut object),
    } = item
    else {
        return item;
    };

    let item_type = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    if item_type.as_deref() != Some("reasoning")
        && !openai_conversation_item_requires_id(item_type.as_deref())
    {
        object.remove("id");
    }
    object.remove("provider_data");

    InputItem::Json {
        value: serde_json::Value::Object(object),
    }
}

fn openai_conversation_item_requires_id(item_type: Option<&str>) -> bool {
    matches!(
        item_type,
        Some(
            "file_search_call"
                | "web_search_call"
                | "computer_call"
                | "code_interpreter_call"
                | "image_generation_call"
                | "local_shell_call"
                | "local_shell_call_output"
                | "mcp_list_tools"
                | "mcp_approval_request"
                | "mcp_call"
                | "item_reference"
        )
    )
}

fn is_unpersistable_openai_conversation_reasoning_item(item: &InputItem) -> bool {
    let InputItem::Json {
        value: serde_json::Value::Object(object),
    } = item
    else {
        return false;
    };
    if object.get("type").and_then(serde_json::Value::as_str) != Some("reasoning") {
        return false;
    }
    !truthy_json_field(object.get("id")) && !truthy_json_field(object.get("encrypted_content"))
}

fn truthy_json_field(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::String(text)) => !text.is_empty(),
        Some(serde_json::Value::Array(values)) => !values.is_empty(),
        Some(serde_json::Value::Object(values)) => !values.is_empty(),
        Some(serde_json::Value::Bool(value)) => *value,
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .map(|value| value != 0)
            .or_else(|| number.as_u64().map(|value| value != 0))
            .or_else(|| number.as_f64().map(|value| value != 0.0))
            .unwrap_or(true),
        Some(serde_json::Value::Null) | None => false,
    }
}

pub(crate) fn validate_session_conversation_settings(
    config: &RunConfig,
    session: &(dyn Session + Sync),
) -> Result<()> {
    if session.conversation_session().is_some() {
        return Ok(());
    }

    if config.conversation_id.is_none()
        && config.previous_response_id.is_none()
        && !config.auto_previous_response_id
    {
        return Ok(());
    }

    Err(UserError {
        message: "Session persistence cannot be combined with conversation_id, previous_response_id, or auto_previous_response_id.".to_owned(),
    }
    .into())
}

fn build_reference_map(
    items: &[InputItem],
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> HashMap<String, Vec<InputItemIdentity>> {
    let mut refs = HashMap::new();
    for item in items {
        let Some(identity) = input_item_identity(item, generated_empty_json_sidecars) else {
            continue;
        };
        refs.entry(session_item_key(item, generated_empty_json_sidecars))
            .or_insert_with(Vec::new)
            .push(identity);
    }
    refs
}

fn consume_reference(
    refs: &mut HashMap<String, Vec<InputItemIdentity>>,
    item: &InputItem,
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> bool {
    let Some(identity) = input_item_identity(item, generated_empty_json_sidecars) else {
        return false;
    };
    let key = session_item_key(item, generated_empty_json_sidecars);
    let Some(identities) = refs.get_mut(&key) else {
        return false;
    };
    let Some(index) = identities
        .iter()
        .position(|candidate| *candidate == identity)
    else {
        return false;
    };
    identities.remove(index);
    if identities.is_empty() {
        refs.remove(&key);
    }
    true
}

fn build_frequency_map(
    items: &[InputItem],
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for item in items {
        let key = session_item_key(item, generated_empty_json_sidecars);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn decrement_count(counts: &mut HashMap<String, usize>, key: &str) {
    if let Some(count) = counts.get_mut(key) {
        *count = count.saturating_sub(1);
    }
}

fn session_item_key(
    item: &InputItem,
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> String {
    serde_json::to_string(&strip_empty_json_object_identity(
        item.clone(),
        generated_empty_json_sidecars,
    ))
    .expect("input items should serialize")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputItemIdentity {
    Text(u64),
    JsonString(u64),
    JsonArray(u64),
    EmptyJsonObject(Uuid),
    JsonObject(u64),
}

fn input_item_identity(
    item: &InputItem,
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> Option<InputItemIdentity> {
    match item {
        InputItem::Text { text } => Some(InputItemIdentity::Text(stable_hash(text))),
        InputItem::Json { value } => json_value_identity(value, generated_empty_json_sidecars),
    }
}

fn json_value_identity(
    value: &serde_json::Value,
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> Option<InputItemIdentity> {
    match value {
        serde_json::Value::String(text) => Some(InputItemIdentity::JsonString(stable_hash(text))),
        serde_json::Value::Array(values) => Some(InputItemIdentity::JsonArray(stable_hash_json(
            &serde_json::Value::Array(values.clone()),
        ))),
        serde_json::Value::Object(map) => {
            if let Some(identity) = empty_json_object_identity(map, generated_empty_json_sidecars) {
                return Some(InputItemIdentity::EmptyJsonObject(identity));
            }
            if map.is_empty() {
                return None;
            }
            Some(InputItemIdentity::JsonObject(stable_hash_json(
                &serde_json::Value::Object(map.clone()),
            )))
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => None,
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn stable_hash_json(value: &serde_json::Value) -> u64 {
    stable_hash(
        &serde_json::to_string(value).expect("json identity values should serialize consistently"),
    )
}

fn seed_empty_json_object_identity(
    items: &mut [InputItem],
    generated_empty_json_sidecars: &mut HashMap<String, Uuid>,
) {
    for item in items {
        let InputItem::Json {
            value: serde_json::Value::Object(map),
        } = item
        else {
            continue;
        };
        if !map.is_empty() {
            continue;
        }

        let identity = Uuid::new_v4();
        let sentinel_key = format!("{EMPTY_JSON_OBJECT_SIDECAR_PREFIX}{identity}__");
        map.insert(
            sentinel_key.clone(),
            serde_json::Value::String(sentinel_key.clone()),
        );
        generated_empty_json_sidecars.insert(sentinel_key, identity);
    }
}

fn empty_json_object_identity(
    map: &serde_json::Map<String, serde_json::Value>,
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> Option<Uuid> {
    if map.len() != 1 {
        return None;
    }

    let (key, value) = map.iter().next()?;
    let identity = *generated_empty_json_sidecars.get(key)?;
    let serde_json::Value::String(stored_key) = value else {
        return None;
    };
    if stored_key != key {
        return None;
    }

    Some(identity)
}

fn strip_empty_json_object_identity(
    item: InputItem,
    generated_empty_json_sidecars: &HashMap<String, Uuid>,
) -> InputItem {
    match item {
        InputItem::Json {
            value: serde_json::Value::Object(mut map),
        } => {
            if let Some((key, _)) = map.iter().next().filter(|_| {
                empty_json_object_identity(&map, generated_empty_json_sidecars).is_some()
            }) {
                let key = key.clone();
                map.remove(&key);
            }
            InputItem::Json {
                value: serde_json::Value::Object(map),
            }
        }
        other => other,
    }
}

fn prefers_new_frequency_match(item: &InputItem) -> bool {
    match item {
        InputItem::Json {
            value:
                serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_),
        } => true,
        InputItem::Json {
            value: serde_json::Value::Object(map),
        } => map.is_empty(),
        InputItem::Text { .. } | InputItem::Json { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputItem;
    use crate::memory::{OpenAIConversationAwareSession, OpenAIConversationSessionState};
    use crate::session::{MemorySession, Session};
    use async_trait::async_trait;
    use futures::FutureExt;
    use serde_json::{Value, json};

    struct OpenAIConversationLikeSession {
        inner: MemorySession,
    }

    impl OpenAIConversationLikeSession {
        fn new() -> Self {
            Self {
                inner: MemorySession::new("openai-conversation"),
            }
        }
    }

    #[async_trait]
    impl Session for OpenAIConversationLikeSession {
        fn session_id(&self) -> &str {
            self.inner.session_id()
        }

        fn preserves_openai_conversation_item_identity(&self) -> bool {
            true
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

        fn conversation_session(&self) -> Option<&dyn OpenAIConversationAwareSession> {
            Some(self)
        }
    }

    #[async_trait]
    impl OpenAIConversationAwareSession for OpenAIConversationLikeSession {
        async fn load_openai_conversation_state(&self) -> Result<OpenAIConversationSessionState> {
            Ok(OpenAIConversationSessionState {
                conversation_id: Some("conv-test".to_owned()),
                previous_response_id: None,
                auto_previous_response_id: true,
            })
        }

        async fn save_openai_conversation_state(
            &self,
            _state: OpenAIConversationSessionState,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn prepares_input_by_prefixing_session_history() {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::from("history")])
            .await
            .expect("history should be added");

        let (prepared, original_input, session_input_items) =
            prepare_input_with_session(&[InputItem::from("new")], &RunConfig::default(), &session)
                .await
                .expect("prepared input should build");

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].as_text(), Some("history"));
        assert_eq!(prepared[1].as_text(), Some("new"));
        assert_eq!(original_input.len(), 1);
        assert_eq!(original_input[0].as_text(), Some("new"));
        assert_eq!(session_input_items.len(), 1);
        assert_eq!(session_input_items[0].as_text(), Some("new"));
    }

    #[tokio::test]
    async fn saves_original_input_and_generated_items_to_session() {
        let session = MemorySession::new("session");
        let count = save_result_to_session(
            &session,
            &[InputItem::from("hello")],
            &[RunItem::Reasoning {
                text: "thinking".to_owned(),
            }],
            ReasoningItemIdPolicy::Preserve,
        )
        .await
        .expect("session should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, 2);
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn save_result_to_session_omits_reasoning_when_policy_requests_it() {
        let session = MemorySession::new("session");

        let count = save_result_to_session(
            &session,
            &[InputItem::from("hello")],
            &[RunItem::Reasoning {
                text: "sensitive chain of thought".to_owned(),
            }],
            ReasoningItemIdPolicy::Omit,
        )
        .await
        .expect("session should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, 1);
        assert_eq!(items, vec![InputItem::from("hello")]);
    }

    #[tokio::test]
    async fn save_result_to_openai_conversation_keeps_reasoning_id_and_strips_provider_data() {
        let session = OpenAIConversationLikeSession::new();

        let count = save_result_to_session(
            &session,
            &[],
            &[RunItem::MessageOutput {
                content: crate::items::OutputItem::Json {
                    value: json!({
                        "type": "reasoning",
                        "id": "rs_openai_conversation",
                        "summary": [],
                        "provider_data": {"server": "metadata"}
                    }),
                },
            }],
            ReasoningItemIdPolicy::Omit,
        )
        .await
        .expect("conversation session should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, 1);
        assert_eq!(items.len(), 1);
        let InputItem::Json { value } = &items[0] else {
            panic!("expected json reasoning item");
        };
        assert_eq!(value.get("type").and_then(Value::as_str), Some("reasoning"));
        assert_eq!(
            value.get("id").and_then(Value::as_str),
            Some("rs_openai_conversation")
        );
        assert!(value.get("provider_data").is_none());
    }

    #[tokio::test]
    async fn save_result_to_openai_conversation_drops_unpersistable_reasoning_item() {
        let session = OpenAIConversationLikeSession::new();

        let count = save_result_to_session(
            &session,
            &[],
            &[RunItem::Reasoning {
                text: "text-only reasoning".to_owned(),
            }],
            ReasoningItemIdPolicy::Omit,
        )
        .await
        .expect("conversation session should process item");

        assert_eq!(count, 1);
        assert!(
            session
                .get_items()
                .await
                .expect("items should load")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn save_result_to_openai_conversation_keeps_reasoning_encrypted_content() {
        let session = OpenAIConversationLikeSession::new();

        let count = save_result_to_session(
            &session,
            &[],
            &[RunItem::MessageOutput {
                content: crate::items::OutputItem::Json {
                    value: json!({
                        "type": "reasoning",
                        "summary": [],
                        "encrypted_content": "encrypted"
                    }),
                },
            }],
            ReasoningItemIdPolicy::Omit,
        )
        .await
        .expect("conversation session should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, 1);
        assert_eq!(items.len(), 1);
        let InputItem::Json { value } = &items[0] else {
            panic!("expected json reasoning item");
        };
        assert_eq!(
            value.get("encrypted_content").and_then(Value::as_str),
            Some("encrypted")
        );
    }

    #[tokio::test]
    async fn save_result_to_openai_conversation_preserves_required_hosted_item_ids() {
        let required_id_types = [
            "file_search_call",
            "web_search_call",
            "computer_call",
            "code_interpreter_call",
            "image_generation_call",
            "local_shell_call",
            "local_shell_call_output",
            "mcp_list_tools",
            "mcp_approval_request",
            "mcp_call",
            "item_reference",
        ];
        let session = OpenAIConversationLikeSession::new();
        let new_items = required_id_types
            .iter()
            .map(|item_type| RunItem::MessageOutput {
                content: OutputItem::Json {
                    value: json!({
                        "type": item_type,
                        "id": format!("{item_type}_abc123"),
                        "status": "completed",
                        "provider_data": {"server": "metadata"}
                    }),
                },
            })
            .collect::<Vec<_>>();

        let count =
            save_result_to_session(&session, &[], &new_items, ReasoningItemIdPolicy::Preserve)
                .await
                .expect("conversation session should save hosted items");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, required_id_types.len());
        assert_eq!(items.len(), required_id_types.len());
        for (item, item_type) in items.iter().zip(required_id_types) {
            let InputItem::Json { value } = item else {
                panic!("expected json hosted item");
            };
            assert_eq!(value.get("type").and_then(Value::as_str), Some(item_type));
            assert_eq!(
                value.get("id").and_then(Value::as_str),
                Some(format!("{item_type}_abc123").as_str())
            );
            assert!(value.get("provider_data").is_none());
        }
    }

    #[tokio::test]
    async fn save_result_to_openai_conversation_strips_optional_ids() {
        let optional_id_items = [
            json!({
                "type": "message",
                "id": "msg_abc",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "hi"}]
            }),
            json!({
                "type": "function_call",
                "id": "fc_abc",
                "call_id": "call_abc",
                "name": "get_weather",
                "arguments": "{}"
            }),
            json!({
                "type": "function_call_output",
                "id": "out_abc",
                "call_id": "call_abc",
                "output": "{}"
            }),
            json!({
                "type": "tool_search_call",
                "id": "ts_abc",
                "status": "completed"
            }),
        ];
        let session = OpenAIConversationLikeSession::new();
        let new_items = optional_id_items
            .into_iter()
            .map(|value| RunItem::MessageOutput {
                content: OutputItem::Json { value },
            })
            .collect::<Vec<_>>();

        let count =
            save_result_to_session(&session, &[], &new_items, ReasoningItemIdPolicy::Preserve)
                .await
                .expect("conversation session should save optional-id items");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, new_items.len());
        assert_eq!(items.len(), new_items.len());
        for item in items {
            let InputItem::Json { value } = item else {
                panic!("expected json item");
            };
            assert!(value.get("id").is_none());
        }
    }

    #[tokio::test]
    async fn session_input_callback_returns_transformed_items_for_persistence() {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::from("history")])
            .await
            .expect("history should be added");
        let config = RunConfig {
            session_input_callback: Some(std::sync::Arc::new(|history, mut new_items| {
                async move {
                    let mut combined = history;
                    let transformed = InputItem::from("[redacted]");
                    new_items[0] = transformed.clone();
                    combined.extend(new_items);
                    Ok(combined)
                }
                .boxed()
            })),
            ..RunConfig::default()
        };

        let (prepared, _original_input, session_items) =
            prepare_input_with_session(&[InputItem::from("secret")], &config, &session)
                .await
                .expect("prepared input should build");

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].as_text(), Some("history"));
        assert_eq!(prepared[1].as_text(), Some("[redacted]"));
        assert_eq!(session_items, vec![InputItem::from("[redacted]")]);
    }

    #[tokio::test]
    async fn session_input_callback_preserves_duplicate_value_provenance() {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::from("same")])
            .await
            .expect("history should be added");
        let config = RunConfig {
            session_input_callback: Some(std::sync::Arc::new(|mut history, mut new_items| {
                async move {
                    let history_item = history.remove(0);
                    let _dropped_new_item = new_items.remove(0);
                    Ok(vec![history_item])
                }
                .boxed()
            })),
            ..RunConfig::default()
        };

        let (prepared, _original_input, session_items) =
            prepare_input_with_session(&[InputItem::from("same")], &config, &session)
                .await
                .expect("prepared input should build");

        assert_eq!(prepared, vec![InputItem::from("same")]);
        assert_eq!(session_items, vec![InputItem::from("same")]);
    }

    #[tokio::test]
    async fn session_input_callback_preserves_history_side_empty_json_object_provenance() {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::Json { value: json!({}) }])
            .await
            .expect("history should be added");
        let config = RunConfig {
            session_input_callback: Some(std::sync::Arc::new(|mut history, mut new_items| {
                async move {
                    let history_item = history.remove(0);
                    let _dropped_new_item = new_items.remove(0);
                    Ok(vec![history_item])
                }
                .boxed()
            })),
            ..RunConfig::default()
        };

        let (prepared, _original_input, session_items) =
            prepare_input_with_session(&[InputItem::Json { value: json!({}) }], &config, &session)
                .await
                .expect("prepared input should build");

        assert_eq!(prepared, vec![InputItem::Json { value: json!({}) }]);
        assert!(session_items.is_empty());
    }

    async fn assert_session_input_callback_preserves_duplicate_json_value_provenance(value: Value) {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::Json {
                value: value.clone(),
            }])
            .await
            .expect("history should be added");
        let config = RunConfig {
            session_input_callback: Some(std::sync::Arc::new(move |_history, mut new_items| {
                async move { Ok(vec![new_items.remove(0)]) }.boxed()
            })),
            ..RunConfig::default()
        };

        let (prepared, _original_input, session_items) = prepare_input_with_session(
            &[InputItem::Json {
                value: value.clone(),
            }],
            &config,
            &session,
        )
        .await
        .expect("prepared input should build");

        assert_eq!(
            prepared,
            vec![InputItem::Json {
                value: value.clone(),
            }]
        );
        assert_eq!(session_items, vec![InputItem::Json { value }]);
    }

    #[tokio::test]
    async fn session_input_callback_preserves_duplicate_json_object_provenance() {
        assert_session_input_callback_preserves_duplicate_json_value_provenance(json!({
            "type": "message",
            "content": "same"
        }))
        .await;
    }

    #[tokio::test]
    async fn session_input_callback_preserves_duplicate_empty_json_object_provenance() {
        assert_session_input_callback_preserves_duplicate_json_value_provenance(json!({})).await;
    }

    #[tokio::test]
    async fn session_input_callback_preserves_user_payload_matching_old_empty_object_sidecar() {
        let sidecar_key = format!("{EMPTY_JSON_OBJECT_SIDECAR_PREFIX}123__");
        let payload = json!({
            sidecar_key.clone(): sidecar_key.clone(),
        });
        let session = MemorySession::new("session");
        let config = RunConfig {
            session_input_callback: Some(std::sync::Arc::new(|_history, mut new_items| {
                async move { Ok(vec![new_items.remove(0)]) }.boxed()
            })),
            ..RunConfig::default()
        };

        let (prepared, _original_input, session_items) = prepare_input_with_session(
            &[InputItem::Json {
                value: payload.clone(),
            }],
            &config,
            &session,
        )
        .await
        .expect("prepared input should build");

        assert_eq!(
            prepared,
            vec![InputItem::Json {
                value: payload.clone(),
            }]
        );
        assert_eq!(session_items, vec![InputItem::Json { value: payload }]);
    }

    #[tokio::test]
    async fn session_input_callback_preserves_duplicate_json_number_provenance() {
        assert_session_input_callback_preserves_duplicate_json_value_provenance(json!(42)).await;
    }

    #[tokio::test]
    async fn session_input_callback_preserves_duplicate_json_bool_provenance() {
        assert_session_input_callback_preserves_duplicate_json_value_provenance(json!(true)).await;
    }

    #[tokio::test]
    async fn session_input_callback_preserves_duplicate_json_null_provenance() {
        assert_session_input_callback_preserves_duplicate_json_value_provenance(Value::Null).await;
    }

    #[test]
    fn empty_json_object_sidecars_are_unique_and_strip_cleanly() {
        let mut items = vec![
            InputItem::Json { value: json!({}) },
            InputItem::Json { value: json!({}) },
        ];
        let mut generated_empty_json_sidecars = HashMap::new();

        seed_empty_json_object_identity(&mut items, &mut generated_empty_json_sidecars);

        let left_identity = input_item_identity(&items[0], &generated_empty_json_sidecars);
        let right_identity = input_item_identity(&items[1], &generated_empty_json_sidecars);
        assert_ne!(left_identity, right_identity);
        assert_eq!(
            session_item_key(&items[0], &generated_empty_json_sidecars),
            r#"{"type":"json","value":{}}"#
        );
        assert_eq!(
            session_item_key(&items[1], &generated_empty_json_sidecars),
            r#"{"type":"json","value":{}}"#
        );
        assert_eq!(
            strip_empty_json_object_identity(items[0].clone(), &generated_empty_json_sidecars),
            InputItem::Json { value: json!({}) }
        );
        assert_eq!(
            strip_empty_json_object_identity(items[1].clone(), &generated_empty_json_sidecars),
            InputItem::Json { value: json!({}) }
        );
    }
}
