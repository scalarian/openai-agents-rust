use std::sync::RwLock;

use once_cell::sync::Lazy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpenAIApi {
    ChatCompletions,
    #[default]
    Responses,
}

#[derive(Clone, Debug, Default)]
struct OpenAISettings {
    api_key: Option<String>,
    tracing_export_api_key: Option<String>,
    api: OpenAIApi,
}

static SETTINGS: Lazy<RwLock<OpenAISettings>> =
    Lazy::new(|| RwLock::new(OpenAISettings::default()));

pub fn set_default_openai_key(key: impl Into<String>) {
    SETTINGS.write().expect("openai defaults lock").api_key = Some(key.into());
}

pub fn default_openai_key() -> Option<String> {
    SETTINGS
        .read()
        .expect("openai defaults lock")
        .api_key
        .clone()
}

pub fn set_tracing_export_api_key(key: impl Into<String>) {
    SETTINGS
        .write()
        .expect("openai defaults lock")
        .tracing_export_api_key = Some(key.into());
}

pub fn tracing_export_api_key() -> Option<String> {
    SETTINGS
        .read()
        .expect("openai defaults lock")
        .tracing_export_api_key
        .clone()
}

pub fn set_default_openai_api(api: OpenAIApi) {
    SETTINGS.write().expect("openai defaults lock").api = api;
}

pub fn default_openai_api() -> OpenAIApi {
    SETTINGS.read().expect("openai defaults lock").api
}
