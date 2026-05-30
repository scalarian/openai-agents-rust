use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, ToolContext, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct ShellArgs {
    command: String,
}

#[derive(Clone, Default)]
struct ShellExampleModel;

#[async_trait]
impl Model for ShellExampleModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text(&request.input, "run_shell_command").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "list-workspace".to_owned(),
                tool_name: "run_shell_command".to_owned(),
                arguments: json!({
                    "command": "printf 'workspace:' && pwd && printf '\\nfiles:\\n' && find . -maxdepth 1 -type f | sort | head -5"
                }),
                namespace: None,
            }]
        } else {
            let shell_output =
                tool_output_text(&request.input, "run_shell_command").unwrap_or_default();
            vec![OutputItem::Text {
                text: format!("Shell command completed:\n{shell_output}"),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 20,
                output_tokens: 14,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ShellExampleProvider {
    model: Arc<ShellExampleModel>,
}

impl ModelProvider for ShellExampleProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let run_shell_command = function_tool(
        "run_shell_command",
        "Run a read-only shell command and return stdout, stderr, and exit status.",
        |_ctx: ToolContext, args: ShellArgs| async move { run_shell(&args.command) },
    )?;

    let agent = Agent::builder("Shell Assistant")
        .instructions("Use the shell command tool for local workspace inspection.")
        .function_tool(run_shell_command)
        .build();
    let result = Runner::new()
        .with_model_provider(Arc::new(ShellExampleProvider::default()))
        .run(&agent, "Show the list of files in the current directory.")
        .await?;
    println!("{}", result.final_output.unwrap_or_default());
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
