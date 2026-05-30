use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::FileSession;
use openai_agents::{
    Agent, AgentsError, InputItem, MemorySession, Model, ModelProvider, ModelRequest,
    ModelResponse, OpenAIConversationsSession, OutputItem, Result as AgentsResult,
    RunInterruptionKind, Runner, Session, ToolContext, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

const TOOL_ECHO: &str = "approved_echo";
const TOOL_NOTE: &str = "approved_note";
const REJECTION_OUTPUT: &str = "Tool execution was not approved.";

#[derive(Clone)]
struct ScenarioStep {
    label: &'static str,
    message: &'static str,
    approval: Approval,
    expected_output: String,
}

#[derive(Clone, Copy)]
enum Approval {
    Approve,
    Reject,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct QueryArgs {
    query: String,
}

#[derive(Clone, Default)]
struct HitlScenarioModel;

#[async_trait]
impl Model for HitlScenarioModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let latest_user = latest_user_text(&request.input);
        let output = if let Some(tool_output) = tool_output_after_latest_user(&request.input) {
            vec![OutputItem::Text { text: tool_output }]
        } else if latest_user.is_empty() {
            vec![OutputItem::Text {
                text: REJECTION_OUTPUT.to_owned(),
            }]
        } else {
            let tool_name = if latest_user.to_lowercase().contains("update note") {
                TOOL_NOTE
            } else {
                TOOL_ECHO
            };
            vec![OutputItem::ToolCall {
                call_id: format!("call-{tool_name}"),
                tool_name: tool_name.to_owned(),
                arguments: json!({ "query": latest_user }),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 20,
                output_tokens: 12,
            },
            response_id: Some("resp_hitl_scenario".to_owned()),
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct HitlScenarioProvider {
    model: Arc<HitlScenarioModel>,
}

impl ModelProvider for HitlScenarioProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let runner = Runner::new().with_model_provider(Arc::new(HitlScenarioProvider::default()));
    let agent = Agent::builder("HITL scenario")
        .instructions("Call the requested approval tool once before responding.")
        .function_tool(approval_echo_tool()?)
        .function_tool(approval_note_tool()?)
        .build();

    let steps = scenario_steps();
    run_memory_session_scenario(&runner, &agent, &steps).await?;
    run_file_session_scenario(&runner, &agent, &steps).await?;
    run_openai_session_scenario(&runner, &agent, &steps).await?;
    Ok(())
}

async fn run_memory_session_scenario(
    runner: &Runner,
    agent: &Agent,
    steps: &[ScenarioStep],
) -> Result<(), AgentsError> {
    let session = MemorySession::new("memory-hitl-scenario");
    println!("[MemorySession] session id: {}", session.session_id());
    for step in steps {
        run_step(runner, agent, &session, "MemorySession", step).await?;
    }
    Ok(())
}

async fn run_file_session_scenario(
    runner: &Runner,
    agent: &Agent,
    steps: &[ScenarioStep],
) -> Result<(), AgentsError> {
    let dir = std::env::temp_dir().join("openai-agents-hitl-session-scenario");
    let session = FileSession::with_session_id(&dir, "file-hitl-scenario");
    session.clear_session().await?;
    println!("[FileSession] file: {}", session.items_path().display());

    run_step(runner, agent, &session, "FileSession turn 1", &steps[0]).await?;
    let rehydrated = FileSession::with_session_id(&dir, "file-hitl-scenario");
    println!("[FileSession] rehydrated: {}", Path::new(&dir).display());
    for step in &steps[1..] {
        run_step(runner, agent, &rehydrated, "FileSession", step).await?;
    }
    rehydrated.clear_session().await?;
    Ok(())
}

async fn run_openai_session_scenario(
    runner: &Runner,
    agent: &Agent,
    steps: &[ScenarioStep],
) -> Result<(), AgentsError> {
    let session = OpenAIConversationsSession::new("openai-hitl-scenario");
    println!(
        "[OpenAIConversationsSession] conversation id: {}",
        session.conversation_id().await
    );
    for step in steps {
        run_step(runner, agent, &session, "OpenAIConversationsSession", step).await?;
    }
    println!(
        "[OpenAIConversationsSession] last response id: {}",
        session.last_response_id().await.unwrap_or_default()
    );
    Ok(())
}

async fn run_step(
    runner: &Runner,
    agent: &Agent,
    session: &(dyn Session + Sync),
    label: &str,
    step: &ScenarioStep,
) -> Result<(), AgentsError> {
    let result = runner
        .run_with_session(agent, step.message, session)
        .await?;
    if result.interruptions.is_empty() {
        return Err(AgentsError::message(format!(
            "[{label}] expected an approval interruption"
        )));
    }

    let mut state = result
        .durable_state()
        .cloned()
        .ok_or_else(|| AgentsError::message("interrupted run did not include state"))?;
    for interruption in &result.interruptions {
        if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
            match step.approval {
                Approval::Approve => state.approve_for_tool(
                    interruption.call_id.clone().unwrap_or_default(),
                    interruption.tool_name.clone(),
                    Some("approved".to_owned()),
                ),
                Approval::Reject => state.reject_for_tool(
                    interruption.call_id.clone().unwrap_or_default(),
                    interruption.tool_name.clone(),
                    Some(REJECTION_OUTPUT.to_owned()),
                ),
            }
        }
    }

    let resumed = runner
        .resume_with_agent_and_session(&state, agent, session)
        .await?;
    let final_output = resumed.final_output.unwrap_or_default();
    if final_output != step.expected_output {
        return Err(AgentsError::message(format!(
            "[{label}] expected `{}` but got `{final_output}`",
            step.expected_output
        )));
    }
    let items = session.get_items().await?;
    println!(
        "[{label}] {} final output: {} (items: {})",
        step.label,
        final_output,
        items.len()
    );
    Ok(())
}

fn scenario_steps() -> Vec<ScenarioStep> {
    vec![
        ScenarioStep {
            label: "turn 1",
            message: "Fetch profile for customer 104.",
            approval: Approval::Approve,
            expected_output: format!("approved:{}", "Fetch profile for customer 104."),
        },
        ScenarioStep {
            label: "turn 2 (rehydrated)",
            message: "Update note for customer 104.",
            approval: Approval::Approve,
            expected_output: format!("approved_note:{}", "Update note for customer 104."),
        },
        ScenarioStep {
            label: "turn 3 (rejected)",
            message: "Delete note for customer 104.",
            approval: Approval::Reject,
            expected_output: REJECTION_OUTPUT.to_owned(),
        },
    ]
}

fn approval_echo_tool() -> Result<openai_agents::FunctionTool, AgentsError> {
    function_tool(
        TOOL_ECHO,
        "Echoes back the provided query after approval.",
        |_ctx: ToolContext, args: QueryArgs| async move {
            Ok::<_, AgentsError>(format!("approved:{}", args.query))
        },
    )
    .map(|tool| tool.with_needs_approval(true))
}

fn approval_note_tool() -> Result<openai_agents::FunctionTool, AgentsError> {
    function_tool(
        TOOL_NOTE,
        "Records the provided query after approval.",
        |_ctx: ToolContext, args: QueryArgs| async move {
            Ok::<_, AgentsError>(format!("approved_note:{}", args.query))
        },
    )
    .map(|tool| tool.with_needs_approval(true))
}

fn latest_user_text(input: &[InputItem]) -> String {
    input
        .iter()
        .rev()
        .find_map(InputItem::as_text)
        .unwrap_or_default()
        .to_owned()
}

fn tool_output_after_latest_user(input: &[InputItem]) -> Option<String> {
    let start_index = input
        .iter()
        .rposition(|item| matches!(item, InputItem::Text { .. }))
        .map_or(0, |index| index + 1);
    input.iter().skip(start_index).rev().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        let item_type = value.get("type").and_then(Value::as_str);
        if !matches!(item_type, Some("tool_call_output" | "function_call_output")) {
            return None;
        }
        let output = value.get("output")?;
        output
            .as_str()
            .map(|text| {
                if text.is_empty() {
                    REJECTION_OUTPUT.to_owned()
                } else {
                    text.to_owned()
                }
            })
            .or_else(|| {
                output
                    .get("text")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
    })
}
