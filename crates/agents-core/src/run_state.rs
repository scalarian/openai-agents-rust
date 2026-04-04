use serde::{Deserialize, Serialize};

/// Mutable state tracked across a run.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunState {
    pub turns: usize,
    pub interrupted: bool,
}
