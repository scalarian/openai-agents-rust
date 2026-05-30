use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage, handoff,
};

#[derive(Clone, Default)]
struct DemoModel {
    calls: Arc<Mutex<usize>>,
}

#[async_trait]
impl Model for DemoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let mut calls = self.calls.lock().expect("demo model calls lock");
        *calls += 1;

        let output = if *calls == 1 {
            vec![OutputItem::Handoff {
                target_agent: "Spanish Assistant".to_owned(),
            }]
        } else {
            vec![OutputItem::Text {
                text: "Hola. Puedo ayudarte en espanol.".to_owned(),
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
struct DemoProvider {
    model: Arc<DemoModel>,
}

impl ModelProvider for DemoProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let spanish_agent = Agent::builder("Spanish Assistant")
        .instructions("You only answer in Spanish.")
        .handoff_description("A Spanish-speaking assistant.")
        .build();

    let triage_agent = Agent::builder("Triage Assistant")
        .instructions("Answer directly unless the user asks for Spanish.")
        .handoff(handoff(spanish_agent))
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(DemoProvider::default()));
    let result = runner
        .run(&triage_agent, "Please answer in Spanish.")
        .await?;

    println!(
        "last_agent={}",
        result
            .last_agent()
            .map(|agent| agent.name.as_str())
            .unwrap_or("unknown")
    );
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
