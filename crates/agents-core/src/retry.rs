use std::sync::Arc;

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize, de};

#[derive(Clone, Debug, Default, Serialize)]
pub struct ModelRetryBackoffSettings {
    pub initial_delay: Option<f64>,
    pub max_delay: Option<f64>,
    pub multiplier: Option<f64>,
    pub jitter: Option<bool>,
}

impl ModelRetryBackoffSettings {
    pub fn validate(&self) -> std::result::Result<(), String> {
        validate_non_negative("initial_delay", self.initial_delay)?;
        validate_non_negative("max_delay", self.max_delay)?;
        validate_non_negative("multiplier", self.multiplier)?;
        Ok(())
    }
}

impl<'de> Deserialize<'de> for ModelRetryBackoffSettings {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawModelRetryBackoffSettings {
            initial_delay: Option<f64>,
            max_delay: Option<f64>,
            multiplier: Option<f64>,
            jitter: Option<bool>,
        }

        let raw = RawModelRetryBackoffSettings::deserialize(deserializer)?;
        let settings = Self {
            initial_delay: raw.initial_delay,
            max_delay: raw.max_delay,
            multiplier: raw.multiplier,
            jitter: raw.jitter,
        };
        settings.validate().map_err(de::Error::custom)?;
        Ok(settings)
    }
}

fn validate_non_negative(field: &str, value: Option<f64>) -> std::result::Result<(), String> {
    if value.is_some_and(|number| number < 0.0) {
        return Err(format!("{field} must be greater than or equal to 0"));
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelRetryNormalizedError {
    pub status_code: Option<u16>,
    pub error_code: Option<String>,
    pub message: Option<String>,
    pub request_id: Option<String>,
    pub retry_after: Option<f64>,
    pub is_abort: bool,
    pub is_network_error: bool,
    pub is_timeout: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelRetryAdvice {
    pub suggested: Option<bool>,
    pub retry_after: Option<f64>,
    pub replay_safety: Option<String>,
    pub reason: Option<String>,
    pub normalized: Option<ModelRetryNormalizedError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRetryAdviceRequest {
    pub attempt: usize,
    pub stream: bool,
    pub previous_response_id: Option<String>,
    pub conversation_id: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RetryDecision {
    pub retry: bool,
    pub delay: Option<f64>,
    pub reason: Option<String>,
}

impl RetryDecision {
    pub fn retry(delay: Option<f64>, reason: impl Into<String>) -> Self {
        Self {
            retry: true,
            delay,
            reason: Some(reason.into()),
        }
    }

    pub fn stop(reason: impl Into<String>) -> Self {
        Self {
            retry: false,
            delay: None,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RetryPolicyContext {
    pub error_message: Option<String>,
    pub attempt: usize,
    pub max_retries: usize,
    pub stream: bool,
    pub normalized: ModelRetryNormalizedError,
    pub provider_advice: Option<ModelRetryAdvice>,
}

pub type RetryPolicy =
    Arc<dyn Fn(RetryPolicyContext) -> BoxFuture<'static, RetryDecision> + Send + Sync>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelRetrySettings {
    pub max_retries: Option<usize>,
    pub backoff: Option<ModelRetryBackoffSettings>,
}

pub fn retry_policies() -> Vec<RetryPolicy> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn model_retry_backoff_settings_reject_negative_deserialized_values() {
        for (field, value) in [
            ("initial_delay", json!({"initial_delay": -0.1})),
            ("max_delay", json!({"max_delay": -0.1})),
            ("multiplier", json!({"multiplier": -0.1})),
        ] {
            let error = serde_json::from_value::<ModelRetryBackoffSettings>(value)
                .expect_err("negative backoff value should be rejected");

            assert!(
                error.to_string().contains(field),
                "error should identify {field}: {error}"
            );
            assert!(
                error.to_string().contains("greater than or equal to 0"),
                "error should explain lower bound: {error}"
            );
        }
    }

    #[test]
    fn model_retry_settings_reject_negative_backoff_dict() {
        let error = serde_json::from_value::<ModelRetrySettings>(
            json!({"backoff": {"initial_delay": -0.1}}),
        )
        .expect_err("negative nested backoff value should be rejected");

        assert!(error.to_string().contains("initial_delay"));
        assert!(error.to_string().contains("greater than or equal to 0"));
    }

    #[test]
    fn model_retry_backoff_settings_allow_zero_values() {
        let settings = serde_json::from_value::<ModelRetryBackoffSettings>(json!({
            "initial_delay": 0.0,
            "max_delay": 0.0,
            "multiplier": 0.0,
            "jitter": false
        }))
        .expect("zero backoff values should be accepted");

        assert_eq!(settings.initial_delay, Some(0.0));
        assert_eq!(settings.max_delay, Some(0.0));
        assert_eq!(settings.multiplier, Some(0.0));
        assert_eq!(settings.jitter, Some(false));
        settings.validate().expect("zero settings should validate");
    }
}
