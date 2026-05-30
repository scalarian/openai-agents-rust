use std::sync::Arc;

use async_trait::async_trait;
use futures::FutureExt;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse,
    ModelSettings, OutputItem, Result as AgentsResult, RunConfig, Runner, Usage, function_tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct PublishArgs {
    title: String,
    body: String,
}

#[derive(Clone, Default)]
struct AnnouncementModel;

#[async_trait]
impl Model for AnnouncementModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(rejection) = tool_output(&request.input, "publish_announcement") {
            vec![OutputItem::Text { text: rejection }]
        } else {
            vec![OutputItem::ToolCall {
                call_id: "call-publish".to_owned(),
                tool_name: "publish_announcement".to_owned(),
                arguments: json!({
                    "title": "Office maintenance",
                    "body": "The office will close at 6 PM today."
                }),
                namespace: None,
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 14,
                output_tokens: 6,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct AnnouncementProvider {
    model: Arc<AnnouncementModel>,
}

impl ModelProvider for AnnouncementProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let publish_announcement = function_tool(
        "publish_announcement",
        "Publish an announcement to users.",
        |_ctx, args: PublishArgs| async move {
            Ok::<_, AgentsError>(format!(
                "Published announcement '{}' with body: {}",
                args.title, args.body
            ))
        },
    )?
    .with_needs_approval(true);

    let agent = Agent::builder("Operations Assistant")
        .instructions(
            "Call the publish_announcement tool for publish requests. If the tool call is rejected, repeat the rejection message exactly.",
        )
        .model_settings(ModelSettings {
            tool_choice: Some("publish_announcement".to_owned()),
            ..ModelSettings::default()
        })
        .function_tool(publish_announcement)
        .build();
    let runner = Runner::new()
        .with_model_provider(Arc::new(AnnouncementProvider::default()))
        .with_config(RunConfig {
            tool_error_formatter: Some(Arc::new(|args| {
                async move {
                    if args.kind == "approval_rejected" {
                        Ok(Some(
                            "Publish action was canceled because approval was rejected.".to_owned(),
                        ))
                    } else {
                        Ok(None)
                    }
                }
                .boxed()
            })),
            ..RunConfig::default()
        });

    let initial = runner
        .run(
            &agent,
            "Please publish an announcement titled 'Office maintenance' with body 'The office will close at 6 PM today.'",
        )
        .await?;

    let mut state = initial
        .durable_state()
        .cloned()
        .ok_or_else(|| AgentsError::message("approval run did not include durable state"))?;
    for interruption in &initial.interruptions {
        println!(
            "approval_required tool={} call_id={}",
            interruption.tool_name.as_deref().unwrap_or_default(),
            interruption.call_id.as_deref().unwrap_or_default()
        );
        state.reject_for_tool(
            interruption.call_id.clone().unwrap_or_default(),
            interruption.tool_name.clone(),
            Some("Publish action was canceled because the reviewer denied approval.".to_owned()),
        );
        println!(
            "rejected={} custom_message=true",
            interruption.tool_name.as_deref().unwrap_or_default()
        );
    }

    let result = runner.resume_with_agent(&state, &agent).await?;
    if let Some(formatter_output) = result.new_items.iter().find_map(tool_call_output_text) {
        println!("formatter_output={formatter_output}");
    }
    println!("final_output={}", result.final_output.unwrap_or_default());
    Ok(())
}

fn tool_output(input: &[InputItem], tool_name: &str) -> Option<String> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("tool_name").and_then(Value::as_str) != Some(tool_name)
        {
            return None;
        }
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn tool_call_output_text(item: &openai_agents::RunItem) -> Option<String> {
    let openai_agents::RunItem::ToolCallOutput { output, .. } = item else {
        return None;
    };
    match output {
        OutputItem::Text { text } => Some(text.clone()),
        OutputItem::Json { value } => Some(value.to_string()),
        OutputItem::Refusal { refusal } => Some(refusal.clone()),
        OutputItem::ToolCall { .. } | OutputItem::Handoff { .. } | OutputItem::Reasoning { .. } => {
            None
        }
    }
}
