use crate::items::RunItem;
use crate::memory::OpenAIConversationSessionState;
use crate::model::ModelResponse;
use crate::run_config::RunConfig;
use crate::run_state::RunState;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub(crate) struct OpenAIServerConversationTracker {
    pub conversation_id: Option<String>,
    pub previous_response_id: Option<String>,
    pub auto_previous_response_id: bool,
    sent_initial_input: bool,
    remaining_initial_input: Option<Vec<crate::items::InputItem>>,
    sent_item_fingerprints: HashSet<String>,
    server_item_ids: HashSet<String>,
    server_tool_call_ids: HashSet<String>,
    prepared_item_source_ids_by_identity: HashMap<InputItemIdentity, Uuid>,
    prepared_item_source_ids_by_fingerprint: HashMap<String, Vec<Uuid>>,
    prepared_item_sources_by_id: HashMap<Uuid, crate::items::InputItem>,
}

impl OpenAIServerConversationTracker {
    pub fn new(config: &RunConfig) -> Self {
        Self {
            conversation_id: config.conversation_id.clone(),
            previous_response_id: config.previous_response_id.clone(),
            auto_previous_response_id: config.auto_previous_response_id,
            sent_initial_input: false,
            remaining_initial_input: None,
            sent_item_fingerprints: HashSet::new(),
            server_item_ids: HashSet::new(),
            server_tool_call_ids: HashSet::new(),
            prepared_item_source_ids_by_identity: HashMap::new(),
            prepared_item_source_ids_by_fingerprint: HashMap::new(),
            prepared_item_sources_by_id: HashMap::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.conversation_id.is_some()
            || self.previous_response_id.is_some()
            || self.auto_previous_response_id
    }

    pub fn previous_response_id(&self) -> Option<&str> {
        self.previous_response_id.as_deref()
    }

    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    pub fn apply_session_state(&mut self, state: &OpenAIConversationSessionState) {
        if self.conversation_id.is_none() {
            self.conversation_id = state.conversation_id.clone();
        }
        if self.previous_response_id.is_none() {
            self.previous_response_id = state.previous_response_id.clone();
        }
        self.auto_previous_response_id |= state.auto_previous_response_id;
    }

    pub fn apply_response(&mut self, response: &ModelResponse) {
        self.track_server_items(response);
        if (self.auto_previous_response_id || self.previous_response_id.is_some())
            && response.response_id.is_some()
        {
            self.previous_response_id = response.response_id.clone();
        }
    }

    pub fn prepare_input(
        &mut self,
        original_input: &[crate::items::InputItem],
        generated_items: &[RunItem],
    ) -> Vec<crate::items::InputItem> {
        let mut prepared = Vec::new();

        if !self.sent_initial_input {
            for item in original_input {
                let prepared_item = item.clone();
                self.register_prepared_item_source(&prepared_item, item.clone());
                prepared.push(prepared_item);
            }
            self.remaining_initial_input =
                (!original_input.is_empty()).then(|| original_input.to_vec());
            self.sent_initial_input = true;
        } else if let Some(remaining) = self.remaining_initial_input.clone() {
            for item in remaining {
                self.register_prepared_item_source(&item, item.clone());
                prepared.push(item);
            }
        }

        for run_item in generated_items {
            let Some(item) = run_item.to_input_item() else {
                continue;
            };
            if self
                .extract_item_id(&item)
                .is_some_and(|item_id| self.server_item_ids.contains(item_id))
            {
                continue;
            }
            if self
                .extract_output_call_id(&item)
                .is_some_and(|call_id| self.server_tool_call_ids.contains(call_id))
            {
                continue;
            }
            let fingerprint = fingerprint_input_item(&item);
            if self.sent_item_fingerprints.contains(&fingerprint) {
                continue;
            }
            let prepared_item = item.clone();
            self.register_prepared_item_source(&prepared_item, item);
            prepared.push(prepared_item);
        }

        prepared
    }

    pub fn mark_input_as_sent(&mut self, items: &[crate::items::InputItem]) {
        for item in items {
            let source = self.consume_prepared_item_source(item);
            let fingerprint = fingerprint_input_item(&source);
            self.sent_item_fingerprints.insert(fingerprint);
            self.remove_remaining_initial_item(&source);
        }
    }

    pub fn register_filtered_input_sources(
        &mut self,
        prepared_input: &[crate::items::InputItem],
        filtered_input: &[crate::items::InputItem],
    ) {
        if prepared_input == filtered_input {
            return;
        }

        let mut available_sources = prepared_input
            .iter()
            .filter_map(|item| {
                self.resolve_prepared_item_source_id(item).map(|source_id| {
                    (
                        source_id,
                        fingerprint_input_item(item),
                        self.prepared_item_sources_by_id
                            .get(&source_id)
                            .cloned()
                            .unwrap_or_else(|| item.clone()),
                    )
                })
            })
            .collect::<Vec<_>>();

        for item in filtered_input {
            let filtered_identity = self.resolve_prepared_item_source_id(item);
            let filtered_fingerprint = fingerprint_input_item(item);
            let source_index = filtered_identity
                .and_then(|source_id| {
                    available_sources
                        .iter()
                        .position(|(available_source_id, _, _)| *available_source_id == source_id)
                })
                .or_else(|| {
                    available_sources
                        .iter()
                        .position(|(_, prepared_fingerprint, _)| {
                            *prepared_fingerprint == filtered_fingerprint
                        })
                })
                .unwrap_or(0);
            let (_, _, source_item) = available_sources.remove(source_index);
            self.register_prepared_item_source(item, source_item);
            if available_sources.is_empty() {
                break;
            }
        }
    }

    pub fn rewind_input(&mut self, items: &[crate::items::InputItem]) {
        let mut rewind_items = Vec::new();
        for item in items {
            let source = self.consume_prepared_item_source(item);
            self.sent_item_fingerprints
                .remove(&fingerprint_input_item(&source));
            rewind_items.push(source);
        }

        if rewind_items.is_empty() {
            return;
        }

        let mut remaining = rewind_items;
        if let Some(existing) = self.remaining_initial_input.take() {
            remaining.extend(existing);
        }
        self.remaining_initial_input = Some(remaining);
    }

    pub fn track_server_items(&mut self, response: &ModelResponse) {
        let mut server_fingerprints = HashSet::new();
        for item in response.to_input_items() {
            if let Some(item_id) = self.extract_item_id(&item).map(ToOwned::to_owned) {
                self.server_item_ids.insert(item_id);
            }
            if let Some(call_id) = self.extract_output_call_id(&item).map(ToOwned::to_owned) {
                self.server_tool_call_ids.insert(call_id);
            }
            let fingerprint = fingerprint_input_item(&item);
            self.sent_item_fingerprints.insert(fingerprint.clone());
            server_fingerprints.insert(fingerprint);
        }

        if let Some(remaining) = self.remaining_initial_input.take() {
            let filtered = remaining
                .into_iter()
                .filter(|item| !server_fingerprints.contains(&fingerprint_input_item(item)))
                .collect::<Vec<_>>();
            self.remaining_initial_input = (!filtered.is_empty()).then_some(filtered);
        }
    }

    pub fn session_state(&self) -> OpenAIConversationSessionState {
        OpenAIConversationSessionState {
            conversation_id: self.conversation_id.clone(),
            previous_response_id: self.previous_response_id.clone(),
            auto_previous_response_id: self.auto_previous_response_id,
        }
    }

    pub fn apply_to_state(&self, state: &mut RunState) {
        state.conversation_id = self.conversation_id.clone();
        state.previous_response_id = self.previous_response_id.clone();
        state.auto_previous_response_id = self.auto_previous_response_id;
    }

    fn register_prepared_item_source(
        &mut self,
        prepared_item: &crate::items::InputItem,
        source_item: crate::items::InputItem,
    ) {
        let source_id = Uuid::new_v4();
        if let Some(identity) = input_item_identity(prepared_item) {
            self.prepared_item_source_ids_by_identity
                .insert(identity, source_id);
        }
        let fingerprint = fingerprint_input_item(prepared_item);
        self.prepared_item_source_ids_by_fingerprint
            .entry(fingerprint)
            .or_default()
            .push(source_id);
        self.prepared_item_sources_by_id
            .insert(source_id, source_item);
    }

    fn consume_prepared_item_source(
        &mut self,
        item: &crate::items::InputItem,
    ) -> crate::items::InputItem {
        let Some(source_id) = self.resolve_prepared_item_source_id(item) else {
            return item.clone();
        };

        if let Some(identity) = input_item_identity(item) {
            self.prepared_item_source_ids_by_identity.remove(&identity);
        }

        let fingerprint = fingerprint_input_item(item);
        if let Some(source_ids) = self
            .prepared_item_source_ids_by_fingerprint
            .get_mut(&fingerprint)
        {
            if let Some(index) = source_ids
                .iter()
                .position(|candidate| *candidate == source_id)
            {
                source_ids.remove(index);
            }
            if source_ids.is_empty() {
                self.prepared_item_source_ids_by_fingerprint
                    .remove(&fingerprint);
            }
        }

        self.prepared_item_sources_by_id
            .remove(&source_id)
            .unwrap_or_else(|| item.clone())
    }

    fn resolve_prepared_item_source(
        &self,
        item: &crate::items::InputItem,
    ) -> crate::items::InputItem {
        self.resolve_prepared_item_source_id(item)
            .and_then(|source_id| self.prepared_item_sources_by_id.get(&source_id).cloned())
            .unwrap_or_else(|| item.clone())
    }

    fn resolve_prepared_item_source_id(&self, item: &crate::items::InputItem) -> Option<Uuid> {
        if let Some(identity) = input_item_identity(item) {
            if let Some(source_id) = self.prepared_item_source_ids_by_identity.get(&identity) {
                return Some(*source_id);
            }
        }

        let fingerprint = fingerprint_input_item(item);
        self.prepared_item_source_ids_by_fingerprint
            .get(&fingerprint)
            .and_then(|source_ids| source_ids.first().copied())
    }

    fn remove_remaining_initial_item(&mut self, item: &crate::items::InputItem) {
        let Some(remaining) = self.remaining_initial_input.as_mut() else {
            return;
        };
        let target = fingerprint_input_item(item);
        if let Some(index) = remaining
            .iter()
            .position(|candidate| fingerprint_input_item(candidate) == target)
        {
            remaining.remove(index);
        }
        if remaining.is_empty() {
            self.remaining_initial_input = None;
        }
    }

    fn extract_item_id<'a>(&self, item: &'a crate::items::InputItem) -> Option<&'a str> {
        match item {
            crate::items::InputItem::Text { .. } => None,
            crate::items::InputItem::Json { value } => {
                value.get("id").and_then(serde_json::Value::as_str)
            }
        }
    }

    fn extract_output_call_id<'a>(&self, item: &'a crate::items::InputItem) -> Option<&'a str> {
        match item {
            crate::items::InputItem::Text { .. } => None,
            crate::items::InputItem::Json { value } => value
                .get("call_id")
                .and_then(serde_json::Value::as_str)
                .filter(|_| value.get("output").is_some()),
        }
    }
}

fn fingerprint_input_item(item: &crate::items::InputItem) -> String {
    serde_json::to_string(item).unwrap_or_else(|_| format!("{item:?}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum InputItemIdentity {
    Text(usize),
    JsonString(usize),
    JsonArray(usize),
    JsonObject { first_key: usize, len: usize },
}

fn input_item_identity(item: &crate::items::InputItem) -> Option<InputItemIdentity> {
    match item {
        crate::items::InputItem::Text { text } => {
            Some(InputItemIdentity::Text(text.as_ptr() as usize))
        }
        crate::items::InputItem::Json { value } => json_value_identity(value),
    }
}

fn json_value_identity(value: &serde_json::Value) -> Option<InputItemIdentity> {
    match value {
        serde_json::Value::String(text) => {
            Some(InputItemIdentity::JsonString(text.as_ptr() as usize))
        }
        serde_json::Value::Array(values) => {
            Some(InputItemIdentity::JsonArray(values.as_ptr() as usize))
        }
        serde_json::Value::Object(map) => {
            map.keys().next().map(|key| InputItemIdentity::JsonObject {
                first_key: key.as_ptr() as usize,
                len: map.len(),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::InputItem;
    use crate::run_config::RunConfig;

    #[test]
    fn tracker_only_replays_unsent_filtered_deltas() {
        let mut tracker = OpenAIServerConversationTracker::new(&RunConfig {
            conversation_id: Some("conv-1".to_owned()),
            ..RunConfig::default()
        });
        let original_input = vec![InputItem::from("first"), InputItem::from("second")];

        let first_prepared = tracker.prepare_input(&original_input, &[]);
        assert_eq!(first_prepared, original_input);

        tracker.mark_input_as_sent(&[InputItem::from("first")]);

        let retried = tracker.prepare_input(&original_input, &[]);
        assert_eq!(retried, vec![InputItem::from("second")]);
    }

    #[test]
    fn tracker_rewinds_sent_state_after_retry() {
        let mut tracker = OpenAIServerConversationTracker::new(&RunConfig {
            conversation_id: Some("conv-1".to_owned()),
            ..RunConfig::default()
        });
        let original_input = vec![InputItem::from("first"), InputItem::from("second")];

        let first_prepared = tracker.prepare_input(&original_input, &[]);
        tracker.mark_input_as_sent(&first_prepared);

        tracker.rewind_input(&first_prepared);

        let retried = tracker.prepare_input(&original_input, &[]);
        assert_eq!(
            retried,
            vec![InputItem::from("first"), InputItem::from("second")]
        );
    }

    #[test]
    fn tracker_marks_rewritten_filtered_items_as_original_sources() {
        let mut tracker = OpenAIServerConversationTracker::new(&RunConfig {
            conversation_id: Some("conv-1".to_owned()),
            ..RunConfig::default()
        });
        let original_input = vec![InputItem::from("hello")];

        let prepared = tracker.prepare_input(&original_input, &[]);
        let filtered = vec![InputItem::from("filtered-hello")];

        tracker.register_filtered_input_sources(&prepared, &filtered);
        tracker.mark_input_as_sent(&filtered);

        let retried = tracker.prepare_input(&original_input, &[]);
        assert!(retried.is_empty());
    }

    #[test]
    fn tracker_preserves_source_identity_when_filter_drops_reorders_and_rewrites_items() {
        let mut tracker = OpenAIServerConversationTracker::new(&RunConfig {
            conversation_id: Some("conv-1".to_owned()),
            ..RunConfig::default()
        });
        let original_input = vec![
            InputItem::Json {
                value: serde_json::json!({"type": "message", "content": "first"}),
            },
            InputItem::Json {
                value: serde_json::json!({"type": "message", "content": "second"}),
            },
            InputItem::Json {
                value: serde_json::json!({"type": "message", "content": "third"}),
            },
        ];

        let mut prepared = tracker.prepare_input(&original_input, &[]);
        let prepared_snapshot = prepared.clone();
        let mut reordered = std::mem::take(&mut prepared);
        let mut rewritten_third = reordered.pop().expect("third prepared item");
        let second = reordered.pop().expect("second prepared item");
        let _dropped_first = reordered.pop().expect("first prepared item");
        if let InputItem::Json { value } = &mut rewritten_third {
            value["content"] = serde_json::Value::String("third-filtered".to_owned());
        } else {
            panic!("expected json input item");
        }
        let filtered = vec![rewritten_third, second];

        tracker.register_filtered_input_sources(&prepared_snapshot, &filtered);
        tracker.mark_input_as_sent(&filtered);

        let retried = tracker.prepare_input(&original_input, &[]);
        assert_eq!(
            retried,
            vec![InputItem::Json {
                value: serde_json::json!({"type": "message", "content": "first"}),
            }]
        );
    }
}
