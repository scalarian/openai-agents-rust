use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReasoningContentSource {
    pub item: Value,
    pub origin_model: Option<String>,
    #[serde(default)]
    pub provider_data: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReasoningContentReplayContext {
    pub model: String,
    pub base_url: Option<String>,
    pub reasoning: ReasoningContentSource,
}

pub fn default_should_replay_reasoning_content(context: &ReasoningContentReplayContext) -> bool {
    if !context.model.to_ascii_lowercase().contains("deepseek") {
        return false;
    }

    context
        .reasoning
        .origin_model
        .as_deref()
        .map(|origin| origin.to_ascii_lowercase().contains("deepseek"))
        .unwrap_or_else(|| context.reasoning.provider_data.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_replays_for_deepseek_by_default() {
        let should = default_should_replay_reasoning_content(&ReasoningContentReplayContext {
            model: "deepseek-chat".to_owned(),
            base_url: None,
            reasoning: ReasoningContentSource {
                item: Value::Null,
                origin_model: Some("deepseek-chat".to_owned()),
                provider_data: BTreeMap::new(),
            },
        });

        assert!(should);
    }
}
