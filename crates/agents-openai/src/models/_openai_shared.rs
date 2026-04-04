use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::defaults::{
    OPENAI_DEFAULT_BASE_URL, default_openai_key, default_openai_websocket_base_url,
    set_default_openai_key,
};

#[derive(Clone, Debug, Default)]
struct SharedState {
    client: Option<reqwest::Client>,
    use_responses_by_default: Option<bool>,
    use_responses_websocket_by_default: Option<bool>,
    base_url: Option<String>,
    websocket_base_url: Option<String>,
}

static SHARED_STATE: Lazy<RwLock<SharedState>> = Lazy::new(|| RwLock::new(SharedState::default()));

pub fn set_default_openai_client(client: reqwest::Client) {
    SHARED_STATE.write().expect("openai shared lock").client = Some(client);
}

pub fn get_default_openai_client() -> Option<reqwest::Client> {
    SHARED_STATE
        .read()
        .expect("openai shared lock")
        .client
        .clone()
}

pub fn set_default_openai_key_shared(key: impl Into<String>) {
    set_default_openai_key(key);
}

pub fn get_default_openai_key() -> Option<String> {
    default_openai_key()
}

pub fn set_use_responses_by_default(value: bool) {
    SHARED_STATE
        .write()
        .expect("openai shared lock")
        .use_responses_by_default = Some(value);
}

pub fn get_use_responses_by_default() -> Option<bool> {
    SHARED_STATE
        .read()
        .expect("openai shared lock")
        .use_responses_by_default
}

pub fn set_use_responses_websocket_by_default(value: bool) {
    SHARED_STATE
        .write()
        .expect("openai shared lock")
        .use_responses_websocket_by_default = Some(value);
}

pub fn get_use_responses_websocket_by_default() -> Option<bool> {
    SHARED_STATE
        .read()
        .expect("openai shared lock")
        .use_responses_websocket_by_default
}

pub fn set_openai_base_url(base_url: impl Into<String>) {
    SHARED_STATE.write().expect("openai shared lock").base_url = Some(base_url.into());
}

pub fn get_openai_base_url() -> String {
    SHARED_STATE
        .read()
        .expect("openai shared lock")
        .base_url
        .clone()
        .unwrap_or_else(|| OPENAI_DEFAULT_BASE_URL.to_owned())
}

pub fn set_default_openai_websocket_base_url(base_url: impl Into<String>) {
    SHARED_STATE
        .write()
        .expect("openai shared lock")
        .websocket_base_url = Some(base_url.into());
}

pub fn get_default_openai_websocket_base_url() -> String {
    SHARED_STATE
        .read()
        .expect("openai shared lock")
        .websocket_base_url
        .clone()
        .unwrap_or_else(|| default_openai_websocket_base_url().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_default_transport_preferences() {
        set_use_responses_by_default(true);
        set_use_responses_websocket_by_default(true);

        assert_eq!(get_use_responses_by_default(), Some(true));
        assert_eq!(get_use_responses_websocket_by_default(), Some(true));
    }
}
