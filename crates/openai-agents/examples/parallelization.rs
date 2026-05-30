use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, RunConfig, Runner, Usage,
};

#[derive(Clone, Default)]
struct TranslationModel;

#[async_trait]
impl Model for TranslationModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .last()
            .unwrap_or_default();

        let text = if instructions.contains("pick the best") {
            pick_best_translation(input)
        } else {
            translate_to_spanish(input)
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 8,
                output_tokens: 4,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct TranslationProvider {
    model: Arc<TranslationModel>,
}

impl ModelProvider for TranslationProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn translate_to_spanish(input: &str) -> String {
    match input.to_lowercase().as_str() {
        text if text.contains("good morning") => "Buenos dias!".to_owned(),
        text if text.contains("hello") || text.contains("hi") => "Hola!".to_owned(),
        _ => format!("Traduccion al espanol: {input}"),
    }
}

fn pick_best_translation(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty() && !line.starts_with("Input:") && !line.starts_with("Translations:")
        })
        .unwrap_or("No translation available.")
        .to_owned()
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let spanish_agent = Agent::builder("spanish_agent")
        .instructions("You translate the user's message to Spanish.")
        .build();

    let translation_picker = Agent::builder("translation_picker")
        .instructions("You pick the best Spanish translation from the given options.")
        .build();

    let msg = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let msg = if msg.trim().is_empty() {
        "Good morning!".to_owned()
    } else {
        msg
    };

    let runner = Runner::new()
        .with_config(RunConfig {
            workflow_name: "Parallel translation".to_owned(),
            group_id: Some("parallel-translation-demo".to_owned()),
            ..RunConfig::default()
        })
        .with_model_provider(Arc::new(TranslationProvider::default()));

    let (res_1, res_2, res_3) = tokio::try_join!(
        runner.run(&spanish_agent, msg.clone()),
        runner.run(&spanish_agent, msg.clone()),
        runner.run(&spanish_agent, msg.clone()),
    )?;

    let outputs = [
        res_1.final_output_text().unwrap_or_default().to_owned(),
        res_2.final_output_text().unwrap_or_default().to_owned(),
        res_3.final_output_text().unwrap_or_default().to_owned(),
    ];
    let translations = outputs.join("\n\n");
    println!("translations:\n{translations}");

    let best_translation = runner
        .run(
            &translation_picker,
            format!("Input: {msg}\n\nTranslations:\n{translations}"),
        )
        .await?;

    println!(
        "best_translation={}",
        best_translation.final_output_text().unwrap_or_default()
    );
    Ok(())
}
