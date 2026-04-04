use std::sync::Arc;

use agents_core::{AgentsError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extensions::experimental::codex::codex_options::CodexOptions;
use crate::extensions::experimental::codex::events::{ThreadEvent, Usage, coerce_thread_event};
use crate::extensions::experimental::codex::exec::{CodexExec, CodexExecArgs};
use crate::extensions::experimental::codex::items::{ThreadItem, is_agent_message_item};
use crate::extensions::experimental::codex::output_schema_file::create_output_schema_file;
use crate::extensions::experimental::codex::thread_options::ThreadOptions;
use crate::extensions::experimental::codex::turn_options::TurnOptions;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextInput {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalImageInput {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserInput {
    Text { text: String },
    LocalImage { path: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    Text(String),
    Items(Vec<UserInput>),
}

impl From<&str> for Input {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for Input {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec<UserInput>> for Input {
    fn from(value: Vec<UserInput>) -> Self {
        Self::Items(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub items: Vec<ThreadItem>,
    pub final_response: String,
    pub usage: Option<Usage>,
}

pub type RunResult = Turn;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamedTurn {
    pub events: Vec<ThreadEvent>,
}

pub type RunStreamedResult = StreamedTurn;

#[derive(Clone, Debug)]
pub struct Thread {
    exec_client: Arc<CodexExec>,
    options: CodexOptions,
    pub thread_options: ThreadOptions,
    pub id: Option<String>,
}

impl Thread {
    pub fn new(
        exec_client: Arc<CodexExec>,
        options: CodexOptions,
        thread_options: ThreadOptions,
        thread_id: Option<String>,
    ) -> Self {
        Self {
            exec_client,
            options,
            thread_options,
            id: thread_id,
        }
    }

    pub async fn run_streamed(
        &mut self,
        input: Input,
        turn_options: Option<TurnOptions>,
    ) -> Result<StreamedTurn> {
        let turn_options = turn_options.unwrap_or_default();
        let output_schema_file = create_output_schema_file(turn_options.output_schema.as_ref())?;
        let (prompt, images) = normalize_input(input);
        let raw_events = self
            .exec_client
            .run(CodexExecArgs {
                input: prompt,
                base_url: self.options.base_url.clone(),
                api_key: self.options.api_key.clone(),
                thread_id: self.id.clone(),
                images: if images.is_empty() {
                    None
                } else {
                    Some(images)
                },
                model: self.thread_options.model.clone(),
                sandbox_mode: self.thread_options.sandbox_mode,
                working_directory: self.thread_options.working_directory.clone(),
                additional_directories: self.thread_options.additional_directories.clone(),
                skip_git_repo_check: self.thread_options.skip_git_repo_check,
                output_schema_file: output_schema_file
                    .schema_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
                model_reasoning_effort: self.thread_options.model_reasoning_effort,
                signal: turn_options.signal.clone(),
                idle_timeout_seconds: turn_options.idle_timeout_seconds,
                network_access_enabled: self.thread_options.network_access_enabled,
                web_search_mode: self.thread_options.web_search_mode,
                web_search_enabled: self.thread_options.web_search_enabled,
                approval_policy: self.thread_options.approval_policy,
            })
            .await?;
        let events = raw_events
            .into_iter()
            .map(|line| {
                let parsed: Value = serde_json::from_str(&line)
                    .map_err(|error| AgentsError::message(error.to_string()))?;
                Ok(coerce_thread_event(parsed))
            })
            .collect::<Result<Vec<_>>>()?;

        for event in &events {
            if let ThreadEvent::ThreadStarted { thread_id } = event {
                self.id = Some(thread_id.clone());
            }
        }

        Ok(StreamedTurn { events })
    }

    pub async fn run(&mut self, input: Input, turn_options: Option<TurnOptions>) -> Result<Turn> {
        let streamed = self.run_streamed(input, turn_options).await?;
        let mut items = Vec::new();
        let mut final_response = String::new();
        let mut usage = None;

        for event in streamed.events {
            match event {
                ThreadEvent::ItemCompleted { item } => {
                    let item =
                        crate::extensions::experimental::codex::items::coerce_thread_item(item);
                    if is_agent_message_item(&item) {
                        if let ThreadItem::AgentMessage(message) = &item {
                            final_response = message.text.clone();
                        }
                    }
                    items.push(item);
                }
                ThreadEvent::TurnCompleted { usage: event_usage } => {
                    usage = event_usage;
                }
                ThreadEvent::TurnFailed { error } => {
                    return Err(AgentsError::message(error.message));
                }
                ThreadEvent::Error { message } => {
                    return Err(AgentsError::message(message));
                }
                _ => {}
            }
        }

        Ok(Turn {
            items,
            final_response,
            usage,
        })
    }
}

fn normalize_input(input: Input) -> (String, Vec<String>) {
    match input {
        Input::Text(text) => (text, Vec::new()),
        Input::Items(items) => {
            let mut prompt_parts = Vec::new();
            let mut images = Vec::new();
            for item in items {
                match item {
                    UserInput::Text { text } => prompt_parts.push(text),
                    UserInput::LocalImage { path } => images.push(path),
                }
            }
            (prompt_parts.join("\n\n"), images)
        }
    }
}
