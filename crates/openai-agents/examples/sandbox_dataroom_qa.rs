use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, Runner, SandboxAgent, SandboxCapability,
    SandboxRunConfig, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const DEFAULT_PROMPT: &str = "How did revenue, gross margin, operating income, and operating cash flow change in FY2025 versus FY2024, and which segment contributed the most revenue?";

#[derive(Clone, Default)]
struct DataroomQaModel;

#[async_trait]
impl Model for DataroomQaModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "inspect-dataroom").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "inspect-dataroom".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "sed -n '1,80p' data/10-k-mdna-overview.txt data/10-k-mdna-liquidity.txt data/10-k-note-segments.txt"
                }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "FY2025 revenue was $1,840 million versus $1,570 million in FY2024, an increase of $270 million [1](data/10-k-mdna-overview.txt:line:3). Gross margin improved from 61.8% to 64.2%, up 2.4 percentage points [2](data/10-k-mdna-overview.txt:line:4). Operating income rose from $310 million to $420 million, up $110 million [3](data/10-k-mdna-overview.txt:line:5). Operating cash flow increased from $390 million to $510 million, up $120 million [4](data/10-k-mdna-liquidity.txt:line:3). The Platform segment contributed the most FY2025 revenue at $1,200 million [5](data/10-k-note-segments.txt:line:3).".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 48,
                output_tokens: 42,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct DataroomQaProvider {
    model: Arc<DataroomQaModel>,
}

impl ModelProvider for DataroomQaProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Dataroom Analyst")
        .model("gpt-5.5")
        .instructions(
            "Answer financial questions using only the synthetic dataroom under data/. Use shell commands for evidence and cite material claims with data/file.txt:line anchors.",
        )
        .default_manifest(dataroom_manifest())
        .capabilities(vec![SandboxCapability::Shell])
        .build();

    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Dataroom Q&A example".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let result = Runner::new()
        .with_model_provider(Arc::new(DataroomQaProvider::default()))
        .run(&prepared.agent, DEFAULT_PROMPT)
        .await?;

    println!("final_output={}", result.final_output.unwrap_or_default());
    prepared.session.cleanup()?;
    Ok(())
}

fn dataroom_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "AGENTS.md",
            File::from_text(
                "# AGENTS.md\n\n\
                 Answer using only files under data/. Cite material claims as markdown links with line anchors.\n",
            ),
        )
        .with_entry(
            "data",
            Dir::new()
                .with_entry(
                    "10-k-mdna-overview.txt",
                    File::from_text(
                        "Management Discussion Overview\n\
                         Metric,FY2025,FY2024\n\
                         Revenue,1840,1570\n\
                         Gross margin,64.2%,61.8%\n\
                         Operating income,420,310\n",
                    ),
                )
                .with_entry(
                    "10-k-mdna-liquidity.txt",
                    File::from_text(
                        "Liquidity and Cash Flows\n\
                         Metric,FY2025,FY2024\n\
                         Net cash provided by operating activities,510,390\n\
                         Capital expenditures,80,70\n\
                         Free cash flow,430,320\n",
                    ),
                )
                .with_entry(
                    "10-k-note-segments.txt",
                    File::from_text(
                        "Segment Revenue\n\
                         Segment,FY2025,FY2024\n\
                         Platform,1200,980\n\
                         Services,640,590\n",
                    ),
                ),
        )
}

fn tool_output_text_for_call<'a>(input: &'a [InputItem], call_id: &str) -> Option<&'a str> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
    })
}
