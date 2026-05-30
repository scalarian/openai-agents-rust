use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, File, InputItem, Manifest, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, RunConfig, RunItem, Runner, SandboxAgent,
    SandboxCapability, SandboxRunConfig, StreamEvent, Usage, handoff,
};
use serde_json::{Value, json};

const DEFAULT_QUESTION: &str = "Review the attached onboarding packet and draft a short internal note for the account executive about what to confirm before kickoff.";

#[derive(Clone, Default)]
struct SandboxHandoffModel;

#[async_trait]
impl Model for SandboxHandoffModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = match request.model.as_deref() {
            Some("intake-model") => vec![OutputItem::Handoff {
                target_agent: "Onboarding Packet Reviewer".to_owned(),
            }],
            Some("reviewer-model") => {
                if tool_output_text_for(&request.input, "sandbox_run_shell").is_some() {
                    vec![OutputItem::Handoff {
                        target_agent: "Account Executive Assistant".to_owned(),
                    }]
                } else {
                    vec![OutputItem::ToolCall {
                        call_id: "review-onboarding-packet".to_owned(),
                        tool_name: "sandbox_run_shell".to_owned(),
                        arguments: json!({
                            "command": "cat customer_background.md kickoff_checklist.md implementation_scope.md"
                        }),
                        namespace: None,
                    }]
                }
            }
            Some("writer-model") => vec![OutputItem::Text {
                text: account_note(&request.input),
            }],
            _ => vec![OutputItem::Text {
                text: "No model route configured for this sandbox handoff example.".to_owned(),
            }],
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 42,
                output_tokens: 20,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct SandboxHandoffProvider {
    model: Arc<SandboxHandoffModel>,
}

impl ModelProvider for SandboxHandoffProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let account_manager = Agent::builder("Account Executive Assistant")
        .model("writer-model")
        .instructions(
            "Write concise internal updates for account teams. Convert reviewed facts into a short note.",
        )
        .build();

    let mut sandbox_reviewer = SandboxAgent::builder("Onboarding Packet Reviewer")
        .model("reviewer-model")
        .instructions(
            "Inspect onboarding documents in the sandbox, verify the facts, then hand off to the account executive assistant.",
        )
        .default_manifest(onboarding_manifest())
        .capabilities(vec![SandboxCapability::Shell])
        .build()
        .into_agent();
    sandbox_reviewer.handoffs.push(handoff(account_manager));

    let intake_agent = Agent::builder("Deal Desk Intake")
        .model("intake-model")
        .instructions(
            "Triage internal requests. If attached documents are needed, hand off to the onboarding packet reviewer immediately.",
        )
        .handoff(handoff(sandbox_reviewer))
        .build();

    let runner = Runner::new()
        .with_config(RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Sandbox handoffs example".to_owned(),
            ..RunConfig::default()
        })
        .with_model_provider(Arc::new(SandboxHandoffProvider::default()));
    let streamed = runner.run_streamed(&intake_agent, DEFAULT_QUESTION).await?;

    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::AgentUpdated(update) => {
                println!("agent={}", update.new_agent.name);
            }
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::HandoffCall { target_agent } => {
                    println!("handoff_call={target_agent}");
                }
                RunItem::ToolCall { tool_name, .. } => {
                    println!("tool_call={tool_name}");
                }
                RunItem::ToolCallOutput {
                    tool_name, output, ..
                } => {
                    println!("tool_output={tool_name}");
                    println!("{}", output_text(&output).trim());
                }
                RunItem::MessageOutput { content } => {
                    println!("message={}", output_text(&content).trim());
                }
                RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::RawResponseEvent(_) | StreamEvent::Lifecycle(_) => {}
        }
    }

    let completed = streamed.wait_for_completion().await?;
    println!(
        "final_agent={}",
        completed
            .last_agent()
            .map(|agent| agent.name.as_str())
            .unwrap_or("unknown")
    );
    println!(
        "final_output={}",
        completed.final_output.unwrap_or_default()
    );
    Ok(())
}

fn onboarding_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "customer_background.md",
            File::from_text(
                "# Customer background\n\n- Customer: Bluebird Logistics.\n- Region: North America.\n- New purchase: analytics workspace plus SSO.\n",
            ),
        )
        .with_entry(
            "kickoff_checklist.md",
            File::from_text(
                "# Kickoff checklist\n\n- Security questionnaire is still in review.\n- Two customer admins still need to complete access training.\n- Target kickoff date is next Tuesday.\n",
            ),
        )
        .with_entry(
            "implementation_scope.md",
            File::from_text(
                "# Implementation scope\n\n- The customer wants historical data migration for 5 years of records.\n- Data engineering support is available only starting next month.\n",
            ),
        )
}

fn account_note(input: &[InputItem]) -> String {
    let facts = input_blob(input);
    let customer = if facts.contains("Bluebird Logistics") {
        "Bluebird Logistics"
    } else {
        "the customer"
    };
    let mut lines = vec![
        format!("Headline: {customer} kickoff needs confirmation before next Tuesday."),
        "Top risks:".to_owned(),
    ];
    lines.push("- Security questionnaire is still in review.".to_owned());
    lines.push("- Two customer admins still need access training.".to_owned());
    lines.push(
        "- Five-year historical migration depends on data engineering availability next month."
            .to_owned(),
    );
    lines.push(
        "Next step: Confirm security status, admin training owners, and migration staffing before locking the kickoff date."
            .to_owned(),
    );
    lines.join("\n")
}

fn input_blob(input: &[InputItem]) -> String {
    input
        .iter()
        .map(|item| match item {
            InputItem::Text { text } => text.clone(),
            InputItem::Json { value } => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
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
