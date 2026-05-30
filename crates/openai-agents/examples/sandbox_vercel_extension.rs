#[cfg(feature = "vercel")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{VercelSandboxClient, VercelSandboxClientOptions};

    let client = VercelSandboxClient::new(VercelSandboxClientOptions {
        token: Some("vercel_example_token".to_owned()),
        workspace_root: Some("/vercel/sandbox".to_owned()),
        exposed_ports: vec![3000],
        idle_timeout: Some(120),
        ..VercelSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=vercel");
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

#[cfg(not(feature = "vercel"))]
fn main() {
    println!("Skipping Vercel sandbox example because the `vercel` feature is not enabled.");
    println!(
        "Run with: cargo run -p openai-agents-rs --features vercel --example sandbox_vercel_extension"
    );
}
