use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, ShellTool, Usage,
};
use serde_json::{Value, json};

const SHELL_SKILL_ID_ENV: &str = "OPENAI_SHELL_SKILL_ID";
const SHELL_SKILL_VERSION_ENV: &str = "OPENAI_SHELL_SKILL_VERSION";
const DEFAULT_SKILL_ID: &str = "skill_698bbe879adc81918725cbc69dcae7960bc5613dadaed377";

#[derive(Clone, Default)]
struct SkillReferenceModel;

#[async_trait]
impl Model for SkillReferenceModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let text = if let Some(skill_id) = skill_reference_id(&request) {
            format!(
                "Agent: shell skill reference configured with skill_id={skill_id}; created /mnt/data/orders.csv; container_id=cntr_skill_ref_demo."
            )
        } else if has_container_reference(&request) {
            "Agent (container reuse): reused cntr_skill_ref_demo and listed /mnt/data/orders.csv."
                .to_owned()
        } else {
            "Shell skill reference was not configured.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 18,
                output_tokens: 16,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct SkillReferenceProvider {
    model: Arc<SkillReferenceModel>,
}

impl ModelProvider for SkillReferenceProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let skill_reference = resolve_skill_reference();
    println!(
        "[info] Using skill reference: {} (version {})",
        skill_reference
            .get("skill_id")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_SKILL_ID),
        skill_reference
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("default")
    );

    let runner = Runner::new().with_model_provider(Arc::new(SkillReferenceProvider::default()));
    let agent1 = Agent::builder("Container Shell Agent (Skill Reference)")
        .instructions("Use the available referenced container skill to answer user requests.")
        .tool(
            ShellTool::new("shell", "Run commands in an OpenAI managed shell container")
                .with_hosted_tool_option(
                    "environment",
                    skill_reference_environment(skill_reference),
                ),
        )
        .build();
    let result1 = runner
        .run(
            &agent1,
            "Use the referenced csv-workbench skill to create /mnt/data/orders.csv and summarize it.",
        )
        .await?;
    let first_output = result1.final_output.unwrap_or_default();
    println!("{first_output}");

    let container_id = extract_container_id(&first_output).unwrap_or("cntr_skill_ref_demo");
    println!("[info] Reusing container_id={container_id}");

    let agent2 = Agent::builder("Container Reference Shell Agent")
        .instructions("Reuse the existing shell container and answer concisely.")
        .tool(
            ShellTool::new(
                "shell",
                "Run commands in an existing OpenAI managed shell container",
            )
            .with_hosted_tool_option("environment", container_reference_environment(container_id)),
        )
        .build();
    let result2 = runner
        .run(&agent2, "Run `ls -la /mnt/data`, then summarize.")
        .await?;
    println!("{}", result2.final_output.unwrap_or_default());
    Ok(())
}

fn resolve_skill_reference() -> Value {
    let skill_id = env::var(SHELL_SKILL_ID_ENV).unwrap_or_else(|_| DEFAULT_SKILL_ID.to_owned());
    let mut reference = json!({
        "type": "skill_reference",
        "skill_id": skill_id
    });
    if let Ok(version) = env::var(SHELL_SKILL_VERSION_ENV) {
        reference["version"] = Value::String(version);
    } else {
        reference["version"] = Value::String("1".to_owned());
    }
    reference
}

fn skill_reference_environment(skill_reference: Value) -> Value {
    json!({
        "type": "container_auto",
        "network_policy": {"type": "disabled"},
        "skills": [skill_reference]
    })
}

fn container_reference_environment(container_id: &str) -> Value {
    json!({
        "type": "container_reference",
        "container_id": container_id
    })
}

fn skill_reference_id(request: &ModelRequest) -> Option<String> {
    request.tools.iter().find_map(|tool| {
        if tool.name != "shell" {
            return None;
        }
        tool.hosted_tool_options
            .get("environment")
            .and_then(|environment| environment.get("skills"))
            .and_then(Value::as_array)
            .and_then(|skills| skills.first())
            .filter(|skill| skill.get("type").and_then(Value::as_str) == Some("skill_reference"))
            .and_then(|skill| skill.get("skill_id").and_then(Value::as_str))
            .map(ToOwned::to_owned)
    })
}

fn has_container_reference(request: &ModelRequest) -> bool {
    request.tools.iter().any(|tool| {
        tool.name == "shell"
            && tool
                .hosted_tool_options
                .get("environment")
                .and_then(|environment| environment.get("type"))
                .and_then(Value::as_str)
                == Some("container_reference")
            && tool
                .hosted_tool_options
                .get("environment")
                .and_then(|environment| environment.get("container_id"))
                .and_then(Value::as_str)
                == Some("cntr_skill_ref_demo")
    })
}

fn extract_container_id(output: &str) -> Option<&str> {
    output
        .split_whitespace()
        .find_map(|word| word.strip_prefix("container_id="))
        .map(|value| value.trim_end_matches('.'))
}
