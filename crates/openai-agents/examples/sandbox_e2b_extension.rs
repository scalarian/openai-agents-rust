#[cfg(feature = "e2b")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{E2BSandboxClient, E2BSandboxClientOptions};

    let client = E2BSandboxClient::new(E2BSandboxClientOptions {
        api_key: Some("e2b_example_key".to_owned()),
        workspace_root: Some("/home/user".to_owned()),
        exposed_ports: vec![8080],
        interactive_pty: true,
        idle_timeout: Some(120),
        ..E2BSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=e2b");
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

#[cfg(not(feature = "e2b"))]
fn main() {
    println!("Skipping E2B sandbox example because the `e2b` feature is not enabled.");
    println!(
        "Run with: cargo run -p openai-agents-rs --features e2b --example sandbox_e2b_extension"
    );
}
