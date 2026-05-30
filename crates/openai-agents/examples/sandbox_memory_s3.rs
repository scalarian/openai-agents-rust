use std::fs;

use openai_agents::{
    AgentsError, Dir, File, LocalSandboxSession, Manifest, RunConfig, SandboxAgent,
    SandboxCapability, SandboxRunConfig, prepare_sandbox_run,
};

fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Sandbox Memory S3 Demo")
        .instructions(
            "Use the persistent memory layout to remember code fixes across fresh sandbox sessions.",
        )
        .default_manifest(memory_s3_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();

    let first = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Sandbox S3 memory first run".to_owned(),
            ..RunConfig::default()
        },
    )?;
    first
        .session
        .apply_patch(openai_agents::ApplyPatchOperation {
            path: "/workspace/src/acme_metrics/report.py".to_owned(),
            replacement: fixed_report_py().to_owned(),
        })?;
    first.session.write_file(
        "/workspace/persistent/memories/memory_summary.md",
        "Fixed src/acme_metrics/report.py by applying tax_rate as a multiplier: subtotal * (1.0 + tax_rate).\n",
    )?;
    first.session.write_file(
        "/workspace/persistent/memories/raw_memories/fix-001.md",
        "Root cause: previous implementation added the tax_rate as a flat amount. Patch: use subtotal * (1.0 + tax_rate).\n",
    )?;
    first.session.write_memory_note(
        "invoice_total_fix",
        "Use subtotal * (1.0 + tax_rate) and add a regression test for $107.50 on 100 at 7.5%.",
    )?;

    let serialized = first.session.serialize_session_state()?;
    let original_workspace = first.session.workspace_root();
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
            workflow_name: "Sandbox S3 memory resumed run".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let remembered = resumed
        .session
        .read_memory_note("invoice_total_fix")?
        .unwrap_or_default();
    resumed.session.write_file(
        "/workspace/tests/test_invoice_regression.py",
        regression_test_py(),
    )?;
    let verification = resumed.session.run_shell(
        "grep -R \"format_invoice_total\\|subtotal \\* (1.0 + tax_rate)\" persistent/memories tests src/acme_metrics/report.py",
    )?;

    println!(
        "memory_summary={}",
        resumed
            .session
            .read_file("/workspace/persistent/memories/memory_summary.md")?
            .trim()
    );
    println!("remembered_note={remembered}");
    println!("verification_exit={}", verification.exit_code);
    println!("verification_stdout:\n{}", verification.stdout.trim());

    resumed.session.cleanup()?;
    Ok(())
}

fn memory_s3_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "README.md",
            File::from_text(
                "# Acme Metrics\n\nSmall demo package for validating invoice total formatting.\n",
            ),
        )
        .with_entry(
            "src",
            Dir::new().with_entry(
                "acme_metrics",
                Dir::new()
                    .with_entry(
                        "__init__.py",
                        File::from_text("from .report import format_invoice_total\n"),
                    )
                    .with_entry(
                        "report.py",
                        File::from_text(
                            "from __future__ import annotations\n\n\
                             def format_invoice_total(subtotal: float, tax_rate: float) -> str:\n\
                             \n\
                                 total = subtotal + tax_rate\n\
                                 return f\"${total:.2f}\"\n",
                        ),
                    ),
            ),
        )
        .with_entry("tests", Dir::new())
        .with_entry(
            "persistent",
            Dir::new()
                .with_entry(
                    "memories",
                    Dir::new()
                        .with_entry("raw_memories", Dir::new())
                        .with_entry("rollout_summaries", Dir::new()),
                )
                .with_entry("sessions", Dir::new()),
        )
}

fn fixed_report_py() -> &'static str {
    "from __future__ import annotations\n\n\
     def format_invoice_total(subtotal: float, tax_rate: float) -> str:\n\
     \n\
         total = subtotal * (1.0 + tax_rate)\n\
         return f\"${total:.2f}\"\n"
}

fn regression_test_py() -> &'static str {
    "from acme_metrics import format_invoice_total\n\n\n\
     def test_format_invoice_total_applies_tax_rate() -> None:\n\
     \n\
         assert format_invoice_total(100.0, 0.075) == \"$107.50\"\n"
}
