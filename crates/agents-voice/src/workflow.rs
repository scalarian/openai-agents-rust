use std::future::Future;
use std::sync::Arc;

use agents_core::Result;

use crate::events::VoiceStreamEvent;
use crate::input::AudioInput;

pub trait VoiceWorkflowBase: Send + Sync {
    fn run<'a>(
        &'a self,
        input: AudioInput,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<VoiceStreamEvent>>> + Send + 'a>>;
}

#[derive(Clone, Default)]
pub struct SingleAgentWorkflowCallbacks;

pub struct VoiceWorkflowHelper;

impl VoiceWorkflowHelper {
    pub fn lifecycle(event: impl Into<String>) -> VoiceStreamEvent {
        VoiceStreamEvent::Lifecycle(crate::events::VoiceStreamEventLifecycle {
            event: event.into(),
        })
    }
}

#[derive(Clone)]
pub struct SingleAgentVoiceWorkflow<F> {
    handler: Arc<F>,
}

impl<F> SingleAgentVoiceWorkflow<F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }
}

impl<F, Fut> VoiceWorkflowBase for SingleAgentVoiceWorkflow<F>
where
    F: Fn(AudioInput) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<VoiceStreamEvent>>> + Send + 'static,
{
    fn run<'a>(
        &'a self,
        input: AudioInput,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<VoiceStreamEvent>>> + Send + 'a>> {
        Box::pin((self.handler)(input))
    }
}
