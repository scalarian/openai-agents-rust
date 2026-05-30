use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage, custom_tool,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct RawEditModel {
    calls: Arc<Mutex<usize>>,
}

#[async_trait]
impl Model for RawEditModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let mut calls = self.calls.lock().expect("raw edit model lock");
        *calls += 1;

        let output = if *calls == 1 {
            vec![OutputItem::CustomToolCall {
                call_id: "call-raw-edit".to_owned(),
                tool_name: "raw_editor".to_owned(),
                input: "hello from raw input".to_owned(),
            }]
        } else {
            let edited = request
                .input
                .iter()
                .find_map(custom_tool_output)
                .unwrap_or_else(|| "missing custom output".to_owned());
            vec![OutputItem::Text {
                text: format!("edited={edited}"),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage::default(),
            response_id: Some(format!("resp-raw-edit-{calls}")),
            request_id: None,
        })
    }
}

fn custom_tool_output(item: &InputItem) -> Option<String> {
    let InputItem::Json { value } = item else {
        return None;
    };
    (value.get("type").and_then(Value::as_str) == Some("custom_tool_call_output"))
        .then(|| {
            value
                .get("output")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .flatten()
}

#[derive(Clone, Default)]
struct RawEditProvider {
    model: Arc<RawEditModel>,
}

impl ModelProvider for RawEditProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let raw_editor = custom_tool("raw_editor", "Edit raw text.", |_ctx, input| async move {
        Ok::<_, AgentsError>(input.to_uppercase())
    })
    .with_format(json!({"type": "text"}));

    let agent = Agent::builder("Raw editor")
        .instructions("Use the raw editor for unstructured text edits.")
        .custom_tool(raw_editor)
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(RawEditProvider::default()))
        .run(&agent, "Uppercase this draft.")
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
