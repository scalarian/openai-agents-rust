use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Token usage reported by a model provider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
