pub use crate::models::interface::{
    Model, ModelProvider, ModelRequest, ModelResponse, ModelTracing,
};

use std::env;

use crate::model_settings::{ModelSettings, ReasoningSettings};

pub const OPENAI_DEFAULT_MODEL_ENV_VARIABLE_NAME: &str = "OPENAI_DEFAULT_MODEL";

pub fn get_default_model() -> String {
    env::var(OPENAI_DEFAULT_MODEL_ENV_VARIABLE_NAME)
        .unwrap_or_else(|_| "gpt-5.4-mini".to_owned())
        .to_lowercase()
}

fn is_versioned_default_model(model_name: &str, base: &str) -> bool {
    model_name == base
        || model_name
            .strip_prefix(&format!("{base}-"))
            .is_some_and(|suffix| {
                let parts = suffix.split('-').collect::<Vec<_>>();
                parts.len() == 3
                    && parts.iter().all(|part| !part.is_empty())
                    && parts
                        .iter()
                        .all(|part| part.chars().all(|character| character.is_ascii_digit()))
            })
}

fn get_default_reasoning_effort(model_name: &str) -> Option<&'static str> {
    if is_versioned_default_model(model_name, "gpt-5") {
        return Some("low");
    }
    if is_versioned_default_model(model_name, "gpt-5.1")
        || is_versioned_default_model(model_name, "gpt-5.2")
        || is_versioned_default_model(model_name, "gpt-5.4")
        || is_versioned_default_model(model_name, "gpt-5.4-mini")
        || is_versioned_default_model(model_name, "gpt-5.4-nano")
        || is_versioned_default_model(model_name, "gpt-5.5")
        || model_name == "gpt-5.3-codex"
    {
        return Some("none");
    }
    if is_versioned_default_model(model_name, "gpt-5.2-pro")
        || is_versioned_default_model(model_name, "gpt-5.4-pro")
    {
        return Some("medium");
    }
    if model_name == "gpt-5.2-codex" {
        return Some("low");
    }
    None
}

pub fn gpt_5_reasoning_settings_required(model_name: &str) -> bool {
    let normalized = model_name.to_lowercase();
    if matches!(
        normalized.as_str(),
        "gpt-5-chat-latest" | "gpt-5.1-chat-latest" | "gpt-5.2-chat-latest" | "gpt-5.3-chat-latest"
    ) {
        return false;
    }
    normalized.starts_with("gpt-5")
}

pub fn is_gpt_5_default() -> bool {
    gpt_5_reasoning_settings_required(&get_default_model())
}

pub fn get_default_model_settings(model: Option<&str>) -> ModelSettings {
    let resolved_model = model.map(str::to_owned).unwrap_or_else(get_default_model);
    let normalized = resolved_model.to_lowercase();
    if gpt_5_reasoning_settings_required(&normalized) {
        return ModelSettings {
            verbosity: Some("low".to_owned()),
            reasoning: get_default_reasoning_effort(&normalized).map(|effort| ReasoningSettings {
                effort: Some(effort.to_owned()),
                summary: None,
            }),
            ..ModelSettings::default()
        };
    }

    ModelSettings::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn default_model_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn default_model_is_current_mini_model() {
        let _lock = default_model_env_lock()
            .lock()
            .expect("default model env lock");
        let _env = EnvVarGuard::unset(OPENAI_DEFAULT_MODEL_ENV_VARIABLE_NAME);

        assert_eq!(get_default_model(), "gpt-5.4-mini");
        assert!(is_gpt_5_default());
        let settings = get_default_model_settings(None);
        assert_eq!(settings.verbosity.as_deref(), Some("low"));
        assert_eq!(
            settings
                .reasoning
                .as_ref()
                .and_then(|value| value.effort.as_deref()),
            Some("none")
        );
    }

    #[test]
    fn detects_gpt5_reasoning_requirement() {
        assert!(gpt_5_reasoning_settings_required("gpt-5"));
        assert!(!gpt_5_reasoning_settings_required("gpt-5-chat-latest"));
        assert!(!gpt_5_reasoning_settings_required("gpt-4.1"));
    }

    #[test]
    fn returns_gpt5_defaults() {
        let settings = get_default_model_settings(Some("gpt-5.4"));
        assert_eq!(settings.verbosity.as_deref(), Some("low"));
        assert_eq!(
            settings
                .reasoning
                .as_ref()
                .and_then(|value| value.effort.as_deref()),
            Some("none")
        );
    }

    #[test]
    fn returns_gpt55_none_reasoning_defaults() {
        let settings = get_default_model_settings(Some("gpt-5.5-2026-04-23"));
        assert_eq!(settings.verbosity.as_deref(), Some("low"));
        assert_eq!(
            settings
                .reasoning
                .as_ref()
                .and_then(|value| value.effort.as_deref()),
            Some("none")
        );
    }

    #[test]
    fn keeps_gpt5_text_only_defaults_for_unknown_variant() {
        let settings = get_default_model_settings(Some("gpt-5-ultra"));
        assert_eq!(settings.verbosity.as_deref(), Some("low"));
        assert!(settings.reasoning.is_none());
    }
}
