use openai_agents::{
    AgentsError, File, Manifest, RunConfig, SandboxAgent, SandboxRunConfig, prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("sandbox engineer")
        .instructions("Use the sandbox workspace for file inspection and edits.")
        .default_manifest(Manifest::default().with_entry(
            "notes.txt",
            File::from_text("release-checks\n- run tests\n- update docs\n"),
        ))
        .build();

    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;

    println!("workspace={}", prepared.session.workspace_root().display());
    println!("files:\n{}", prepared.session.list_files("/workspace")?);

    prepared
        .session
        .write_file("/workspace/generated.txt", "created by the sandbox\n")?;
    let shell = prepared
        .session
        .run_shell("cat notes.txt generated.txt | wc -l")?;
    println!("shell exit={}", shell.exit_code);
    println!("line count={}", shell.stdout.trim());

    prepared.session.cleanup()?;
    Ok(())
}
