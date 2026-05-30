use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, ShellTool, Usage,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct LocalShellSkillModel;

#[async_trait]
impl Model for LocalShellSkillModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let text = if has_local_skill(&request) {
            "Agent: local csv-workbench skill configured; created /tmp/test_orders.csv; totals by region are north=275 and south=310; failed orders=1.".to_owned()
        } else if has_local_shell(&request) {
            "Agent (reuse): /tmp/test_orders.csv exists and is ready for follow-up shell work."
                .to_owned()
        } else {
            "Local shell skill was not configured.".to_owned()
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
struct LocalShellSkillProvider {
    model: Arc<LocalShellSkillModel>,
}

impl ModelProvider for LocalShellSkillProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let runner = Runner::new().with_model_provider(Arc::new(LocalShellSkillProvider::default()));

    let agent1 = Agent::builder("Local Shell Agent (Local Skill)")
        .instructions("Use the available local shell skill to answer user requests.")
        .tool(
            ShellTool::new("shell", "Run commands through a local shell environment")
                .with_hosted_tool_option("environment", local_skill_environment()),
        )
        .build();
    let result1 = runner
        .run(
            &agent1,
            "Use the csv-workbench skill. Create /tmp/test_orders.csv and summarize totals by region.",
        )
        .await?;
    println!("{}", result1.final_output.unwrap_or_default());

    let agent2 = Agent::builder("Local Shell Agent (Reuse)")
        .instructions("Reuse the existing local shell and answer concisely.")
        .tool(
            ShellTool::new("shell", "Run commands through a local shell environment")
                .with_hosted_tool_option("environment", json!({"type": "local"})),
        )
        .build();
    let result2 = runner
        .run(
            &agent2,
            "Run `ls -la /tmp/test_orders.csv`, then summarize.",
        )
        .await?;
    println!("{}", result2.final_output.unwrap_or_default());
    Ok(())
}

fn local_skill_environment() -> Value {
    json!({
        "type": "local",
        "skills": [{
            "type": "local",
            "name": "csv-workbench",
            "description": "Analyze CSV files and return concise numeric summaries.",
            "path": "examples/tools/skills/csv-workbench"
        }]
    })
}

fn has_local_skill(request: &ModelRequest) -> bool {
    request.tools.iter().any(|tool| {
        tool.name == "shell"
            && tool
                .hosted_tool_options
                .get("environment")
                .and_then(|environment| environment.get("type"))
                .and_then(Value::as_str)
                == Some("local")
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

fn has_local_shell(request: &ModelRequest) -> bool {
    request.tools.iter().any(|tool| {
        tool.name == "shell"
            && tool
                .hosted_tool_options
                .get("environment")
                .and_then(|environment| environment.get("type"))
                .and_then(Value::as_str)
                == Some("local")
    })
}
