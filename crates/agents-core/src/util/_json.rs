use serde::de::DeserializeOwned;

use crate::errors::Result;
use crate::exceptions::ModelBehaviorError;
use crate::tracing::SpanError;
use crate::util::attach_error_to_current_span;

pub fn validate_json<T>(json_str: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json_str).map_err(|error| {
        attach_error_to_current_span(SpanError {
            message: "Invalid JSON provided".to_owned(),
            data: None,
        });
        ModelBehaviorError {
            message: format!("Invalid JSON when parsing `{json_str}`: {error}"),
        }
        .into()
    })
}
