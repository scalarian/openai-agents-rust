use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, MCPGetPromptResult, MCPPrompt, MCPPromptArgument,
    MCPPromptContent, MCPPromptMessage, MCPPromptTextContent, MCPServer, MCPServerStreamableHttp,
    MCPServerStreamableHttpParams, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage,
};
use serde_json::json;

const PROMPT_NAME: &str = "generate_code_review_instructions";

#[derive(Clone, Default)]
struct CodeReviewModel;

#[async_trait]
impl Model for CodeReviewModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let focus = request
            .instructions
            .as_deref()
            .and_then(|instructions| instructions.split("focus on ").nth(1))
            .and_then(|tail| tail.split('.').next())
            .unwrap_or("general code quality");

        let code_summary = if input_text(&request.input).contains("os.system") {
            "The code builds a shell command from unsanitized user input and passes it to os.system."
        } else {
            "The submitted code needs a standard correctness and maintainability review."
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: format!(
                    "Overall Assessment: {code_summary}\nSpecific Issue: command injection risk.\nRecommended Improvement: avoid shell interpolation, pass arguments directly, or validate input strictly.\nReview focus: {focus}."
                ),
            }],
            usage: Usage {
                input_tokens: 28,
                output_tokens: 42,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct CodeReviewProvider {
    model: Arc<CodeReviewModel>,
}

impl ModelProvider for CodeReviewProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let server = MCPServerStreamableHttp::new(
        "Simple Prompt Server",
        MCPServerStreamableHttpParams {
            url: "http://127.0.0.1:18080/mcp".to_owned(),
            ..MCPServerStreamableHttpParams::default()
        },
    )
    .with_prompts(vec![MCPPrompt {
        name: PROMPT_NAME.to_owned(),
        description: Some("Generate agent instructions for code review tasks.".to_owned()),
        arguments: vec![
            MCPPromptArgument {
                name: "focus".to_owned(),
                description: Some("Review focus area.".to_owned()),
                required: Some(false),
            },
            MCPPromptArgument {
                name: "language".to_owned(),
                description: Some("Programming language under review.".to_owned()),
                required: Some(false),
            },
        ],
        ..MCPPrompt::default()
    }])
    .with_prompt_results(HashMap::from([(
        PROMPT_NAME.to_owned(),
        MCPGetPromptResult {
            description: Some("Code review prompt".to_owned()),
            messages: vec![MCPPromptMessage {
                role: "user".to_owned(),
                content: MCPPromptContent::Text(MCPPromptTextContent {
                    text: "You are a senior python code review specialist. Analyze code quality, security, performance, and best practices with focus on security vulnerabilities.".to_owned(),
                }),
            }],
        },
    )]));

    server.connect().await?;
    let prompts = server.list_prompts(None).await?;
    println!("Available prompts:");
    for prompt in prompts.prompts {
        println!(
            "- {}: {}",
            prompt.name,
            prompt.description.unwrap_or_default()
        );
    }

    let prompt = server
        .get_prompt(
            PROMPT_NAME,
            json!({"focus": "security vulnerabilities", "language": "python"}),
        )
        .await?;
    let instructions = prompt_text(&prompt);
    println!("Generated instructions from prompt: {PROMPT_NAME}");
    server.cleanup().await?;

    let agent = Agent::builder("Code Reviewer Agent")
        .instructions(instructions)
        .build();
    let result = Runner::new()
        .with_model_provider(Arc::new(CodeReviewProvider::default()))
        .run(
            &agent,
            "Please review this code:\n\ndef process_user_input(user_input):\n    command = f\"echo {user_input}\"\n    os.system(command)\n    return \"Command executed\"\n",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn prompt_text(prompt: &MCPGetPromptResult) -> String {
    prompt
        .messages
        .iter()
        .map(|message| match &message.content {
            MCPPromptContent::Text(content) => content.text.clone(),
            MCPPromptContent::Json { value } => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn input_text(input: &[InputItem]) -> String {
    input
        .iter()
        .map(|item| match item {
            InputItem::Text { text } => text.clone(),
            InputItem::Json { value } => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
