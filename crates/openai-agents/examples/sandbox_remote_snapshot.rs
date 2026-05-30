use std::fs;

use openai_agents::{
    AgentsError, File, LocalSandboxSession, Manifest, RunConfig, SandboxAgent, SandboxCapability,
    SandboxRunConfig, prepare_sandbox_run,
};

const SNAPSHOT_CHECK_PATH: &str = "/workspace/snapshot-check.txt";
const SNAPSHOT_CHECK_CONTENT: &str = "snapshot round-trip ok\n";

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Remote Snapshot Assistant")
        .instructions("Inspect snapshot-backed workspace state before answering.")
        .default_manifest(snapshot_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::Shell,
        ])
        .build();

    let initial = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Sandbox snapshot example: create".to_owned(),
            ..RunConfig::default()
        },
    )?;
    initial
        .session
        .write_file(SNAPSHOT_CHECK_PATH, SNAPSHOT_CHECK_CONTENT)?;
    let serialized = initial.session.serialize_session_state()?;
    let original_workspace = initial.session.workspace_root();
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
            workflow_name: "Sandbox snapshot example: resume".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let restored = resumed.session.read_file(SNAPSHOT_CHECK_PATH)?;
    if restored != SNAPSHOT_CHECK_CONTENT {
        resumed.session.cleanup()?;
        return Err(AgentsError::message(format!(
            "snapshot round-trip failed: expected {SNAPSHOT_CHECK_CONTENT:?}, got {restored:?}"
        )));
    }

    println!("snapshot_roundtrip=ok");
    println!(
        "workspace_restored={}",
        resumed.session.workspace_root().exists()
    );
    println!("restored_file={}", restored.trim());
    let shell = resumed
        .session
        .run_shell("cat README.md status.md snapshot-check.txt | wc -l")?;
    println!("workspace_line_count={}", shell.stdout.trim());

    resumed.session.cleanup()?;
    Ok(())
}

fn snapshot_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "README.md",
            File::from_text(
                "# Snapshot Demo\n\nThis workspace shows a sandbox session restoring from a serialized snapshot.\n",
            ),
        )
        .with_entry(
            "status.md",
            File::from_text(
                "# Status\n\n- The first session writes a snapshot check file.\n- The resumed session verifies it came back.\n",
            ),
        )
}
