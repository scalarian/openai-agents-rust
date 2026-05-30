use agents_core::Result;

use crate::agent::RealtimeAgent;
use crate::config::RealtimeRunConfig;
use crate::events::RealtimeEvent;
use crate::model::RealtimeModelConfig;
use crate::openai_realtime::{OpenAIRealtimeWebSocketModel, TransportConfig};
use crate::session::RealtimeSession;

#[derive(Clone, Debug, Default)]
pub struct RealtimeRunner {
    agent: RealtimeAgent,
    config: RealtimeRunConfig,
}

impl RealtimeRunner {
    pub fn new(agent: RealtimeAgent) -> Self {
        Self {
            agent,
            config: RealtimeRunConfig::default(),
        }
    }

    pub fn with_config(mut self, config: RealtimeRunConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn run(&self) -> Result<RealtimeSession> {
        let mut effective_agent = self.agent.clone();
        effective_agent.model_settings = Some(self.model_settings_for_agent(&effective_agent));
        let model_name = self
            .config
            .model_settings
            .as_ref()
            .and_then(|settings| settings.model_name.clone())
            .or_else(|| {
                self.agent
                    .model_settings
                    .as_ref()
                    .and_then(|settings| settings.model_name.clone())
            });
        let session = RealtimeSession::new(model_name.clone());
        session
            .attach_model_driver(Box::new(OpenAIRealtimeWebSocketModel {
                config: RealtimeModelConfig { model: model_name },
                transport: TransportConfig::default(),
                connected: false,
                last_connection_url: None,
                last_session_payload: None,
                applied_settings: None,
            }))
            .await;
        session.connect(Some(effective_agent.clone())).await?;
        session.update_agent(effective_agent).await?;
        Ok(session)
    }

    fn model_settings_for_agent(
        &self,
        agent: &RealtimeAgent,
    ) -> crate::config::RealtimeSessionModelSettings {
        let mut settings = agent.model_settings.clone().unwrap_or_default();
        if settings.instructions.is_none() {
            settings.instructions = agent.instructions.clone();
        }

        let session_tools = agent.session_tools();
        if settings.tools.is_none() {
            settings.tools = Some(session_tools);
        }

        if let Some(update) = &self.config.model_settings {
            settings = settings.merge(update);
        }

        settings.normalize_effective()
    }

    pub async fn run_text_turn(
        &self,
        session: &RealtimeSession,
        text: &str,
    ) -> Result<RealtimeEvent> {
        let mut events = session.send_text(text).await?;
        Ok(events
            .iter()
            .find(|event| matches!(event, RealtimeEvent::TranscriptDelta(_)))
            .cloned()
            .or_else(|| events.pop())
            .unwrap_or_else(|| {
                RealtimeEvent::TranscriptDelta(crate::events::RealtimeTranscriptDeltaEvent {
                    text: text.to_owned(),
                })
            }))
    }
}
