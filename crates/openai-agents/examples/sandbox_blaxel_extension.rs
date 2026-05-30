#[cfg(feature = "blaxel")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{
        BlaxelSandboxClient, BlaxelSandboxClientOptions, DEFAULT_BLAXEL_WORKSPACE_ROOT,
    };

    let client = BlaxelSandboxClient::new(BlaxelSandboxClientOptions {
        token: Some("blaxel_example_token".to_owned()),
        workspace_root: Some(DEFAULT_BLAXEL_WORKSPACE_ROOT.to_owned()),
        exposed_ports: vec![3000],
        interactive_pty: true,
        idle_timeout: Some(180),
        ..BlaxelSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=blaxel");
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

#[cfg(not(feature = "blaxel"))]
fn main() {
    println!("Skipping Blaxel sandbox example because the `blaxel` feature is not enabled.");
    println!(
        "Run with: cargo run -p openai-agents-rs --features blaxel --example sandbox_blaxel_extension"
    );
}
