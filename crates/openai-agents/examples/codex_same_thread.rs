use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::extensions::experimental::codex::{
    ApprovalMode, CodexToolOptions, ModelReasoningEffort, SandboxMode, ThreadOptions, codex_tool,
};
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, ModelSettings,
    OutputItem, Result as AgentsResult, Runner, Usage,
};

const THREAD_ID: &str = "thread_demo_reused";

#[derive(Clone, Default)]
struct CodexSameThreadModel;

#[async_trait]
impl Model for CodexSameThreadModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_named_codex = request
            .tools
            .iter()
            .any(|tool| tool.name == "codex_engineer");
        let prompt = request
            .input
            .iter()
            .filter_map(|item| item.as_text())
            .collect::<Vec<_>>()
            .join(" ");
        let turn = if prompt.contains("Turn 2") {
            "turn 2"
        } else {
            "turn 1"
        };
        let text = if has_named_codex {
            format!("{turn}: named Codex tool `codex_engineer` is configured to reuse {THREAD_ID}.")
        } else {
            format!("{turn}: named Codex tool was not configured.")
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 18,
                output_tokens: 12,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CodexSameThreadProvider {
    model: Arc<CodexSameThreadModel>,
}

impl ModelProvider for CodexSameThreadProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let codex = codex_tool(CodexToolOptions {
        name: Some("codex_engineer".to_owned()),
        description: Some("Run Codex CLI in read-only mode and reuse one thread.".to_owned()),
        thread_id: Some(THREAD_ID.to_owned()),
        default_thread_options: Some(ThreadOptions {
            model: Some("gpt-5.5".to_owned()),
            sandbox_mode: Some(SandboxMode::ReadOnly),
            model_reasoning_effort: Some(ModelReasoningEffort::Low),
            network_access_enabled: Some(true),
            web_search_enabled: Some(false),
            approval_policy: Some(ApprovalMode::Never),
            ..ThreadOptions::default()
        }),
        ..CodexToolOptions::default()
    })?;
    let agent = Agent::builder("Codex Agent (same thread)")
        .instructions("Always use the named Codex tool and keep the same thread between turns.")
        .model_settings(ModelSettings {
            tool_choice: Some("codex_engineer".to_owned()),
            ..ModelSettings::default()
        })
        .function_tool(codex)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(CodexSameThreadProvider::default()));

    let first = runner
        .run(&agent, "Turn 1: inspect AGENTS.md.")
        .await?
        .final_output
        .unwrap_or_default();
    println!("{first}");

    let second = runner
        .run(&agent, "Turn 2: continue from the same Codex thread.")
        .await?
        .final_output
        .unwrap_or_default();
    println!("{second}");
    println!(
        "same_thread_reused={}",
        first.contains(THREAD_ID) && second.contains(THREAD_ID)
    );
    Ok(())
}
