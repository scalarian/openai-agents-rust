use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;

use futures::StreamExt;
use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentsError, ModelSettings, OpenAIProvider,
    OutputItem, RunConfig, RunInterruptionKind, RunItem, RunOptions, RunResultStreaming, Runner,
    StreamEvent, function_tool, set_tracing_disabled,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct LookupOrderArgs {
    order_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RefundArgs {
    order_id: String,
    amount: f64,
    reason: String,
}

#[derive(Debug, Serialize)]
struct Order {
    order_id: String,
    status: String,
    delivered_days_ago: u32,
    amount: f64,
    currency: String,
    item: String,
}

async fn run_streamed_turn(
    runner: &Runner,
    agent: &Agent,
    prompt: &str,
    previous_response_id: Option<String>,
) -> Result<String, AgentsError> {
    println!("\nUser: {prompt}\n");

    let streamed = runner
        .run_streamed_with_options(
            agent,
            vec![prompt.into()],
            RunOptions {
                previous_response_id,
                ..RunOptions::default()
            },
        )
        .await?;
    drain_stream(&streamed).await;
    let mut result = streamed.wait_for_completion().await?;

    while !result.interruptions.is_empty() {
        let mut state = result
            .durable_state()
            .cloned()
            .ok_or_else(|| AgentsError::message("interrupted run did not include durable state"))?;
        for interruption in &result.interruptions {
            println!(
                "[approval] auto-approving {} {}",
                interruption.tool_name.as_deref().unwrap_or_default(),
                interruption.call_id.as_deref().unwrap_or_default()
            );
            if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
                state.approve_for_tool(
                    interruption.call_id.clone().unwrap_or_default(),
                    interruption.tool_name.clone(),
                    Some("approved by example auto mode".to_owned()),
                );
            }
        }

        let resumed = runner.resume_streamed_with_agent(&state, agent).await?;
        drain_stream(&resumed).await;
        result = resumed.wait_for_completion().await?;
    }

    let response_id = result
        .last_response_id()
        .ok_or_else(|| AgentsError::message("streamed run completed without a response id"))?
        .to_owned();
    println!("response_id={response_id}");
    println!("final_output={}", result.final_output.unwrap_or_default());
    Ok(response_id)
}

async fn drain_stream(streamed: &RunResultStreaming) {
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::RawResponseEvent(raw) => {
                if raw.type_name == "response.output_text.delta"
                    && let Some(delta) = raw.data.get("delta").and_then(Value::as_str)
                {
                    print!("{delta}");
                }
            }
            StreamEvent::RunItemEvent(event) => match event.item {
                RunItem::ToolCall {
                    tool_name,
                    arguments,
                    ..
                } => println!("\n[tool call] {tool_name}({arguments})"),
                RunItem::ToolCallOutput { output, .. } => {
                    println!("[tool result] {}", output_text(&output));
                }
                RunItem::MessageOutput { content } => {
                    println!("\nAssistant:\n{}", output_text(&content));
                }
                RunItem::HandoffCall { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::Lifecycle(event) if event.name == "tool_approval_required" => {
                println!("[approval required] {}", event.data.unwrap_or_default());
            }
            StreamEvent::AgentUpdated(_) | StreamEvent::Lifecycle(_) => {}
        }
    }
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

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    set_tracing_disabled(true);

    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy".to_owned());
    if api_key == "dummy" {
        println!("Skipping run because OPENAI_API_KEY is not set.");
        return Ok(());
    }

    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.5".to_owned());
    let mut provider = OpenAIProvider::new()
        .with_api_key(api_key)
        .with_use_responses(true)
        .with_use_responses_websocket(true);
    if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
        provider = provider.with_base_url(base_url);
    }
    if let Ok(websocket_base_url) = env::var("OPENAI_WEBSOCKET_BASE_URL") {
        provider = provider.with_websocket_base_url(websocket_base_url);
    }
    let provider = Arc::new(provider);

    let policy_agent = Agent::builder("RefundPolicySpecialist")
        .instructions(
            "Orders delivered within 7 days are eligible for a full refund. Older delivered orders are not eligible.",
        )
        .model(model.clone())
        .model_settings(ModelSettings {
            max_output_tokens: Some(120),
            ..ModelSettings::default()
        })
        .build();

    let mut orders = BTreeMap::new();
    orders.insert(
        "ORD-1001".to_owned(),
        Order {
            order_id: "ORD-1001".to_owned(),
            status: "delivered".to_owned(),
            delivered_days_ago: 3,
            amount: 49.99,
            currency: "USD".to_owned(),
            item: "Wireless Mouse".to_owned(),
        },
    );
    orders.insert(
        "ORD-2002".to_owned(),
        Order {
            order_id: "ORD-2002".to_owned(),
            status: "delivered".to_owned(),
            delivered_days_ago: 12,
            amount: 129.0,
            currency: "USD".to_owned(),
            item: "Keyboard".to_owned(),
        },
    );
    let orders = Arc::new(orders);

    let lookup_order = {
        let orders = orders.clone();
        function_tool(
            "lookup_order",
            "Return deterministic order data for the demo.",
            move |_ctx, args: LookupOrderArgs| {
                let orders = orders.clone();
                async move {
                    Ok::<_, AgentsError>(orders.get(&args.order_id).map_or_else(
                        || {
                            json!({
                                "order_id": args.order_id,
                                "status": "unknown",
                                "delivered_days_ago": 999,
                                "amount": 0.0,
                                "currency": "USD",
                                "item": "unknown"
                            })
                        },
                        |order| serde_json::to_value(order).unwrap_or_else(|_| json!({})),
                    ))
                }
            },
        )?
    };
    let submit_refund = function_tool(
        "submit_refund",
        "Create a refund request. This tool requires approval.",
        |_ctx, args: RefundArgs| async move {
            let ticket = if args.order_id == "ORD-1001" {
                "RF-1001".to_owned()
            } else {
                format!(
                    "RF-{}",
                    args.order_id.chars().rev().take(4).collect::<String>()
                )
            };
            Ok::<_, AgentsError>(json!({
                "refund_ticket": ticket,
                "order_id": args.order_id,
                "amount": args.amount,
                "reason": args.reason,
                "status": "approved_pending_processing"
            }))
        },
    )?
    .with_needs_approval(true);

    let mut policy_tool_options = AgentAsToolOptions::<AgentAsToolInput>::default();
    policy_tool_options.run_config = Some(RunConfig {
        model_provider: Some(provider.clone()),
        ..RunConfig::default()
    });
    let support_agent = Agent::builder("SupportAgent")
        .instructions(
            "For refund requests: call lookup_order, call refund_policy_specialist, and if eligible call submit_refund. When asked for only the refund ticket, return only the ticket token.",
        )
        .model(model)
        .model_settings(ModelSettings {
            max_output_tokens: Some(200),
            ..ModelSettings::default()
        })
        .function_tool(lookup_order)
        .function_tool(policy_agent.as_tool::<AgentAsToolInput>(
            Some("refund_policy_specialist"),
            Some("Check refund eligibility and explain the policy decision."),
            policy_tool_options,
        )?)
        .function_tool(submit_refund)
        .build();

    let runner = Runner::new().with_model_provider(provider);
    let response_id = run_streamed_turn(
        &runner,
        &support_agent,
        "Customer wants a refund for order ORD-1001 because the mouse arrived damaged. Check the order, ask the refund policy specialist, and if it is eligible submit the refund. Reply with only the refund ticket.",
        None,
    )
    .await?;
    run_streamed_turn(
        &runner,
        &support_agent,
        "What refund ticket did you just create? Reply with only the ticket.",
        Some(response_id),
    )
    .await?;

    Ok(())
}
