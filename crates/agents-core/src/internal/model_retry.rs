use std::future::Future;
use std::time::Duration;

use crate::errors::{AgentsError, Result};
use crate::model::ModelResponse;
use crate::retry::{
    ModelRetryNormalizedError, ModelRetrySettings, RetryDecision, RetryPolicyContext,
};
use crate::usage::Usage;

pub(crate) fn apply_retry_attempt_usage(previous: Usage, next: Usage) -> Usage {
    crate::internal::agent_runner_helpers::merge_usage(previous, next)
}

const DEFAULT_INITIAL_DELAY_SECONDS: f64 = 0.25;
const DEFAULT_MAX_DELAY_SECONDS: f64 = 2.0;
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

pub(crate) async fn get_response_with_retry<F, Fut>(
    mut operation: F,
    retry_settings: Option<ModelRetrySettings>,
    previous_response_id: Option<String>,
    conversation_id: Option<String>,
) -> Result<ModelResponse>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<ModelResponse>>,
{
    let max_retries = retry_settings
        .as_ref()
        .and_then(|settings| settings.max_retries)
        .unwrap_or_default();
    let mut attempt = 1usize;

    loop {
        match operation().await {
            Ok(mut response) => {
                response.usage = apply_retry_attempt_usage(response.usage, Usage::default());
                return Ok(response);
            }
            Err(error) => {
                let Some(settings) = retry_settings.as_ref() else {
                    return Err(error);
                };
                if attempt > max_retries {
                    return Err(error);
                }
                let decision = evaluate_retry(
                    error.clone(),
                    attempt,
                    max_retries,
                    settings,
                    previous_response_id.as_deref(),
                    conversation_id.as_deref(),
                )
                .await;
                if !decision.retry {
                    return Err(error);
                }

                sleep_for_retry(decision.delay.unwrap_or_default()).await;
                attempt += 1;
            }
        }
    }
}

async fn evaluate_retry(
    error: AgentsError,
    attempt: usize,
    max_retries: usize,
    settings: &ModelRetrySettings,
    previous_response_id: Option<&str>,
    conversation_id: Option<&str>,
) -> RetryDecision {
    let normalized = normalize_retry_error(&error);
    if normalized.is_abort {
        return RetryDecision::stop("abort-like error");
    }

    let Some(policy) = &settings.policy else {
        return RetryDecision::stop("no retry policy configured");
    };
    let mut decision = policy(RetryPolicyContext {
        error_message: Some(error.to_string()),
        attempt,
        max_retries,
        stream: false,
        normalized: normalized.clone(),
        provider_advice: None,
    })
    .await;
    if !decision.retry {
        return decision;
    }

    if previous_response_id.is_some() || conversation_id.is_some() {
        decision.retry = false;
        decision.reason = decision
            .reason
            .or_else(|| Some("stateful model requests are not automatically replayed".to_owned()));
        return decision;
    }

    if decision.delay.is_none() {
        decision.delay = normalized
            .retry_after
            .or_else(|| Some(default_retry_delay(attempt, settings)));
    }
    decision
}

fn normalize_retry_error(error: &AgentsError) -> ModelRetryNormalizedError {
    let message = error.to_string();
    let lower = message.to_lowercase();
    ModelRetryNormalizedError {
        status_code: parse_status_code(&lower),
        error_code: None,
        message: Some(message),
        request_id: None,
        retry_after: None,
        is_abort: lower.contains("cancel") || lower.contains("abort"),
        is_network_error: lower.contains("network")
            || lower.contains("connection")
            || lower.contains("socket")
            || lower.contains("transport"),
        is_timeout: lower.contains("timeout") || lower.contains("timed out"),
    }
}

fn parse_status_code(message: &str) -> Option<u16> {
    [408, 409, 429, 500, 502, 503, 504]
        .into_iter()
        .find(|status| message.contains(&status.to_string()))
}

fn default_retry_delay(attempt: usize, settings: &ModelRetrySettings) -> f64 {
    let backoff = settings.backoff.as_ref();
    let initial_delay = backoff
        .and_then(|value| value.initial_delay)
        .unwrap_or(DEFAULT_INITIAL_DELAY_SECONDS);
    let max_delay = backoff
        .and_then(|value| value.max_delay)
        .unwrap_or(DEFAULT_MAX_DELAY_SECONDS);
    let multiplier = backoff
        .and_then(|value| value.multiplier)
        .unwrap_or(DEFAULT_BACKOFF_MULTIPLIER);

    (initial_delay * multiplier.powi((attempt.saturating_sub(1)) as i32)).min(max_delay)
}

async fn sleep_for_retry(delay: f64) {
    if delay <= 0.0 {
        return;
    }
    tokio::time::sleep(Duration::from_secs_f64(delay)).await;
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use crate::retry::{ModelRetryBackoffSettings, retry_policies};

    use super::*;

    #[tokio::test]
    async fn retries_model_response_when_policy_allows() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let retry_settings = ModelRetrySettings {
            max_retries: Some(2),
            backoff: Some(ModelRetryBackoffSettings {
                initial_delay: Some(0.0),
                max_delay: Some(0.0),
                multiplier: Some(1.0),
                jitter: Some(false),
            }),
            policy: Some(retry_policies::network_error()),
        };

        let response = get_response_with_retry(
            || {
                let attempts = attempts.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt == 1 {
                        return Err(AgentsError::message("network error"));
                    }

                    Ok(ModelResponse {
                        response_id: Some("resp-ok".to_owned()),
                        ..ModelResponse::default()
                    })
                }
            },
            Some(retry_settings),
            None,
            None,
        )
        .await
        .expect("retry should recover");

        assert_eq!(response.response_id.as_deref(), Some("resp-ok"));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn does_not_replay_stateful_requests_without_replay_approval() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let retry_settings = ModelRetrySettings {
            max_retries: Some(2),
            backoff: Some(ModelRetryBackoffSettings {
                initial_delay: Some(0.0),
                max_delay: Some(0.0),
                multiplier: Some(1.0),
                jitter: Some(false),
            }),
            policy: Some(retry_policies::network_error()),
        };

        let error = get_response_with_retry(
            || {
                let attempts = attempts.clone();
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err(AgentsError::message("network error"))
                }
            },
            Some(retry_settings),
            Some("resp-stateful".to_owned()),
            None,
        )
        .await
        .expect_err("stateful request should not replay");

        assert!(error.to_string().contains("network error"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
