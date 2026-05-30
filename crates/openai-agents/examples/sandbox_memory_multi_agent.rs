use std::fs;

use openai_agents::{
    AgentsError, Dir, File, LocalSandboxSession, Manifest, RunConfig, SandboxAgent,
    SandboxRunConfig, prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let manifest = shared_manifest();
    let gtm_agent = SandboxAgent::builder("GTM analyst")
        .instructions("Analyze GTM data and keep GTM memory separate.")
        .default_manifest(manifest.clone())
        .build();
    let engineering_agent = SandboxAgent::builder("Engineering fixer")
        .instructions("Fix code and keep engineering memory separate.")
        .default_manifest(manifest)
        .build();

    let gtm = prepare_sandbox_run(
        &gtm_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "GTM memory layout example".to_owned(),
            ..RunConfig::default()
        },
    )?;
    gtm.session.write_file(
        "/workspace/gtm_hypothesis.md",
        "Healthcare accounts with high trial events are the strongest Q2 outreach segment.\n",
    )?;
    gtm.session.write_memory_note(
        "gtm:last_analysis",
        "Northstar Health is the strongest healthcare expansion lead.",
    )?;

    let engineering = prepare_sandbox_run(
        &engineering_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                session: Some(gtm.session.clone()),
                ..SandboxRunConfig::default()
            }),
            workflow_name: "Engineering memory layout example".to_owned(),
            ..RunConfig::default()
        },
    )?;
    engineering.session.write_file(
        "/workspace/src/acme_metrics/report.py",
        "from __future__ import annotations\n\n\
         def format_invoice_total(subtotal: float, tax_rate: float) -> str:\n\
         \n\
             total = subtotal * (1.0 + tax_rate)\n\
             return f\"${total:.2f}\"\n",
    )?;
    engineering.session.write_memory_note(
        "engineering:last_fix",
        "Fixed invoice totals by applying tax_rate as a rate, not a flat amount.",
    )?;

    let serialized = engineering.session.serialize_session_state()?;
    let original_workspace = engineering.session.workspace_root();
    let restored_state = LocalSandboxSession::deserialize_session_state(serialized)?;
    fs::remove_dir_all(&original_workspace).map_err(|error| {
        AgentsError::message(format!(
            "failed to remove original workspace {}: {error}",
            original_workspace.display()
        ))
    })?;

    let resumed = prepare_sandbox_run(
        &gtm_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig {
                session_state: Some(restored_state),
                ..SandboxRunConfig::default()
            }),
            workflow_name: "Multi-agent memory resumed".to_owned(),
            ..RunConfig::default()
        },
    )?;
    println!(
        "gtm_note={}",
        resumed
            .session
            .read_memory_note("gtm:last_analysis")?
            .unwrap_or_default()
    );
    println!(
        "engineering_note={}",
        resumed
            .session
            .read_memory_note("engineering:last_fix")?
            .unwrap_or_default()
    );
    println!(
        "gtm_artifact={}",
        resumed
            .session
            .read_file("/workspace/gtm_hypothesis.md")?
            .trim()
    );
    println!(
        "engineering_file_contains_rate_fix={}",
        resumed
            .session
            .read_file("/workspace/src/acme_metrics/report.py")?
            .contains("subtotal * (1.0 + tax_rate)")
    );

    resumed.session.cleanup()?;
    Ok(())
}

fn shared_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "data",
            Dir::new().with_entry(
                "leads.csv",
                File::from_text(
                    "account,segment,seats,trial_events,monthly_spend\n\
                     Northstar Health,healthcare,240,98,18000\n\
                     Beacon Retail,retail,75,18,4200\n\
                     Apex Fintech,financial-services,180,76,13500\n",
                ),
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
}
