use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    AgentsError, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, RunItem, Runner, SandboxAgent,
    SandboxCapability, SandboxRunConfig, StreamEvent, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const DEFAULT_QUESTION: &str = "Summarize this sandbox project in 2 sentences.";

#[derive(Clone, Default)]
struct SandboxBasicModel;

#[async_trait]
impl Model for SandboxBasicModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(shell_output) =
            tool_output_text_for(&request.input, "sandbox_run_shell")
        {
            vec![OutputItem::Text {
                text: format!(
                    "This sandbox contains a tiny demo project with a README, notes, and a Python source file. The shell inspection found:\n{}",
                    stdout_section(shell_output).trim()
                ),
            }]
        } else if tool_output_text_for(&request.input, "sandbox_list_files").is_some() {
            vec![OutputItem::ToolCall {
                call_id: "call-sandbox-shell".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "find . -maxdepth 2 -type f | sort | sed 's#^./##'"
                }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-sandbox-list".to_owned(),
                tool_name: "sandbox_list_files".to_owned(),
                arguments: json!({"path": "/workspace"}),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 30,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct SandboxBasicProvider {
    model: Arc<SandboxBasicModel>,
}

impl ModelProvider for SandboxBasicProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let manifest = build_manifest();
    let sandbox_agent = SandboxAgent::builder("Local Sandbox Assistant")
        .model("gpt-5.5")
        .instructions(
            "Answer questions about the sandbox workspace. Inspect the project before answering, and keep the response concise.",
        )
        .default_manifest(manifest)
        .capabilities(vec![SandboxCapability::Filesystem, SandboxCapability::Shell])
        .build();

    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;

    println!("=== Sandbox Basic Example ===");
    println!("workspace={}", prepared.session.workspace_root().display());
    println!(
        "initial files:\n{}",
        prepared.session.list_files("/workspace")?
    );

    let runner = Runner::new().with_model_provider(Arc::new(SandboxBasicProvider::default()));
    let streamed = runner
        .run_streamed(&prepared.agent, DEFAULT_QUESTION)
        .await?;

    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::ToolCall { tool_name, .. } => {
                    println!("[tool call] {tool_name}");
                }
                RunItem::ToolCallOutput {
                    tool_name, output, ..
                } => {
                    println!("[tool output] {tool_name}");
                    println!("{}", output_text(&output).trim());
                }
                RunItem::MessageOutput { content } => {
                    println!("assistant> {}", output_text(&content).trim());
                }
                RunItem::HandoffCall { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::AgentUpdated(_)
            | StreamEvent::RawResponseEvent(_)
            | StreamEvent::Lifecycle(_) => {}
        }
    }

    let _final_result = streamed.wait_for_completion().await?;
    prepared.session.cleanup()?;
    Ok(())
}

fn build_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "README.md",
            File::from_text(
                "# Demo Project\n\nThis sandbox contains a tiny demo project for the local sandbox runner.\nThe goal is to show how Runner can prepare a sandbox workspace.\n",
            ),
        )
        .with_entry(
            "src/app.py",
            File::from_text("def greet(name: str) -> str:\n    return f\"Hello, {name}!\"\n"),
        )
        .with_entry(
            "docs/notes.md",
            File::from_text(
                "# Notes\n\n- The example is intentionally minimal.\n- The model should inspect files through sandbox tools.\n",
            ),
        )
}

fn tool_output_text_for<'a>(input: &'a [InputItem], tool_name: &str) -> Option<&'a str> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("tool_name").and_then(Value::as_str) != Some(tool_name)
        {
            return None;
        }
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
    })
}

fn stdout_section(output: &str) -> &str {
    output
        .split_once("stdout:\n")
        .map(|(_, after_stdout)| after_stdout)
        .and_then(|after_stdout| {
            after_stdout
                .split_once("\nstderr:")
                .map(|(stdout, _)| stdout)
        })
        .unwrap_or(output)
}

fn output_text(output: &OutputItem) -> String {
    match output {
        OutputItem::Text { text } => text.clone(),
        OutputItem::Json { value } => value.to_string(),
        OutputItem::Refusal { refusal } => refusal.clone(),
        OutputItem::ToolCall { .. } | OutputItem::Handoff { .. } | OutputItem::Reasoning { .. } => {
            String::new()
        }
    }
}
