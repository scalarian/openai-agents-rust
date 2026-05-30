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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FileSearchToolOptions {
    pub vector_store_ids: Vec<String>,
    pub max_num_results: Option<u64>,
    pub include_search_results: bool,
    pub ranking_options: Option<Value>,
    pub filters: Option<Value>,
}

impl FileSearchToolOptions {
    pub fn new<I, S>(vector_store_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            vector_store_ids: vector_store_ids.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    fn into_hosted_tool_parts(self) -> (BTreeMap<String, Value>, Vec<String>) {
        let mut options = BTreeMap::from([(
            "vector_store_ids".to_owned(),
            Value::Array(
                self.vector_store_ids
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        )]);
        if let Some(max_num_results) = self.max_num_results {
            options.insert("max_num_results".to_owned(), max_num_results.into());
        }
        if let Some(ranking_options) = self.ranking_options {
            options.insert("ranking_options".to_owned(), ranking_options);
        }
        if let Some(filters) = self.filters {
            options.insert("filters".to_owned(), filters);
        }
        let includes = if self.include_search_results {
            vec!["file_search_call.results".to_owned()]
        } else {
            Vec::new()
        };
        (options, includes)
    }
}

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

pub fn file_search_tool_with_vector_store_ids<I, S>(vector_store_ids: I) -> StaticTool
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    file_search_tool_with_options(FileSearchToolOptions::new(vector_store_ids))
}

pub fn file_search_tool_with_options(options: FileSearchToolOptions) -> StaticTool {
    let (hosted_tool_options, hosted_tool_includes) = options.into_hosted_tool_parts();
    file_search_tool()
        .with_hosted_tool_options(hosted_tool_options)
        .with_hosted_tool_includes(hosted_tool_includes)
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
