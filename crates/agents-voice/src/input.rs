use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioInput {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamedAudioInput {
    pub mime_type: String,
    pub chunks: Vec<Vec<u8>>,
}
