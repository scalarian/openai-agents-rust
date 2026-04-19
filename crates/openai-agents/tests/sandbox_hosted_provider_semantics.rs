use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}
