use std::sync::Arc;

use async_trait::async_trait;
use futures::FutureExt;
use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentsError, InputItem, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunContext, RunContextWrapper,
    RunOptions, Runner, Usage, set_default_agent_runner,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct ConditionalToolModel;

#[async_trait]
impl Model for ConditionalToolModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input_text = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n");

        let output = if instructions.contains("multilingual assistant") {
            let tool_outputs = collect_tool_outputs(&request.input);
            if tool_outputs.is_empty() {
                request
                    .tools
                    .iter()
                    .map(|tool| OutputItem::ToolCall {
                        call_id: format!("call-{}", tool.name),
                        tool_name: tool.name.clone(),
                        arguments: json!({ "input": input_text }),
                        namespace: None,
                    })
                    .collect()
            } else {
                vec![OutputItem::Text {
                    text: tool_outputs.join("\n"),
                }]
            }
        } else {
            vec![OutputItem::Text {
                text: language_response(&instructions),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 8,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ConditionalToolProvider {
    model: Arc<ConditionalToolModel>,
}

impl ModelProvider for ConditionalToolProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn collect_tool_outputs(input: &[InputItem]) -> Vec<String> {
    input
        .iter()
        .filter_map(|item| {
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
        .collect()
}

fn language_response(instructions: &str) -> String {
    if instructions.contains("spanish") {
        "Spanish: Buenos dias.".to_owned()
    } else if instructions.contains("french") {
        "French: Bonjour.".to_owned()
    } else if instructions.contains("italian") {
        "Italian: Buongiorno.".to_owned()
    } else {
        "No language selected.".to_owned()
    }
}

fn language_preference() -> String {
    let choice = std::env::args().nth(1).unwrap_or_else(|| "2".to_owned());
    match choice.as_str() {
        "1" | "spanish_only" => "spanish_only".to_owned(),
        "3" | "european" => "european".to_owned(),
        _ => "french_spanish".to_owned(),
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let spanish_agent = Agent::builder("spanish_agent")
        .instructions("You respond in Spanish.")
        .build();
    let french_agent = Agent::builder("french_agent")
        .instructions("You respond in French.")
        .build();
    let italian_agent = Agent::builder("italian_agent")
        .instructions("You respond in Italian.")
        .build();

    let respond_spanish = spanish_agent.as_tool::<AgentAsToolInput>(
        Some("respond_spanish"),
        Some("Respond to the user's question in Spanish."),
        AgentAsToolOptions::default(),
    )?;

    let mut french_options = AgentAsToolOptions::default();
    french_options.is_enabled = Some(Arc::new(|context, _agent| {
        async move {
            matches!(
                context.context.conversation_id.as_deref(),
                Some("french_spanish") | Some("european")
            )
        }
        .boxed()
    }));
    let respond_french = french_agent.as_tool::<AgentAsToolInput>(
        Some("respond_french"),
        Some("Respond to the user's question in French."),
        french_options,
    )?;

    let mut italian_options = AgentAsToolOptions::default();
    italian_options.is_enabled = Some(Arc::new(|context, _agent| {
        async move { matches!(context.context.conversation_id.as_deref(), Some("european")) }
            .boxed()
    }));
    let respond_italian = italian_agent.as_tool::<AgentAsToolInput>(
        Some("respond_italian"),
        Some("Respond to the user's question in Italian."),
        italian_options,
    )?;

    let orchestrator = Agent::builder("orchestrator")
        .instructions(
            "You are a multilingual assistant. You must call all available tools to provide \
            responses in different languages. Never respond in languages yourself.",
        )
        .function_tool(respond_spanish)
        .function_tool(respond_french)
        .function_tool(respond_italian)
        .build();

    let preference = language_preference();
    let context = RunContext {
        conversation_id: Some(preference.clone()),
        ..RunContext::default()
    };
    let wrapped_context = RunContextWrapper::new(context.clone());
    let available_tools = orchestrator
        .get_all_function_tools(&wrapped_context)
        .await?;
    let tool_names = available_tools
        .iter()
        .map(|tool| tool.definition.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    println!("language_preference={preference}");
    println!("available_tools={tool_names}");

    let runner = Runner::new().with_model_provider(Arc::new(ConditionalToolProvider::default()));
    set_default_agent_runner(Some(runner.clone()));

    let result = runner
        .run_with_options(
            &orchestrator,
            vec![InputItem::Text {
                text: "How do you say good morning?".to_owned(),
            }],
            RunOptions {
                context: Some(context),
                ..RunOptions::default()
            },
        )
        .await?;

    println!("response=\n{}", result.final_output.unwrap_or_default());
    Ok(())
}
