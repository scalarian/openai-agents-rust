use openai_agents::{
    Agent, CodeInterpreterToolOptions, InputItem, MCPListToolsItem, Model, ModelProvider,
    ModelRequest, ModelResponse, OutputItem, Result as AgentsResult, Runner, ToolSearchCallItem,
    ToolSearchOutputItem, Usage, code_interpreter_tool, code_interpreter_tool_with_options,
    file_search_tool, image_generation_tool, tool_search_tool, web_search_tool,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct RecordingToolsModel {
    tools: Arc<Mutex<Vec<Vec<String>>>>,
}

#[async_trait::async_trait]
impl Model for RecordingToolsModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        self.tools
            .lock()
            .expect("recorded tools lock")
            .push(request.tools.iter().map(|tool| tool.name.clone()).collect());
        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: "done".to_owned(),
            }],
            usage: Usage::default(),
            response_id: None,
            request_id: None,
        })
    }
}

struct RecordingToolsProvider {
    model: Arc<RecordingToolsModel>,
}

impl ModelProvider for RecordingToolsProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[test]
fn facade_hosted_tool_helpers_are_constructible() {
    let tool_names = vec![
        code_interpreter_tool().definition.name,
        file_search_tool().definition.name,
        image_generation_tool().definition.name,
        tool_search_tool().definition.name,
        web_search_tool().definition.name,
    ];

    assert_eq!(
        tool_names,
        vec![
            "code_interpreter",
            "file_search",
            "image_generation",
            "tool_search",
            "web_search",
        ]
    );

    let configured = code_interpreter_tool_with_options(CodeInterpreterToolOptions {
        container: Some(json!({"type": "auto"})),
    });
    assert_eq!(
        configured.definition.hosted_tool_options.get("container"),
        Some(&json!({"type": "auto"}))
    );
}

#[test]
fn facade_exports_hosted_tool_search_run_items() {
    let call_item = ToolSearchCallItem {
        raw_item: InputItem::Json {
            value: json!({
                "type": "tool_search_call",
                "call_id": "search-1",
                "created_by": "server",
            }),
        },
    };
    let output_item = ToolSearchOutputItem {
        raw_item: InputItem::Json {
            value: json!({
                "type": "tool_search_output",
                "call_id": "search-1",
                "created_by": "server",
                "results": [{"name": "lookup"}],
            }),
        },
    };
    let mcp_item = MCPListToolsItem {
        raw_item: InputItem::Json {
            value: json!({
                "type": "mcp_list_tools",
                "server_label": "docs",
                "tools": [{"name": "lookup"}],
            }),
        },
    };

    assert_eq!(
        call_item.to_input_item(),
        InputItem::Json {
            value: json!({
                "type": "tool_search_call",
                "call_id": "search-1",
            })
        }
    );
    assert_eq!(
        output_item.to_input_item(),
        InputItem::Json {
            value: json!({
                "type": "tool_search_output",
                "call_id": "search-1",
                "results": [{"name": "lookup"}],
            })
        }
    );
    assert_eq!(
        mcp_item.to_input_item(),
        InputItem::Json {
            value: json!({
                "type": "mcp_list_tools",
                "server_label": "docs",
                "tools": [{"name": "lookup"}],
            })
        }
    );
}

#[tokio::test]
async fn facade_run_delivers_static_hosted_tools_to_model_request() {
    let model = Arc::new(RecordingToolsModel::default());
    let agent = Agent::builder("assistant").tool(web_search_tool()).build();

    Runner::new()
        .with_model_provider(Arc::new(RecordingToolsProvider {
            model: model.clone(),
        }))
        .run(&agent, "search")
        .await
        .expect("run should succeed");

    assert_eq!(
        model.tools.lock().expect("recorded tools lock").as_slice(),
        &[vec!["web_search".to_owned()]]
    );
}
