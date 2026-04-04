use std::future::Future;

use crate::errors::Result;
use crate::model::ModelResponse;
use crate::usage::Usage;

pub(crate) fn apply_retry_attempt_usage(previous: Usage, next: Usage) -> Usage {
    crate::internal::agent_runner_helpers::merge_usage(previous, next)
}

pub(crate) async fn get_response_with_retry<F, Fut>(operation: F) -> Result<ModelResponse>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<ModelResponse>>,
{
    operation().await
}
