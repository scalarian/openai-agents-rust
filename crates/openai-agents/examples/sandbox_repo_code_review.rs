use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, Runner, SandboxAgent, SandboxCapability,
    SandboxRunConfig, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const DEFAULT_PROMPT: &str = "Review the sandboxed sample repository. Run tests, inspect the workflow and simple.py, then write review artifacts into output/.";

#[derive(Clone, Default)]
struct RepoReviewModel;

#[async_trait]
impl Model for RepoReviewModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "run-tests").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "run-tests".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({"command": "cd repo && sh tests/test_simple.sh"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "read-workflow").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-workflow".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/repo/.github/workflows/test.yml"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "read-simple").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-simple".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/repo/src/sample/simple.py"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-review").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-review".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/review.md",
                    "replacement": review_markdown()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-findings").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-findings".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/findings.jsonl",
                    "replacement": findings_jsonl()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-patch").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-patch".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/fix.patch",
                    "replacement": fix_patch()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "verify-artifacts").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "verify-artifacts".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "find output -maxdepth 1 -type f | sort && grep -h 'repo/src/sample/simple.py' output/findings.jsonl output/fix.patch"
                }),
                namespace: None,
            }]
        } else {
            let test_output = tool_output_text_for_call(&request.input, "run-tests")
                .map(stdout_section)
                .unwrap_or_default()
                .trim()
                .to_owned();
            vec![OutputItem::Text {
                text: format!(
                    "Wrote code-review artifacts for two findings. Test command `sh tests/test_simple.sh` reported: {test_output}"
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 72,
                output_tokens: 34,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct RepoReviewProvider {
    model: Arc<RepoReviewModel>,
}

impl ModelProvider for RepoReviewProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Code Reviewer")
        .model("gpt-5.5")
        .instructions(
            "Review the mounted repository like a maintainer. Run tests, return exactly two findings, and write review.md, findings.jsonl, and fix.patch into output/.",
        )
        .default_manifest(repo_review_manifest())
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
            workflow_name: "Sandbox repo code review example".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let result = Runner::new()
        .with_model_provider(Arc::new(RepoReviewProvider::default()))
        .run(&prepared.agent, DEFAULT_PROMPT)
        .await?;

    println!("final_output={}", result.final_output.unwrap_or_default());
    println!(
        "review_artifacts={}",
        prepared
            .session
            .run_shell("find output -maxdepth 1 -type f | sort")?
            .stdout
            .trim()
    );
    println!(
        "findings:\n{}",
        prepared
            .session
            .read_file("/workspace/output/findings.jsonl")?
            .trim()
    );
    prepared.session.cleanup()?;
    Ok(())
}

fn repo_review_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "AGENTS.md",
            File::from_text(
                "# AGENTS.md\n\n\
                 - Review `repo/` like a maintainer.\n\
                 - Run `sh tests/test_simple.sh` from `repo/`.\n\
                 - Return exactly two findings for `.github/workflows/test.yml` and `src/sample/simple.py`.\n",
            ),
        )
        .with_entry(
            "repo",
            Dir::new()
                .with_entry(
                    ".github",
                    Dir::new().with_entry(
                        "workflows",
                        Dir::new().with_entry(
                            "test.yml",
                            File::from_text(
                                "name: tests\n\
                                 on: [push]\n\
                                 jobs:\n\
                                 \n\
                                   test:\n\
                                 \n\
                                     runs-on: ubuntu-latest\n\
                                 \n\
                                     steps:\n\
                                 \n\
                                       - uses: actions/checkout@v4\n\
                                       - run: nox -s tests\n",
                            ),
                        ),
                    ),
                )
                .with_entry(
                    "src",
                    Dir::new().with_entry(
                        "sample",
                        Dir::new()
                            .with_entry("__init__.py", File::from_text(""))
                            .with_entry(
                                "simple.py",
                                File::from_text("def add_one(number):\n    return number + 1\n"),
                            ),
                    ),
                )
                .with_entry(
                    "tests",
                    Dir::new().with_entry(
                        "test_simple.sh",
                        File::from_text(
                            "#!/usr/bin/env sh\n\
                             set -eu\n\
                             PYTHONPATH=src python3 - <<'PY'\n\
                             from sample.simple import add_one\n\
                             assert add_one(1) == 2\n\
                             print('1 passed')\n\
                             PY\n",
                        ),
                    ),
                ),
        )
        .with_entry("output", Dir::new())
}

fn review_markdown() -> &'static str {
    "# Review Summary\n\n\
     Test command: `sh tests/test_simple.sh`\n\
     Result: passed.\n\n\
     1. `repo/.github/workflows/test.yml`: `nox -s tests` is invoked without installing nox or the project test dependencies, so fresh CI runners can fail before tests execute.\n\
     2. `repo/src/sample/simple.py`: `add_one` should expose `number: int` and `-> int` so callers and type checkers get the intended contract.\n"
}

fn findings_jsonl() -> &'static str {
    "{\"file\":\"repo/.github/workflows/test.yml\",\"line_number\":12,\"comment\":\"`nox -s tests` runs without installing nox or project test dependencies first; add an install/setup step so CI is reliable on fresh runners.\"}\n\
     {\"file\":\"repo/src/sample/simple.py\",\"line_number\":1,\"comment\":\"`add_one` should be typed as `def add_one(number: int) -> int:` to document the integer contract.\"}\n"
}

fn fix_patch() -> &'static str {
    "diff --git a/repo/src/sample/simple.py b/repo/src/sample/simple.py\n\
     --- a/repo/src/sample/simple.py\n\
     +++ b/repo/src/sample/simple.py\n\
     @@ -1,2 +1,2 @@\n\
     -def add_one(number):\n\
     +def add_one(number: int) -> int:\n\
          return number + 1\n"
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
