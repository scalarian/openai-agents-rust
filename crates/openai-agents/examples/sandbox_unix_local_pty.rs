use std::time::Duration;

use openai_agents::{
    AgentsError, File, Manifest, RunConfig, SandboxAgent, SandboxCapability, SandboxRunConfig,
    prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Unix-local PTY Demo")
        .instructions("Use one interactive process when state matters.")
        .default_manifest(Manifest::default().with_entry(
            "README.md",
            File::from_text(
                "# Unix-local PTY Agent Example\n\nThis workspace demonstrates stateful PTY interaction.\n",
            ),
        ))
        .capabilities(vec![SandboxCapability::Shell])
        .build();

    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Unix-local PTY example".to_owned(),
            ..RunConfig::default()
        },
    )?;

    println!("workspace={}", prepared.session.workspace_root().display());
    let pty = prepared.session.open_pty("python3 -u -i -q")?;
    pty.write_stdin("value = 5 + 5\nprint('first=' + str(value))\n")?;
    let first = pty.wait_for_output("first=10", Duration::from_secs(5))?;
    println!(
        "{}",
        last_matching_line(&first, "first=").unwrap_or("first=<missing>")
    );

    pty.write_stdin("value += 5\nprint('second=' + str(value))\nexit()\n")?;
    let second = pty.wait_for_output("second=15", Duration::from_secs(5))?;
    println!(
        "{}",
        last_matching_line(&second, "second=").unwrap_or("second=<missing>")
    );

    let status = pty.wait()?;
    println!("pty_exit={status}");
    prepared.session.cleanup()?;
    Ok(())
}

fn last_matching_line<'a>(output: &'a str, needle: &str) -> Option<&'a str> {
    output
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with(needle))
}
