use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::FileSession;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunInterruptionKind, Runner, Session, ToolContext, Usage,
    function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct LookupArgs {}

#[derive(Clone, Default)]
struct FileHitlModel;

#[async_trait]
impl Model for FileHitlModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output =
            if let Some(profile) = latest_tool_output(&request.input, "lookup_customer_profile") {
                vec![OutputItem::Text {
                    text: format!("Customer profile summary: {profile}"),
                }]
            } else {
                vec![OutputItem::ToolCall {
                    call_id: "call-profile".to_owned(),
                    tool_name: "lookup_customer_profile".to_owned(),
                    arguments: json!({}),
                    namespace: None,
                }]
            };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 18,
                output_tokens: 12,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FileHitlProvider {
    model: Arc<FileHitlModel>,
}

impl ModelProvider for FileHitlProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let dir = std::env::temp_dir().join("openai-agents-file-hitl-example");
    let session = FileSession::with_session_id(&dir, "support_101");
    session.clear_session().await?;

    let mut directory = BTreeMap::new();
    directory.insert(
        "101".to_owned(),
        "Customer Kaz S. is tier gold, prefers SMS follow ups, and values concise summaries."
            .to_owned(),
    );
    let user_id = "101".to_owned();
    let lookup_customer_profile = function_tool(
        "lookup_customer_profile",
        "Look up stored profile details for a customer by internal id.",
        move |_ctx: ToolContext, _args: LookupArgs| {
            let directory = directory.clone();
            let user_id = user_id.clone();
            async move {
                Ok::<_, AgentsError>(
                    directory
                        .get(&user_id)
                        .cloned()
                        .unwrap_or_else(|| "No customer found for that id.".to_owned()),
                )
            }
        },
    )?
    .with_needs_approval(true);

    let agent = Agent::builder("File HITL assistant")
        .instructions(
            "For every user turn you must call lookup_customer_profile before responding.",
        )
        .function_tool(lookup_customer_profile)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(FileHitlProvider::default()));

    println!("=== File Session + HITL Example ===");
    println!("items_path={}", session.items_path().display());
    println!("state_path={}", session.state_path().display());

    let initial = runner
        .run_with_session(&agent, "Summarize the customer profile.", &session)
        .await?;
    let state = initial
        .durable_state()
        .ok_or_else(|| AgentsError::message("interrupted run did not include state"))?;
    session.save_run_state(state).await?;
    println!("saved_interrupted_state=true");

    let mut restored_state = session
        .load_run_state()
        .await?
        .ok_or_else(|| AgentsError::message("expected saved run state"))?;
    for interruption in &initial.interruptions {
        println!(
            "approval_request tool={} call_id={}",
            interruption.tool_name.as_deref().unwrap_or_default(),
            interruption.call_id.as_deref().unwrap_or_default()
        );
        if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
            restored_state.approve_for_tool(
                interruption.call_id.clone().unwrap_or_default(),
                interruption.tool_name.clone(),
                Some("approved by operator".to_owned()),
            );
            println!("Approved tool call.");
        }
    }

    let result = runner
        .resume_with_agent_and_session(&restored_state, &agent, &session)
        .await?;
    if let Some(state) = result.durable_state() {
        session.save_run_state(state).await?;
    }
    println!("Assistant: {}", result.final_output.unwrap_or_default());

    let items = session.get_items().await?;
    println!("session_items={}", items.len());
    println!(
        "state_reloaded={}",
        session.load_state_json().await?.is_some()
    );
    session.clear_session().await?;
    println!("session_cleared=true");
    Ok(())
}

fn latest_tool_output(input: &[InputItem], tool_name: &str) -> Option<String> {
    input.iter().rev().find_map(|item| {
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
            .map(ToOwned::to_owned)
    })
}
