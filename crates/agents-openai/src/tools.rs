use agents_core::StaticTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type WebSearchTool = StaticTool;
pub type FileSearchTool = StaticTool;
pub type CodeInterpreterTool = StaticTool;
pub type ToolSearchTool = StaticTool;
pub type ImageGenerationTool = StaticTool;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchToolOptions {
    pub filters: Option<Value>,
    pub user_location: Option<Value>,
    pub search_context_size: String,
    pub external_web_access: Option<bool>,
}

impl Default for WebSearchToolOptions {
    fn default() -> Self {
        Self {
            filters: None,
            user_location: None,
            search_context_size: "medium".to_owned(),
            external_web_access: None,
        }
    }
}

impl WebSearchToolOptions {
    fn into_hosted_tool_options(self) -> BTreeMap<String, Value> {
        let mut options = BTreeMap::from([
            (
                "search_context_size".to_owned(),
                Value::String(self.search_context_size),
            ),
            ("filters".to_owned(), self.filters.unwrap_or(Value::Null)),
            (
                "user_location".to_owned(),
                self.user_location.unwrap_or(Value::Null),
            ),
        ]);
        if let Some(external_web_access) = self.external_web_access {
            options.insert("external_web_access".to_owned(), external_web_access.into());
        }
        options
    }
}

pub fn web_search_tool() -> StaticTool {
    web_search_tool_with_options(WebSearchToolOptions::default())
}

pub fn web_search_tool_with_options(options: WebSearchToolOptions) -> StaticTool {
    StaticTool::new(
        "web_search",
        "Search the public web via OpenAI hosted search.",
    )
    .with_hosted_tool_options(options.into_hosted_tool_options())
}

pub fn file_search_tool() -> StaticTool {
    StaticTool::new(
        "file_search",
        "Search indexed files through the OpenAI file search tool.",
    )
}

pub fn code_interpreter_tool() -> StaticTool {
    StaticTool::new(
        "code_interpreter",
        "Run short code snippets in the hosted OpenAI code interpreter.",
    )
}

pub fn tool_search_tool() -> StaticTool {
    StaticTool::new(
        "tool_search",
        "Search tools available to the OpenAI runtime.",
    )
}

pub fn image_generation_tool() -> StaticTool {
    StaticTool::new(
        "image_generation",
        "Generate or edit images with OpenAI hosted tooling.",
    )
}
