use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use openai_agents::sandbox::{Dir, File, LocalDir, Manifest, prepare_sandbox_run};
use openai_agents::{
    ApplyPatchOperation, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem, RunConfig,
    RunContext, RunContextWrapper, Runner, SandboxAgent, SandboxRunConfig, Tool, ToolContext,
    ToolOutput, Usage,
};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct CapturingSandboxModel {
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

#[async_trait]
impl Model for CapturingSandboxModel {
    async fn generate(&self, request: ModelRequest) -> openai_agents::Result<ModelResponse> {
        self.requests.lock().await.push(request.clone());
        Ok(ModelResponse {
            model: request.model.clone(),
            output: vec![OutputItem::Text {
                text: request
                    .input
                    .last()
                    .and_then(|item| item.as_text())
                    .unwrap_or_default()
                    .to_owned(),
            }],
            usage: Usage::default(),
            response_id: Some("sandbox-response".to_owned()),
            request_id: Some("sandbox-request".to_owned()),
        })
    }
}

#[derive(Clone)]
struct CapturingSandboxProvider {
    model: Arc<dyn Model>,
}

impl ModelProvider for CapturingSandboxProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn tool_context(tool_name: &str) -> ToolContext {
    ToolContext::new(
        RunContextWrapper::new(RunContext::default()),
        tool_name,
        &format!("call-{tool_name}"),
        "{}",
    )
}

fn unique_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("openai-agents-{label}-{nanos}"))
}

#[tokio::test]
async fn sandbox_agent_preparation_exposes_defaults_and_workspace_context() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = CapturingSandboxProvider {
        model: Arc::new(CapturingSandboxModel {
            requests: requests.clone(),
        }),
    };
    let sandbox_agent = SandboxAgent::builder("sandbox")
        .instructions("Inspect the prepared workspace before editing files.")
        .build();
    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig {
            manifest: Some(
                Manifest::default().with_entry("README.md", File::from_text("workspace readme\n")),
            ),
            ..SandboxRunConfig::default()
        }),
        ..RunConfig::default()
    };

    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config).expect("sandbox run prepares");
    Runner::new()
        .with_model_provider(Arc::new(provider))
        .run(&prepared.agent, "summarize the workspace")
        .await
        .expect("prepared sandbox run succeeds");

    let request = requests.lock().await.remove(0);
    let tool_names = request
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        tool_names,
        vec![
            "sandbox_list_files",
            "sandbox_read_file",
            "sandbox_run_shell",
            "sandbox_apply_patch",
        ]
    );
    let instructions = request.instructions.expect("sandbox instructions");
    assert!(instructions.contains("filesystem"));
    assert!(instructions.contains("shell"));
    assert!(instructions.contains("apply_patch"));
    assert!(instructions.contains("/workspace"));
    assert!(instructions.contains("README.md"));
    assert!(instructions.contains("Inspect the prepared workspace before editing files."));

    prepared.session.cleanup().expect("cleanup succeeds");
}

#[test]
fn sandbox_fresh_runs_use_isolated_temp_workspaces() {
    let manifest = Manifest::default().with_entry(
        "notes/todo.txt",
        File::from_text("runner-owned workspace\n"),
    );
    let sandbox_agent = SandboxAgent::builder("sandbox").build();
    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig {
            manifest: Some(manifest.clone()),
            ..SandboxRunConfig::default()
        }),
        ..RunConfig::default()
    };

    let prepared_a = prepare_sandbox_run(&sandbox_agent, &run_config).expect("first prep succeeds");
    let prepared_b =
        prepare_sandbox_run(&sandbox_agent, &run_config).expect("second prep succeeds");

    let root_a = prepared_a.session.workspace_root();
    let root_b = prepared_b.session.workspace_root();
    assert!(root_a.is_absolute());
    assert!(root_b.is_absolute());
    assert_ne!(root_a, root_b);
    assert!(prepared_a.session.runner_owned());
    assert!(prepared_b.session.runner_owned());
    assert_eq!(manifest.root, "/workspace");
    assert_ne!(root_a, PathBuf::from(manifest.root.clone()));
    assert!(root_a.join("notes/todo.txt").is_file());
    assert!(root_b.join("notes/todo.txt").is_file());

    prepared_a
        .session
        .cleanup()
        .expect("first cleanup succeeds");
    prepared_b
        .session
        .cleanup()
        .expect("second cleanup succeeds");

    assert!(!root_a.exists());
    assert!(!root_b.exists());
}

#[tokio::test]
async fn sandbox_manifest_entries_are_ready_before_first_tool_call() {
    let source_root = std::env::temp_dir().join("sandbox-localdir-source");
    if source_root.exists() {
        fs::remove_dir_all(&source_root).expect("remove stale source root");
    }
    fs::create_dir_all(source_root.join("nested")).expect("create localdir source");
    fs::write(source_root.join("nested/data.txt"), "copied bytes\n").expect("write source file");

    let sandbox_agent = SandboxAgent::builder("sandbox").build();
    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig {
            manifest: Some(
                Manifest::default()
                    .with_entry("notes.txt", File::from_text("inline bytes\n"))
                    .with_entry(
                        "project",
                        Dir::new().with_entry("src/lib.rs", File::from_text("pub fn demo() {}\n")),
                    )
                    .with_entry("copied", LocalDir::new(&source_root)),
            ),
            ..SandboxRunConfig::default()
        }),
        ..RunConfig::default()
    };

    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config).expect("sandbox prep succeeds");
    let list_tool = prepared
        .agent
        .find_function_tool("sandbox_list_files", None)
        .expect("list tool is attached");
    let read_tool = prepared
        .agent
        .find_function_tool("sandbox_read_file", None)
        .expect("read tool is attached");

    let root_listing = list_tool
        .invoke(
            tool_context("sandbox_list_files"),
            serde_json::json!({ "path": "/workspace" }),
        )
        .await
        .expect("list tool runs");
    let ToolOutput::Text(root_listing) = root_listing else {
        panic!("list tool should return text output");
    };
    let root_listing = root_listing.text;
    assert!(root_listing.contains("notes.txt"));
    assert!(root_listing.contains("project"));
    assert!(root_listing.contains("copied"));

    let inline_bytes = read_tool
        .invoke(
            tool_context("sandbox_read_file"),
            serde_json::json!({ "path": "/workspace/notes.txt" }),
        )
        .await
        .expect("inline file is readable");
    let ToolOutput::Text(inline_bytes) = inline_bytes else {
        panic!("read tool should return text output");
    };
    let inline_bytes = inline_bytes.text;
    assert_eq!(inline_bytes, "inline bytes\n");

    let nested_bytes = read_tool
        .invoke(
            tool_context("sandbox_read_file"),
            serde_json::json!({ "path": "/workspace/project/src/lib.rs" }),
        )
        .await
        .expect("dir child is readable");
    let ToolOutput::Text(nested_bytes) = nested_bytes else {
        panic!("read tool should return text output");
    };
    let nested_bytes = nested_bytes.text;
    assert_eq!(nested_bytes, "pub fn demo() {}\n");

    let copied_bytes = read_tool
        .invoke(
            tool_context("sandbox_read_file"),
            serde_json::json!({ "path": "/workspace/copied/nested/data.txt" }),
        )
        .await
        .expect("localdir file is readable");
    let ToolOutput::Text(copied_bytes) = copied_bytes else {
        panic!("read tool should return text output");
    };
    let copied_bytes = copied_bytes.text;
    assert_eq!(copied_bytes, "copied bytes\n");

    prepared.session.cleanup().expect("cleanup succeeds");
    fs::remove_dir_all(&source_root).expect("remove source root");
}

#[test]
fn localdir_staging_rejects_symlinked_or_swapped_sources() {
    let source_root = unique_temp_path("sandbox-localdir-symlink-root");
    let nested = source_root.join("nested");
    fs::create_dir_all(&nested).expect("create source tree");
    fs::write(source_root.join("plain.txt"), "plain\n").expect("write plain file");
    fs::write(nested.join("real.txt"), "real\n").expect("write nested file");

    #[cfg(unix)]
    std::os::unix::fs::symlink("../plain.txt", nested.join("linked.txt"))
        .expect("create source symlink");

    let sandbox_agent = SandboxAgent::builder("sandbox").build();
    let manifest = Manifest::default().with_entry("copied", LocalDir::new(&source_root));
    let error = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                manifest: Some(manifest),
                ..SandboxRunConfig::default()
            }),
            ..RunConfig::default()
        },
    )
    .expect_err("symlinked localdir source should be rejected");

    let message = error.to_string();
    assert!(message.contains("symlink"), "unexpected error: {message}");
    assert!(
        message.contains("linked.txt"),
        "unexpected error: {message}"
    );

    let stable_root = unique_temp_path("sandbox-localdir-stable-root");
    fs::create_dir_all(stable_root.join("src")).expect("create stable source tree");
    fs::write(stable_root.join("src/lib.rs"), "pub fn ok() {}\n").expect("write stable file");

    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                manifest: Some(
                    Manifest::default().with_entry("copied", LocalDir::new(&stable_root)),
                ),
                ..SandboxRunConfig::default()
            }),
            ..RunConfig::default()
        },
    )
    .expect("stable real directory should stage successfully");

    let copied = prepared
        .session
        .read_file("/workspace/copied/src/lib.rs")
        .expect("staged file should be readable");
    assert_eq!(copied, "pub fn ok() {}\n");

    let workspace_root = prepared.session.workspace_root();
    assert!(workspace_root.join("copied/src/lib.rs").is_file());

    prepared.session.cleanup().expect("cleanup succeeds");
    assert!(
        !workspace_root.exists(),
        "runner-owned workspace should be removed"
    );

    fs::remove_dir_all(&source_root).expect("remove source root");
    fs::remove_dir_all(&stable_root).expect("remove stable source root");
}

#[test]
fn sandbox_paths_reject_workspace_escape() {
    let sandbox_agent = SandboxAgent::builder("sandbox").build();
    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig {
            manifest: Some(Manifest::default().with_entry("notes.txt", File::from_text("hello\n"))),
            ..SandboxRunConfig::default()
        }),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config).expect("sandbox prep succeeds");

    let inside_path = "/workspace/notes.txt";
    assert_eq!(
        prepared
            .session
            .read_file(inside_path)
            .expect("inside reads should succeed"),
        "hello\n"
    );

    prepared
        .session
        .write_file("/workspace/generated.txt", "generated\n")
        .expect("inside writes should succeed");
    assert_eq!(
        prepared
            .session
            .read_file("generated.txt")
            .expect("relative inside reads should succeed"),
        "generated\n"
    );

    prepared
        .session
        .apply_patch(ApplyPatchOperation {
            path: "/workspace/generated.txt".to_owned(),
            replacement: "patched\n".to_owned(),
        })
        .expect("inside patch should succeed");
    assert_eq!(
        prepared
            .session
            .read_file("/workspace/generated.txt")
            .expect("patched file should be readable"),
        "patched\n"
    );

    let host_outside = unique_temp_path("sandbox-host-outside");
    fs::write(&host_outside, "host-secret\n").expect("write host file");

    #[cfg(unix)]
    std::os::unix::fs::symlink(
        &host_outside,
        prepared.session.workspace_root().join("escape-link"),
    )
    .expect("create workspace escape symlink");

    for escaped in [
        "../outside.txt",
        "/tmp/outside.txt",
        "/workspace/escape-link",
    ] {
        let error = prepared
            .session
            .read_file(escaped)
            .expect_err("escape read should fail");
        assert!(
            error
                .to_string()
                .contains("path must stay within the sandbox workspace"),
            "unexpected error: {error}"
        );
    }

    let write_error = prepared
        .session
        .write_file("/workspace/escape-link", "mutated\n")
        .expect_err("write through escape symlink should fail");
    assert!(
        write_error
            .to_string()
            .contains("path must stay within the sandbox workspace"),
        "unexpected error: {write_error}"
    );
    assert_eq!(
        fs::read_to_string(&host_outside).expect("host file should remain readable"),
        "host-secret\n"
    );

    let patch_error = prepared
        .session
        .apply_patch(ApplyPatchOperation {
            path: "/workspace/escape-link".to_owned(),
            replacement: "patched-host\n".to_owned(),
        })
        .expect_err("patch through escape symlink should fail");
    assert!(
        patch_error
            .to_string()
            .contains("path must stay within the sandbox workspace"),
        "unexpected error: {patch_error}"
    );
    assert_eq!(
        fs::read_to_string(&host_outside).expect("host file should remain unchanged"),
        "host-secret\n"
    );

    prepared.session.cleanup().expect("cleanup succeeds");
    fs::remove_file(&host_outside).expect("remove host file");
}
