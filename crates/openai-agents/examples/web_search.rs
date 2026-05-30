use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, Usage, WebSearchToolOptions, web_search_tool_with_options,
};
use serde_json::json;

#[derive(Clone, Default)]
struct WebSearchModel;

#[async_trait]
impl Model for WebSearchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_new_york_search = request.tools.iter().any(|tool| {
            tool.name == "web_search"
                && tool
                    .hosted_tool_options
                    .get("user_location")
                    .and_then(|value| value.get("city"))
                    .and_then(serde_json::Value::as_str)
                    == Some("New York")
        });
        let text = if has_new_york_search {
            "Local sports update: New York fans are tracking playoff races across several teams."
                .to_owned()
        } else {
            "No web search tool was configured.".to_owned()
        };

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text { text }],
            usage: Usage {
                input_tokens: 10,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct WebSearchProvider {
    model: Arc<WebSearchModel>,
}

impl ModelProvider for WebSearchProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Web searcher")
        .instructions("You are a helpful agent.")
        .tool(web_search_tool_with_options(WebSearchToolOptions {
            user_location: Some(json!({"type": "approximate", "city": "New York"})),
            ..WebSearchToolOptions::default()
        }))
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(WebSearchProvider::default()))
        .run(
            &agent,
            "search the web for 'local sports news' and give me 1 interesting update in a sentence.",
        )
        .await?;

    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
