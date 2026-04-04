use agents_core::Result;

use crate::agent::RealtimeAgent;
use crate::events::RealtimeEvent;
use crate::session::RealtimeSession;

#[derive(Clone, Debug)]
pub struct RealtimeRunner {
    agent: RealtimeAgent,
}

impl RealtimeRunner {
    pub fn new(agent: RealtimeAgent) -> Self {
        Self { agent }
    }

    pub async fn run_text_turn(
        &self,
        session: &mut RealtimeSession,
        text: &str,
    ) -> Result<RealtimeEvent> {
        let _ = &self.agent;
        Ok(session.push_transcript_delta(text))
    }
}
