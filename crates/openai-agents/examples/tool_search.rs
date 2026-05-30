use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, FunctionTool, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelSettings, OutputItem, Result as AgentsResult, RunItem, RunResult, Runner, ToolContext,
    Usage, function_tool, tool_qualified_name, tool_search_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct CustomerArgs {
    customer_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct InvoiceArgs {
    invoice_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TrackingArgs {
    tracking_number: String,
}

#[derive(Clone, Default)]
struct ToolSearchModel;

#[async_trait]
impl Model for ToolSearchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_for(&request.input, "get_shipping_eta").is_some() {
            vec![OutputItem::Text {
                text: "Tracking ZX-123 is estimated for 2026-03-06 14:00 JST.".to_owned(),
            }]
        } else if tool_output_for(&request.input, "list_open_orders").is_some() {
            vec![OutputItem::Text {
                text: "Avery Chen is an enterprise customer with open orders ord_1042 awaiting fulfillment and ord_1049 pending approval.".to_owned(),
            }]
        } else if input_mentions(&request.input, "ZX-123") {
            vec![
                OutputItem::Json {
                    value: json!({
                        "type": "tool_search_call",
                        "call_id": "search-shipping",
                        "query": "get_shipping_eta"
                    }),
                },
                OutputItem::Json {
                    value: json!({
                        "type": "tool_search_output",
                        "call_id": "search-shipping",
                        "tools": [{"type": "function", "name": "get_shipping_eta"}]
                    }),
                },
                OutputItem::ToolCall {
                    call_id: "call-shipping-eta".to_owned(),
                    tool_name: "get_shipping_eta".to_owned(),
                    arguments: json!({"tracking_number": "ZX-123"}),
                    namespace: None,
                },
            ]
        } else if input_mentions(&request.input, "customer_42") {
            vec![
                OutputItem::Json {
                    value: json!({
                        "type": "tool_search_call",
                        "call_id": "search-crm",
                        "query": "crm"
                    }),
                },
                OutputItem::Json {
                    value: json!({
                        "type": "tool_search_output",
                        "call_id": "search-crm",
                        "tools": [{"type": "namespace", "name": "crm"}]
                    }),
                },
                OutputItem::ToolCall {
                    call_id: "call-profile".to_owned(),
                    tool_name: "get_customer_profile".to_owned(),
                    arguments: json!({"customer_id": "customer_42"}),
                    namespace: Some("crm".to_owned()),
                },
                OutputItem::ToolCall {
                    call_id: "call-open-orders".to_owned(),
                    tool_name: "list_open_orders".to_owned(),
                    arguments: json!({"customer_id": "customer_42"}),
                    namespace: Some("crm".to_owned()),
                },
            ]
        } else {
            vec![OutputItem::Text {
                text: "No matching deferred tool search scenario.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 24,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ToolSearchProvider {
    model: Arc<ToolSearchModel>,
}

impl ModelProvider for ToolSearchProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let runner = Runner::new().with_model_provider(Arc::new(ToolSearchProvider::default()));

    let namespaced_agent = Agent::builder("Operations assistant")
        .model("gpt-5.5")
        .instructions(
            "For customer questions, load the full `crm` namespace. Do not search `billing` unless the user asks about invoices.",
        )
        .model_settings(ModelSettings {
            parallel_tool_calls: Some(false),
            ..ModelSettings::default()
        })
        .function_tool(namespaced_tool(customer_profile_tool()?, "crm"))
        .function_tool(namespaced_tool(open_orders_tool()?, "crm"))
        .function_tool(namespaced_tool(invoice_status_tool()?, "billing"))
        .tool(tool_search_tool())
        .build();

    let namespaced = runner
        .run(
            &namespaced_agent,
            "Look up customer_42 and list their open orders.",
        )
        .await?;
    print_result(
        "Tool search with namespaces",
        &namespaced,
        &["crm", "billing"],
    );

    let top_level_agent = Agent::builder("Shipping assistant")
        .model("gpt-5.5")
        .instructions(
            "For ETA questions, search `get_shipping_eta` before calling tools. Do not search credits unless asked.",
        )
        .model_settings(ModelSettings {
            parallel_tool_calls: Some(false),
            ..ModelSettings::default()
        })
        .function_tool(shipping_eta_tool()?.with_defer_loading(true))
        .function_tool(shipping_credit_balance_tool()?.with_defer_loading(true))
        .tool(tool_search_tool())
        .build();

    let top_level = runner
        .run(
            &top_level_agent,
            "Can you get my ETA for tracking number ZX-123?",
        )
        .await?;
    print_result(
        "Tool search with top-level deferred tools",
        &top_level,
        &["get_shipping_eta", "get_shipping_credit_balance"],
    );

    Ok(())
}

fn customer_profile_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "get_customer_profile",
        "Fetch a CRM customer profile.",
        |_ctx: ToolContext, args: CustomerArgs| async move {
            let profile = match args.customer_id.as_str() {
                "customer_42" => json!({
                    "customer_id": "customer_42",
                    "full_name": "Avery Chen",
                    "tier": "enterprise"
                }),
                customer_id => json!({"customer_id": customer_id, "tier": "unknown"}),
            };
            Ok::<_, AgentsError>(serde_json::to_string_pretty(&profile).unwrap_or_default())
        },
    )
}

fn open_orders_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "list_open_orders",
        "List open orders for a customer.",
        |_ctx: ToolContext, args: CustomerArgs| async move {
            let orders = match args.customer_id.as_str() {
                "customer_42" => json!([
                    {"order_id": "ord_1042", "status": "awaiting fulfillment"},
                    {"order_id": "ord_1049", "status": "pending approval"}
                ]),
                _ => json!([]),
            };
            Ok::<_, AgentsError>(serde_json::to_string_pretty(&orders).unwrap_or_default())
        },
    )
}

fn invoice_status_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "get_invoice_status",
        "Look up the status of an invoice.",
        |_ctx: ToolContext, args: InvoiceArgs| async move {
            let status = match args.invoice_id.as_str() {
                "inv_2001" => "paid",
                _ => "unknown",
            };
            Ok::<_, AgentsError>(status.to_owned())
        },
    )
}

fn shipping_eta_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "get_shipping_eta",
        "Look up a shipment ETA by tracking number.",
        |_ctx: ToolContext, args: TrackingArgs| async move {
            let eta = match args.tracking_number.as_str() {
                "ZX-123" => "2026-03-06 14:00 JST",
                _ => "unavailable",
            };
            Ok::<_, AgentsError>(eta.to_owned())
        },
    )
}

fn shipping_credit_balance_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "get_shipping_credit_balance",
        "Look up the available shipping credit balance for a customer.",
        |_ctx: ToolContext, args: CustomerArgs| async move {
            let balance = match args.customer_id.as_str() {
                "customer_42" => "$125.00",
                _ => "$0.00",
            };
            Ok::<_, AgentsError>(balance.to_owned())
        },
    )
}

fn namespaced_tool(mut tool: FunctionTool, namespace: &str) -> FunctionTool {
    tool.definition.namespace = Some(namespace.to_owned());
    tool.with_defer_loading(true)
}

fn input_mentions(input: &[InputItem], needle: &str) -> bool {
    input.iter().any(|item| match item {
        InputItem::Text { text } => text.contains(needle),
        InputItem::Json { value } => value.to_string().contains(needle),
    })
}

fn tool_output_for<'a>(input: &'a [InputItem], tool_name: &str) -> Option<&'a Value> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) == Some("tool_call_output")
            && value.get("tool_name").and_then(Value::as_str) == Some(tool_name)
        {
            value.get("output")
        } else {
            None
        }
    })
}

fn loaded_paths(result: &RunResult) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for value in raw_tool_search_outputs(&result.new_items) {
        for tool in value
            .get("tools")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let path = match tool.get("type").and_then(Value::as_str) {
                Some("namespace") => tool.get("name"),
                Some("function") => tool.get("name"),
                _ => tool.get("server_label"),
            };
            if let Some(path) = path.and_then(Value::as_str) {
                paths.insert(path.to_owned());
            }
        }
    }
    paths.into_iter().collect()
}

fn raw_tool_search_outputs(items: &[RunItem]) -> Vec<&Value> {
    items
        .iter()
        .filter_map(|item| {
            let RunItem::MessageOutput {
                content: OutputItem::Json { value },
            } = item
            else {
                return None;
            };
            (value.get("type").and_then(Value::as_str) == Some("tool_search_output"))
                .then_some(value)
        })
        .collect()
}

fn print_result(title: &str, result: &RunResult, registered_paths: &[&str]) {
    let loaded = loaded_paths(result);
    let untouched = registered_paths
        .iter()
        .copied()
        .filter(|path| !loaded.iter().any(|loaded| loaded == path))
        .collect::<Vec<_>>();

    println!("## {title}");
    println!("### Final output");
    println!("{}", result.final_output.as_deref().unwrap_or_default());
    println!("\n### Loaded paths");
    println!("- registered: {}", registered_paths.join(", "));
    println!(
        "- loaded: {}",
        if loaded.is_empty() {
            "none".to_owned()
        } else {
            loaded.join(", ")
        }
    );
    println!(
        "- untouched: {}",
        if untouched.is_empty() {
            "none".to_owned()
        } else {
            untouched.join(", ")
        }
    );
    println!("\n### Relevant items");
    for item in &result.new_items {
        match item {
            RunItem::MessageOutput {
                content: OutputItem::Json { value },
            } if value
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|item_type| item_type.starts_with("tool_search_")) =>
            {
                println!("- message_output: {value}");
            }
            RunItem::ToolCall {
                tool_name,
                arguments,
                namespace,
                ..
            } => {
                let qualified = tool_qualified_name(tool_name, namespace.as_deref())
                    .unwrap_or_else(|| tool_name.clone());
                println!("- tool_call: {qualified} {arguments}");
            }
            RunItem::ToolCallOutput {
                tool_name,
                output,
                namespace,
                ..
            } => {
                let qualified = tool_qualified_name(tool_name, namespace.as_deref())
                    .unwrap_or_else(|| tool_name.clone());
                println!("- tool_output: {qualified} {}", output_text(output));
            }
            _ => {}
        }
    }
    println!();
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
