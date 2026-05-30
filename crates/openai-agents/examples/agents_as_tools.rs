use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentsError, ItemHelpers, RunItem, Runner,
};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let spanish_agent = Agent::builder("spanish_agent")
        .instructions("Translate the user's message to Spanish.")
        .handoff_description("An English to Spanish translator.")
        .build();
    let french_agent = Agent::builder("french_agent")
        .instructions("Translate the user's message to French.")
        .handoff_description("An English to French translator.")
        .build();
    let italian_agent = Agent::builder("italian_agent")
        .instructions("Translate the user's message to Italian.")
        .handoff_description("An English to Italian translator.")
        .build();

    let translate_to_spanish = spanish_agent.as_tool::<AgentAsToolInput>(
        Some("translate_to_spanish"),
        Some("Translate the user's message to Spanish."),
        AgentAsToolOptions::default(),
    )?;
    let translate_to_french = french_agent.as_tool::<AgentAsToolInput>(
        Some("translate_to_french"),
        Some("Translate the user's message to French."),
        AgentAsToolOptions::default(),
    )?;
    let translate_to_italian = italian_agent.as_tool::<AgentAsToolInput>(
        Some("translate_to_italian"),
        Some("Translate the user's message to Italian."),
        AgentAsToolOptions::default(),
    )?;

    let orchestrator_agent = Agent::builder("orchestrator_agent")
        .instructions(
            "You are a translation agent. Use the provided tools for translation requests. \
            If asked for multiple translations, call the relevant tools in order. \
            Never translate on your own.",
        )
        .function_tool(translate_to_spanish)
        .function_tool(translate_to_french)
        .function_tool(translate_to_italian)
        .build();

    let synthesizer_agent = Agent::builder("synthesizer_agent")
        .instructions(
            "Inspect the translations, correct them if needed, and produce a final response.",
        )
        .build();

    let prompt = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let prompt = if prompt.trim().is_empty() {
        "Translate 'Hello, world!' to French and Spanish.".to_owned()
    } else {
        prompt
    };

    let runner = Runner::new();
    let orchestrator_result = runner.run(&orchestrator_agent, prompt).await?;
    for item in &orchestrator_result.new_items {
        if let RunItem::MessageOutput { content } = item
            && let Some(text) = ItemHelpers::extract_text(content)
        {
            println!("translation step: {text}");
        }
    }

    let synthesizer_result = runner
        .run_items(&synthesizer_agent, orchestrator_result.to_input_list())
        .await?;

    println!("{}", synthesizer_result.final_output.unwrap_or_default());
    Ok(())
}
