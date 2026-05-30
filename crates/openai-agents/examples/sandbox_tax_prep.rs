use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, RunItem, Runner, SandboxAgent,
    SandboxCapability, SandboxRunConfig, StreamEvent, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const DEFAULT_PROMPT: &str = "Generate a demo 1040 filing summary for filing year 2025. Use the provided W-2 data and write the finalized artifact into output/.";

#[derive(Clone, Default)]
struct TaxPrepModel;

#[async_trait]
impl Model for TaxPrepModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "read-w2").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-w2".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/taxpayer_data/sample_w2.txt"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "read-form").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-form".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/reference_forms/f1040.txt"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-summary").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-summary".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/filled_1040_summary.txt",
                    "replacement": filled_1040_summary()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "verify-output").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "verify-output".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "grep -E 'Filing year|Refund|Amount due' output/filled_1040_summary.txt"
                }),
                namespace: None,
            }]
        } else {
            let verification =
                tool_output_text_for_call(&request.input, "verify-output").unwrap_or_default();
            vec![OutputItem::Text {
                text: format!(
                    "Created `output/filled_1040_summary.txt` for the demo filing packet. Verification found:\n{}",
                    stdout_section(verification).trim()
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 62,
                output_tokens: 28,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct TaxPrepProvider {
    model: Arc<TaxPrepModel>,
}

impl ModelProvider for TaxPrepProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Tax Prep Assistant")
        .model("gpt-5.5")
        .instructions(
            "Compute a demo federal tax filing summary using only workspace files. Save finalized artifacts in output/ and summarize key amounts.",
        )
        .default_manifest(tax_prep_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();

    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        workflow_name: "Sandbox tax prep demo".to_owned(),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;

    let runner = Runner::new().with_model_provider(Arc::new(TaxPrepProvider::default()));
    let streamed = runner.run_streamed(&prepared.agent, DEFAULT_PROMPT).await?;

    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::ToolCall {
                    tool_name, call_id, ..
                } => {
                    println!("[tool call] {tool_name} ({})", call_id.unwrap_or_default());
                }
                RunItem::ToolCallOutput {
                    tool_name, output, ..
                } => {
                    println!("[tool output] {tool_name}: {}", output_text(&output).trim());
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
    let generated = prepared
        .session
        .read_file("/workspace/output/filled_1040_summary.txt")?;
    println!("generated_artifact:\n{}", generated.trim());
    prepared.session.cleanup()?;
    Ok(())
}

fn tax_prep_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "taxpayer_data",
            Dir::new().with_entry(
                "sample_w2.txt",
                File::from_text(
                    "Taxpayer: Jordan Lee\n\
                     Filing year: 2025\n\
                     Wages: 72000\n\
                     Federal income tax withheld: 8200\n\
                     Employer: Example Robotics LLC\n",
                ),
            ),
        )
        .with_entry(
            "reference_forms",
            Dir::new().with_entry(
                "f1040.txt",
                File::from_text(
                    "Demo Form 1040 fields:\n\
                     - Filing status\n\
                     - Wages\n\
                     - Standard deduction\n\
                     - Taxable income\n\
                     - Total tax\n\
                     - Payments\n\
                     - Refund or amount due\n",
                ),
            ),
        )
        .with_entry("output", Dir::new())
}

fn filled_1040_summary() -> &'static str {
    "# Filled 1040 Demo Summary\n\n\
     Filing year: 2025\n\
     Taxpayer: Jordan Lee\n\
     Filing status: Single\n\
     Wages: $72,000.00\n\
     Standard deduction: $15,000.00\n\
     Taxable income: $57,000.00\n\
     Estimated total tax: $6,600.00\n\
     Federal tax withheld: $8,200.00\n\
     Refund: $1,600.00\n\
     Amount due: $0.00\n\n\
     This artifact is synthetic and intended only for the sandbox tax prep demo.\n"
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
        OutputItem::ToolCall { .. } | OutputItem::Handoff { .. } | OutputItem::Reasoning { .. } => {
            String::new()
        }
    }
}
