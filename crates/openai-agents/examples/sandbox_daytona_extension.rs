#[cfg(feature = "daytona")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{
        DEFAULT_DAYTONA_WORKSPACE_ROOT, DaytonaSandboxClient, DaytonaSandboxClientOptions,
    };

    let client = DaytonaSandboxClient::new(DaytonaSandboxClientOptions {
        api_key: Some("daytona_example_key".to_owned()),
        workspace_root: Some(DEFAULT_DAYTONA_WORKSPACE_ROOT.to_owned()),
        exposed_ports: vec![8080],
        interactive_pty: true,
        idle_timeout: Some(180),
        ..DaytonaSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=daytona");
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

#[cfg(not(feature = "daytona"))]
fn main() {
    println!("Skipping Daytona sandbox example because the `daytona` feature is not enabled.");
    println!(
        "Run with: cargo run -p openai-agents-rs --features daytona --example sandbox_daytona_extension"
    );
}
