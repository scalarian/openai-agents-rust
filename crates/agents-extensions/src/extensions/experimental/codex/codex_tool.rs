use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use agents_core::{FunctionTool, Result, ToolContext, function_tool};

use crate::extensions::experimental::codex::codex::Codex;
use crate::extensions::experimental::codex::codex_options::CodexOptions;
use crate::extensions::experimental::codex::events::{ThreadEvent, Usage};
use crate::extensions::experimental::codex::thread::{Input, Thread, UserInput};
use crate::extensions::experimental::codex::thread_options::ThreadOptions;
use crate::extensions::experimental::codex::turn_options::TurnOptions;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexToolInputItem {
    Text { text: String },
    LocalImage { path: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CodexToolParameters {
    pub inputs: Vec<CodexToolInputItem>,
    pub thread_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CodexToolRunContextParameters {
    pub inputs: Vec<CodexToolInputItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaPrimitive {
    #[serde(rename = "type")]
    pub type_name: String,
    pub description: Option<String>,
    pub r#enum: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaArray {
    #[serde(rename = "type")]
    pub type_name: String,
    pub description: Option<String>,
    pub items: Option<Box<OutputSchemaPrimitive>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaPropertyDescriptor {
    pub name: String,
    pub description: Option<String>,
    pub schema: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaDescriptor {
    pub title: Option<String>,
    pub description: Option<String>,
    pub properties: Option<Vec<OutputSchemaPropertyDescriptor>>,
    pub required: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexToolResult {
    pub thread_id: Option<String>,
    pub response: String,
    pub usage: Option<Usage>,
}

impl CodexToolResult {
    pub fn as_dict(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[derive(Clone, Debug)]
pub struct CodexToolStreamEvent {
    pub event: ThreadEvent,
    pub thread: Thread,
    pub tool_call: Value,
}

#[derive(Clone, Debug, Default)]
pub struct CodexToolOptions {
    pub name: Option<String>,
    pub description: Option<String>,
    pub output_schema: Option<OutputSchemaDescriptor>,
    pub codex: Option<Codex>,
    pub codex_options: Option<CodexOptions>,
    pub default_thread_options: Option<ThreadOptions>,
    pub thread_id: Option<String>,
    pub default_turn_options: Option<TurnOptions>,
    pub persist_session: bool,
}

pub fn codex_tool(options: CodexToolOptions) -> Result<FunctionTool> {
    let tool_name = options.name.clone().unwrap_or_else(|| "codex".to_owned());
    let description = options
        .description
        .clone()
        .unwrap_or_else(|| "Run a Codex CLI task".to_owned());
    let codex = options.codex.clone();
    let codex_options = options.codex_options.clone();
    let default_thread_options = options.default_thread_options.clone();
    let default_turn_options = options.default_turn_options.clone();
    let pinned_thread_id = options.thread_id.clone();

    function_tool(
        tool_name,
        description,
        move |_ctx: ToolContext, args: CodexToolParameters| {
            let codex = codex.clone();
            let codex_options = codex_options.clone();
            let default_thread_options = default_thread_options.clone();
            let default_turn_options = default_turn_options.clone();
            let pinned_thread_id = pinned_thread_id.clone();
            async move {
                let codex = match codex.clone() {
                    Some(codex) => codex,
                    None => Codex::new(codex_options.clone())?,
                };

                let thread_id = args.thread_id.clone().or(pinned_thread_id.clone());
                let mut thread = match thread_id {
                    Some(thread_id) => {
                        codex.resume_thread(thread_id, default_thread_options.clone())
                    }
                    None => codex.start_thread(default_thread_options.clone()),
                };

                let turn = thread
                    .run(
                        Input::Items(
                            args.inputs
                                .into_iter()
                                .map(|item| match item {
                                    CodexToolInputItem::Text { text } => UserInput::Text { text },
                                    CodexToolInputItem::LocalImage { path } => {
                                        UserInput::LocalImage { path }
                                    }
                                })
                                .collect(),
                        ),
                        default_turn_options.clone(),
                    )
                    .await?;

                let result = CodexToolResult {
                    thread_id: thread.id.clone(),
                    response: turn.final_response,
                    usage: turn.usage,
                };
                Ok::<_, agents_core::AgentsError>(json!(result))
            }
        },
    )
}
