use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentAsToolOptions, AgentsError, InputItem, Model, ModelProvider, ModelRequest,
    ModelResponse, OutputItem, Result as AgentsResult, Runner, Usage, set_default_agent_runner,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct TranslationInput {
    text: String,
    source: String,
    target: String,
}

#[derive(Clone, Default)]
struct TranslationToolModel;

#[async_trait]
impl Model for TranslationToolModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input_text = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n");

        let output = if instructions.contains("task dispatcher") {
            if let Some(translation) = tool_translation_output(&request.input) {
                vec![OutputItem::Text {
                    text: format!("Translator agent as tool: {translation}"),
                }]
            } else {
                vec![OutputItem::ToolCall {
                    call_id: "call-translate-text".to_owned(),
                    tool_name: "translate_text".to_owned(),
                    arguments: json!({
                        "text": "Hola",
                        "source": "Spanish",
                        "target": "French"
                    }),
                    namespace: None,
                }]
            }
        } else {
            vec![OutputItem::Text {
                text: translate(&input_text),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 8,
                output_tokens: 6,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct TranslationToolProvider {
    model: Arc<TranslationToolModel>,
}

impl ModelProvider for TranslationToolProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn translate(input: &str) -> String {
    if input.contains("Hola") && input.to_lowercase().contains("french") {
        "Bonjour.".to_owned()
    } else {
        format!("Translated: {input}")
    }
}

fn tool_translation_output(input: &[InputItem]) -> Option<String> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        let output = value.get("output")?;
        match output.get("type").and_then(Value::as_str) {
            Some("text") => output
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_owned),
            Some("json") => output.get("value").map(Value::to_string),
            _ => None,
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let translator = Agent::builder("translator")
        .instructions(
            "Translate the input text into the target language. \
            If the target is not clear, ask the user for clarification.",
        )
        .build();

    let translate_text = translator.as_tool::<TranslationInput>(
        Some("translate_text"),
        Some(
            "Translate text between languages. Provide text, source language, and target language.",
        ),
        AgentAsToolOptions::default(),
    )?;

    let orchestrator = Agent::builder("orchestrator")
        .instructions(
            "You are a task dispatcher. Always call the tool with sufficient input. \
            Do not handle the translation yourself.",
        )
        .function_tool(translate_text)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(TranslationToolProvider::default()));
    set_default_agent_runner(Some(runner.clone()));

    let query = "Translate \"Hola\" from Spanish to French.";
    let direct = runner.run(&translator, query).await?;
    println!(
        "Translator agent direct run: {}",
        direct.final_output.unwrap_or_default()
    );

    let nested = runner.run(&orchestrator, query).await?;
    println!("{}", nested.final_output.unwrap_or_default());

    Ok(())
}
