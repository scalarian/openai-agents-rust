use std::sync::Arc;

use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use openai_agents::extensions::remove_all_tools_from_handoff_input;
use openai_agents::{
    Agent, AgentsError, HandoffInputData, HandoffInputFilter, InputItem, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunItem, Runner, StreamEvent,
    ToInputListMode, Usage, function_tool, handoff,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct RandomArgs {
    max: u64,
}

#[derive(Clone, Default)]
struct MessageFilterModel;

#[async_trait]
impl Model for MessageFilterModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let prompt = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        let instructions = request.instructions.clone().unwrap_or_default();

        let text = if instructions.starts_with("You only speak Spanish") {
            let toolish_items = request
                .input
                .iter()
                .filter(|item| is_toolish_input(item))
                .count();
            format!(
                "No se tu nombre; vives en Nueva York. filtered_input_items={} toolish_items={toolish_items}",
                request.input.len()
            )
        } else if prompt.contains("por favor habla") {
            return Ok(ModelResponse {
                model: request.model,
                output: vec![OutputItem::Handoff {
                    target_agent: "Spanish Assistant".to_owned(),
                }],
                usage: Usage {
                    input_tokens: 9,
                    output_tokens: 2,
                },
                response_id: None,
                request_id: None,
            });
        } else if prompt.contains("population") {
            "New York City has about 8.5 million people.".to_owned()
        } else if let Some(number) = latest_tool_output(&request.input) {
            format!("Sure, here's a random number between 0 and 100: {number}.")
        } else if prompt.contains("random number") {
            return Ok(ModelResponse {
                model: request.model,
                output: vec![OutputItem::ToolCall {
                    call_id: "call-random".to_owned(),
                    tool_name: "random_number_tool".to_owned(),
                    arguments: json!({ "max": 100 }),
                    namespace: None,
                }],
                usage: Usage {
                    input_tokens: 8,
                    output_tokens: 2,
                },
                response_id: None,
                request_id: None,
            });
        } else {
            "Hi Sora.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
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
struct MessageFilterProvider {
    model: Arc<MessageFilterModel>,
}

impl ModelProvider for MessageFilterProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn spanish_handoff_message_filter() -> HandoffInputFilter {
    Arc::new(|data: HandoffInputData| {
        async move {
            let mut filtered = remove_all_tools_from_handoff_input(data);
            filtered.input_history = filtered.input_history.into_iter().skip(2).collect();
            filtered
        }
        .boxed()
    })
}

fn append_user(mut input: Vec<InputItem>, text: &str) -> Vec<InputItem> {
    input.push(InputItem::Text {
        text: text.to_owned(),
    });
    input
}

fn latest_tool_output(input: &[InputItem]) -> Option<String> {
    input.iter().rev().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output") {
            return None;
        }
        value
            .get("output")
            .and_then(|output| match output.get("type").and_then(Value::as_str) {
                Some("text") => output
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                Some("json") => output.get("value").map(Value::to_string),
                _ => None,
            })
    })
}

fn is_toolish_input(item: &InputItem) -> bool {
    let InputItem::Json { value } = item else {
        return false;
    };
    matches!(
        value.get("type").and_then(Value::as_str),
        Some("tool_call" | "tool_call_output" | "handoff_call" | "handoff_output")
    )
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let random_number_tool = function_tool(
        "random_number_tool",
        "Return a random integer between 0 and the given maximum.",
        |_ctx, args: RandomArgs| async move { Ok::<_, AgentsError>(json!(args.max.min(37))) },
    )?;

    let first_agent = Agent::builder("Assistant")
        .instructions("Be extremely concise.")
        .function_tool(random_number_tool)
        .build();
    let spanish_agent = Agent::builder("Spanish Assistant")
        .instructions("You only speak Spanish and are extremely concise.")
        .handoff_description("A Spanish-speaking assistant.")
        .build();
    let second_agent = Agent::builder("Assistant")
        .instructions("Be helpful. If the user speaks Spanish, hand off to the Spanish assistant.")
        .handoff(handoff(spanish_agent).with_input_filter(spanish_handoff_message_filter()))
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(MessageFilterProvider::default()));

    let result = runner.run(&first_agent, "Hi, my name is Sora.").await?;
    println!("Step 1 done");

    let result = runner
        .run_items(
            &first_agent,
            append_user(
                result.to_input_list(),
                "Can you generate a random number between 0 and 100?",
            ),
        )
        .await?;
    println!("Step 2 done");

    let result = runner
        .run_items(
            &second_agent,
            append_user(
                result.to_input_list(),
                "I live in New York City. What's the population of the city?",
            ),
        )
        .await?;
    println!("Step 3 done");

    let stream_result = runner
        .run_items_streamed(
            &second_agent,
            append_user(
                result.to_input_list(),
                "Por favor habla en espanol. Cual es mi nombre y donde vivo?",
            ),
        )
        .await?;
    let mut events = stream_result.stream_events();
    while let Some(event) = events.next().await {
        if let StreamEvent::RunItemEvent(event) = event
            && let RunItem::MessageOutput { content } = event.item
            && let OutputItem::Text { text } = content
        {
            println!("{text}");
        }
    }
    let result = stream_result.wait_for_completion().await?;
    println!("Step 4 done");

    println!("\n===Final messages===\n");
    for item in result.to_input_list_mode(ToInputListMode::Normalized) {
        println!(
            "{}",
            serde_json::to_string_pretty(&item).unwrap_or_else(|error| error.to_string())
        );
    }

    Ok(())
}
