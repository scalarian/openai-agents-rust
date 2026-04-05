use crate::tracing::setup::get_trace_provider;
use crate::tracing::{Trace, trace};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn create_trace_for_run(workflow_name: &str) -> Trace {
    let mut trace = trace(workflow_name, None, None, None, None, false);
    get_trace_provider().start_trace(&mut trace, true);
    trace
}

pub fn create_trace_for_run_with_options(
    workflow_name: &str,
    trace_id: Option<Uuid>,
    group_id: Option<String>,
    metadata: Option<BTreeMap<String, Value>>,
    tracing: Option<&crate::tracing::TracingConfig>,
    disabled: bool,
) -> Trace {
    let mut trace = trace(
        workflow_name,
        trace_id,
        group_id,
        metadata,
        tracing,
        disabled,
    );
    get_trace_provider().start_trace(&mut trace, true);
    trace
}

#[derive(Clone, Debug)]
pub struct TraceCtxManager {
    trace: Option<Trace>,
}

impl TraceCtxManager {
    pub fn new(workflow_name: &str) -> Self {
        Self {
            trace: Some(create_trace_for_run(workflow_name)),
        }
    }

    pub fn with_options(
        workflow_name: &str,
        trace_id: Option<Uuid>,
        group_id: Option<String>,
        metadata: Option<BTreeMap<String, Value>>,
        tracing: Option<&crate::tracing::TracingConfig>,
        disabled: bool,
    ) -> Self {
        Self {
            trace: Some(create_trace_for_run_with_options(
                workflow_name,
                trace_id,
                group_id,
                metadata,
                tracing,
                disabled,
            )),
        }
    }

    pub fn trace(&self) -> &Trace {
        self.trace
            .as_ref()
            .expect("trace manager should own a trace")
    }

    pub fn finish(&mut self) -> Trace {
        let mut trace = self.trace.take().expect("trace manager should own a trace");
        get_trace_provider().finish_trace(&mut trace, true);
        trace
    }
}

impl Drop for TraceCtxManager {
    fn drop(&mut self) {
        if let Some(mut trace) = self.trace.take() {
            get_trace_provider().finish_trace(&mut trace, true);
        }
    }
}
