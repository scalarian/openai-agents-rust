use openai_agents::realtime::{
    OpenAIRealtimeSIPModel, RealtimeAgent, RealtimeAudioFormat, RealtimeRunConfig, RealtimeRunner,
    RealtimeSessionModelSettings, RealtimeTurnDetectionConfig, TransportConfig,
    realtime_handoff_with_tool_name,
};
use openai_agents::{AgentsError, function_tool};
use schemars::JsonSchema;
use serde::Deserialize;

const WELCOME_MESSAGE: &str = "Hello, this is ABC customer service. How can I help you today?";

#[derive(Debug, Deserialize, JsonSchema)]
struct FaqLookupArgs {
    question: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateCustomerRecordArgs {
    customer_id: String,
    note: String,
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let faq_lookup_tool = function_tool(
        "faq_lookup_tool",
        "Lookup frequently asked questions.",
        |_ctx, args: FaqLookupArgs| async move {
            let question = args.question.to_lowercase();
            let answer = if question.contains("plan")
                || question.contains("wifi")
                || question.contains("wi-fi")
            {
                "We provide complimentary Wi-Fi. Join the ABC-Customer network.".to_owned()
            } else if question.contains("billing") || question.contains("invoice") {
                "Your latest invoice is available in the ABC portal under Billing > History."
                    .to_owned()
            } else {
                "I am not sure about that. I can transfer you back to triage.".to_owned()
            };
            Ok::<_, AgentsError>(answer)
        },
    )?;

    let update_customer_record = function_tool(
        "update_customer_record",
        "Record a short note about the caller.",
        |_ctx, args: UpdateCustomerRecordArgs| async move {
            Ok::<_, AgentsError>(format!(
                "Recorded note for {}: {}",
                args.customer_id, args.note
            ))
        },
    )?;

    let faq_agent = RealtimeAgent::new("FAQ Agent")
        .with_handoff_description("Handles frequently asked questions and account inquiries.")
        .with_instructions(
            "Use faq_lookup_tool for answers. If hands-on help is needed, transfer to triage.",
        )
        .with_function_tool(faq_lookup_tool);

    let records_agent = RealtimeAgent::new("Records Agent")
        .with_handoff_description("Updates customer records with notes and confirmations.")
        .with_instructions(
            "Confirm the customer ID, capture a short note, and use update_customer_record.",
        )
        .with_function_tool(update_customer_record);

    let triage_agent = RealtimeAgent::new("Triage Agent")
        .with_handoff_description("Greets callers and routes them to the right specialist.")
        .with_instructions(format!(
            "Always begin the call by saying exactly: '{WELCOME_MESSAGE}'. Then gather context and hand off when appropriate."
        ))
        .with_handoff(realtime_handoff_with_tool_name(
            faq_agent,
            "transfer_to_faq_agent",
        ))
        .with_handoff(realtime_handoff_with_tool_name(
            records_agent,
            "transfer_to_records_agent",
        ));

    let call_id =
        std::env::var("OPENAI_REALTIME_CALL_ID").unwrap_or_else(|_| "call_demo_123".to_owned());
    let sip_model = OpenAIRealtimeSIPModel {
        transport: TransportConfig {
            call_id: Some(call_id.clone()),
            ..TransportConfig::default()
        },
        ..OpenAIRealtimeSIPModel::default()
    };
    let connection_url = sip_model.connection_url();

    let runner = RealtimeRunner::new(triage_agent).with_config(RealtimeRunConfig {
        model_settings: Some(RealtimeSessionModelSettings {
            model_name: Some("gpt-realtime-2".to_owned()),
            input_audio_format: Some(RealtimeAudioFormat::G711Ulaw),
            output_audio_format: Some(RealtimeAudioFormat::G711Ulaw),
            turn_detection: Some(RealtimeTurnDetectionConfig {
                kind: Some("semantic_vad".to_owned()),
                interrupt_response: Some(true),
                ..RealtimeTurnDetectionConfig::default()
            }),
            ..RealtimeSessionModelSettings::default()
        }),
        ..RealtimeRunConfig::default()
    });

    let session = runner.run_with_model(sip_model).await?;
    let tool_names = session
        .model_settings()
        .await
        .and_then(|settings| settings.tools)
        .unwrap_or_default()
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();
    let events = session
        .send_text("Caller asks about billing and invoice history.")
        .await?;

    println!("call_id={call_id}");
    println!("connection_url={connection_url}");
    println!("realtime_tools={}", tool_names.join(","));
    println!("event_count={}", events.len());
    println!("transcript={}", session.transcript().await);
    session.close().await?;
    Ok(())
}
