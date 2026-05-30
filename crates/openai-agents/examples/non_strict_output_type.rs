use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentOutputSchema, AgentOutputSchemaBase, AgentsError, Model, ModelBehaviorError,
    ModelProvider, ModelRequest, ModelResponse, OutputItem, OutputSchemaDefinition,
    Result as AgentsResult, Runner, Usage,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct OutputType {
    jokes: BTreeMap<u32, String>,
}

#[derive(Clone, Default)]
struct StructuredOutputModel;

#[async_trait]
impl Model for StructuredOutputModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: json!({
                    "jokes": {
                        "1": "Why did the test pass? It had good assertions.",
                        "2": "Why did the compiler relax? The lifetimes checked out.",
                        "3": "Why was the schema calm? Every field knew its type."
                    },
                    "unexpected": true
                })
                .to_string(),
            }],
            usage: Usage {
                input_tokens: 8,
                output_tokens: 16,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct StructuredOutputProvider {
    model: Arc<StructuredOutputModel>,
}

impl ModelProvider for StructuredOutputProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

struct CustomOutputSchema;

impl AgentOutputSchemaBase for CustomOutputSchema {
    fn is_plain_text(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "CustomOutputSchema"
    }

    fn json_schema(&self) -> std::result::Result<Value, openai_agents::UserError> {
        Ok(json!({
            "type": "object",
            "properties": {
                "jokes": {
                    "type": "object",
                    "additionalProperties": { "type": "string" }
                }
            },
            "required": ["jokes"]
        }))
    }

    fn is_strict_json_schema(&self) -> bool {
        false
    }

    fn validate_json(&self, json_str: &str) -> std::result::Result<Value, ModelBehaviorError> {
        let value: Value = serde_json::from_str(json_str).map_err(|error| ModelBehaviorError {
            message: error.to_string(),
        })?;
        let jokes = value
            .get("jokes")
            .and_then(Value::as_object)
            .ok_or_else(|| ModelBehaviorError {
                message: "missing jokes object".to_owned(),
            })?;
        Ok(Value::Array(jokes.values().cloned().collect()))
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let runner = Runner::new().with_model_provider(Arc::new(StructuredOutputProvider::default()));
    let input = "Tell me 3 short jokes.";

    let strict_schema = AgentOutputSchema::<OutputType>::new(true);
    let strict_agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant.")
        .output_schema(OutputSchemaDefinition::from_agent_output_schema(
            "final_output",
            &strict_schema,
        )?)
        .build();

    let strict_result = runner.run(&strict_agent, input).await?;
    match strict_schema.validate_json(strict_result.final_output_text().unwrap_or_default()) {
        Ok(value) => println!("strict_output={value}"),
        Err(error) => println!("strict_error_expected={}", error.message),
    }

    let non_strict_schema = AgentOutputSchema::<OutputType>::new(false);
    let non_strict_agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant.")
        .output_schema(OutputSchemaDefinition::from_agent_output_schema(
            "final_output",
            &non_strict_schema,
        )?)
        .build();

    let non_strict_result = runner.run(&non_strict_agent, input).await?;
    let normalized = non_strict_schema
        .validate_json(non_strict_result.final_output_text().unwrap_or_default())?;
    println!("non_strict_output={normalized}");

    let custom_schema = CustomOutputSchema;
    let custom_agent = Agent::builder("Assistant")
        .instructions("You are a helpful assistant.")
        .output_schema(OutputSchemaDefinition::from_agent_output_schema(
            custom_schema.name(),
            &custom_schema,
        )?)
        .build();

    let custom_result = runner.run(&custom_agent, input).await?;
    let custom_output =
        custom_schema.validate_json(custom_result.final_output_text().unwrap_or_default())?;
    println!("custom_output={custom_output}");

    Ok(())
}
