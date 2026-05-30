use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentsError, File, InputItem, Manifest, Model,
    ModelProvider, ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunConfig,
    RunItem, Runner, SandboxAgent, SandboxCapability, SandboxRunConfig, ToolContext, Usage,
    function_tool, get_default_agent_runner, set_default_agent_runner,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct DiscountArgs {
    discount_percent: u8,
}

#[derive(Clone, Default)]
struct SandboxAgentToolModel;

#[async_trait]
impl Model for SandboxAgentToolModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = match request.model.as_deref() {
            Some("orchestrator-model") => orchestrator_output(&request.input),
            Some("pricing-model") => pricing_output(&request.input),
            Some("rollout-model") => rollout_output(&request.input),
            _ => vec![OutputItem::Text {
                text: "No model route configured for this sandbox agents-as-tools example."
                    .to_owned(),
            }],
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 48,
                output_tokens: 20,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct SandboxAgentToolProvider {
    model: Arc<SandboxAgentToolModel>,
}

impl ModelProvider for SandboxAgentToolProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let pricing_agent = SandboxAgent::builder("Pricing Packet Reviewer")
        .model("pricing-model")
        .instructions("Inspect the pricing packet and return a concise commercial risk review.")
        .default_manifest(pricing_manifest())
        .capabilities(vec![SandboxCapability::Shell])
        .build();
    let rollout_agent = SandboxAgent::builder("Rollout Risk Reviewer")
        .model("rollout-model")
        .instructions("Inspect the rollout packet and return a concise delivery risk review.")
        .default_manifest(rollout_manifest())
        .capabilities(vec![SandboxCapability::Shell])
        .build();

    let pricing_tool = pricing_agent.as_tool::<AgentAsToolInput>(
        Some("review_pricing_packet"),
        Some("Inspect the pricing packet and summarize commercial risk."),
        sandbox_tool_options(),
    )?;
    let rollout_tool = rollout_agent.as_tool::<AgentAsToolInput>(
        Some("review_rollout_risk"),
        Some("Inspect the rollout packet and summarize implementation risk."),
        sandbox_tool_options(),
    )?;

    let orchestrator = Agent::builder("Revenue Operations Coordinator")
        .model("orchestrator-model")
        .instructions(
            "Use review_pricing_packet, review_rollout_risk, and get_discount_approval_rule before making a renewal recommendation.",
        )
        .function_tool(pricing_tool)
        .function_tool(rollout_tool)
        .function_tool(discount_rule_tool()?)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(SandboxAgentToolProvider::default()));
    let previous_runner = get_default_agent_runner();
    set_default_agent_runner(Some(runner.clone()));
    let result = runner
        .run(
            &orchestrator,
            "Review the Acme renewal materials and give a short deal desk recommendation.",
        )
        .await;
    set_default_agent_runner(Some(previous_runner));
    let result = result?;

    println!("[tools used] {}", tool_names(&result.new_items).join(", "));
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn sandbox_tool_options() -> AgentAsToolOptions<AgentAsToolInput> {
    let mut options = AgentAsToolOptions::default();
    options.run_config = Some(RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        ..RunConfig::default()
    });
    options.max_turns = Some(4);
    options
}

fn orchestrator_output(input: &[InputItem]) -> Vec<OutputItem> {
    if tool_output_text_for(input, "review_pricing_packet").is_none() {
        return vec![OutputItem::ToolCall {
            call_id: "pricing-review".to_owned(),
            tool_name: "review_pricing_packet".to_owned(),
            arguments: json!({"input": "Inspect pricing_summary.md and commercial_notes.md."}),
            namespace: None,
        }];
    }
    if tool_output_text_for(input, "review_rollout_risk").is_none() {
        return vec![OutputItem::ToolCall {
            call_id: "rollout-review".to_owned(),
            tool_name: "review_rollout_risk".to_owned(),
            arguments: json!({"input": "Inspect rollout_plan.md and support_history.md."}),
            namespace: None,
        }];
    }
    if tool_output_text_for(input, "get_discount_approval_rule").is_none() {
        return vec![OutputItem::ToolCall {
            call_id: "approval-rule".to_owned(),
            tool_name: "get_discount_approval_rule".to_owned(),
            arguments: json!({"discount_percent": 15}),
            namespace: None,
        }];
    }

    let pricing = tool_output_text_for(input, "review_pricing_packet").unwrap_or_default();
    let rollout = tool_output_text_for(input, "review_rollout_risk").unwrap_or_default();
    let approval = tool_output_text_for(input, "get_discount_approval_rule").unwrap_or_default();
    vec![OutputItem::Text {
        text: format!(
            "Recommendation: proceed with director approval and delivery follow-up.\nPricing: {pricing}\nRollout: {rollout}\nApproval: {approval}"
        ),
    }]
}

fn pricing_output(input: &[InputItem]) -> Vec<OutputItem> {
    if tool_output_text_for_call(input, "pricing-shell").is_none() {
        return vec![OutputItem::ToolCall {
            call_id: "pricing-shell".to_owned(),
            tool_name: "sandbox_run_shell".to_owned(),
            arguments: json!({"command": "cat pricing_summary.md commercial_notes.md"}),
            namespace: None,
        }];
    }

    vec![OutputItem::Text {
        text: "requested_discount_percent=15; requested_term_months=24; pricing_risk=medium; evidence=pricing_summary.md,commercial_notes.md"
            .to_owned(),
    }]
}

fn rollout_output(input: &[InputItem]) -> Vec<OutputItem> {
    if tool_output_text_for_call(input, "rollout-shell").is_none() {
        return vec![OutputItem::ToolCall {
            call_id: "rollout-shell".to_owned(),
            tool_name: "sandbox_run_shell".to_owned(),
            arguments: json!({"command": "cat rollout_plan.md support_history.md"}),
            namespace: None,
        }];
    }

    vec![OutputItem::Text {
        text: "rollout_risk=medium; blockers=regional admin training,SSO migration in week two; evidence=rollout_plan.md,support_history.md"
            .to_owned(),
    }]
}

fn pricing_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "pricing_summary.md",
            File::from_text(
                "# Pricing summary\n\n- Current annual contract: $220,000.\n- Requested renewal term: 24 months.\n- Requested discount: 15 percent.\n- Account executive target discount band: 8 to 10 percent.\n",
            ),
        )
        .with_entry(
            "commercial_notes.md",
            File::from_text(
                "# Commercial notes\n\n- The customer expanded from 120 to 170 paid seats in the last 6 months.\n- Procurement asked for one final concession to close before quarter end.\n",
            ),
        )
}

fn rollout_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "rollout_plan.md",
            File::from_text(
                "# Rollout plan\n\n- Customer wants a 30-day rollout for three new regional teams.\n- Regional admins have not completed training yet.\n- SSO migration is scheduled for the second week of the rollout.\n",
            ),
        )
        .with_entry(
            "support_history.md",
            File::from_text(
                "# Support history\n\n- Two high-priority onboarding tickets were closed in the last quarter.\n- No open production incidents.\n- Customer success manager asked for a phased launch if the contract closes.\n",
            ),
        )
}

fn discount_rule_tool() -> Result<openai_agents::FunctionTool, AgentsError> {
    function_tool(
        "get_discount_approval_rule",
        "Return the internal approver required for a proposed discount.",
        |_ctx: ToolContext, args: DiscountArgs| async move {
            let rule = if args.discount_percent <= 10 {
                "Discounts up to 10 percent can be approved by the account executive."
            } else if args.discount_percent <= 15 {
                "Discounts from 11 to 15 percent require regional sales director approval."
            } else {
                "Discounts above 15 percent require finance and regional sales director approval."
            };
            Ok::<_, AgentsError>(rule.to_owned())
        },
    )
}

fn tool_names(items: &[RunItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            RunItem::ToolCall { tool_name, .. } => Some(tool_name.clone()),
            _ => None,
        })
        .collect()
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
