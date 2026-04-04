use serde::{Deserialize, Serialize};

/// Top-level SDK configuration shared across crates.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SdkConfig {
    pub app_name: Option<String>,
    pub environment: Option<String>,
    pub tracing_enabled: bool,
}
