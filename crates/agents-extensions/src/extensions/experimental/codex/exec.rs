use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use agents_core::{AgentsError, Result};
use serde::{Deserialize, Serialize};

use crate::extensions::experimental::codex::thread_options::{
    ApprovalMode, ModelReasoningEffort, SandboxMode, WebSearchMode,
};

/// Arguments passed to a `codex exec --experimental-json` invocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CodexExecArgs {
    pub input: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub thread_id: Option<String>,
    pub images: Option<Vec<String>>,
    pub model: Option<String>,
    pub sandbox_mode: Option<SandboxMode>,
    pub working_directory: Option<String>,
    pub additional_directories: Option<Vec<String>>,
    pub skip_git_repo_check: Option<bool>,
    pub output_schema_file: Option<String>,
    pub model_reasoning_effort: Option<ModelReasoningEffort>,
    #[serde(skip)]
    pub signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub idle_timeout_seconds: Option<f64>,
    pub network_access_enabled: Option<bool>,
    pub web_search_mode: Option<WebSearchMode>,
    pub web_search_enabled: Option<bool>,
    pub approval_policy: Option<ApprovalMode>,
}

/// Thin subprocess wrapper for the Codex CLI.
#[derive(Clone, Debug)]
pub struct CodexExec {
    executable_path: PathBuf,
    env_override: Option<BTreeMap<String, String>>,
    pub subprocess_stream_limit_bytes: Option<usize>,
}

impl CodexExec {
    pub fn new(
        executable_path: Option<String>,
        env: Option<BTreeMap<String, String>>,
        subprocess_stream_limit_bytes: Option<usize>,
    ) -> Result<Self> {
        let executable_path = executable_path
            .map(PathBuf::from)
            .or_else(find_codex_path)
            .ok_or_else(|| AgentsError::message("could not find `codex` executable on PATH"))?;
        Ok(Self {
            executable_path,
            env_override: env,
            subprocess_stream_limit_bytes,
        })
    }

    pub fn build_command_args(&self, args: &CodexExecArgs) -> Vec<String> {
        let mut command_args = vec!["exec".to_owned(), "--experimental-json".to_owned()];

        if let Some(model) = &args.model {
            command_args.extend(["--model".to_owned(), model.clone()]);
        }
        if let Some(sandbox_mode) = args.sandbox_mode {
            command_args.extend([
                "--sandbox".to_owned(),
                serde_json::to_string(&sandbox_mode)
                    .unwrap_or_else(|_| "\"workspace-write\"".to_owned())
                    .trim_matches('"')
                    .to_owned(),
            ]);
        }
        if let Some(working_directory) = &args.working_directory {
            command_args.extend(["--cd".to_owned(), working_directory.clone()]);
        }
        if let Some(additional_directories) = &args.additional_directories {
            for directory in additional_directories {
                command_args.extend(["--add-dir".to_owned(), directory.clone()]);
            }
        }
        if args.skip_git_repo_check.unwrap_or(false) {
            command_args.push("--skip-git-repo-check".to_owned());
        }
        if let Some(output_schema_file) = &args.output_schema_file {
            command_args.extend(["--output-schema".to_owned(), output_schema_file.clone()]);
        }
        if let Some(reasoning_effort) = args.model_reasoning_effort {
            command_args.extend([
                "--config".to_owned(),
                format!(
                    "model_reasoning_effort=\"{}\"",
                    serde_json::to_string(&reasoning_effort)
                        .unwrap_or_else(|_| "\"medium\"".to_owned())
                        .trim_matches('"')
                ),
            ]);
        }
        if let Some(network_access_enabled) = args.network_access_enabled {
            command_args.extend([
                "--config".to_owned(),
                format!(
                    "sandbox_workspace_write.network_access={}",
                    network_access_enabled
                ),
            ]);
        }
        if let Some(web_search_mode) = args.web_search_mode {
            command_args.extend([
                "--config".to_owned(),
                format!(
                    "web_search=\"{}\"",
                    serde_json::to_string(&web_search_mode)
                        .unwrap_or_else(|_| "\"disabled\"".to_owned())
                        .trim_matches('"')
                ),
            ]);
        }
        if let Some(approval_policy) = args.approval_policy {
            command_args.extend([
                "--config".to_owned(),
                format!(
                    "approval_policy=\"{}\"",
                    serde_json::to_string(&approval_policy)
                        .unwrap_or_else(|_| "\"never\"".to_owned())
                        .trim_matches('"')
                ),
            ]);
        }
        if let Some(thread_id) = &args.thread_id {
            command_args.extend(["resume".to_owned(), thread_id.clone()]);
        }
        if let Some(images) = &args.images {
            for image in images {
                command_args.extend(["--image".to_owned(), image.clone()]);
            }
        }

        command_args.push("-".to_owned());
        command_args
    }

    pub async fn run(&self, args: CodexExecArgs) -> Result<Vec<String>> {
        if args
            .signal
            .as_ref()
            .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Relaxed))
        {
            return Err(AgentsError::message("codex exec cancelled before launch"));
        }

        let executable = self.executable_path.clone();
        let env = self.build_env(&args);
        let command_args = self.build_command_args(&args);
        let input = args.input.clone();

        tokio::task::spawn_blocking(move || {
            let mut command = Command::new(executable);
            command.args(&command_args);
            command.stdin(Stdio::piped());
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            if let Some(working_directory) = &args.working_directory {
                command.current_dir(working_directory);
            }
            if let Some(env) = env {
                command.env_clear();
                command.envs(env);
            }

            let mut child = command
                .spawn()
                .map_err(|error| AgentsError::message(error.to_string()))?;
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin
                    .write_all(input.as_bytes())
                    .map_err(|error| AgentsError::message(error.to_string()))?;
            }
            let output = child
                .wait_with_output()
                .map_err(|error| AgentsError::message(error.to_string()))?;
            if !output.status.success() {
                return Err(AgentsError::message(format!(
                    "Codex exec exited with code {:?}: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>())
        })
        .await
        .map_err(|error| AgentsError::message(error.to_string()))?
    }

    fn build_env(&self, args: &CodexExecArgs) -> Option<BTreeMap<String, String>> {
        let mut env = self.env_override.clone();
        if let Some(base_url) = &args.base_url {
            env.get_or_insert_with(BTreeMap::new)
                .insert("OPENAI_BASE_URL".to_owned(), base_url.clone());
        }
        if let Some(api_key) = &args.api_key {
            env.get_or_insert_with(BTreeMap::new)
                .insert("CODEX_API_KEY".to_owned(), api_key.clone());
        }
        env
    }
}

pub fn find_codex_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CODEX_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    let candidate_names = if cfg!(windows) {
        vec!["codex.exe", "codex.cmd", "codex.bat"]
    } else {
        vec!["codex"]
    };

    let path_var = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path_var) {
        for candidate in &candidate_names {
            let path = directory.join(candidate);
            if is_executable(&path) {
                return Some(path);
            }
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}
