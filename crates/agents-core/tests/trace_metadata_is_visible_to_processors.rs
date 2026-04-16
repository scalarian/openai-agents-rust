use std::sync::{Arc, Mutex, OnceLock};

use agents_core::{
    Agent, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem, Result, RunConfig,
    Runner, Span, Trace, TracingProcessor, set_trace_processors,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
struct RecordingTraceProcessor {
    traces: Mutex<Vec<Trace>>,
    spans: Mutex<Vec<Span>>,
}

impl TracingProcessor for RecordingTraceProcessor {
    fn on_trace_start(&self, _trace: &Trace) {}

    fn on_trace_end(&self, trace: &Trace) {
        self.traces.lock().expect("traces lock").push(trace.clone());
    }

    fn on_span_start(&self, _span: &Span) {}

    fn on_span_end(&self, span: &Span) {
        self.spans.lock().expect("spans lock").push(span.clone());
    }
}

#[derive(Clone)]
struct StaticProvider;

struct StaticModel;

#[async_trait]
impl Model for StaticModel {
    async fn generate(&self, _request: ModelRequest) -> Result<ModelResponse> {
        Ok(ModelResponse {
            model: Some("gpt-5".to_owned()),
            output: vec![OutputItem::Text {
                text: "done".to_owned(),
            }],
            ..Default::default()
        })
    }
}

impl ModelProvider for StaticProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        Arc::new(StaticModel)
    }
}

fn trace_test_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[tokio::test]
async fn trace_metadata_is_visible_to_processors() {
    let _guard = trace_test_lock().lock().await;

    let processor = Arc::new(RecordingTraceProcessor::default());
    set_trace_processors(vec![processor.clone()]);

    let trace_id = uuid::Uuid::new_v4();
    let result = Runner::new()
        .with_model_provider(Arc::new(StaticProvider))
        .with_config(RunConfig {
            trace_id: Some(trace_id.to_string()),
            group_id: Some("group-explicit".to_owned()),
            trace_metadata: Some(std::collections::BTreeMap::from([
                ("source".to_owned(), json!("integration-test")),
                ("nested".to_owned(), json!({"value": 1})),
            ])),
            ..Default::default()
        })
        .run(&Agent::builder("assistant").build(), "hello")
        .await
        .expect("run should succeed");

    let finished_trace = processor
        .traces
        .lock()
        .expect("traces lock")
        .iter()
        .find(|trace| trace.id == trace_id)
        .cloned()
        .expect("trace should be exported");
    assert_eq!(finished_trace.id, trace_id);
    assert_eq!(finished_trace.group_id.as_deref(), Some("group-explicit"));
    assert_eq!(
        finished_trace.metadata.get("source"),
        Some(&json!("integration-test"))
    );
    assert_eq!(
        finished_trace.metadata.get("nested"),
        Some(&json!({"value": 1}))
    );
    assert_eq!(result.trace.as_ref().map(|trace| trace.id), Some(trace_id));

    let generation_span = processor
        .spans
        .lock()
        .expect("spans lock")
        .iter()
        .find(|span| span.name == "generation" && span.trace_id == trace_id)
        .cloned()
        .expect("generation span should be exported");
    assert_eq!(generation_span.trace_id, trace_id);
    assert_eq!(
        generation_span.metadata.get("source"),
        Some(&json!("integration-test"))
    );
    assert_eq!(
        generation_span.metadata.get("nested"),
        Some(&json!({"value": 1}))
    );

    let exported_span = serde_json::to_value(&generation_span).expect("span should serialize");
    assert_eq!(exported_span["trace_id"], json!(trace_id));
    assert_eq!(
        exported_span["metadata"]["source"],
        json!("integration-test")
    );
    assert_eq!(exported_span["metadata"]["nested"], json!({"value": 1}));
}
