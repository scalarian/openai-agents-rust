#[cfg(feature = "modal")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use openai_agents::extensions::sandbox::{ModalSandboxClient, ModalSandboxClientOptions};

    let client = ModalSandboxClient::new(ModalSandboxClientOptions {
        token: Some("modal_example_token".to_owned()),
        workspace_root: Some("/workspace".to_owned()),
        exposed_ports: vec![3000],
        idle_timeout: Some(180),
        ..ModalSandboxClientOptions::default()
    });
    let session = client.create()?;
    let encoded = client.serialize_session_state(session.state())?;
    let decoded = client.deserialize_session_state(encoded.clone())?;
    let resumed = client.resume(decoded)?;

    println!("provider=modal");
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

#[cfg(not(feature = "modal"))]
fn main() {
    println!("Skipping Modal sandbox example because the `modal` feature is not enabled.");
    println!(
        "Run with: cargo run -p openai-agents-rs --features modal --example sandbox_modal_extension"
    );
}
