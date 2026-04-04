use serde::{Deserialize, Serialize};

use crate::config::RealtimeSessionModelSettings;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RealtimeAgent {
    pub name: String,
    pub instructions: Option<String>,
    pub model_settings: Option<RealtimeSessionModelSettings>,
}

impl RealtimeAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instructions: None,
            model_settings: None,
        }
    }
}

pub trait RealtimeAgentHooks: Send + Sync {}

pub trait RealtimeRunHooks: Send + Sync {}
