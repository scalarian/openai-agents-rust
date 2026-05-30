use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunConfig, RunItem, Runner, StreamEvent, Usage, handoff,
};

#[derive(Clone, Default)]
struct RoutingModel;

#[async_trait]
impl Model for RoutingModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input_text = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();

        let output = if instructions.contains("handoff to the appropriate agent") {
            vec![OutputItem::Handoff {
                target_agent: route_for_input(&input_text).to_owned(),
            }]
        } else {
            vec![OutputItem::Text {
                text: specialist_response(&instructions, &input_text),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 6,
                output_tokens: 6,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct RoutingProvider {
    model: Arc<RoutingModel>,
}

impl ModelProvider for RoutingProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn route_for_input(input: &str) -> &'static str {
    if input.contains("french") || input.contains("francais") || input.contains("bonjour") {
        "french_agent"
    } else if input.contains("spanish") || input.contains("espanol") || input.contains("hola") {
        "spanish_agent"
    } else {
        "english_agent"
    }
}

fn specialist_response(instructions: &str, input: &str) -> String {
    if instructions.contains("french") {
        if input.contains("good evening") {
            "Bonsoir.".to_owned()
        } else {
            "Bonjour. Je peux vous aider en francais.".to_owned()
        }
    } else if instructions.contains("spanish") {
        "Hola. Puedo ayudarte en espanol.".to_owned()
    } else {
        "Hello. I can help in English.".to_owned()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let french_agent = Agent::builder("french_agent")
        .instructions("You only speak French.")
        .handoff_description("A French-speaking assistant.")
        .build();
    let spanish_agent = Agent::builder("spanish_agent")
        .instructions("You only speak Spanish.")
        .handoff_description("A Spanish-speaking assistant.")
        .build();
    let english_agent = Agent::builder("english_agent")
        .instructions("You only speak English.")
        .handoff_description("An English-speaking assistant.")
        .build();

    let triage_agent = Agent::builder("triage_agent")
        .instructions("Handoff to the appropriate agent based on the language of the request.")
        .handoff(handoff(french_agent))
        .handoff(handoff(spanish_agent))
        .handoff(handoff(english_agent))
        .build();

    let msg = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let msg = if msg.trim().is_empty() {
        "Hello, how do I say good evening in French?".to_owned()
    } else {
        msg
    };

    let runner = Runner::new()
        .with_config(RunConfig {
            workflow_name: "Routing example".to_owned(),
            group_id: Some("routing-demo".to_owned()),
            ..RunConfig::default()
        })
        .with_model_provider(Arc::new(RoutingProvider::default()));

    let streamed = runner
        .run_items_streamed(&triage_agent, vec![InputItem::Text { text: msg }])
        .await?;
    let mut events = streamed.stream_events();

    while let Some(event) = events.next().await {
        match event {
            StreamEvent::AgentUpdated(update) => {
                println!("routed_to={}", update.new_agent.name);
            }
            StreamEvent::RunItemEvent(event) => match &event.item {
                RunItem::HandoffCall { target_agent } => {
                    println!("handoff_call={target_agent}");
                }
                RunItem::MessageOutput { content } => {
                    if let Some(text) = content.as_text() {
                        println!("message={text}");
                    }
                }
                RunItem::ToolCall { .. }
                | RunItem::ToolCallOutput { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            },
            StreamEvent::RawResponseEvent(_) | StreamEvent::Lifecycle(_) => {}
        }
    }

    let completed = streamed.wait_for_completion().await?;
    println!(
        "final_agent={}",
        completed
            .last_agent()
            .map(|agent| agent.name.as_str())
            .unwrap_or("unknown")
    );
    println!(
        "final_output={}",
        completed.final_output.unwrap_or_default()
    );
    Ok(())
}
