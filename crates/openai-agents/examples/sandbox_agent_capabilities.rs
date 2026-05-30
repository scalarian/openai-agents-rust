use openai_agents::{
    AgentsError, File, Manifest, RunConfig, SandboxAgent, SandboxCapability, SandboxRunConfig,
    prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let manifest = Manifest::default().with_entry(
        "README.md",
        File::from_text("# Capability Workspace\n\nProject code name: atlas.\n"),
    );

    let filesystem_agent = SandboxAgent::builder("Filesystem capability")
        .default_manifest(manifest.clone())
        .capabilities(vec![SandboxCapability::Filesystem])
        .build();
    let filesystem = prepare(&filesystem_agent)?;
    println!(
        "filesystem_tools={}",
        function_tool_names(&filesystem.agent).join(",")
    );
    println!(
        "readme={}",
        filesystem
            .session
            .read_file("/workspace/README.md")?
            .lines()
            .next()
            .unwrap_or_default()
    );

    let patch_agent = SandboxAgent::builder("Patch capability")
        .default_manifest(manifest.clone())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
        ])
        .build();
    let patch = prepare(&patch_agent)?;
    println!(
        "patch_tools={}",
        function_tool_names(&patch.agent).join(",")
    );
    patch
        .session
        .apply_patch(openai_agents::ApplyPatchOperation {
            path: "/workspace/verification.txt".to_owned(),
            replacement: "patched=true\n".to_owned(),
        })?;
    println!(
        "patched_file={}",
        patch
            .session
            .read_file("/workspace/verification.txt")?
            .trim()
    );

    let shell_agent = SandboxAgent::builder("Shell capability")
        .default_manifest(manifest)
        .capabilities(vec![SandboxCapability::Shell])
        .build();
    let shell = prepare(&shell_agent)?;
    println!(
        "shell_tools={}",
        function_tool_names(&shell.agent).join(",")
    );
    let output = shell.session.run_shell("printf shell_ready")?;
    println!("shell_output={}", output.stdout.trim());

    filesystem.session.cleanup()?;
    patch.session.cleanup()?;
    shell.session.cleanup()?;
    Ok(())
}

fn prepare(agent: &SandboxAgent) -> Result<openai_agents::PreparedSandboxRun, AgentsError> {
    prepare_sandbox_run(
        agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            ..RunConfig::default()
        },
    )
}

fn function_tool_names(agent: &openai_agents::Agent) -> Vec<String> {
    let mut names = agent
        .function_tools
        .iter()
        .map(|tool| tool.definition.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}
