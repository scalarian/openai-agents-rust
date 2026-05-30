use std::sync::Arc;

use async_trait::async_trait;
use futures::FutureExt;
use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentToolStreamEvent, AgentsError, InputItem,
    Model, ModelProvider, ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunItem,
    Runner, StreamEvent, Usage, function_tool, set_default_agent_runner,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct BillingArgs {
    customer_id: Option<String>,
    question: String,
}

#[derive(Clone, Default)]
struct BillingModel;

#[async_trait]
impl Model for BillingModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let input_text = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n");

        let has_billing_status_tool = request
            .tools
            .iter()
            .any(|tool| tool.name == "billing_status_checker");
        let output = if let Some(tool_output) = first_tool_output(&request.input) {
            vec![OutputItem::Text { text: tool_output }]
        } else if has_billing_status_tool {
            vec![OutputItem::ToolCall {
                call_id: "call-billing-status".to_owned(),
                tool_name: "billing_status_checker".to_owned(),
                arguments: json!({
                    "customer_id": "ABC123",
                    "question": input_text
                }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-billing-agent".to_owned(),
                tool_name: "billing_agent".to_owned(),
                arguments: json!({ "input": input_text }),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct BillingProvider {
    model: Arc<BillingModel>,
}

impl ModelProvider for BillingProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn first_tool_output(input: &[InputItem]) -> Option<String> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        let output = value.get("output")?;
        match output.get("type").and_then(Value::as_str) {
            Some("text") => output
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_owned),
            Some("json") => output.get("value").map(Value::to_string),
            _ => None,
        }
    })
}

fn handle_stream(event: AgentToolStreamEvent) -> AgentsResult<()> {
    let call = event
        .tool_call
        .as_ref()
        .map(|tool_call| tool_call.id.as_str())
        .unwrap_or("unknown");
    println!(
        "[stream] agent={} call={} event={}",
        event.agent.name,
        call,
        stream_event_name(&event.event)
    );
    Ok(())
}

fn stream_event_name(event: &StreamEvent) -> &'static str {
    match event {
        StreamEvent::RawResponseEvent(_) => "raw_response_event",
        StreamEvent::RunItemEvent(item) => match &item.item {
            RunItem::ToolCall { .. } => "tool_call",
            RunItem::ToolCallOutput { .. } => "tool_call_output",
            RunItem::MessageOutput { .. } => "message_output",
            RunItem::HandoffCall { .. } => "handoff_call",
            RunItem::HandoffOutput { .. } => "handoff_output",
            RunItem::Reasoning { .. } => "reasoning",
        },
        StreamEvent::AgentUpdated(_) => "agent_updated",
        StreamEvent::Lifecycle(_) => "lifecycle",
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let billing_status_checker = function_tool(
        "billing_status_checker",
        "Answer questions about customer billing status.",
        |_ctx, args: BillingArgs| async move {
            let normalized = args.question.to_lowercase();
            if normalized.contains("bill") || normalized.contains("billing") {
                Ok::<_, AgentsError>(format!(
                    "This customer (ID: {})'s bill is $100",
                    args.customer_id.unwrap_or_else(|| "unknown".to_owned())
                ))
            } else {
                Ok("I can only answer questions about billing.".to_owned())
            }
        },
    )?;

    let billing_agent = Agent::builder("Billing Agent")
        .instructions("You are a billing agent that answers billing questions.")
        .function_tool(billing_status_checker)
        .build();

    let mut billing_tool_options = AgentAsToolOptions::<AgentAsToolInput>::default();
    billing_tool_options.on_stream = Some(Arc::new(|event| {
        async move { handle_stream(event) }.boxed()
    }));
    let billing_agent_tool = billing_agent.as_tool::<AgentAsToolInput>(
        Some("billing_agent"),
        Some("You are a billing agent that answers billing questions."),
        billing_tool_options,
    )?;

    let main_agent = Agent::builder("Customer Support Agent")
        .instructions(
            "You are a customer support agent. Always call the billing agent to answer billing \
            questions and return the billing agent response to the user.",
        )
        .function_tool(billing_agent_tool)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(BillingProvider::default()));
    set_default_agent_runner(Some(runner.clone()));

    let result = runner
        .run(
            &main_agent,
            "Hello, my customer ID is ABC123. How much is my bill for this month?",
        )
        .await?;

    println!(
        "\nfinal_response={}",
        result.final_output.unwrap_or_default()
    );
    Ok(())
}
