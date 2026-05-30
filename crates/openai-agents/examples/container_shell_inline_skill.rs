use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, ShellTool, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct InlineSkillModel;

#[async_trait]
impl Model for InlineSkillModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let text = if has_inline_skill(&request) {
            "Agent: inline csv-workbench skill configured; generated /mnt/data/orders.csv; totals by region are west=420 and east=315; failed orders=1; container_id=cntr_csv_demo.".to_owned()
        } else if has_container_reference(&request) {
            "Agent (container reuse): reused cntr_csv_demo and found /mnt/data/orders.csv."
                .to_owned()
        } else {
            "Shell container skill was not configured.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 18,
                output_tokens: 18,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct InlineSkillProvider {
    model: Arc<InlineSkillModel>,
}

impl ModelProvider for InlineSkillProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let runner = Runner::new().with_model_provider(Arc::new(InlineSkillProvider::default()));

    let agent1 = Agent::builder("Container Shell Agent (Inline Skill)")
        .instructions("Use the available container skill to answer user requests.")
        .tool(
            ShellTool::new("shell", "Run commands in an OpenAI managed shell container")
                .with_hosted_tool_option("environment", inline_skill_environment()),
        )
        .build();
    let result1 = runner
        .run(
            &agent1,
            "Use the csv-workbench skill to create /mnt/data/orders.csv and summarize it.",
        )
        .await?;
    let first_output = result1.final_output.unwrap_or_default();
    println!("{first_output}");

    let container_id = extract_container_id(&first_output).unwrap_or("cntr_csv_demo");
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

fn inline_skill_environment() -> Value {
    json!({
        "type": "container_auto",
        "network_policy": {"type": "disabled"},
        "skills": [{
            "type": "inline",
            "name": "csv-workbench",
            "description": "Analyze CSV files in /mnt/data and return concise numeric summaries.",
            "source": {
                "type": "text",
                "media_type": "text/markdown",
                "data": "# csv-workbench\nRead CSV files from /mnt/data and summarize numeric columns by group."
            }
        }]
    })
}

fn container_reference_environment(container_id: &str) -> Value {
    json!({
        "type": "container_reference",
        "container_id": container_id
    })
}

fn has_inline_skill(request: &ModelRequest) -> bool {
    request.tools.iter().any(|tool| {
        tool.name == "shell"
            && tool
                .hosted_tool_options
                .get("environment")
                .and_then(|environment| environment.get("type"))
                .and_then(Value::as_str)
                == Some("container_auto")
            && tool
                .hosted_tool_options
                .get("environment")
                .and_then(|environment| environment.get("skills"))
                .and_then(Value::as_array)
                .is_some_and(|skills| {
                    skills.iter().any(|skill| {
                        skill.get("name").and_then(Value::as_str) == Some("csv-workbench")
                    })
                })
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
                == Some("cntr_csv_demo")
    })
}

fn extract_container_id(output: &str) -> Option<&str> {
    output
        .split_whitespace()
        .find_map(|word| word.strip_prefix("container_id="))
        .map(|value| value.trim_end_matches('.'))
}
