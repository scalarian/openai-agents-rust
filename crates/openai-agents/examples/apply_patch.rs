use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunInterruptionKind, Runner, ToolContext, Usage, apply_diff,
    function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct PatchArgs {
    path: String,
    replacement: String,
}

#[derive(Clone, Default)]
struct ApplyPatchModel;

#[async_trait]
impl Model for ApplyPatchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "create-tasks").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "create-tasks".to_owned(),
                tool_name: "apply_patch".to_owned(),
                arguments: json!({
                    "path": "tasks.md",
                    "replacement": initial_tasks()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "update-tasks").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "update-tasks".to_owned(),
                tool_name: "apply_patch".to_owned(),
                arguments: json!({
                    "path": "tasks.md",
                    "replacement": updated_tasks()
                }),
                namespace: None,
            }]
        } else {
            vec![OutputItem::Text {
                text: "Created tasks.md and then checked off the last two entries.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 26,
                output_tokens: 14,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct ApplyPatchProvider {
    model: Arc<ApplyPatchModel>,
}

impl ModelProvider for ApplyPatchProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let workspace = temp_workspace()?;
    let workspace_for_tool = Arc::new(workspace.clone());
    let apply_patch = function_tool(
        "apply_patch",
        "Replace a file inside the temporary workspace after approval.",
        move |_ctx: ToolContext, args: PatchArgs| {
            let workspace = workspace_for_tool.clone();
            async move { apply_patch_replacement(&workspace, &args.path, &args.replacement) }
        },
    )?
    .with_needs_approval(true);

    let agent = Agent::builder("Patch Assistant")
        .instructions("Use apply_patch to create and update files in the temporary workspace.")
        .function_tool(apply_patch)
        .build();
    let runner = Runner::new().with_model_provider(Arc::new(ApplyPatchProvider::default()));
    let mut result = runner
        .run(
            &agent,
            "Create tasks.md, then check off the last two items.",
        )
        .await?;

    while !result.interruptions.is_empty() {
        let mut state = result
            .durable_state()
            .cloned()
            .ok_or_else(|| AgentsError::message("apply_patch run did not include state"))?;
        for interruption in &result.interruptions {
            println!(
                "approval_request tool={} call_id={}",
                interruption.tool_name.as_deref().unwrap_or_default(),
                interruption.call_id.as_deref().unwrap_or_default()
            );
            if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
                state.approve_for_tool(
                    interruption.call_id.clone().unwrap_or_default(),
                    interruption.tool_name.clone(),
                    Some("approved patch operation".to_owned()),
                );
            }
        }
        result = runner.resume_with_agent(&state, &agent).await?;
    }

    println!("final_output={}", result.final_output.unwrap_or_default());
    println!(
        "tasks_md:\n{}",
        fs::read_to_string(workspace.join("tasks.md"))
            .map_err(|error| AgentsError::message(error.to_string()))?
            .trim()
    );
    fs::remove_dir_all(&workspace).map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(())
}

fn apply_patch_replacement(
    workspace: &Path,
    relative_path: &str,
    replacement: &str,
) -> Result<String, AgentsError> {
    let target = resolve_under(workspace, relative_path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| AgentsError::message(error.to_string()))?;
    }
    let original = fs::read_to_string(&target).unwrap_or_default();
    let patched = apply_diff(&original, replacement);
    fs::write(&target, patched).map_err(|error| AgentsError::message(error.to_string()))?;
    Ok(format!("patched {}", target.display()))
}

fn resolve_under(root: &Path, relative_path: &str) -> Result<PathBuf, AgentsError> {
    let target = root.join(relative_path);
    let normalized_parent = target
        .parent()
        .unwrap_or(root)
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    if !normalized_parent.starts_with(root) {
        return Err(AgentsError::message("patch path escapes workspace"));
    }
    Ok(target)
}

fn temp_workspace() -> Result<PathBuf, AgentsError> {
    let path = std::env::temp_dir().join(format!("agents-apply-patch-{}", std::process::id()));
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|error| AgentsError::message(error.to_string()))?;
    }
    fs::create_dir_all(&path).map_err(|error| AgentsError::message(error.to_string()))?;
    path.canonicalize()
        .map_err(|error| AgentsError::message(error.to_string()))
}

fn initial_tasks() -> &'static str {
    "# Shopping Checklist\n\n\
     - [ ] apples\n\
     - [ ] rice\n\
     - [ ] coffee\n\
     - [ ] olive oil\n\
     - [ ] soap\n"
}

fn updated_tasks() -> &'static str {
    "# Shopping Checklist\n\n\
     - [ ] apples\n\
     - [ ] rice\n\
     - [ ] coffee\n\
     - [x] olive oil\n\
     - [x] soap\n"
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
