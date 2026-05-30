use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, File, FunctionTool, InputItem, MCPServer, MCPTool, Manifest, Model,
    ModelProvider, ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunConfig,
    RunItem, Runner, SandboxAgent, SandboxCapability, SandboxRunConfig, StaticTool, ToolContext,
    ToolOutput, Usage, function_tool, prepare_sandbox_run,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct DiscountArgs {
    discount_percent: u8,
}

#[derive(Debug)]
struct ReferencePolicyServer;

#[async_trait]
impl MCPServer for ReferencePolicyServer {
    fn name(&self) -> &str {
        "reference-policy"
    }

    async fn connect(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn list_tools(&self) -> AgentsResult<Vec<MCPTool>> {
        Ok(vec![MCPTool {
            name: "lookup_reference_policy".to_owned(),
            description: Some("Look up an internal reference policy topic.".to_owned()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "topic": { "type": "string" }
                },
                "required": ["topic"],
                "additionalProperties": false
            })),
            ..MCPTool::default()
        }])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _meta: Option<Value>,
    ) -> AgentsResult<ToolOutput> {
        if tool_name != "lookup_reference_policy" {
            return Err(AgentsError::message(format!(
                "unknown reference policy tool `{tool_name}`"
            )));
        }
        let topic = arguments
            .get("topic")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let result = match topic {
            "security_review" => {
                "Security review remains open until the data export workflow questionnaire is approved."
            }
            "discount_approval" => {
                "Discount approval policy must be confirmed before procurement receives final terms."
            }
            _ => "No reference policy entry found for that topic.",
        };
        Ok(ToolOutput::from(result))
    }
}

#[derive(Clone, Default)]
struct SandboxWithToolsModel;

#[async_trait]
impl Model for SandboxWithToolsModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for(&request.input, "sandbox_run_shell").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "inspect-renewal".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({"command": "cat renewal_request.md account_notes.md"}),
                namespace: None,
            }]
        } else if tool_output_text_for(&request.input, "get_discount_approval_path").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "discount-approval".to_owned(),
                tool_name: "get_discount_approval_path".to_owned(),
                arguments: json!({"discount_percent": 14}),
                namespace: None,
            }]
        } else if tool_output_text_for(&request.input, "lookup_reference_policy").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "security-policy".to_owned(),
                tool_name: "lookup_reference_policy".to_owned(),
                arguments: json!({"topic": "security_review"}),
                namespace: None,
            }]
        } else {
            let approval = tool_output_text_for(&request.input, "get_discount_approval_path")
                .unwrap_or_default();
            let policy =
                tool_output_text_for(&request.input, "lookup_reference_policy").unwrap_or_default();
            vec![OutputItem::Text {
                text: format!(
                    "Discount approval: {approval}\nSecurity review: {policy}\nAccount note: Contoso expanded usage in two plants, but procurement needs the approval map before the March 28 close date."
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 40,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct SandboxWithToolsProvider {
    model: Arc<SandboxWithToolsModel>,
}

impl ModelProvider for SandboxWithToolsProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Renewal Review Assistant")
        .model("gpt-5.5")
        .instructions(
            "Inspect the renewal packet, confirm discount routing with the host function tool, and confirm security status with MCP before answering.",
        )
        .default_manifest(renewal_manifest())
        .capabilities(vec![SandboxCapability::Shell])
        .build();
    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;
    let session = prepared.session.clone();
    let mut agent = prepared.agent;

    attach_function_tool(&mut agent, discount_approval_tool()?);
    agent.mcp_servers.push(Arc::new(ReferencePolicyServer));

    let result = Runner::new()
        .with_model_provider(Arc::new(SandboxWithToolsProvider::default()))
        .run(
            &agent,
            "Review this enterprise renewal request and confirm approval and security policy.",
        )
        .await?;

    println!("[tools used] {}", tool_names(&result.new_items).join(", "));
    println!("{}", result.final_output.unwrap_or_default());
    session.cleanup()?;
    Ok(())
}

fn renewal_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "renewal_request.md",
            File::from_text(
                "# Renewal request\n\n- Customer: Contoso Manufacturing.\n- Requested discount: 14 percent.\n- Renewal term: 12 months.\n- Requested close date: March 28.\n",
            ),
        )
        .with_entry(
            "account_notes.md",
            File::from_text(
                "# Account notes\n\n- The customer expanded usage in two plants this quarter.\n- Security review for the new data export workflow was opened last week.\n- Procurement wants a final approval map before they send the order form.\n",
            ),
        )
}

fn discount_approval_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "get_discount_approval_path",
        "Return the approver required for a proposed discount percentage.",
        |_ctx: ToolContext, args: DiscountArgs| async move {
            let path = if args.discount_percent <= 10 {
                "The account executive can approve discounts up to 10 percent."
            } else if args.discount_percent <= 15 {
                "The regional sales director must approve discounts from 11 to 15 percent."
            } else {
                "Finance and the regional sales director must both approve discounts above 15 percent."
            };
            Ok::<_, AgentsError>(path.to_owned())
        },
    )
}

fn attach_function_tool(agent: &mut Agent, tool: FunctionTool) {
    agent.tools.push(StaticTool {
        definition: tool.definition.clone(),
    });
    agent.function_tools.push(tool);
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
