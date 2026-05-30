use std::sync::{Arc, RwLock};

use crate::tracing::config::TracingConfig;
use crate::tracing::processor_interface::TracingProcessor;
use crate::tracing::scope::Scope;
use crate::tracing::span_data::SpanData;
use crate::tracing::util::{gen_group_id, gen_span_id, gen_trace_id, time_iso};
use crate::tracing::{Span, Trace};

#[derive(Default)]
pub struct SynchronousMultiTracingProcessor {
    processors: RwLock<Vec<Arc<dyn TracingProcessor>>>,
}

impl SynchronousMultiTracingProcessor {
    pub fn add_tracing_processor(&self, processor: Arc<dyn TracingProcessor>) {
        self.processors
            .write()
            .expect("tracing processors lock")
            .push(processor);
    }

    pub fn set_processors(&self, processors: Vec<Arc<dyn TracingProcessor>>) {
        *self.processors.write().expect("tracing processors lock") = processors;
    }

    pub fn on_trace_start(&self, trace: &Trace) {
        for processor in self
            .processors
            .read()
            .expect("tracing processors lock")
            .iter()
        {
            processor.on_trace_start(trace);
        }
    }

    pub fn on_trace_end(&self, trace: &Trace) {
        for processor in self
            .processors
            .read()
            .expect("tracing processors lock")
            .iter()
        {
            processor.on_trace_end(trace);
        }
    }

    pub fn on_span_start(&self, span: &Span) {
        for processor in self
            .processors
            .read()
            .expect("tracing processors lock")
            .iter()
        {
            processor.on_span_start(span);
        }
    }

    pub fn on_span_end(&self, span: &Span) {
        for processor in self
            .processors
            .read()
            .expect("tracing processors lock")
            .iter()
        {
            processor.on_span_end(span);
        }
    }

    pub fn shutdown(&self) {
        for processor in self
            .processors
            .read()
            .expect("tracing processors lock")
            .iter()
        {
            processor.shutdown();
        }
    }

    pub fn force_flush(&self) {
        for processor in self
            .processors
            .read()
            .expect("tracing processors lock")
            .iter()
        {
            processor.force_flush();
        }
    }
}

pub trait TraceProvider: Send + Sync {
    fn register_processor(&self, processor: Arc<dyn TracingProcessor>);
    fn set_processors(&self, processors: Vec<Arc<dyn TracingProcessor>>);
    fn get_current_trace(&self) -> Option<Trace>;
    fn get_current_span(&self) -> Option<Span>;
    fn set_disabled(&self, disabled: bool);
    fn time_iso(&self) -> String;
    fn gen_trace_id(&self) -> uuid::Uuid;
    fn gen_span_id(&self) -> uuid::Uuid;
    fn gen_group_id(&self) -> String;
    fn create_trace(
        &self,
        name: &str,
        trace_id: Option<uuid::Uuid>,
        group_id: Option<String>,
        metadata: Option<std::collections::BTreeMap<String, serde_json::Value>>,
        tracing: Option<&TracingConfig>,
        disabled: bool,
    ) -> Trace;
    fn create_span(
        &self,
        name: &str,
        span_data: SpanData,
        span_id: Option<uuid::Uuid>,
        parent: Option<&Span>,
        trace: Option<&Trace>,
        disabled: bool,
    ) -> Span;
    fn start_trace(&self, trace: &mut Trace, mark_as_current: bool);
    fn finish_trace(&self, trace: &mut Trace, reset_current: bool);
    fn start_span(&self, span: &mut Span, mark_as_current: bool);
    fn finish_span(&self, span: &mut Span, reset_current: bool);
    fn shutdown(&self) {}
    fn force_flush(&self) {}
}

pub struct DefaultTraceProvider {
    processors: SynchronousMultiTracingProcessor,
    disabled: RwLock<bool>,
}

impl Default for DefaultTraceProvider {
    fn default() -> Self {
        Self {
            processors: SynchronousMultiTracingProcessor::default(),
            disabled: RwLock::new(false),
        }
    }
}

impl TraceProvider for DefaultTraceProvider {
    fn register_processor(&self, processor: Arc<dyn TracingProcessor>) {
        self.processors.add_tracing_processor(processor);
    }

    fn set_processors(&self, processors: Vec<Arc<dyn TracingProcessor>>) {
        self.processors.set_processors(processors);
    }

    fn get_current_trace(&self) -> Option<Trace> {
        Scope::get_current_trace()
    }

    fn get_current_span(&self) -> Option<Span> {
        Scope::get_current_span()
    }

    fn set_disabled(&self, disabled: bool) {
        *self.disabled.write().expect("trace disabled lock") = disabled;
    }

    fn time_iso(&self) -> String {
        time_iso()
    }

    fn gen_trace_id(&self) -> uuid::Uuid {
        gen_trace_id()
    }

    fn gen_span_id(&self) -> uuid::Uuid {
        gen_span_id()
    }

    fn gen_group_id(&self) -> String {
        gen_group_id()
    }

    fn create_trace(
        &self,
        name: &str,
        trace_id: Option<uuid::Uuid>,
        group_id: Option<String>,
        metadata: Option<std::collections::BTreeMap<String, serde_json::Value>>,
        tracing: Option<&TracingConfig>,
        disabled: bool,
    ) -> Trace {
        let disabled = disabled || *self.disabled.read().expect("trace disabled lock");
        Trace {
            id: trace_id.unwrap_or_else(gen_trace_id),
            workflow_name: name.to_owned(),
            group_id,
            metadata: metadata.unwrap_or_default(),
            tracing_api_key: tracing.and_then(|config| config.api_key.clone()),
            disabled,
            started_at: None,
            ended_at: None,
        }
    }

    fn create_span(
        &self,
        name: &str,
        span_data: SpanData,
        span_id: Option<uuid::Uuid>,
        parent: Option<&Span>,
        trace: Option<&Trace>,
        disabled: bool,
    ) -> Span {
        let current_trace = self.get_current_trace();
        let current_span = self.get_current_span();
        let missing_active_trace = trace.is_none() && parent.is_none() && current_trace.is_none();
        let trace_id = trace
            .map(|trace| trace.id)
            .or_else(|| parent.map(|span| span.trace_id))
            .or_else(|| current_trace.as_ref().map(|trace| trace.id))
            .unwrap_or_else(gen_trace_id);
        let metadata = trace
            .map(|trace| trace.metadata.clone())
            .or_else(|| parent.map(|span| span.metadata.clone()))
            .or_else(|| current_trace.as_ref().map(|trace| trace.metadata.clone()))
            .unwrap_or_default();
        Span {
            id: span_id.unwrap_or_else(gen_span_id),
            trace_id,
            parent_id: parent
                .map(|span| span.id)
                .or_else(|| current_span.as_ref().map(|span| span.id)),
            metadata,
            name: name.to_owned(),
            started_at: None,
            ended_at: None,
            error: None,
            data: span_data,
            disabled: disabled
                || *self.disabled.read().expect("trace disabled lock")
                || missing_active_trace
                || trace.is_some_and(|trace| trace.disabled)
                || parent.is_some_and(|span| span.disabled)
                || current_trace.as_ref().is_some_and(|trace| trace.disabled)
                || current_span.as_ref().is_some_and(|span| span.disabled),
        }
    }

    fn start_trace(&self, trace: &mut Trace, mark_as_current: bool) {
        trace.start();
        if mark_as_current {
            Scope::set_current_trace(Some(trace.clone()));
        }
        if !trace.disabled {
            self.processors.on_trace_start(trace);
        }
    }

    fn finish_trace(&self, trace: &mut Trace, reset_current: bool) {
        trace.finish();
        if !trace.disabled {
            self.processors.on_trace_end(trace);
        }
        if reset_current {
            Scope::set_current_trace(None);
        }
    }

    fn start_span(&self, span: &mut Span, mark_as_current: bool) {
        span.start();
        if mark_as_current {
            Scope::set_current_span(Some(span.clone()));
        }
        if !span.disabled {
            self.processors.on_span_start(span);
        }
    }

    fn finish_span(&self, span: &mut Span, reset_current: bool) {
        span.finish();
        if !span.disabled {
            self.processors.on_span_end(span);
        }
        if reset_current {
            Scope::set_current_span(None);
        }
    }

    fn shutdown(&self) {
        self.processors.shutdown();
    }

    fn force_flush(&self) {
        self.processors.force_flush();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::tracing::scope::Scope;
    use crate::tracing::span_data::AgentSpanData;

    use super::*;

    #[derive(Default)]
    struct RecordingProcessor {
        spans: Mutex<Vec<Span>>,
    }

    impl TracingProcessor for RecordingProcessor {
        fn on_trace_start(&self, _trace: &Trace) {}

        fn on_trace_end(&self, _trace: &Trace) {}

        fn on_span_start(&self, _span: &Span) {}

        fn on_span_end(&self, span: &Span) {
            self.spans
                .lock()
                .expect("recorded spans lock")
                .push(span.clone());
        }
    }

    fn agent_span_data(name: &str) -> SpanData {
        SpanData::Agent(AgentSpanData {
            name: name.to_owned(),
            ..AgentSpanData::default()
        })
    }

    #[test]
    fn span_without_active_trace_is_disabled_and_not_exported() {
        Scope::set_current_trace(None);
        Scope::set_current_span(None);
        let provider = DefaultTraceProvider::default();
        let processor = Arc::new(RecordingProcessor::default());
        provider.register_processor(processor.clone());

        let mut span = provider.create_span(
            "missing-trace",
            agent_span_data("missing-trace"),
            None,
            None,
            None,
            false,
        );

        assert!(span.disabled);
        provider.start_span(&mut span, true);
        provider.finish_span(&mut span, true);

        assert!(
            processor
                .spans
                .lock()
                .expect("recorded spans lock")
                .is_empty()
        );
        assert!(provider.get_current_span().is_none());
        Scope::set_current_trace(None);
        Scope::set_current_span(None);
    }

    #[test]
    fn span_with_current_trace_is_exported() {
        Scope::set_current_trace(None);
        Scope::set_current_span(None);
        let provider = DefaultTraceProvider::default();
        let processor = Arc::new(RecordingProcessor::default());
        provider.register_processor(processor.clone());
        let mut trace = provider.create_trace("active", None, None, None, None, false);
        provider.start_trace(&mut trace, true);

        let mut span =
            provider.create_span("child", agent_span_data("child"), None, None, None, false);
        provider.start_span(&mut span, true);
        provider.finish_span(&mut span, true);
        provider.finish_trace(&mut trace, true);

        assert!(!span.disabled);
        assert_eq!(span.trace_id, trace.id);
        assert_eq!(
            processor.spans.lock().expect("recorded spans lock").len(),
            1
        );
        assert!(provider.get_current_trace().is_none());
        assert!(provider.get_current_span().is_none());
    }
}
