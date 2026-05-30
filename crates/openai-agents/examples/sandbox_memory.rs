use std::fs;

use openai_agents::{
    AgentsError, File, LocalSandboxSession, Manifest, RunConfig, SandboxAgent, SandboxRunConfig,
    prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Sandbox Memory Demo")
        .instructions("Use sandbox session memory to remember prior workspace work.")
        .default_manifest(acme_metrics_manifest())
        .build();

    let initial = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Sandbox memory example: first run".to_owned(),
            ..RunConfig::default()
        },
    )?;
    initial.session.write_file(
        "/workspace/src/acme_metrics/report.py",
        "from __future__ import annotations\n\n\
         def format_invoice_total(subtotal: float, tax_rate: float) -> str:\n\
         \n\
             total = subtotal * (1.0 + tax_rate)\n\
             return f\"${total:.2f}\"\n",
    )?;
    initial.session.write_memory_note(
        "last_fix",
        "Fixed format_invoice_total to multiply subtotal by 1 + tax_rate.",
    )?;
    println!(
        "[first run] note={}",
        initial
            .session
            .read_memory_note("last_fix")?
            .unwrap_or_default()
    );

    let serialized = initial.session.serialize_session_state()?;
    let original_workspace = initial.session.workspace_root();
    let restored_state = LocalSandboxSession::deserialize_session_state(serialized)?;
    fs::remove_dir_all(&original_workspace).map_err(|error| {
        AgentsError::message(format!(
            "failed to remove original workspace {}: {error}",
            original_workspace.display()
        ))
    })?;

    let resumed = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                session_state: Some(restored_state),
                ..SandboxRunConfig::default()
            }),
            workflow_name: "Sandbox memory example: resumed run".to_owned(),
            ..RunConfig::default()
        },
    )?;
    println!(
        "[resumed run] workspace_restored={}",
        resumed.session.workspace_root().exists()
    );
    println!(
        "[resumed run] note={}",
        resumed
            .session
            .read_memory_note("last_fix")?
            .unwrap_or_default()
    );
    resumed.session.write_file(
        "/workspace/tests/test_report_regression.py",
        "from acme_metrics import format_invoice_total\n\n\n\
         def test_negative_regression() -> None:\n\
         \n\
             assert format_invoice_total(100.0, 0.075) == \"$107.50\"\n",
    )?;
    resumed.session.write_memory_note(
        "last_fix",
        "Added regression coverage for the invoice total tax-rate bug.",
    )?;

    let resumed_state =
        LocalSandboxSession::deserialize_session_state(resumed.session.serialize_session_state()?)?;
    let resumed_again = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                session_state: Some(resumed_state),
                ..SandboxRunConfig::default()
            }),
            workflow_name: "Sandbox memory example: second resume".to_owned(),
            ..RunConfig::default()
        },
    )?;
    println!(
        "[second resume] note={}",
        resumed_again
            .session
            .read_memory_note("last_fix")?
            .unwrap_or_default()
    );
    println!(
        "[second resume] files:\n{}",
        resumed_again.session.list_files("/workspace/tests")?
    );

    resumed_again.session.cleanup()?;
    Ok(())
}

fn acme_metrics_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "README.md",
            File::from_text(
                "# Acme Metrics\n\nSmall demo package for validating invoice total formatting.\n",
            ),
        )
        .with_entry(
            "src/acme_metrics/__init__.py",
            File::from_text("from .report import format_invoice_total\n"),
        )
        .with_entry(
            "src/acme_metrics/report.py",
            File::from_text(
                "from __future__ import annotations\n\n\
                 def format_invoice_total(subtotal: float, tax_rate: float) -> str:\n\
                 \n\
                     total = subtotal + tax_rate\n\
                     return f\"${total:.2f}\"\n",
            ),
        )
        .with_entry(
            "tests/test_report.py",
            File::from_text(
                "from acme_metrics import format_invoice_total\n\n\n\
                 def test_format_invoice_total_applies_tax_rate() -> None:\n\
                 \n\
                     assert format_invoice_total(100.0, 0.075) == \"$107.50\"\n",
            ),
        )
}
