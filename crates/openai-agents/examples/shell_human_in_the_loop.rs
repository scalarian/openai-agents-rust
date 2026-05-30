use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunInterruptionKind, Runner, ToolContext, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct ShellArgs {
    command: String,
}

#[derive(Clone, Default)]
struct ShellHitlModel;

#[async_trait]
impl Model for ShellHitlModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text(&request.input, "run_shell_command").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "inspect-workspace".to_owned(),
                tool_name: "run_shell_command".to_owned(),
                arguments: json!({
                    "command": "printf 'approved shell run\\n' && find crates/openai-agents/examples -maxdepth 1 -name 'shell*.rs' -print | sort"
                }),
                namespace: None,
            }]
        } else {
            let shell_output =
                tool_output_text(&request.input, "run_shell_command").unwrap_or_default();
            vec![OutputItem::Text {
                text: format!("Approved shell command completed:\n{shell_output}"),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 22,
                output_tokens: 14,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ShellHitlProvider {
    model: Arc<ShellHitlModel>,
}

impl ModelProvider for ShellHitlProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let run_shell_command = function_tool(
        "run_shell_command",
        "Run a local shell command after human approval.",
        |_ctx: ToolContext, args: ShellArgs| async move { run_shell(&args.command) },
    )?
    .with_needs_approval(true);

    let agent = Agent::builder("Shell HITL Assistant")
        .instructions("Ask for approval before running shell commands.")
        .function_tool(run_shell_command)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(ShellHitlProvider::default()));
    let initial = runner
        .run(
            &agent,
            "List shell examples in the workspace after approval.",
        )
        .await?;
    let mut state = initial
        .durable_state()
        .cloned()
        .ok_or_else(|| AgentsError::message("shell approval run did not include state"))?;
    for interruption in &initial.interruptions {
        println!(
            "approval_request tool={} call_id={}",
            interruption.tool_name.as_deref().unwrap_or_default(),
            interruption.call_id.as_deref().unwrap_or_default()
        );
        if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
            state.approve_for_tool(
                interruption.call_id.clone().unwrap_or_default(),
                interruption.tool_name.clone(),
                Some("approved shell inspection".to_owned()),
            );
        }
    }

    let resumed = runner.resume_with_agent(&state, &agent).await?;
    println!("{}", resumed.final_output.unwrap_or_default());
    Ok(())
}

fn run_shell(command: &str) -> Result<String, AgentsError> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(format!(
        "exit_code: {}\nstdout:\n{}\nstderr:\n{}",
        output.status.code().unwrap_or_default(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn tool_output_text(input: &[InputItem], tool_name: &str) -> Option<String> {
    input.iter().find_map(|item| {
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
