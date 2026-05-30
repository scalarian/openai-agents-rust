#[cfg(feature = "cloudflare")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{
        CloudflareSandboxClient, CloudflareSandboxClientOptions,
    };

    let client = CloudflareSandboxClient::new(CloudflareSandboxClientOptions {
        api_key: Some("cloudflare_example_key".to_owned()),
        workspace_root: Some("/workspace".to_owned()),
        base_url: Some("https://sandbox-worker.example.com".to_owned()),
        exposed_ports: vec![8787],
        interactive_pty: true,
        idle_timeout: Some(90),
        ..CloudflareSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=cloudflare");
    println!("auth_source={}", session.resolved_auth_source());
    println!("workspace_root={}", session.state().workspace_root);
    println!(
        "base_url={}",
        session.state().base_url.clone().unwrap_or_default()
    );
    println!("supports_pty={}", session.supports_pty());
    println!("serialized={}", serde_json::to_string_pretty(&encoded)?);
    println!(
        "resumed_preserved={}",
        resumed.state().start_state_preserved
    );
    Ok(())
}

#[cfg(not(feature = "cloudflare"))]
fn main() {
    println!(
        "Skipping Cloudflare sandbox example because the `cloudflare` feature is not enabled."
    );
    println!(
        "Run with: cargo run -p openai-agents-rs --features cloudflare --example sandbox_cloudflare_extension"
    );
}
