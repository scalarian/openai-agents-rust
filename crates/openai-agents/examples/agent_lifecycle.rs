use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentHookContext, AgentHooks, AgentsError, InputItem, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunContextWrapper, Runner,
    ToolContext, ToolDefinition, ToolOutput, Usage, function_tool, handoff,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct RandomArgs {
    max: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MultiplyArgs {
    x: u64,
}

#[derive(Clone, Default)]
struct LifecycleModel;

#[async_trait]
impl Model for LifecycleModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.clone().unwrap_or_default();
        let output = if instructions.contains("Multiply") {
            if let Some(value) = latest_tool_number(&request.input, "multiply_by_two") {
                vec![OutputItem::Text {
                    text: format!("number={value}"),
                }]
            } else {
                vec![OutputItem::ToolCall {
                    call_id: "call-multiply".to_owned(),
                    tool_name: "multiply_by_two".to_owned(),
                    arguments: json!({ "x": 37 }),
                    namespace: None,
                }]
            }
        } else if latest_tool_number(&request.input, "random_number").is_some() {
            vec![OutputItem::Handoff {
                target_agent: "Multiply Agent".to_owned(),
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-random".to_owned(),
                tool_name: "random_number".to_owned(),
                arguments: json!({ "max": 50 }),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 8,
                output_tokens: 6,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct LifecycleProvider {
    model: Arc<LifecycleModel>,
}

impl ModelProvider for LifecycleProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

struct CustomAgentHooks {
    display_name: &'static str,
    counter: Mutex<usize>,
}

impl CustomAgentHooks {
    fn new(display_name: &'static str) -> Self {
        Self {
            display_name,
            counter: Mutex::new(0),
        }
    }

    fn next(&self) -> usize {
        let mut counter = self.counter.lock().expect("agent hook counter");
        *counter += 1;
        *counter
    }
}

#[async_trait]
impl AgentHooks for CustomAgentHooks {
    async fn on_start(&self, _context: &AgentHookContext, agent: &Agent) {
        println!(
            "### ({}) {}: Agent {} started",
            self.display_name,
            self.next(),
            agent.name
        );
    }

    async fn on_tool_start(&self, _context: &ToolContext, agent: &Agent, tool: &ToolDefinition) {
        println!(
            "### ({}) {}: Agent {} started tool {}",
            self.display_name,
            self.next(),
            agent.name,
            tool.name
        );
    }

    async fn on_tool_end(
        &self,
        _context: &ToolContext,
        agent: &Agent,
        tool: &ToolDefinition,
        result: &ToolOutput,
    ) {
        println!(
            "### ({}) {}: Agent {} ended tool {} with result {}",
            self.display_name,
            self.next(),
            agent.name,
            tool.name,
            tool_output_text(result)
        );
    }

    async fn on_handoff(&self, _context: &RunContextWrapper, from_agent: &Agent, to_agent: &Agent) {
        println!(
            "### ({}) {}: Agent {} handed off to {}",
            self.display_name,
            self.next(),
            from_agent.name,
            to_agent.name
        );
    }

    async fn on_end(&self, _context: &AgentHookContext, agent: &Agent, output: Option<&str>) {
        println!(
            "### ({}) {}: Agent {} ended with output {}",
            self.display_name,
            self.next(),
            agent.name,
            output.unwrap_or_default()
        );
    }
}

fn latest_tool_number(input: &[InputItem], tool_name: &str) -> Option<u64> {
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
            .and_then(|output| output.get("value"))
            .and_then(Value::as_u64)
    })
}

fn tool_output_text(output: &ToolOutput) -> String {
    match output {
        ToolOutput::Text(value) => value.text.clone(),
        ToolOutput::Json { value } => value.to_string(),
        ToolOutput::Image(_) | ToolOutput::File(_) => format!("{output:?}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let random_number = function_tool(
        "random_number",
        "Generate a random number from 0 to max.",
        |_ctx, args: RandomArgs| async move { Ok::<_, AgentsError>(json!(args.max.min(37))) },
    )?;
    let multiply_by_two = function_tool(
        "multiply_by_two",
        "Return x times two.",
        |_ctx, args: MultiplyArgs| async move { Ok::<_, AgentsError>(json!(args.x * 2)) },
    )?;

    let multiply_agent = Agent::builder("Multiply Agent")
        .instructions("Multiply the number by 2 and then return the final result.")
        .function_tool(multiply_by_two)
        .hooks(Arc::new(CustomAgentHooks::new("Multiply Agent")))
        .build();
    let start_agent = Agent::builder("Start Agent")
        .instructions(
            "Generate a random number. If it's even, stop. If it's odd, hand off to the multiply agent.",
        )
        .function_tool(random_number)
        .handoff(handoff(multiply_agent))
        .hooks(Arc::new(CustomAgentHooks::new("Start Agent")))
        .build();

    Runner::new()
        .with_model_provider(Arc::new(LifecycleProvider::default()))
        .run(&start_agent, "Generate a random number between 0 and 50.")
        .await?;

    println!("Done!");
    Ok(())
}
