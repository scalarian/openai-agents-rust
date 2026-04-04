use serde::{Deserialize, Serialize};

/// Computer-use environment metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Computer {
    pub environment: Option<String>,
    pub display_name: Option<String>,
}
