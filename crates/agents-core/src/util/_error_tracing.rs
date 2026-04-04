use crate::tracing::{Span, SpanError, get_current_span};

pub fn attach_error_to_span(span: &mut Span, error: SpanError) {
    span.error = Some(error);
}

pub fn attach_error_to_current_span(error: SpanError) {
    if let Some(mut span) = get_current_span() {
        attach_error_to_span(&mut span, error);
    }
}
