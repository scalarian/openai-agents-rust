use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::experimental::codex::{
    ApprovalMode, CodexToolOptions, ModelReasoningEffort, SandboxMode, ThreadOptions, TurnOptions,
    codex_tool,
};
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};

#[derive(Clone, Default)]
struct CodexExampleModel;

#[async_trait]
impl Model for CodexExampleModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_codex_tool = request
            .tools
            .iter()
            .any(|tool| tool.name == "codex" && tool.description.contains("read-only"));
        let text = if has_codex_tool {
            "Codex tool configured for read-only workspace inspection with approval_policy=never. In a live run, the model would call this tool to inspect pyproject.toml and AGENTS.md.".to_owned()
        } else {
            "Codex tool was not configured.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 18,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CodexExampleProvider {
    model: Arc<CodexExampleModel>,
}

impl ModelProvider for CodexExampleProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let codex = codex_tool(CodexToolOptions {
        description: Some("Run Codex CLI in read-only mode for workspace inspection.".to_owned()),
        default_thread_options: Some(ThreadOptions {
            model: Some("gpt-5.5".to_owned()),
            sandbox_mode: Some(SandboxMode::ReadOnly),
            model_reasoning_effort: Some(ModelReasoningEffort::Low),
            network_access_enabled: Some(true),
            web_search_enabled: Some(false),
            approval_policy: Some(ApprovalMode::Never),
            ..ThreadOptions::default()
        }),
        default_turn_options: Some(TurnOptions {
            idle_timeout_seconds: Some(60.0),
            ..TurnOptions::default()
        }),
        ..CodexToolOptions::default()
    })?;
    let agent = Agent::builder("Codex Agent")
        .instructions(
            "Use the Codex tool for read-only local workspace inspection and answer concisely.",
        )
        .function_tool(codex)
        .build();
    let result = Runner::new()
        .with_model_provider(Arc::new(CodexExampleProvider::default()))
        .run(
            &agent,
            "Inspect pyproject.toml and summarize repository requirements.",
        )
        .await?;
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
