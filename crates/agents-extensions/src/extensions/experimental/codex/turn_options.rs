use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Per-turn options for Codex execution.
#[derive(Clone, Debug, Default)]
pub struct TurnOptions {
    pub output_schema: Option<Value>,
    pub signal: Option<Arc<AtomicBool>>,
    pub idle_timeout_seconds: Option<f64>,
}

pub fn coerce_turn_options(options: Option<TurnOptions>) -> Option<TurnOptions> {
    options
}
