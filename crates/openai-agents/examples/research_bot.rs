use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelSettings, OutputItem, OutputSchemaDefinition, ReasoningSettings, Result as AgentsResult,
    RunItem, Runner, StreamEvent, Usage, web_search_tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct WebSearchItem {
    reason: String,
    query: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct WebSearchPlan {
    searches: Vec<WebSearchItem>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ReportData {
    short_summary: String,
    markdown_report: String,
    follow_up_questions: Vec<String>,
}

#[derive(Clone, Default)]
struct ResearchModel;

#[async_trait]
impl Model for ResearchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input = input_text(&request.input);

        let text = if instructions.contains("come up with a set of web searches") {
            json!({
                "searches": [
                    {
                        "reason": "Understand projected EV adoption and load growth.",
                        "query": "electric vehicle adoption grid demand projections"
                    },
                    {
                        "reason": "Find mitigation strategies for charging peaks.",
                        "query": "managed charging vehicle to grid distribution grid"
                    },
                    {
                        "reason": "Assess infrastructure investment needs.",
                        "query": "utility grid upgrades for EV charging infrastructure"
                    }
                ]
            })
            .to_string()
        } else if instructions.contains("senior researcher") {
            json!({
                "short_summary": "EV adoption increases electricity demand, but the larger challenge is localized peak load from unmanaged charging. Managed charging, time-of-use pricing, and targeted distribution upgrades can reduce strain while preserving reliability.",
                "markdown_report": "# Impact of Electric Vehicles on the Grid\n\nElectric vehicles add flexible load to the power system. The total energy demand is meaningful but manageable when charging is shifted away from evening peaks.\n\n## Key Findings\n\n- Distribution circuits near dense charging clusters need targeted upgrades.\n- Managed charging and time-of-use rates reduce peak stress.\n- Vehicle-to-grid programs can eventually provide capacity, but require standards, incentives, and customer trust.\n\n## Conclusion\n\nThe grid impact is less about annual energy and more about timing, location, and coordination. Utilities that combine forecasting, managed charging, and local infrastructure planning can support EV growth without broad reliability degradation.",
                "follow_up_questions": [
                    "Which regions face the highest transformer overload risk?",
                    "How quickly can managed charging programs scale?",
                    "What incentives make vehicle-to-grid participation attractive?"
                ]
            })
            .to_string()
        } else {
            summarize_search(&input)
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 36,
                output_tokens: 48,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ResearchProvider {
    model: Arc<ResearchModel>,
}

impl ModelProvider for ResearchProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let query = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();
    let query = if query.is_empty() {
        "Impact of electric vehicles on the grid.".to_owned()
    } else {
        query
    };

    let reasoning = Some(ReasoningSettings {
        effort: Some("medium".to_owned()),
        summary: None,
    });
    let planner_agent = Agent::builder("PlannerAgent")
        .instructions(
            "You are a helpful research assistant. Given a query, come up with a set of web searches to perform.",
        )
        .model("gpt-5.5")
        .model_settings(ModelSettings {
            reasoning: reasoning.clone(),
            ..ModelSettings::default()
        })
        .output_schema(OutputSchemaDefinition::from_output_type::<WebSearchPlan>(true)?)
        .build();
    let search_agent = Agent::builder("Search agent")
        .instructions(
            "You are a research assistant. Given a search term, you search the web for that term and produce a concise summary.",
        )
        .model("gpt-5.5")
        .tool(web_search_tool())
        .build();
    let writer_agent = Agent::builder("WriterAgent")
        .instructions(
            "You are a senior researcher tasked with writing a cohesive report for a research query.",
        )
        .model("gpt-5-mini")
        .model_settings(ModelSettings {
            reasoning,
            ..ModelSettings::default()
        })
        .output_schema(OutputSchemaDefinition::from_output_type::<ReportData>(true)?)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(ResearchProvider::default()));

    println!("Starting research for: {query}");
    let plan_result = runner
        .run(&planner_agent, format!("Query: {query}"))
        .await?;
    let search_plan: WebSearchPlan = parse_json_output(&plan_result)?;
    println!("Will perform {} searches", search_plan.searches.len());

    let mut search_results = Vec::new();
    for item in &search_plan.searches {
        println!("Searching: {}", item.query);
        let input = format!(
            "Search term: {}\nReason for searching: {}",
            item.query, item.reason
        );
        let result = runner.run(&search_agent, input).await?;
        search_results.push(result.final_output.unwrap_or_default());
    }
    println!(
        "Searches finished: {}/{} succeeded",
        search_results.len(),
        search_plan.searches.len()
    );

    let writer_input = format!(
        "Original query: {query}\nSummarized search results: {}",
        serde_json::to_string(&search_results).unwrap_or_default()
    );
    let streamed = runner.run_streamed(&writer_agent, writer_input).await?;
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        if let StreamEvent::RunItemEvent(event) = event
            && let RunItem::MessageOutput { content } = event.item
            && content.as_text().is_some()
        {
            println!("Writing report...");
        }
    }
    let report_result = streamed.wait_for_completion().await?;
    let report: ReportData = parse_json_output(&report_result)?;

    println!("\n=====REPORT SUMMARY=====\n");
    println!("{}", report.short_summary);
    println!("\n=====REPORT=====\n");
    println!("{}", report.markdown_report);
    println!("\n=====FOLLOW UP QUESTIONS=====\n");
    for question in report.follow_up_questions {
        println!("- {question}");
    }

    Ok(())
}

fn summarize_search(input: &str) -> String {
    if input.contains("managed charging") {
        "Managed charging shifts flexible EV load away from peak periods, reducing transformer and feeder stress while preserving driver needs.".to_owned()
    } else if input.contains("upgrades") {
        "Utilities prioritize distribution upgrades around high-density charging corridors, fleet depots, and constrained residential circuits.".to_owned()
    } else {
        "EV adoption raises electricity demand, but the impact depends heavily on charging time, location, and available grid flexibility.".to_owned()
    }
}

fn parse_json_output<T>(result: &openai_agents::RunResult) -> Result<T, AgentsError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(result.final_output_text().unwrap_or_default())
        .map_err(|error| AgentsError::message(error.to_string()))
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
