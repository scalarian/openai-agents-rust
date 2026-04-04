use thiserror::Error;

/// Errors produced by the Rust Agents SDK scaffold.
#[derive(Debug, Error)]
pub enum AgentsError {
    #[error("model provider is not configured")]
    ModelProviderNotConfigured,
    #[error("model provider resolved no model")]
    ModelUnavailable,
    #[error("{message}")]
    Message { message: String },
}

impl AgentsError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AgentsError>;
