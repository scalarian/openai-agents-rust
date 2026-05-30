use openai_agents::realtime::{RealtimeAgent, RealtimeRunner, realtime_handoff_with_tool_name};
use openai_agents::{AgentsError, function_tool};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct FaqLookupArgs {
    question: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateSeatArgs {
    confirmation_number: String,
    new_seat: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let faq_lookup_tool = function_tool(
        "faq_lookup_tool",
        "Lookup frequently asked questions.",
        |_ctx, args: FaqLookupArgs| async move {
            let question = args.question.to_lowercase();
            let answer = if question.contains("wifi") || question.contains("wi-fi") {
                "We have free wifi on the plane; join Airline-Wifi.".to_owned()
            } else if question.contains("bag") || question.contains("baggage") {
                "One carry-on bag is allowed under 50 pounds and 22 x 14 x 9 inches.".to_owned()
            } else {
                "I do not know the answer to that question.".to_owned()
            };
            Ok::<_, AgentsError>(answer)
        },
    )?;

    let update_seat = function_tool(
        "update_seat",
        "Update the seat for a given confirmation number.",
        |_ctx, args: UpdateSeatArgs| async move {
            Ok::<_, AgentsError>(format!(
                "Updated seat to {} for confirmation number {}.",
                args.new_seat, args.confirmation_number
            ))
        },
    )?
    .with_needs_approval(true);

    let get_weather = function_tool(
        "get_weather",
        "Get the weather in a city.",
        |_ctx, args: WeatherArgs| async move {
            Ok::<_, AgentsError>(format!("The weather in {} is sunny.", args.city))
        },
    )?;

    let faq_agent = RealtimeAgent::new("FAQ Agent")
        .with_handoff_description("A helpful agent that can answer airline FAQs.")
        .with_instructions("Use the FAQ lookup tool to answer airline policy questions.")
        .with_function_tool(faq_lookup_tool);

    let seat_booking_agent = RealtimeAgent::new("Seat Booking Agent")
        .with_handoff_description("A helpful agent that can update a seat on a flight.")
        .with_instructions(
            "Ask for the confirmation number and desired seat, then use update_seat.",
        )
        .with_function_tool(update_seat);

    let triage_agent = RealtimeAgent::new("Triage Agent")
        .with_handoff_description("Delegates customer requests to the right airline agent.")
        .with_instructions(
            "Triage airline support requests and transfer to the FAQ or seat booking agent.",
        )
        .with_function_tool(get_weather)
        .with_handoff(realtime_handoff_with_tool_name(
            faq_agent,
            "transfer_to_faq_agent",
        ))
        .with_handoff(realtime_handoff_with_tool_name(
            seat_booking_agent,
            "transfer_to_seat_booking_agent",
        ));

    let session = RealtimeRunner::new(triage_agent).run().await?;
    let settings = session.model_settings().await.unwrap_or_default();
    let tool_names = settings
        .tools
        .unwrap_or_default()
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();

    println!(
        "active_agent={}",
        session
            .active_agent()
            .await
            .map(|agent| agent.name)
            .unwrap_or_default()
    );
    println!("realtime_tools={}", tool_names.join(","));
    session.close().await?;
    Ok(())
}
