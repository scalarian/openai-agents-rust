use agents_core::{InputItem, ToolDefinition};
use serde_json::{Value, json};

use crate::chatcmpl_helpers::ChatCmplHelpers;

#[derive(Clone, Debug, Default)]
pub struct Converter;

impl Converter {
    pub fn payload(
        model: &str,
        instructions: Option<&str>,
        input: &[InputItem],
        tools: &[ToolDefinition],
    ) -> Value {
        let mut messages = Vec::new();
        if let Some(instructions) = instructions {
            messages.push(json!({
                "role": "system",
                "content": instructions,
            }));
        }
        messages.extend(ChatCmplHelpers::input_to_messages(input));

        let mut payload = json!({
            "model": model,
            "messages": messages,
        });
        if let Some(object) = payload.as_object_mut() {
            let tool_payload = ChatCmplHelpers::tools_to_payload(tools);
            if !tool_payload.is_empty() {
                object.insert("tools".to_owned(), Value::Array(tool_payload));
                object.insert("tool_choice".to_owned(), Value::String("auto".to_owned()));
            }
        }
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_chat_completions_payload() {
        let payload = Converter::payload(
            "gpt-4.1",
            Some("Be brief"),
            &[InputItem::from("hello")],
            &[ToolDefinition::new("search", "Search")],
        );

        assert_eq!(payload["messages"][0]["role"], "system");
        assert_eq!(payload["messages"][1]["content"], "hello");
        assert_eq!(payload["tool_choice"], "auto");
    }
}
