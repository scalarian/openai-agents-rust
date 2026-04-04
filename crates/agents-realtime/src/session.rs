use serde::{Deserialize, Serialize};

use crate::events::{RealtimeEvent, RealtimeTranscriptDeltaEvent};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RealtimeSession {
    pub model: Option<String>,
    pub connected: bool,
    transcript: String,
    events: Vec<RealtimeEvent>,
}

impl RealtimeSession {
    pub fn new(model: Option<String>) -> Self {
        Self {
            model,
            connected: false,
            transcript: String::new(),
            events: Vec::new(),
        }
    }

    pub fn connect(mut self) -> Self {
        self.connected = true;
        self
    }

    pub fn push_transcript_delta(&mut self, text: impl Into<String>) -> RealtimeEvent {
        let text = text.into();
        self.transcript.push_str(&text);
        let event = RealtimeEvent::TranscriptDelta(RealtimeTranscriptDeltaEvent { text });
        self.events.push(event.clone());
        event
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    pub fn events(&self) -> &[RealtimeEvent] {
        &self.events
    }
}
