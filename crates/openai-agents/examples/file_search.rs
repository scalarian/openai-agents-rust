use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, FileSearchToolOptions, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunItem, Runner, Usage, file_search_tool_with_options,
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct FileSearchModel;

#[async_trait]
impl Model for FileSearchModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let has_file_search = request.tools.iter().any(|tool| {
            tool.name == "file_search"
                && tool
                    .hosted_tool_options
                    .get("vector_store_ids")
                    .and_then(Value::as_array)
                    .is_some_and(|ids| ids.iter().any(|id| id == "vs_arrakis_example"))
                && tool
                    .hosted_tool_options
                    .get("max_num_results")
                    .and_then(Value::as_u64)
                    == Some(3)
                && tool
                    .hosted_tool_includes
                    .iter()
                    .any(|include| include == "file_search_call.results")
        });
        let output = if has_file_search {
            vec![
                OutputItem::Json {
                    value: json!({
                        "type": "file_search_call",
                        "id": "fs_arrakis",
                        "queries": ["Arrakis"],
                        "results": [{
                            "file_id": "file_arrakis_note",
                            "filename": "example.txt",
                            "score": 0.97,
                            "text": "Arrakis was inspired by water scarcity as a metaphor for oil and other finite resources."
                        }]
                    }),
                },
                OutputItem::Text {
                    text: "Arrakis was inspired by water scarcity as a metaphor for oil and other finite resources.".to_owned(),
                },
            ]
        } else {
            vec![OutputItem::Text {
                text: "No file search tool was configured.".to_owned(),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 16,
                output_tokens: 12,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct FileSearchProvider {
    model: Arc<FileSearchModel>,
}

impl ModelProvider for FileSearchProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let vector_store_id = "vs_arrakis_example";
    println!("### Prepared vector store ###");
    println!("Using indexed file example.txt in vector store {vector_store_id}");

    let agent = Agent::builder("File searcher")
        .instructions(
            "You are a helpful agent. Answer only based on information in the vector store.",
        )
        .tool(file_search_tool_with_options(FileSearchToolOptions {
            vector_store_ids: vec![vector_store_id.to_owned()],
            max_num_results: Some(3),
            include_search_results: true,
            ..FileSearchToolOptions::default()
        }))
        .build();

    let result = Runner::new()
        .with_model_provider(Arc::new(FileSearchProvider::default()))
        .run(
            &agent,
            "Be concise, and tell me 1 sentence about Arrakis I might not know.",
        )
        .await?;

    println!("\n### Final output ###");
    println!("{}", result.final_output.unwrap_or_default());

    println!("\n### Output items ###");
    for item in file_search_items(&result.new_items) {
        println!("{item}");
    }
    Ok(())
}

fn file_search_items(items: &[RunItem]) -> Vec<&Value> {
    items
        .iter()
        .filter_map(|item| {
            let RunItem::MessageOutput {
                content: OutputItem::Json { value },
            } = item
            else {
                return None;
            };
            (value.get("type").and_then(Value::as_str) == Some("file_search_call")).then_some(value)
        })
        .collect()
}
