use agents_core::{ModelRetryAdvice, ModelRetryNormalizedError};

pub fn get_openai_retry_advice(normalized: &ModelRetryNormalizedError) -> ModelRetryAdvice {
    let status_code = normalized.status_code.unwrap_or_default();
    let retryable_status = matches!(status_code, 408 | 409 | 429 | 500 | 502 | 503 | 504);
    let retryable_code = normalized
        .error_code
        .as_deref()
        .map(|code| matches!(code, "rate_limit_exceeded" | "server_error" | "overloaded"))
        .unwrap_or(false);
    let retryable = normalized.is_network_error
        || normalized.is_timeout
        || (!normalized.is_abort && (retryable_status || retryable_code));

    ModelRetryAdvice {
        suggested: Some(retryable),
        retry_after: normalized.retry_after,
        replay_safety: Some(if normalized.request_id.is_some() {
            "idempotent".to_owned()
        } else {
            "best_effort".to_owned()
        }),
        reason: Some(if retryable {
            "openai transport error is retryable".to_owned()
        } else {
            "openai transport error is not retryable".to_owned()
        }),
        normalized: Some(normalized.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_rate_limits_as_retryable() {
        let advice = get_openai_retry_advice(&ModelRetryNormalizedError {
            status_code: Some(429),
            retry_after: Some(2.0),
            ..ModelRetryNormalizedError::default()
        });

        assert_eq!(advice.suggested, Some(true));
        assert_eq!(advice.retry_after, Some(2.0));
    }
}
