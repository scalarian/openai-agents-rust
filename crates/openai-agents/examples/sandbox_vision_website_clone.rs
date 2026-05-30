use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, Runner, SandboxAgent, SandboxCapability,
    SandboxRunConfig, Usage, prepare_sandbox_run,
};
use serde_json::{Value, json};

const DEFAULT_PROMPT: &str = "Inspect the reference screen notes and build a static HTML/CSS reproduction under output/site/.";

#[derive(Clone, Default)]
struct VisionCloneModel;

#[async_trait]
impl Model for VisionCloneModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "read-reference").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-reference".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/reference/reference-site-notes.md"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-notes").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-notes".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/visual-notes.md",
                    "replacement": visual_notes()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-html").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-html".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/site/index.html",
                    "replacement": index_html()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-css").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-css".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/site/styles.css",
                    "replacement": styles_css()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "verify-site").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "verify-site".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "test -s output/site/index.html && test -s output/site/styles.css && grep -h 'Fleet Command' output/site/index.html"
                }),
                namespace: None,
            }]
        } else {
            let verification = tool_output_text_for_call(&request.input, "verify-site")
                .map(stdout_section)
                .unwrap_or_default()
                .trim()
                .to_owned();
            vec![OutputItem::Text {
                text: format!(
                    "Built `output/site/index.html` and `output/site/styles.css` from the reference notes. Verification:\n{verification}"
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 68,
                output_tokens: 36,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct VisionCloneProvider {
    model: Arc<VisionCloneModel>,
}

impl ModelProvider for VisionCloneProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Vision Website Clone Builder")
        .model("gpt-5.5")
        .instructions(
            "Build a single-screen static website clone from the reference notes. Write output/visual-notes.md, output/site/index.html, and output/site/styles.css.",
        )
        .default_manifest(vision_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();

    let prepared = prepare_sandbox_run(
        &sandbox_agent,
        &RunConfig {
            sandbox: Some(SandboxRunConfig::default()),
            workflow_name: "Vision website clone example".to_owned(),
            ..RunConfig::default()
        },
    )?;
    let result = Runner::new()
        .with_model_provider(Arc::new(VisionCloneProvider::default()))
        .run(&prepared.agent, DEFAULT_PROMPT)
        .await?;

    println!("final_output={}", result.final_output.unwrap_or_default());
    println!(
        "site_files:\n{}",
        prepared
            .session
            .run_shell("find output -maxdepth 3 -type f | sort")?
            .stdout
            .trim()
    );
    prepared.session.cleanup()?;
    Ok(())
}

fn vision_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "AGENTS.md",
            File::from_text(
                "# Vision UI Reproduction Instructions\n\n\
                 Read reference/reference-site-notes.md before writing code. Create a static single-screen clone under output/site/.\n",
            ),
        )
        .with_entry(
            "reference",
            Dir::new().with_entry(
                "reference-site-notes.md",
                File::from_text(
                    "# Reference Screen Notes\n\n\
                     Product: Fleet Command\n\
                     Layout: dark top navigation, left KPI rail, central operations table, right alert panel.\n\
                     Typography: compact operational dashboard type, no hero section.\n\
                     Colors: charcoal background, white panels, green status marks, amber incident badge.\n",
                ),
            ),
        )
        .with_entry("output", Dir::new().with_entry("site", Dir::new()))
}

fn visual_notes() -> &'static str {
    "# Visual Notes\n\n\
     - Single viewport operations dashboard.\n\
     - Dense KPI rail on the left, table in the center, alerts on the right.\n\
     - Use restrained contrast and status color only for operational state.\n"
}

fn index_html() -> &'static str {
    concat!(
        "<!doctype html>\n",
        "<html lang=\"en\">\n",
        "<head>\n",
        "  <meta charset=\"utf-8\">\n",
        "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
        "  <title>Fleet Command</title>\n",
        "  <link rel=\"stylesheet\" href=\"styles.css\">\n",
        "</head>\n",
        "<body>\n",
        "  <header><strong>Fleet Command</strong><nav>Robots Routes Incidents</nav></header>\n",
        "  <main>\n",
        "    <aside class=\"kpis\"><p>Active robots</p><b>42</b><p>On-time routes</p><b>97%</b></aside>\n",
        "    <section class=\"board\"><h1>Warehouse Operations</h1><table><tr><th>Robot</th><th>Zone</th><th>Status</th></tr><tr><td>R2</td><td>Aisle 4</td><td><span class=\"ok\">Charging</span></td></tr><tr><td>K1</td><td>Dock 2</td><td><span class=\"ok\">Idle</span></td></tr></table></section>\n",
        "    <aside class=\"alerts\"><h2>Alerts</h2><p><span class=\"warn\">Amber</span> Dock 3 queue building</p></aside>\n",
        "  </main>\n",
        "</body>\n",
        "</html>\n",
    )
}

fn styles_css() -> &'static str {
    concat!(
        ":root{font-family:Inter,Arial,sans-serif;color:#182026;background:#111820;}\n",
        "body{margin:0;background:#111820;}\n",
        "header{height:56px;display:flex;align-items:center;justify-content:space-between;padding:0 24px;color:#f6f8fa;border-bottom:1px solid #26313b;}\n",
        "nav{font-size:13px;color:#aab5bf;word-spacing:16px;}\n",
        "main{display:grid;grid-template-columns:180px 1fr 240px;gap:16px;padding:16px;}\n",
        ".kpis,.board,.alerts{background:#f7f9fb;color:#182026;border-radius:8px;padding:18px;}\n",
        ".kpis p{margin:0 0 6px;color:#5d6b75;font-size:12px;text-transform:uppercase;}\n",
        ".kpis b{display:block;margin:0 0 20px;font-size:30px;}\n",
        "h1,h2{margin:0 0 16px;font-size:20px;}\n",
        "table{width:100%;border-collapse:collapse;font-size:14px;}\n",
        "th,td{text-align:left;padding:12px;border-bottom:1px solid #d8e0e6;}\n",
        ".ok{color:#0f7b4f;font-weight:700;}\n",
        ".warn{background:#f6c85f;border-radius:4px;padding:2px 6px;font-weight:700;}\n",
        "@media(max-width:760px){main{grid-template-columns:1fr}.kpis{display:grid;grid-template-columns:1fr 1fr;gap:8px}}\n",
    )
}

fn tool_output_text_for_call<'a>(input: &'a [InputItem], call_id: &str) -> Option<&'a str> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
    })
}

fn stdout_section(output: &str) -> &str {
    output
        .split_once("stdout:\n")
        .map(|(_, after_stdout)| after_stdout)
        .and_then(|after_stdout| {
            after_stdout
                .split_once("\nstderr:")
                .map(|(stdout, _)| stdout)
        })
        .unwrap_or(output)
}
