use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use openai_agents::{
    AgentsError, File, Manifest, RunConfig, SandboxAgent, SandboxCapability, SandboxPathGrant,
    SandboxRunConfig, prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let temp_root = unique_temp_dir()?;
    let external_dir = temp_root.join("external");
    let scratch_dir = temp_root.join("scratch");
    fs::create_dir_all(&external_dir).map_err(|error| AgentsError::message(error.to_string()))?;
    fs::create_dir_all(&scratch_dir).map_err(|error| AgentsError::message(error.to_string()))?;
    let external_note = external_dir.join("external_renewal_note.md");
    let blocked_note = external_dir.join("blocked.txt");
    let scratch_note = scratch_dir.join("scratch_summary.md");
    fs::write(
        &external_note,
        "# External renewal note\n\nDiscount authority above 10 percent needs CFO approval.\n",
    )
    .map_err(|error| AgentsError::message(error.to_string()))?;

    let manifest = Manifest::default()
        .with_entry(
            "account_brief.md",
            File::from_text(
                "# Northwind Health\n\n- Annual contract value: $148,000.\n- Renewal date: 2026-04-15.\n",
            ),
        )
        .with_entry(
            "usage_notes.md",
            File::from_text(
                "# Usage notes\n\n- Weekly active users increased 18 percent.\n- One SSO issue remains unresolved.\n",
            ),
        )
        .with_extra_path_grant(
            SandboxPathGrant::new(&external_dir)
                .read_only(true)
                .description("read-only external renewal packet notes"),
        )
        .with_extra_path_grant(
            SandboxPathGrant::new(&scratch_dir)
                .description("temporary renewal packet scratch files"),
        );
    let sandbox_agent = SandboxAgent::builder("Renewal Packet Analyst")
        .instructions("Inspect renewal packet files and use granted paths deliberately.")
        .default_manifest(manifest)
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::Shell,
        ])
        .build();
    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Unix local sandbox review".to_owned(),
            ..RunConfig::default()
        },
    )?;

    println!("workspace={}", prepared.session.workspace_root().display());
    println!("workspace_files:\n{}", prepared.session.list_files(".")?);
    println!(
        "external_note={}",
        prepared
            .session
            .read_file(path_str(&external_note)?)?
            .lines()
            .next()
            .unwrap_or_default()
    );
    println!(
        "read_only_write_blocked={}",
        prepared
            .session
            .write_file(path_str(&blocked_note)?, "should fail\n")
            .is_err()
    );
    prepared
        .session
        .write_file(path_str(&scratch_note)?, "sdk scratch output\n")?;
    println!(
        "scratch_note={}",
        fs::read_to_string(&scratch_note)
            .map_err(|error| AgentsError::message(error.to_string()))?
            .trim()
    );
    let shell = prepared
        .session
        .run_shell("cat account_brief.md usage_notes.md | wc -l")?;
    println!("workspace_line_count={}", shell.stdout.trim());

    prepared.session.cleanup()?;
    fs::remove_dir_all(&temp_root).map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(())
}

fn unique_temp_dir() -> Result<PathBuf, AgentsError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AgentsError::message(error.to_string()))?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "openai-agents-unix-local-runner-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(root)
}

fn path_str(path: &PathBuf) -> Result<&str, AgentsError> {
    path.to_str()
        .ok_or_else(|| AgentsError::message(format!("path is not UTF-8: {}", path.display())))
}
