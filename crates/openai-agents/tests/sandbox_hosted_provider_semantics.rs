use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use openai_agents::extensions::{
    BlaxelSandboxClient, BlaxelSandboxClientOptions, CloudflareSandboxClient,
    CloudflareSandboxClientOptions, DEFAULT_DAYTONA_WORKSPACE_ROOT, DEFAULT_VERCEL_WORKSPACE_ROOT,
    DaytonaSandboxClient, DaytonaSandboxClientOptions, E2BSandboxClient, E2BSandboxClientOptions,
    RunloopSandboxClient, RunloopSandboxClientOptions, VercelSandboxClient,
    VercelSandboxClientOptions,
};
use serde_json::Value;

struct ProviderCase {
    feature: &'static str,
    facade_prefix: &'static str,
    extensions_prefix: &'static str,
}

#[test]
fn hosted_provider_feature_matrix_builds_and_exports_symbols() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve");

    let cases = [
        ProviderCase {
            feature: "e2b",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
        ProviderCase {
            feature: "modal",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
        ProviderCase {
            feature: "daytona",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
        ProviderCase {
            feature: "blaxel",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
        ProviderCase {
            feature: "cloudflare",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
        ProviderCase {
            feature: "runloop",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
        ProviderCase {
            feature: "vercel",
            facade_prefix: "openai_agents::extensions",
            extensions_prefix: "agents_extensions",
        },
    ];

    for case in cases {
        let provider = provider_ident(case.feature);
        let crate_dir = create_temp_crate(&workspace_root, case.feature, &provider);
        let manifest_path = crate_dir.join("Cargo.toml");
        let target_dir = workspace_root
            .join("target")
            .join("hosted-provider-feature-matrix")
            .join(case.feature);
        let main_rs = crate_dir.join("src/main.rs");
        let program = sandbox_export_program(
            case.feature,
            &provider,
            case.facade_prefix,
            case.extensions_prefix,
        );

        fs::write(&main_rs, program).expect("main.rs should write");

        let output = Command::new(cargo_bin())
            .arg("check")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .env("CARGO_TARGET_DIR", &target_dir)
            .output()
            .expect("cargo check should run");

        assert!(
            output.status.success(),
            "feature `{}` failed to compile.\nstdout:\n{}\nstderr:\n{}",
            case.feature,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let _ = fs::remove_dir_all(&crate_dir);
    }
}

#[test]
fn hosted_provider_create_resume_and_auth_precedence() {
    let _env_guard = env_lock();

    unsafe {
        std::env::set_var("E2B_API_KEY", "env-e2b-key");
        std::env::remove_var("CLOUDFLARE_SANDBOX_API_KEY");
    }

    let e2b_client = E2BSandboxClient::new(E2BSandboxClientOptions {
        api_key: Some("explicit-e2b-key".to_owned()),
        ..Default::default()
    });
    let e2b_created = e2b_client.create().expect("e2b create should succeed");
    assert_eq!(e2b_created.resolved_auth_source(), "explicit");
    assert_eq!(e2b_created.resolved_auth_value(), "explicit-e2b-key");
    assert_eq!(e2b_created.state().workspace_root, "/workspace");

    let daytona_client = DaytonaSandboxClient::new(DaytonaSandboxClientOptions {
        api_key: Some("daytona-key".to_owned()),
        ..Default::default()
    });
    let daytona_created = daytona_client
        .create()
        .expect("daytona create should succeed");
    assert_eq!(
        daytona_created.state().workspace_root,
        DEFAULT_DAYTONA_WORKSPACE_ROOT
    );
    let daytona_resumed = daytona_client
        .resume(daytona_created.state().clone())
        .expect("daytona resume should succeed");
    assert_eq!(
        daytona_resumed.state().session_id,
        daytona_created.state().session_id
    );
    assert!(daytona_resumed.state().start_state_preserved);

    let vercel_client = VercelSandboxClient::new(VercelSandboxClientOptions {
        token: Some("vercel-token".to_owned()),
        workspace_root: Some("/tmp/custom-root".to_owned()),
        ..Default::default()
    });
    let vercel_created = vercel_client
        .create()
        .expect("vercel create should succeed");
    assert_eq!(vercel_created.state().workspace_root, "/tmp/custom-root");
    let vercel_resumed = vercel_client
        .resume(vercel_created.state().clone())
        .expect("vercel resume should succeed");
    assert_eq!(vercel_resumed.state().workspace_root, "/tmp/custom-root");

    let cloudflare_missing_auth = CloudflareSandboxClient::new(CloudflareSandboxClientOptions {
        workspace_root: Some("/workspace".to_owned()),
        ..Default::default()
    });
    let auth_error = cloudflare_missing_auth
        .create()
        .expect_err("missing cloudflare auth should fail");
    assert!(
        auth_error
            .to_string()
            .contains("CLOUDFLARE_SANDBOX_API_KEY"),
        "unexpected error: {auth_error}"
    );

    let cloudflare_bad_root = CloudflareSandboxClient::new(CloudflareSandboxClientOptions {
        api_key: Some("cloudflare-key".to_owned()),
        workspace_root: Some("/tmp/not-supported".to_owned()),
        ..Default::default()
    });
    let root_error = cloudflare_bad_root
        .create()
        .expect_err("cloudflare should reject a non-/workspace root");
    assert!(
        root_error.to_string().contains("/workspace"),
        "unexpected error: {root_error}"
    );

    unsafe {
        std::env::remove_var("E2B_API_KEY");
        std::env::remove_var("CLOUDFLARE_SANDBOX_API_KEY");
    }
}

#[test]
fn hosted_provider_state_is_secret_safe() {
    let _env_guard = env_lock();

    let client = BlaxelSandboxClient::new(BlaxelSandboxClientOptions {
        token: Some("bl-secret-token".to_owned()),
        client_timeout_s: Some(45),
        workspace_root: Some("/workspace/project".to_owned()),
        base_url: Some("https://sandbox.example.test".to_owned()),
        exposed_ports: vec![3000],
        interactive_pty: true,
        ..Default::default()
    });
    let session = client.create().expect("blaxel create should succeed");
    let payload = client
        .serialize_session_state(session.state())
        .expect("state should serialize");

    let object = payload
        .as_object()
        .expect("serialized state should be an object");
    assert!(!object.contains_key("token"));
    assert!(!object.contains_key("api_key"));
    assert!(!object.contains_key("client_timeout_s"));
    assert_eq!(
        object.get("workspace_root").and_then(Value::as_str),
        Some("/workspace/project")
    );
    assert_eq!(
        object
            .get("exposed_ports")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        object.get("interactive_pty").and_then(Value::as_bool),
        Some(true)
    );

    let restored = client
        .deserialize_session_state(payload)
        .expect("state should deserialize");
    assert_eq!(restored.workspace_root, "/workspace/project");
    assert_eq!(restored.exposed_ports, vec![3000]);
    assert!(restored.interactive_pty);
}

#[test]
fn hosted_provider_capabilities_preserve_ports_and_pty_flags() {
    let _env_guard = env_lock();

    let e2b_client = E2BSandboxClient::new(E2BSandboxClientOptions {
        api_key: Some("e2b-key".to_owned()),
        exposed_ports: vec![3000, 4000],
        interactive_pty: true,
        ..Default::default()
    });
    let e2b_created = e2b_client.create().expect("e2b create should succeed");
    assert_eq!(e2b_created.state().exposed_ports, vec![3000, 4000]);
    assert!(e2b_created.state().interactive_pty);
    assert!(e2b_created.supports_pty());
    let e2b_resumed = e2b_client
        .resume(e2b_created.state().clone())
        .expect("e2b resume should succeed");
    assert_eq!(e2b_resumed.state().exposed_ports, vec![3000, 4000]);
    assert!(e2b_resumed.state().interactive_pty);
    assert!(e2b_resumed.state().start_state_preserved);

    let runloop_client = RunloopSandboxClient::new(RunloopSandboxClientOptions {
        api_key: Some("runloop-key".to_owned()),
        interactive_pty: true,
        ..Default::default()
    });
    let runloop_error = runloop_client
        .create()
        .expect_err("runloop should reject PTY requests");
    assert!(
        runloop_error.to_string().contains("interactive PTY"),
        "unexpected error: {runloop_error}"
    );

    let vercel_client = VercelSandboxClient::new(VercelSandboxClientOptions {
        token: Some("vercel-token".to_owned()),
        exposed_ports: vec![8080],
        ..Default::default()
    });
    let vercel_created = vercel_client
        .create()
        .expect("vercel create should preserve exposed ports");
    assert_eq!(
        vercel_created.state().workspace_root,
        DEFAULT_VERCEL_WORKSPACE_ROOT
    );
    assert_eq!(vercel_created.state().exposed_ports, vec![8080]);
    assert!(!vercel_created.supports_pty());
}

fn create_temp_crate(workspace_root: &Path, feature: &str, provider: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should move forward")
        .as_nanos();
    let crate_dir = workspace_root
        .join("target")
        .join("tmp")
        .join(format!("hosted-provider-{feature}-{unique}"));

    fs::create_dir_all(crate_dir.join("src")).expect("temp crate src dir should exist");

    let cargo_toml = format!(
        r#"[package]
name = "hosted-provider-{feature}"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
openai_agents = {{ package = "openai-agents-rs", path = "{openai_agents}", default-features = false, features = ["{feature}"] }}
agents_extensions = {{ package = "openai-agents-extensions-rs", path = "{agents_extensions}", default-features = false, features = ["{feature}"] }}
"#,
        feature = feature,
        openai_agents = display_path(&workspace_root.join("crates/openai-agents")),
        agents_extensions = display_path(&workspace_root.join("crates/agents-extensions")),
    );

    fs::write(crate_dir.join("Cargo.toml"), cargo_toml).expect("Cargo.toml should write");
    fs::write(crate_dir.join("src/main.rs"), format!("// {provider}\n"))
        .expect("placeholder main.rs should write");

    crate_dir
}

fn sandbox_export_program(
    feature: &str,
    provider: &str,
    facade_prefix: &str,
    extensions_prefix: &str,
) -> String {
    let client = format!("{provider}SandboxClient");
    let options = format!("{provider}SandboxClientOptions");
    let session = format!("{provider}SandboxSession");
    let state = format!("{provider}SandboxSessionState");

    format!(
        r#"use {facade_prefix}::sandbox::{{{client}, {options}, {session}, {state}}};
use {facade_prefix}::{{{client} as FacadeRootClient, {options} as FacadeRootOptions, {session} as FacadeRootSession, {state} as FacadeRootState}};
use {extensions_prefix}::sandbox::{{{client} as ExtensionsClient, {options} as ExtensionsOptions, {session} as ExtensionsSession, {state} as ExtensionsState}};
use {extensions_prefix}::{{{client} as ExtensionsRootClient, {options} as ExtensionsRootOptions, {session} as ExtensionsRootSession, {state} as ExtensionsRootState}};

fn main() {{
    let options = {options}::default();
    let state = {state}::default();
    let client = {client}::new(options.clone());
    let session = {session}::new(state.clone());

    let _facade_root_client: FacadeRootClient = client.clone();
    let _facade_root_options: FacadeRootOptions = options.clone();
    let _facade_root_session: FacadeRootSession = session.clone();
    let _facade_root_state: FacadeRootState = state.clone();

    let _extensions_client: ExtensionsClient = client.clone();
    let _extensions_options: ExtensionsOptions = options.clone();
    let _extensions_session: ExtensionsSession = session.clone();
    let _extensions_state: ExtensionsState = state.clone();

    let _extensions_root_client: ExtensionsRootClient = client;
    let _extensions_root_options: ExtensionsRootOptions = options;
    let _extensions_root_session: ExtensionsRootSession = session;
    let _extensions_root_state: ExtensionsRootState = state;

    let _ = "{feature}";
}}
"#,
        facade_prefix = facade_prefix,
        extensions_prefix = extensions_prefix,
        client = client,
        options = options,
        session = session,
        state = state,
        feature = feature,
    )
}

fn provider_ident(feature: &str) -> String {
    match feature {
        "e2b" => "E2B".to_owned(),
        other => {
            let mut chars = other.chars();
            let first = chars.next().expect("feature should not be empty");
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        }
    }
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned")
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}
