use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    OutputSchemaDefinition, Result as AgentsResult, RunConfig, Runner, Usage,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct EvaluationFeedback {
    feedback: String,
    score: EvaluationScore,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum EvaluationScore {
    Pass,
    NeedsImprovement,
    Fail,
}

impl EvaluationScore {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::NeedsImprovement => "needs_improvement",
            Self::Fail => "fail",
        }
    }
}

#[derive(Clone, Default)]
struct JudgeModel;

#[async_trait]
impl Model for JudgeModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let instructions = request.instructions.unwrap_or_default().to_lowercase();
        let input_text = request
            .input
            .iter()
            .filter_map(InputItem::as_text)
            .collect::<Vec<_>>()
            .join("\n");

        let text = if instructions.contains("evaluate a story outline") {
            evaluate_outline(&input_text)
        } else {
            generate_outline(&input_text)
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 12,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct JudgeProvider {
    model: Arc<JudgeModel>,
}

impl ModelProvider for JudgeProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn generate_outline(input: &str) -> String {
    if input.contains("Feedback:") {
        "A detective on an orbital station follows a false distress call to uncover a hidden alien archive.".to_owned()
    } else {
        "A detective solves a mystery in space.".to_owned()
    }
}

fn evaluate_outline(input: &str) -> String {
    if input.contains("Feedback:") {
        r#"{"feedback":"The outline has a clear setting, hook, and mystery.","score":"pass"}"#
            .to_owned()
    } else {
        r#"{"feedback":"Make the setting and central mystery more specific.","score":"needs_improvement"}"#.to_owned()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let story_outline_generator = Agent::builder("story_outline_generator")
        .instructions(
            "You generate a very short story outline based on the user's input. \
            If there is any feedback provided, use it to improve the outline.",
        )
        .build();

    let evaluator_schema = OutputSchemaDefinition::from_output_type::<EvaluationFeedback>(true)?;
    let evaluator = Agent::builder("evaluator")
        .instructions(
            "You evaluate a story outline and decide if it's good enough. \
            If it's not good enough, provide feedback on what needs to be improved. \
            Never give it a pass on the first try.",
        )
        .output_schema(evaluator_schema)
        .build();

    let msg = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let msg = if msg.trim().is_empty() {
        "A detective story in space.".to_owned()
    } else {
        msg
    };

    let runner = Runner::new()
        .with_config(RunConfig {
            workflow_name: "LLM as a judge".to_owned(),
            group_id: Some("llm-as-a-judge-demo".to_owned()),
            ..RunConfig::default()
        })
        .with_model_provider(Arc::new(JudgeProvider::default()));

    let mut input_items = vec![InputItem::Text { text: msg }];
    let mut latest_outline = String::new();

    for round in 1..=3 {
        let story_outline_result = runner
            .run_items(&story_outline_generator, input_items.clone())
            .await?;

        input_items = story_outline_result.to_input_list();
        latest_outline = story_outline_result
            .final_output_text()
            .unwrap_or_default()
            .to_owned();
        println!("story_outline_generated_round={round}");

        let evaluator_result = runner.run_items(&evaluator, input_items.clone()).await?;
        let evaluation: EvaluationFeedback =
            serde_json::from_str(evaluator_result.final_output_text().unwrap_or_default())
                .map_err(|error| AgentsError::message(error.to_string()))?;

        println!("evaluator_score={}", evaluation.score.as_str());
        if evaluation.score == EvaluationScore::Pass {
            println!("story_outline_accepted=true");
            break;
        }

        println!("re_running_with_feedback=true");
        input_items.push(InputItem::Text {
            text: format!("Feedback: {}", evaluation.feedback),
        });
    }

    println!("final_story_outline={latest_outline}");
    Ok(())
}
