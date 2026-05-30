use agents_core::Handoff;

use crate::agent::RealtimeAgent;

pub trait IntoRealtimeHandoff {
    fn into_realtime_handoff(self) -> Handoff;
}

impl IntoRealtimeHandoff for Handoff {
    fn into_realtime_handoff(self) -> Handoff {
        self
    }
}

impl IntoRealtimeHandoff for RealtimeAgent {
    fn into_realtime_handoff(self) -> Handoff {
        let description = self
            .handoff_description
            .clone()
            .or_else(|| self.instructions.clone())
            .unwrap_or_default();
        Handoff::new(self.name).with_description(description)
    }
}

pub fn realtime_handoff(target: impl IntoRealtimeHandoff) -> Handoff {
    target.into_realtime_handoff()
}

pub fn realtime_handoff_with_tool_name(
    agent: RealtimeAgent,
    tool_name: impl Into<String>,
) -> Handoff {
    realtime_handoff(agent).with_tool_name(tool_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realtime_handoff_accepts_realtime_agent() {
        let handoff = realtime_handoff_with_tool_name(
            RealtimeAgent::new("FAQ Agent")
                .with_handoff_description("Answers airline FAQs.")
                .with_instructions("Use the FAQ database."),
            "transfer_to_faq_agent",
        );

        assert_eq!(handoff.target, "FAQ Agent");
        assert_eq!(handoff.tool_name, "transfer_to_faq_agent");
        assert_eq!(
            handoff.description.as_deref(),
            Some("Answers airline FAQs.")
        );
    }
}
