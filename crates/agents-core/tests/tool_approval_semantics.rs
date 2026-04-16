use std::sync::{Arc, Mutex};

use agents_core::{
    Agent, AgentsError, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    RunInterruptionKind, RunItem, Runner, Usage, function_tool,
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Default)]
struct ApprovalModel {
    calls: Arc<Mutex<usize>>,
}

#[async_trait]
impl Model for ApprovalModel {
    async fn generate(&self, request: ModelRequest) -> agents_core::Result<ModelResponse> {
        let mut calls = self.calls.lock().expect("approval model lock");
        *calls += 1;

        if *calls == 1 {
            return Ok(ModelResponse {
                model: request.model,
                output: vec![
                    OutputItem::Reasoning {
                        text: "need approval".to_owned(),
                    },
                    OutputItem::ToolCall {
                        call_id: "call-1".to_owned(),
                        tool_name: "search".to_owned(),
                        arguments: json!({ "query": "rust" }),
                        namespace: None,
                    },
                ],
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
                response_id: Some("resp-1".to_owned()),
                request_id: Some("req-1".to_owned()),
            });
        }

        let tool_result = request
            .input
            .iter()
            .filter_map(|item| match item {
                agents_core::InputItem::Json { value } => value
                    .get("type")
                    .and_then(|kind| (kind == "tool_call_output").then_some(value))
                    .and_then(|value| value.get("output"))
                    .and_then(|output| output.get("text"))
                    .and_then(|text| text.as_str()),
                agents_core::InputItem::Text { .. } => None,
            })
            .find(|text| text.starts_with("result:"))
            .expect("tool output should be replayed into the resumed turn")
            .to_owned();

        Ok(ModelResponse {
            model: request.model,
            output: vec![OutputItem::Text {
                text: format!("final:{tool_result}"),
            }],
            usage: Usage {
                input_tokens: 7,
                output_tokens: 4,
            },
            response_id: Some("resp-2".to_owned()),
            request_id: Some("req-2".to_owned()),
        })
    }
}

#[derive(Clone)]
struct ApprovalProvider {
    model: Arc<ApprovalModel>,
}

impl ModelProvider for ApprovalProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchArgs {
    query: String,
}

#[tokio::test]
async fn runner_interrupts_and_resumes_tool_approval() {
    let provider = Arc::new(ApprovalProvider {
        model: Arc::new(ApprovalModel::default()),
    });
    let search_tool =
        function_tool(
            "search",
            "Search documents",
            |_ctx, args: SearchArgs| async move {
                Ok::<_, AgentsError>(format!("result:{}", args.query))
            },
        )
        .expect("function tool should build")
        .with_needs_approval(true);
    let agent = Agent::builder("assistant")
        .function_tool(search_tool)
        .build();

    let initial = Runner::new()
        .with_model_provider(provider.clone())
        .run(&agent, "hello")
        .await
        .expect("initial run should succeed");

    assert!(initial.final_output.is_none());
    assert_eq!(initial.interruptions.len(), 1);
    assert!(matches!(
        initial
            .interruptions
            .first()
            .and_then(|step| step.kind.clone()),
        Some(RunInterruptionKind::ToolApproval)
    ));

    let mut state = initial
        .durable_state()
        .cloned()
        .expect("state should exist");
    state.approve_for_tool(
        "call-1",
        Some("search".to_owned()),
        Some("approved".to_owned()),
    );

    let resumed = Runner::new()
        .with_model_provider(provider)
        .resume_with_agent(&state, &agent)
        .await
        .expect("resume should succeed");

    assert_eq!(resumed.final_output.as_deref(), Some("final:result:rust"));
    assert!(resumed.new_items.iter().any(|item| {
        matches!(
            item,
            RunItem::ToolCallOutput {
                tool_name,
                call_id,
                ..
            } if tool_name == "search" && call_id.as_deref() == Some("call-1")
        )
    }));
}

#[tokio::test]
async fn runner_rejects_unbound_tool_approval_on_resume() {
    let provider = Arc::new(ApprovalProvider {
        model: Arc::new(ApprovalModel::default()),
    });
    let search_tool =
        function_tool(
            "search",
            "Search documents",
            |_ctx, args: SearchArgs| async move {
                Ok::<_, AgentsError>(format!("result:{}", args.query))
            },
        )
        .expect("function tool should build")
        .with_needs_approval(true);
    let agent = Agent::builder("assistant")
        .function_tool(search_tool)
        .build();

    let initial = Runner::new()
        .with_model_provider(provider.clone())
        .run(&agent, "hello")
        .await
        .expect("initial run should succeed");

    let mut state = initial
        .durable_state()
        .cloned()
        .expect("state should exist");
    state.approve("call-1", Some("approved".to_owned()));

    let resumed = Runner::new()
        .with_model_provider(provider)
        .resume_with_agent(&state, &agent)
        .await;

    assert!(matches!(resumed, Err(AgentsError::User(_))));
}
