use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Process-level configuration for the Codex CLI integration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexOptions {
    pub codex_path_override: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub codex_subprocess_stream_limit_bytes: Option<usize>,
}

pub fn coerce_codex_options(options: Option<CodexOptions>) -> Option<CodexOptions> {
    options
}
