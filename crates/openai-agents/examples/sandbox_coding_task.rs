use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, RunItem, Runner, SandboxAgent,
    SandboxCapability, SandboxRunConfig, StreamEvent, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const TARGET_TEST_CMD: &str = "sh tests/test_credit_note.sh";
const DEFAULT_PROMPT: &str = "Open `repo/task.md`, fix the bug in `repo/credit_note.sh`, run `sh tests/test_credit_note.sh` from `repo/`, and summarize the change.";

#[derive(Clone, Default)]
struct SandboxCodingModel;

#[async_trait]
impl Model for SandboxCodingModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "read-task").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-task".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/repo/task.md"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "read-credit-note").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-credit-note".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/repo/credit_note.sh"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "patch-credit-note").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "patch-credit-note".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/repo/credit_note.sh",
                    "replacement": fixed_credit_note_script()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "run-credit-note-tests").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "run-credit-note-tests".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": format!("cd repo && {TARGET_TEST_CMD}")
                }),
                namespace: None,
            }]
        } else {
            let test_output =
                tool_output_text_for_call(&request.input, "run-credit-note-tests").unwrap_or("");
            vec![OutputItem::Text {
                text: format!(
                    "Updated `repo/credit_note.sh` to emit a credit label and normalize negative amounts before formatting. Verification `{TARGET_TEST_CMD}` completed with:\n{}",
                    stdout_section(test_output).trim()
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 54,
                output_tokens: 24,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct SandboxCodingProvider {
    model: Arc<SandboxCodingModel>,
}

impl ModelProvider for SandboxCodingProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Sandbox engineer")
        .model("gpt-5.5")
        .instructions(
            "Inspect the repo, make the smallest correct change, run the target check, and summarize file changes and risks.",
        )
        .default_manifest(example_repo_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();

    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        workflow_name: "Sandbox coding task example".to_owned(),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;

    let runner = Runner::new().with_model_provider(Arc::new(SandboxCodingProvider::default()));
    let streamed = runner.run_streamed(&prepared.agent, DEFAULT_PROMPT).await?;

    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::ToolCall {
                    tool_name, call_id, ..
                } => {
                    println!("tool_call={} {}", call_id.unwrap_or_default(), tool_name);
                }
                RunItem::ToolCallOutput {
                    tool_name,
                    output,
                    call_id,
                    ..
                } => {
                    println!("tool_output={} {}", call_id.unwrap_or_default(), tool_name);
                    println!("{}", output_text(&output).trim());
                }
                RunItem::MessageOutput { content } => {
                    println!("message={}", output_text(&content).trim());
                }
                RunItem::HandoffCall { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::AgentUpdated(_)
            | StreamEvent::RawResponseEvent(_)
            | StreamEvent::Lifecycle(_) => {}
        }
    }

    let result = streamed.wait_for_completion().await?;
    println!("final_output={}", result.final_output.unwrap_or_default());

    let verification = prepared
        .session
        .run_shell(&format!("cd repo && {TARGET_TEST_CMD}"))?;
    let verification_text = format!("{}{}", verification.stdout, verification.stderr);
    if verification.exit_code != 0 || !verification_text.contains("2 passed") {
        prepared.session.cleanup()?;
        return Err(AgentsError::message(format!(
            "post-run verification failed:\n{verification_text}"
        )));
    }

    println!("verification_command={TARGET_TEST_CMD}");
    println!("verification_result={}", verification.stdout.trim());
    println!("updated_credit_note.sh:");
    println!(
        "{}",
        prepared
            .session
            .read_file("/workspace/repo/credit_note.sh")?
            .trim_end()
    );
    prepared.session.cleanup()?;
    Ok(())
}

fn example_repo_manifest() -> Manifest {
    Manifest::default().with_entry(
        "repo",
        Dir::new()
            .with_entry(
                "README.md",
                File::from_text(
                    "# Credit Note Example Repo\n\nThis tiny repo lets a sandbox coding agent inspect, patch, and test one shell script.\n",
                ),
            )
            .with_entry(
                "task.md",
                File::from_text(
                    "# Task\n\n`credit_note.sh` formats a credit note incorrectly:\n\n- It prints a debit label instead of a credit label.\n- It preserves the sign instead of always showing the credited amount as positive.\n\nUse the smallest correct fix, then run this exact verification command from the `repo/` directory:\n\n`sh tests/test_credit_note.sh`\n\nDo not change the test expectations.\n",
                ),
            )
            .with_entry(
                "credit_note.sh",
                File::from_text(
                    "#!/bin/sh\n\ncustomer=\"$1\"\namount=\"$2\"\n\nprintf 'Credit note for %s: -$%s debit.\\n' \"$customer\" \"$amount\"\n",
                ),
            )
            .with_entry(
                "tests",
                Dir::new().with_entry(
                    "test_credit_note.sh",
                    File::from_text(
                        "#!/bin/sh\nset -eu\n\nactual_positive=\"$(sh credit_note.sh Northwind 12.50)\"\nif [ \"$actual_positive\" != 'Credit note for Northwind: $12.50 credit.' ]; then\n    printf 'expected positive case to pass, got: %s\\n' \"$actual_positive\" >&2\n    exit 1\nfi\n\nactual_negative=\"$(sh credit_note.sh Northwind -12.50)\"\nif [ \"$actual_negative\" != 'Credit note for Northwind: $12.50 credit.' ]; then\n    printf 'expected negative case to pass, got: %s\\n' \"$actual_negative\" >&2\n    exit 1\nfi\n\nprintf '2 passed\\n'\n",
                    ),
                ),
            ),
    )
}

fn fixed_credit_note_script() -> &'static str {
    "#!/bin/sh\n\ncustomer=\"$1\"\namount=\"$2\"\n\ncase \"$amount\" in\n    -*) amount=\"${amount#-}\" ;;\nesac\n\nprintf 'Credit note for %s: $%s credit.\\n' \"$customer\" \"$amount\"\n"
}

fn tool_output_text_for_call<'a>(input: &'a [InputItem], call_id: &str) -> Option<&'a str> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("call_id").and_then(Value::as_str) != Some(call_id)
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
        OutputItem::ToolCall { .. }
        | OutputItem::CustomToolCall { .. }
        | OutputItem::Handoff { .. }
        | OutputItem::Reasoning { .. } => String::new(),
    }
}
