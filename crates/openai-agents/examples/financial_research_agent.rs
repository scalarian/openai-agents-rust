use std::sync::Arc;

use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentToolOutputExtractor, AgentToolRunResult,
    AgentsError, FunctionTool, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, OutputSchemaDefinition, Result as AgentsResult, RunItem, Runner, StreamEvent,
    Usage, set_default_agent_runner, web_search_tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct FinancialSearchItem {
    reason: String,
    query: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct FinancialSearchPlan {
    searches: Vec<FinancialSearchItem>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct AnalysisSummary {
    summary: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct FinancialReportData {
    short_summary: String,
    markdown_report: String,
    follow_up_questions: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct VerificationResult {
    verified: bool,
    issues: String,
}

#[derive(Clone, Default)]
struct FinancialResearchModel;

#[async_trait]
impl Model for FinancialResearchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input = input_text(&request.input);

        let text = if instructions.contains("financial research planner") {
            json!({
                "searches": [
                    {
                        "reason": "Ground the long-term revenue discussion in services and installed base.",
                        "query": "Apple long term revenue drivers services installed base"
                    },
                    {
                        "reason": "Gather context on product cycle and hardware demand risks.",
                        "query": "Apple iPhone demand product cycle risks"
                    },
                    {
                        "reason": "Review regulatory and competitive risks.",
                        "query": "Apple regulatory risk app store competition"
                    }
                ]
            })
            .to_string()
        } else if instructions.contains("company fundamentals") {
            json!({
                "summary": "Apple's fundamentals are supported by a large installed base, high-margin services, disciplined capital returns, and ecosystem retention. Hardware cycles still drive reported revenue volatility, so long-term analysis should separate replacement demand from recurring services expansion."
            })
            .to_string()
        } else if instructions.contains("risk analyst") {
            json!({
                "summary": "Key risks include smartphone replacement-cycle weakness, regulatory pressure on App Store economics, supply-chain concentration, and competitive pressure in AI-enabled devices and services. The analysis should avoid claims about unreleased quarterly results."
            })
            .to_string()
        } else if instructions.contains("senior financial analyst") {
            if saw_tool_output(&request.input, "fundamentals_analysis")
                && saw_tool_output(&request.input, "risk_analysis")
            {
                json!({
                    "short_summary": "Apple's long-term drivers remain the installed base, services growth, ecosystem retention, and product innovation. The main risks are hardware-cycle softness, regulation, supply-chain concentration, and competition; this demo intentionally avoids unreleased quarterly claims.",
                    "markdown_report": "# Apple Long-Term Revenue Drivers and Key Risks\n\n## Executive Summary\n\nApple's long-term revenue profile is anchored by its installed base, services attachment, ecosystem retention, and periodic hardware innovation. Services can add recurring, higher-margin revenue, while the hardware base creates distribution for accessories, subscriptions, and future device categories.\n\n## Fundamentals\n\nThe fundamentals analyst highlights the installed base, high-margin services, capital returns, and ecosystem retention. Hardware cycles can still create volatility, so a durable view should distinguish replacement demand from recurring services expansion.\n\n## Risks\n\nThe risk analyst flags replacement-cycle weakness, App Store regulation, supply-chain concentration, and competition in AI-enabled devices and services. These risks are material enough that the report should stay caveated and avoid implying knowledge of unreleased quarterly results.\n\n## Conclusion\n\nA balanced view treats Apple as a high-quality franchise with meaningful recurring revenue opportunities, while recognizing that valuation and growth expectations depend on execution across product cycles, services regulation, and new platform investments.",
                    "follow_up_questions": [
                        "How much services growth is driven by pricing versus user growth?",
                        "What revenue exposure could App Store rule changes create?",
                        "Which new product categories could materially affect long-term growth?"
                    ]
                })
                .to_string()
            } else {
                String::new()
            }
        } else if instructions.contains("meticulous auditor") {
            json!({
                "verified": true,
                "issues": "The report is internally consistent, appropriately caveated, and does not rely on unreleased quarterly results."
            })
            .to_string()
        } else {
            summarize_financial_search(&input)
        };

        let output = if instructions.contains("senior financial analyst") && text.is_empty() {
            vec![
                OutputItem::ToolCall {
                    call_id: "call-fundamentals".to_owned(),
                    tool_name: "fundamentals_analysis".to_owned(),
                    arguments: json!({ "input": input }),
                    namespace: None,
                },
                OutputItem::ToolCall {
                    call_id: "call-risk".to_owned(),
                    tool_name: "risk_analysis".to_owned(),
                    arguments: json!({ "input": input }),
                    namespace: None,
                },
            ]
        } else {
            vec![OutputItem::Text { text }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 48,
                output_tokens: 64,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FinancialResearchProvider {
    model: Arc<FinancialResearchModel>,
}

impl ModelProvider for FinancialResearchProvider {
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
        "Write a short analysis of Apple's long-term revenue drivers and key risks. Avoid making claims about unreleased quarterly results.".to_owned()
    } else {
        query
    };

    let planner_agent = Agent::builder("FinancialPlannerAgent")
        .instructions("You are a financial research planner. Produce a set of web searches to gather the context needed.")
        .model("o3-mini")
        .output_schema(OutputSchemaDefinition::from_output_type::<FinancialSearchPlan>(true)?)
        .build();
    let search_agent = Agent::builder("FinancialSearchAgent")
        .instructions("You are a research assistant specializing in financial topics. Use web search to retrieve context and produce a short summary.")
        .model("gpt-5.5")
        .tool(web_search_tool())
        .build();
    let financials_agent = Agent::builder("FundamentalsAnalystAgent")
        .instructions("You are a financial analyst focused on company fundamentals such as revenue, profit, margins and growth trajectory.")
        .output_schema(OutputSchemaDefinition::from_output_type::<AnalysisSummary>(true)?)
        .build();
    let risk_agent = Agent::builder("RiskAnalystAgent")
        .instructions(
            "You are a risk analyst looking for potential red flags in a company's outlook.",
        )
        .output_schema(OutputSchemaDefinition::from_output_type::<AnalysisSummary>(
            true,
        )?)
        .build();
    let verifier_agent = Agent::builder("VerificationAgent")
        .instructions("You are a meticulous auditor. Verify the report is internally consistent and appropriately caveated.")
        .model("gpt-5.5")
        .output_schema(OutputSchemaDefinition::from_output_type::<VerificationResult>(true)?)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(FinancialResearchProvider::default()));
    set_default_agent_runner(Some(runner.clone()));

    println!("Starting financial research for: {query}");
    let plan_result = runner
        .run(&planner_agent, format!("Query: {query}"))
        .await?;
    let search_plan: FinancialSearchPlan = parse_json_output(&plan_result)?;
    println!("Will perform {} searches", search_plan.searches.len());

    let mut search_results = Vec::new();
    for item in &search_plan.searches {
        println!("Searching: {}", item.query);
        let result = runner
            .run(
                &search_agent,
                format!("Search term: {}\nReason: {}", item.query, item.reason),
            )
            .await?;
        search_results.push(result.final_output.unwrap_or_default());
    }
    println!(
        "Searches finished: {}/{} succeeded",
        search_results.len(),
        search_plan.searches.len()
    );

    let fundamentals_tool = analyst_tool(
        &financials_agent,
        "fundamentals_analysis",
        "Use for key financial metrics",
    )?;
    let risk_tool = analyst_tool(&risk_agent, "risk_analysis", "Use for potential red flags")?;
    let writer_agent = Agent::builder("FinancialWriterAgent")
        .instructions("You are a senior financial analyst. Synthesize search summaries into a markdown report and use available analysis tools.")
        .model("gpt-5.5")
        .function_tool(fundamentals_tool)
        .function_tool(risk_tool)
        .output_schema(OutputSchemaDefinition::from_output_type::<FinancialReportData>(true)?)
        .build();

    let writer_input = format!(
        "Original query: {query}\nSummarized search results: {}",
        serde_json::to_string(&search_results).unwrap_or_default()
    );
    let streamed = runner.run_streamed(&writer_agent, writer_input).await?;
    let mut events = streamed.stream_events();
    while let Some(event) = events.next().await {
        if let StreamEvent::RunItemEvent(event) = event {
            match event.item {
                RunItem::ToolCall { tool_name, .. } => println!("Calling {tool_name}"),
                RunItem::ToolCallOutput { tool_name, .. } => {
                    println!("Received {tool_name} summary")
                }
                RunItem::MessageOutput { .. } => println!("Writing financial report..."),
                RunItem::HandoffCall { .. }
                | RunItem::CustomToolCall { .. }
                | RunItem::CustomToolCallOutput { .. }
                | RunItem::HandoffOutput { .. }
                | RunItem::Reasoning { .. } => {}
            }
        }
    }
    let report_result = streamed.wait_for_completion().await?;
    let report: FinancialReportData = parse_json_output(&report_result)?;

    let verification_result = runner
        .run(&verifier_agent, report.markdown_report.clone())
        .await?;
    let verification: VerificationResult = parse_json_output(&verification_result)?;

    println!("\n=====REPORT=====\n");
    println!("{}", report.markdown_report);
    println!("\n=====FOLLOW UP QUESTIONS=====\n");
    for question in &report.follow_up_questions {
        println!("- {question}");
    }
    println!("\n=====VERIFICATION=====\n");
    println!(
        "verified={} issues={}",
        verification.verified, verification.issues
    );

    Ok(())
}

fn analyst_tool(agent: &Agent, name: &str, description: &str) -> Result<FunctionTool, AgentsError> {
    let mut options = AgentAsToolOptions::<AgentAsToolInput>::default();
    options.custom_output_extractor = Some(summary_extractor());
    agent.as_tool::<AgentAsToolInput>(Some(name), Some(description), options)
}

fn summary_extractor() -> AgentToolOutputExtractor {
    Arc::new(|run_result| {
        async move {
            let output = match run_result {
                AgentToolRunResult::Run(result) => result.final_output.unwrap_or_default(),
                AgentToolRunResult::Streaming(streamed) => streamed
                    .wait_for_completion()
                    .await?
                    .final_output
                    .unwrap_or_default(),
            };
            let summary: AnalysisSummary = serde_json::from_str(&output)
                .map_err(|error| AgentsError::message(error.to_string()))?;
            Ok(summary.summary)
        }
        .boxed()
    })
}

fn summarize_financial_search(input: &str) -> String {
    if input.contains("regulatory") {
        "Regulatory scrutiny can pressure App Store economics and platform control, creating uncertainty around services margins.".to_owned()
    } else if input.contains("demand") {
        "Hardware demand remains cyclical, with iPhone replacement timing and regional competition influencing reported growth.".to_owned()
    } else {
        "Apple's installed base and services ecosystem remain central to long-term revenue resilience and monetization.".to_owned()
    }
}

fn saw_tool_output(input: &[InputItem], tool_name: &str) -> bool {
    input.iter().any(|item| {
        let InputItem::Json { value } = item else {
            return false;
        };
        value.get("type").and_then(Value::as_str) == Some("tool_call_output")
            && value.get("tool_name").and_then(Value::as_str) == Some(tool_name)
    })
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
