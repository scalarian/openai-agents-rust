use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeAudioFormat {
    Pcm16,
    G711Ulaw,
    G711Alaw,
    Custom(String),
}

impl Default for RealtimeAudioFormat {
    fn default() -> Self {
        Self::Pcm16
    }
}

pub fn to_realtime_audio_format(value: impl AsRef<str>) -> RealtimeAudioFormat {
    match value.as_ref() {
        "pcm16" => RealtimeAudioFormat::Pcm16,
        "g711_ulaw" => RealtimeAudioFormat::G711Ulaw,
        "g711_alaw" => RealtimeAudioFormat::G711Alaw,
        other => RealtimeAudioFormat::Custom(other.to_owned()),
    }
}
