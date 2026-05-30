use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentHookContext, AgentHooks, AgentsError, InputItem, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunConfig, RunContextWrapper,
    RunHooks, Runner, ToolContext, ToolDefinition, ToolOutput, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct LookupArgs {
    query: String,
}

#[derive(Clone, Default)]
struct LifecycleModel;

#[async_trait]
impl Model for LifecycleModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(tool_output) = latest_tool_output(&request.input) {
            vec![OutputItem::Text {
                text: format!("final={tool_output}"),
            }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-lookup".to_owned(),
                tool_name: "lookup".to_owned(),
                arguments: json!({ "query": "rust" }),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 4,
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

#[derive(Default)]
struct ExampleRunHooks;

#[async_trait]
impl RunHooks for ExampleRunHooks {
    async fn on_agent_start(&self, context: &AgentHookContext, agent: &Agent) {
        println!("run.agent_start name={} turn={}", agent.name, context.turn);
    }

    async fn on_llm_start(
        &self,
        _context: &RunContextWrapper,
        agent: &Agent,
        _system_prompt: Option<&str>,
        input_items: &[InputItem],
    ) {
        println!(
            "run.llm_start agent={} input_items={}",
            agent.name,
            input_items.len()
        );
    }

    async fn on_llm_end(
        &self,
        _context: &RunContextWrapper,
        agent: &Agent,
        response: &ModelResponse,
    ) {
        println!(
            "run.llm_end agent={} output_items={}",
            agent.name,
            response.output.len()
        );
    }

    async fn on_tool_start(&self, context: &ToolContext, _agent: &Agent, tool: &ToolDefinition) {
        println!(
            "run.tool_start tool={} call_id={} args={}",
            tool.name, context.tool_call_id, context.tool_arguments
        );
    }

    async fn on_tool_end(
        &self,
        context: &ToolContext,
        _agent: &Agent,
        tool: &ToolDefinition,
        result: &ToolOutput,
    ) {
        println!(
            "run.tool_end tool={} call_id={} result={}",
            tool.name,
            context.tool_call_id,
            tool_output_text(result)
        );
    }

    async fn on_agent_end(&self, context: &AgentHookContext, agent: &Agent, output: Option<&str>) {
        println!(
            "run.agent_end name={} turn={} output={}",
            agent.name,
            context.turn,
            output.unwrap_or_default()
        );
    }
}

#[derive(Default)]
struct ExampleAgentHooks;

#[async_trait]
impl AgentHooks for ExampleAgentHooks {
    async fn on_start(&self, context: &AgentHookContext, agent: &Agent) {
        println!("agent.start name={} turn={}", agent.name, context.turn);
    }

    async fn on_tool_start(&self, context: &ToolContext, _agent: &Agent, tool: &ToolDefinition) {
        println!(
            "agent.tool_start tool={} call_id={}",
            tool.name, context.tool_call_id
        );
    }

    async fn on_tool_end(
        &self,
        context: &ToolContext,
        _agent: &Agent,
        tool: &ToolDefinition,
        result: &ToolOutput,
    ) {
        println!(
            "agent.tool_end tool={} call_id={} result={}",
            tool.name,
            context.tool_call_id,
            tool_output_text(result)
        );
    }

    async fn on_end(&self, context: &AgentHookContext, agent: &Agent, output: Option<&str>) {
        println!(
            "agent.end name={} turn={} output={}",
            agent.name,
            context.turn,
            output.unwrap_or_default()
        );
    }
}

fn latest_tool_output(input: &[InputItem]) -> Option<String> {
    input.iter().rev().find_map(|item| match item {
        InputItem::Json { value }
            if value.get("type").and_then(Value::as_str) == Some("tool_call_output") =>
        {
            value
                .get("output")
                .and_then(|output| output.get("text"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        }
        _ => None,
    })
}

fn tool_output_text(output: &ToolOutput) -> String {
    match output {
        ToolOutput::Text(text) => text.text.clone(),
        ToolOutput::Json { value } => value.to_string(),
        ToolOutput::Image(_) | ToolOutput::File(_) => format!("{output:?}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let lookup = function_tool(
        "lookup",
        "Look up a short fact.",
        |_ctx, args: LookupArgs| async move { Ok::<_, AgentsError>(format!("found: {}", args.query)) },
    )?;

    let agent = Agent::builder("Lifecycle Assistant")
        .instructions("Use the lookup tool, then return the result.")
        .function_tool(lookup)
        .hooks(Arc::new(ExampleAgentHooks))
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(LifecycleProvider::default()))
        .with_config(RunConfig {
            run_hooks: Some(Arc::new(ExampleRunHooks)),
            ..RunConfig::default()
        })
        .run(&agent, "Look up Rust.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
