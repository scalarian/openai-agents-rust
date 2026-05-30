use openai_agents::{Agent, AgentsError, GuardrailFunctionOutput, input_guardrail, run};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let math_homework_guardrail =
        input_guardrail("math-homework", |_ctx, _agent, input| async move {
            let text = input
                .iter()
                .filter_map(|item| item.as_text())
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();

            if text.contains("solve for x") || text.contains("homework") {
                Ok(GuardrailFunctionOutput::tripwire(Some(json!({
                    "reason": "math_homework"
                }))))
            } else {
                Ok(GuardrailFunctionOutput::allow(None))
            }
        });

    let agent = Agent::builder("Customer support agent")
        .instructions("You are a customer support agent. Keep answers short and practical.")
        .input_guardrail(math_homework_guardrail)
        .build();

    match run(&agent, "Can you help me solve for x: 2x + 5 = 11?").await {
        Ok(result) => println!("{}", result.final_output.unwrap_or_default()),
        Err(AgentsError::InputGuardrailTripwire(error)) => {
            println!(
                "blocked by input guardrail: {}",
                error.guardrail_result.guardrail_name
            );
        }
        Err(error) => return Err(error),
    }

    Ok(())
}
