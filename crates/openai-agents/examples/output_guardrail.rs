use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, GuardrailFunctionOutput, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, Runner, Usage, output_guardrail,
};
use serde_json::json;

#[derive(Clone, Default)]
struct DemoModel;

#[async_trait]
impl Model for DemoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let input_text = request
            .input
            .iter()
            .filter_map(|item| item.as_text())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let text = if input_text.contains("phone") {
            "I found a phone number: 650-123-4567.".to_owned()
        } else {
            "The capital of California is Sacramento.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 3,
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
    let phone_number_check =
        output_guardrail("phone-number-check", |_ctx, _agent, output| async move {
            let combined_output = output
                .iter()
                .filter_map(|item| item.as_text())
                .collect::<Vec<_>>()
                .join("\n");

            if combined_output.contains("650") {
                Ok(GuardrailFunctionOutput::tripwire(Some(json!({
                    "phone_number_in_response": true
                }))))
            } else {
                Ok(GuardrailFunctionOutput::allow(Some(json!({
                    "phone_number_in_response": false
                }))))
            }
        });

    let agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant.")
        .output_guardrail(phone_number_check)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(DemoProvider::default()));

    let safe_result = runner
        .run(&agent, "What's the capital of California?")
        .await?;
    println!(
        "first_message={}",
        safe_result.final_output.unwrap_or_default()
    );

    match runner
        .run(
            &agent,
            "My phone number is 650-123-4567. Where do you think I live?",
        )
        .await
    {
        Ok(result) => println!(
            "guardrail_did_not_trip={}",
            result.final_output.unwrap_or_default()
        ),
        Err(AgentsError::OutputGuardrailTripwire(error)) => {
            println!(
                "guardrail_tripped={}",
                error.guardrail_result.guardrail_name
            );
            println!(
                "info={}",
                error
                    .guardrail_result
                    .output
                    .output_info
                    .unwrap_or_default()
            );
        }
        Err(error) => return Err(error),
    }

    Ok(())
}
