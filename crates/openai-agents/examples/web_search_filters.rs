use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, ModelSettings,
    OutputItem, ReasoningSettings, Result as AgentsResult, RunItem, Runner, Usage,
    WebSearchToolOptions, web_search_tool_with_options,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct FilteredWebSearchModel;

#[async_trait]
impl Model for FilteredWebSearchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_domain_filters = request.tools.iter().any(|tool| {
            tool.name == "web_search"
                && tool
                    .hosted_tool_options
                    .get("filters")
                    .and_then(|filters| filters.get("allowed_domains"))
                    .and_then(Value::as_array)
                    .is_some_and(|domains| {
                        domains.iter().any(|domain| domain == "platform.openai.com")
                            && domains
                                .iter()
                                .any(|domain| domain == "developers.openai.com")
                    })
        });
        let output = if has_domain_filters {
            vec![
                OutputItem::Json {
                    value: json!({
                        "type": "web_search_call",
                        "action": {
                            "sources": [
                                {"url": "https://platform.openai.com/docs/guides/tools-web-search"},
                                {"url": "https://developers.openai.com/resources/"}
                            ]
                        }
                    }),
                },
                OutputItem::Text {
                    text: "OpenAI developer docs describe hosted web search, source includes, and domain filtering for Responses API tool use.".to_owned(),
                },
            ]
        } else {
            vec![OutputItem::Text {
                text: "No filtered web search tool was configured.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 18,
                output_tokens: 10,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FilteredWebSearchProvider {
    model: Arc<FilteredWebSearchModel>,
}

impl ModelProvider for FilteredWebSearchProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("WebOAI website searcher")
        .model("gpt-5-nano")
        .instructions(
            "Search OpenAI developer documentation and platform docs. Ignore unrelated domains.",
        )
        .tool(web_search_tool_with_options(WebSearchToolOptions {
            filters: Some(json!({
                "allowed_domains": ["developers.openai.com", "platform.openai.com"]
            })),
            search_context_size: "medium".to_owned(),
            ..WebSearchToolOptions::default()
        }))
        .model_settings(ModelSettings {
            reasoning: Some(ReasoningSettings {
                effort: Some("low".to_owned()),
                summary: None,
            }),
            verbosity: Some("low".to_owned()),
            response_include: vec!["web_search_call.action.sources".to_owned()],
            ..ModelSettings::default()
        })
        .build();
    let today = "2026-05-30";
    let query = format!(
        "Write a summary of the latest OpenAI API and developer platform updates from the last few weeks (today is {today})."
    );

    let result = Runner::new()
        .with_model_provider(Arc::new(FilteredWebSearchProvider::default()))
        .run(&agent, query)
        .await?;

    println!("### Sources ###");
    for url in normalized_source_urls(&result.new_items) {
        println!("- {url}");
    }
    println!();
    println!("### Final output ###");
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}

fn normalized_source_urls(items: &[RunItem]) -> Vec<String> {
    let allowed_hosts = ["developers.openai.com", "platform.openai.com"];
    let mut urls = BTreeSet::new();
    for item in items {
        let RunItem::MessageOutput {
            content: OutputItem::Json { value },
        } = item
        else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("web_search_call") {
            continue;
        }
        let Some(sources) = value
            .get("action")
            .and_then(|action| action.get("sources"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for source in sources {
            let Some(url) = source.get("url").and_then(Value::as_str) else {
                continue;
            };
            if allowed_hosts
                .iter()
                .any(|host| url.starts_with(&format!("https://{host}/")))
            {
                urls.insert(url.trim_end_matches('/').to_owned());
            }
        }
    }
    urls.into_iter().collect()
}
