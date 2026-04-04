use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct STTWebsocketConnectionError {
    pub message: String,
}

impl Display for STTWebsocketConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for STTWebsocketConnectionError {}
