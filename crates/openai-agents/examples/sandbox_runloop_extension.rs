#[cfg(feature = "runloop")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{
        DEFAULT_RUNLOOP_WORKSPACE_ROOT, RunloopSandboxClient, RunloopSandboxClientOptions,
    };

    let client = RunloopSandboxClient::new(RunloopSandboxClientOptions {
        api_key: Some("runloop_example_key".to_owned()),
        workspace_root: Some(DEFAULT_RUNLOOP_WORKSPACE_ROOT.to_owned()),
        exposed_ports: vec![3000],
        idle_timeout: Some(300),
        ..RunloopSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=runloop");
    println!("auth_source={}", session.resolved_auth_source());
    println!("workspace_root={}", session.state().workspace_root);
    println!("supports_pty={}", session.supports_pty());
    println!("serialized={}", serde_json::to_string_pretty(&encoded)?);
    println!(
        "resumed_preserved={}",
        resumed.state().start_state_preserved
    );
    Ok(())
}

#[cfg(not(feature = "runloop"))]
fn main() {
    println!("Skipping Runloop sandbox example because the `runloop` feature is not enabled.");
    println!(
        "Run with: cargo run -p openai-agents-rs --features runloop --example sandbox_runloop_extension"
    );
}
