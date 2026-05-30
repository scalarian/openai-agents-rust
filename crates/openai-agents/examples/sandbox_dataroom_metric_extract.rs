use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, Runner, SandboxAgent, SandboxCapability,
    SandboxRunConfig, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const DEFAULT_PROMPT: &str = "Extract explicit financial metrics from the synthetic 10-K dataroom and write one JSONL row per metric-period-source into output/financial_metrics.jsonl.";

#[derive(Clone, Default)]
struct DataroomExtractModel;

#[async_trait]
impl Model for DataroomExtractModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "inspect-sources").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "inspect-sources".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "find data -type f | sort && sed -n '1,80p' data/*.txt"
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-jsonl").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-jsonl".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/financial_metrics.jsonl",
                    "replacement": financial_metrics_jsonl()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "verify-jsonl").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "verify-jsonl".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "wc -l output/financial_metrics.jsonl && grep 'Platform segment revenue' output/financial_metrics.jsonl"
                }),
                namespace: None,
            }]
        } else {
            let verification = tool_output_text_for_call(&request.input, "verify-jsonl")
                .map(stdout_section)
                .unwrap_or_default()
                .trim()
                .to_owned();
            vec![OutputItem::Text {
                text: format!(
                    "Wrote `output/financial_metrics.jsonl` with metric-period-source rows. Verification:\n{verification}"
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 70,
                output_tokens: 36,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct DataroomExtractProvider {
    model: Arc<DataroomExtractModel>,
}

impl ModelProvider for DataroomExtractProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("10-K Metric Extractor")
        .model("gpt-5.5")
        .instructions(
            "Extract financial metrics from data/. Use shell for source inspection and write one JSONL row per metric-period-source into output/.",
        )
        .default_manifest(dataroom_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();

    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Dataroom metric extraction example".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let result = Runner::new()
        .with_model_provider(Arc::new(DataroomExtractProvider::default()))
        .run(&prepared.agent, DEFAULT_PROMPT)
        .await?;

    println!("final_output={}", result.final_output.unwrap_or_default());
    println!(
        "metrics_jsonl:\n{}",
        prepared
            .session
            .read_file("/workspace/output/financial_metrics.jsonl")?
            .trim()
    );
    prepared.session.cleanup()?;
    Ok(())
}

fn dataroom_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "data",
            Dir::new()
                .with_entry(
                    "10-k-mdna-overview.txt",
                    File::from_text(
                        "Metric,FY2025,FY2024,Unit\n\
                         Revenue,1840,1570,USD millions\n\
                         Gross margin,64.2,61.8,percent\n\
                         Operating income,420,310,USD millions\n",
                    ),
                )
                .with_entry(
                    "10-k-mdna-liquidity.txt",
                    File::from_text(
                        "Metric,FY2025,FY2024,Unit\n\
                         Net cash provided by operating activities,510,390,USD millions\n\
                         Capital expenditures,80,70,USD millions\n\
                         Free cash flow,430,320,USD millions\n",
                    ),
                )
                .with_entry(
                    "10-k-note-segments.txt",
                    File::from_text(
                        "Metric,Segment,FY2025,FY2024,Unit\n\
                         Platform segment revenue,Platform,1200,980,USD millions\n\
                         Services segment revenue,Services,640,590,USD millions\n",
                    ),
                ),
        )
        .with_entry("output", Dir::new())
}

fn financial_metrics_jsonl() -> &'static str {
    "{\"source_file\":\"data/10-k-mdna-overview.txt\",\"filing_section\":\"MD&A overview\",\"metric_name\":\"Revenue\",\"fiscal_period\":\"FY2025\",\"value\":1840,\"unit\":\"USD millions\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-overview.txt\",\"filing_section\":\"MD&A overview\",\"metric_name\":\"Revenue\",\"fiscal_period\":\"FY2024\",\"value\":1570,\"unit\":\"USD millions\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-overview.txt\",\"filing_section\":\"MD&A overview\",\"metric_name\":\"Gross margin\",\"fiscal_period\":\"FY2025\",\"value\":64.2,\"unit\":\"percent\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-overview.txt\",\"filing_section\":\"MD&A overview\",\"metric_name\":\"Gross margin\",\"fiscal_period\":\"FY2024\",\"value\":61.8,\"unit\":\"percent\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-overview.txt\",\"filing_section\":\"MD&A overview\",\"metric_name\":\"Operating income\",\"fiscal_period\":\"FY2025\",\"value\":420,\"unit\":\"USD millions\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-overview.txt\",\"filing_section\":\"MD&A overview\",\"metric_name\":\"Operating income\",\"fiscal_period\":\"FY2024\",\"value\":310,\"unit\":\"USD millions\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-liquidity.txt\",\"filing_section\":\"Liquidity\",\"metric_name\":\"Net cash provided by operating activities\",\"fiscal_period\":\"FY2025\",\"value\":510,\"unit\":\"USD millions\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-mdna-liquidity.txt\",\"filing_section\":\"Liquidity\",\"metric_name\":\"Net cash provided by operating activities\",\"fiscal_period\":\"FY2024\",\"value\":390,\"unit\":\"USD millions\",\"segment\":null}\n\
     {\"source_file\":\"data/10-k-note-segments.txt\",\"filing_section\":\"Segments\",\"metric_name\":\"Platform segment revenue\",\"fiscal_period\":\"FY2025\",\"value\":1200,\"unit\":\"USD millions\",\"segment\":\"Platform\"}\n\
     {\"source_file\":\"data/10-k-note-segments.txt\",\"filing_section\":\"Segments\",\"metric_name\":\"Platform segment revenue\",\"fiscal_period\":\"FY2024\",\"value\":980,\"unit\":\"USD millions\",\"segment\":\"Platform\"}\n"
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

fn stdout_section(output: &str) -> &str {
    output
        .split_once("stdout:\n")
        .map(|(_, after_stdout)| after_stdout)
        .and_then(|after_stdout| {
            after_stdout
                .split_once("\nstderr:")
                .map(|(stdout, _)| stdout)
        })
        .unwrap_or(output)
}
