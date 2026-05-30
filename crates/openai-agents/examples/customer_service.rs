use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, FunctionTool, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunItem, Runner, ToolContext, Usage, function_tool,
    handoff,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Default)]
struct AirlineAgentContext {
    confirmation_number: Option<String>,
    seat_number: Option<String>,
    flight_number: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FaqArgs {
    question: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateSeatArgs {
    confirmation_number: String,
    new_seat: String,
}

#[derive(Clone, Default)]
struct CustomerServiceModel;

#[async_trait]
impl Model for CustomerServiceModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input = input_text(&request.input).to_lowercase();

        let output = if instructions.contains("triage") {
            vec![OutputItem::Handoff {
                target_agent: if input.contains("seat") {
                    "Seat Booking Agent".to_owned()
                } else {
                    "FAQ Agent".to_owned()
                },
            }]
        } else if instructions.contains("faq agent") {
            if let Some(answer) = latest_tool_output(&request.input, "faq_lookup_tool") {
                vec![OutputItem::Text { text: answer }]
            } else {
                vec![OutputItem::ToolCall {
                    call_id: Some("call-faq".to_owned()).unwrap(),
                    tool_name: "faq_lookup_tool".to_owned(),
                    arguments: json!({ "question": input }),
                    namespace: None,
                }]
            }
        } else if instructions.contains("seat booking agent") {
            if let Some(answer) = latest_tool_output(&request.input, "update_seat") {
                vec![OutputItem::Text { text: answer }]
            } else {
                vec![OutputItem::ToolCall {
                    call_id: "call-seat".to_owned(),
                    tool_name: "update_seat".to_owned(),
                    arguments: json!({
                        "confirmation_number": "ABC123",
                        "new_seat": "12A"
                    }),
                    namespace: None,
                }]
            }
        } else {
            vec![OutputItem::Text {
                text: "I can help with airline FAQs and seat changes.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 24,
                output_tokens: 16,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CustomerServiceProvider {
    model: Arc<CustomerServiceModel>,
}

impl ModelProvider for CustomerServiceProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let context = Arc::new(Mutex::new(AirlineAgentContext {
        flight_number: Some("FLT-208".to_owned()),
        ..AirlineAgentContext::default()
    }));

    let faq_agent = Agent::builder("FAQ Agent")
        .handoff_description("A helpful agent that can answer questions about the airline.")
        .instructions("You are an FAQ agent. Use faq_lookup_tool to answer airline questions.")
        .function_tool(faq_lookup_tool()?)
        .build();
    let seat_booking_agent = Agent::builder("Seat Booking Agent")
        .handoff_description("A helpful agent that can update a seat on a flight.")
        .instructions(
            "You are a seat booking agent. Ask for the confirmation number and desired seat, then use update_seat.",
        )
        .function_tool(update_seat_tool(context.clone())?)
        .build();
    let triage_agent = Agent::builder("Triage Agent")
        .instructions("You are a triage agent. Handoff to FAQ Agent or Seat Booking Agent.")
        .handoff(handoff(faq_agent))
        .handoff(handoff(seat_booking_agent))
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(CustomerServiceProvider::default()));

    for message in [
        "How many bags can I bring on the plane?",
        "Please move my seat to 12A for confirmation ABC123.",
    ] {
        println!("Customer: {message}");
        let result = runner.run(&triage_agent, message).await?;
        print_run_items(&result.new_items);
        println!("Final: {}", result.final_output.unwrap_or_default());
        println!();
    }

    let snapshot = context.lock().expect("airline context lock");
    println!(
        "Context: flight={:?} confirmation={:?} seat={:?}",
        snapshot.flight_number, snapshot.confirmation_number, snapshot.seat_number
    );
    Ok(())
}

fn faq_lookup_tool() -> Result<FunctionTool, AgentsError> {
    function_tool(
        "faq_lookup_tool",
        "Lookup frequently asked questions.",
        |_ctx: ToolContext, args: FaqArgs| async move {
            let question = args.question.to_lowercase();
            let answer = if ["bag", "baggage", "luggage", "carry-on"]
                .iter()
                .any(|keyword| question.contains(keyword))
            {
                "You are allowed to bring one bag on the plane. It must be under 50 pounds and 22 inches x 14 inches x 9 inches."
            } else if ["seat", "seats", "seating", "plane"]
                .iter()
                .any(|keyword| question.contains(keyword))
            {
                "There are 120 seats on the plane, including 22 business class seats and 98 economy seats."
            } else if ["wifi", "internet", "wireless"]
                .iter()
                .any(|keyword| question.contains(keyword))
            {
                "We have free wifi on the plane. Join Airline-Wifi."
            } else {
                "I do not know the answer to that airline question."
            };
            Ok::<_, AgentsError>(answer.to_owned())
        },
    )
}

fn update_seat_tool(context: Arc<Mutex<AirlineAgentContext>>) -> Result<FunctionTool, AgentsError> {
    function_tool(
        "update_seat",
        "Update the seat for a given confirmation number.",
        move |_ctx: ToolContext, args: UpdateSeatArgs| {
            let context = context.clone();
            async move {
                let mut state = context.lock().expect("airline context lock");
                state.confirmation_number = Some(args.confirmation_number.clone());
                state.seat_number = Some(args.new_seat.clone());
                let flight = state
                    .flight_number
                    .clone()
                    .unwrap_or_else(|| "FLT-UNKNOWN".to_owned());
                Ok::<_, AgentsError>(format!(
                    "Updated seat to {} for confirmation number {} on {flight}.",
                    args.new_seat, args.confirmation_number
                ))
            }
        },
    )
}

fn input_text(input: &[InputItem]) -> String {
    input
        .iter()
        .map(|item| match item {
            InputItem::Text { text } => text.clone(),
            InputItem::Json { value } => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn latest_tool_output(input: &[InputItem], tool_name: &str) -> Option<String> {
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
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn print_run_items(items: &[RunItem]) {
    for item in items {
        match item {
            RunItem::HandoffCall { target_agent } => {
                println!("Triage Agent: handoff to {target_agent}");
            }
            RunItem::MessageOutput { content } => {
                if let Some(text) = content.as_text() {
                    println!("Message: {text}");
                }
            }
            RunItem::ToolCall { tool_name, .. } => {
                println!("Tool call: {tool_name}");
            }
            RunItem::ToolCallOutput {
                tool_name, output, ..
            } => {
                println!(
                    "Tool output ({tool_name}): {}",
                    output.as_text().unwrap_or_default()
                );
            }
            RunItem::HandoffOutput { source_agent } => {
                println!("Handoff completed from {source_agent}");
            }
            RunItem::Reasoning { .. } => {}
        }
    }
}
