use std::fs;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    AgentsError, File, InputItem, LocalSandboxSession, Manifest, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, RunConfig, Runner,
    SandboxAgent, SandboxCapability, SandboxRunConfig, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const BUILD_PROMPT: &str = "Build a small warehouse robot operations status module with a health check and typed robot status lookup.";
const RESUME_PROMPT: &str =
    "Now add test coverage for the health check, known robot status, and unknown robot behavior.";

#[derive(Clone, Default)]
struct ResumeTutorialModel;

#[async_trait]
impl Model for ResumeTutorialModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let is_test_turn = input_text(&request.input).contains("test coverage");
        let output = if is_test_turn {
            resumed_test_turn(&request.input)
        } else {
            initial_build_turn(&request.input)
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 56,
                output_tokens: 30,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ResumeTutorialProvider {
    model: Arc<ResumeTutorialModel>,
}

impl ModelProvider for ResumeTutorialProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Vibe Coder")
        .model("gpt-5.5")
        .instructions(
            "Follow AGENTS.md. Build the app in the sandbox, run local smoke tests, and preserve work across resumed sessions.",
        )
        .default_manifest(resume_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(ResumeTutorialProvider::default()));

    let first = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Sandbox resume tutorial build".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let build_result = runner.run(&first.agent, BUILD_PROMPT).await?;
    println!(
        "build_final={}",
        build_result.final_output.unwrap_or_default()
    );

    let serialized = first.session.serialize_session_state()?;
    let original_workspace = first.session.workspace_root();
    let restored_state = LocalSandboxSession::deserialize_session_state(serialized)?;
    fs::remove_dir_all(&original_workspace).map_err(|error| {
        AgentsError::message(format!(
            "failed to remove original workspace {}: {error}",
            original_workspace.display()
        ))
    })?;

    let resumed = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                session_state: Some(restored_state),
                ..SandboxRunConfig::default()
            }),
            workflow_name: "Sandbox resume tutorial tests".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let test_result = runner.run(&resumed.agent, RESUME_PROMPT).await?;
    println!(
        "test_final={}",
        test_result.final_output.unwrap_or_default()
    );
    println!(
        "workspace_files:\n{}",
        resumed
            .session
            .run_shell("find . -maxdepth 2 -type f | sort")?
            .stdout
            .trim()
    );

    resumed.session.cleanup()?;
    Ok(())
}

fn initial_build_turn(input: &[InputItem]) -> Vec<OutputItem> {
    if tool_output_text_for_call(input, "write-app").is_none() {
        vec![OutputItem::ToolCall {
            call_id: "write-app".to_owned(),
            tool_name: "sandbox_apply_patch".to_owned(),
            arguments: json!({
                "path": "/workspace/app.py",
                "replacement": app_py()
            }),
            namespace: None,
        }]
    } else if tool_output_text_for_call(input, "smoke-app").is_none() {
        vec![OutputItem::ToolCall {
            call_id: "smoke-app".to_owned(),
            tool_name: "sandbox_run_shell".to_owned(),
            arguments: json!({
                "command": "python3 -m py_compile app.py && python3 - <<'PY'\nfrom app import health, robot_status\nassert health()['status'] == 'ok'\nassert robot_status('r2')['status'] == 'charging'\nprint('smoke passed')\nPY"
            }),
            namespace: None,
        }]
    } else {
        let smoke = tool_output_text_for_call(input, "smoke-app")
            .map(stdout_section)
            .unwrap_or_default()
            .trim()
            .to_owned();
        vec![OutputItem::Text {
            text: format!("Built app.py and verified it before freezing the sandbox: {smoke}"),
        }]
    }
}

fn resumed_test_turn(input: &[InputItem]) -> Vec<OutputItem> {
    if tool_output_text_for_call(input, "read-existing-app").is_none() {
        vec![OutputItem::ToolCall {
            call_id: "read-existing-app".to_owned(),
            tool_name: "sandbox_read_file".to_owned(),
            arguments: json!({"path": "/workspace/app.py"}),
            namespace: None,
        }]
    } else if tool_output_text_for_call(input, "write-tests").is_none() {
        vec![OutputItem::ToolCall {
            call_id: "write-tests".to_owned(),
            tool_name: "sandbox_apply_patch".to_owned(),
            arguments: json!({
                "path": "/workspace/tests/test_app.py",
                "replacement": test_app_py()
            }),
            namespace: None,
        }]
    } else if tool_output_text_for_call(input, "run-tests").is_none() {
        vec![OutputItem::ToolCall {
            call_id: "run-tests".to_owned(),
            tool_name: "sandbox_run_shell".to_owned(),
            arguments: json!({"command": "python3 tests/test_app.py"}),
            namespace: None,
        }]
    } else {
        let tests = tool_output_text_for_call(input, "run-tests")
            .map(stdout_section)
            .unwrap_or_default()
            .trim()
            .to_owned();
        vec![OutputItem::Text {
            text: format!("Resumed the frozen sandbox and added tests/test_app.py: {tests}"),
        }]
    }
}

fn resume_manifest() -> Manifest {
    Manifest::default().with_entry(
        "AGENTS.md",
        File::from_text(
            "# AGENTS.md\n\n\
             - Use Python modules for the operations API demo.\n\
             - Keep the app dependency-free for local smoke tests.\n\
             - Preserve generated files across resumed sandbox sessions.\n",
        ),
    )
}

fn app_py() -> &'static str {
    concat!(
        "from __future__ import annotations\n",
        "\n",
        "ROBOTS: dict[str, str] = {\n",
        "    \"r2\": \"charging\",\n",
        "    \"k1\": \"idle\",\n",
        "}\n",
        "\n",
        "def health() -> dict[str, str]:\n",
        "    return {\"status\": \"ok\"}\n",
        "\n",
        "def robot_status(robot_id: str) -> dict[str, str]:\n",
        "    if robot_id not in ROBOTS:\n",
        "        raise KeyError(robot_id)\n",
        "    return {\"robot_id\": robot_id, \"status\": ROBOTS[robot_id]}\n",
    )
}

fn test_app_py() -> &'static str {
    concat!(
        "from __future__ import annotations\n",
        "\n",
        "import sys\n",
        "from pathlib import Path\n",
        "\n",
        "sys.path.insert(0, str(Path(__file__).resolve().parents[1]))\n",
        "\n",
        "from app import health, robot_status\n",
        "\n",
        "assert health()[\"status\"] == \"ok\"\n",
        "assert robot_status(\"r2\") == {\"robot_id\": \"r2\", \"status\": \"charging\"}\n",
        "try:\n",
        "    robot_status(\"missing\")\n",
        "except KeyError:\n",
        "    pass\n",
        "else:\n",
        "    raise AssertionError(\"unknown robot should raise KeyError\")\n",
        "\n",
        "print(\"3 passed\")\n",
    )
}

fn input_text(input: &[InputItem]) -> String {
    input
        .iter()
        .filter_map(InputItem::as_text)
        .collect::<Vec<_>>()
        .join("\n")
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
