use std::env;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const OPENAI_AGENT_HARNESS_ID_ENV_VAR: &str = "OPENAI_AGENT_HARNESS_ID";
pub const OPENAI_HARNESS_ID_TRACE_METADATA_KEY: &str = "agent_harness_id";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenAIAgentRegistrationConfig {
    pub harness_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedOpenAIAgentRegistrationConfig {
    pub harness_id: String,
}

static DEFAULT_AGENT_REGISTRATION: Lazy<RwLock<Option<OpenAIAgentRegistrationConfig>>> =
    Lazy::new(|| RwLock::new(None));

pub fn set_default_openai_agent_registration(config: Option<OpenAIAgentRegistrationConfig>) {
    *DEFAULT_AGENT_REGISTRATION
        .write()
        .expect("openai agent registration defaults lock") = config;
}

pub fn get_default_openai_agent_registration() -> Option<OpenAIAgentRegistrationConfig> {
    DEFAULT_AGENT_REGISTRATION
        .read()
        .expect("openai agent registration defaults lock")
        .clone()
}

pub fn set_default_openai_harness<T: Into<String>>(harness_id: Option<T>) {
    set_default_openai_agent_registration(harness_id.map(|harness_id| {
        OpenAIAgentRegistrationConfig {
            harness_id: Some(harness_id.into()),
        }
    }));
}

pub fn resolve_openai_agent_registration_config(
    config: Option<&OpenAIAgentRegistrationConfig>,
) -> Option<ResolvedOpenAIAgentRegistrationConfig> {
    let default = get_default_openai_agent_registration();
    let harness_id = resolve_non_empty_str(
        config.and_then(|config| config.harness_id.as_deref()),
        default
            .as_ref()
            .and_then(|config| config.harness_id.as_deref()),
        env::var(OPENAI_AGENT_HARNESS_ID_ENV_VAR).ok().as_deref(),
    )?;
    Some(ResolvedOpenAIAgentRegistrationConfig { harness_id })
}

pub fn merge_openai_harness_id_into_metadata(
    metadata: Option<&BTreeMap<String, Value>>,
    harness_id: Option<&str>,
) -> Option<BTreeMap<String, Value>> {
    let Some(harness_id) = harness_id.and_then(non_empty_owned) else {
        return metadata.cloned();
    };
    if metadata.is_some_and(|metadata| metadata.contains_key(OPENAI_HARNESS_ID_TRACE_METADATA_KEY))
    {
        return metadata.cloned();
    }
    let mut merged = metadata.cloned().unwrap_or_default();
    merged.insert(
        OPENAI_HARNESS_ID_TRACE_METADATA_KEY.to_owned(),
        Value::String(harness_id),
    );
    Some(merged)
}

fn resolve_non_empty_str(
    explicit: Option<&str>,
    default: Option<&str>,
    env_value: Option<&str>,
) -> Option<String> {
    explicit
        .and_then(non_empty_owned)
        .or_else(|| default.and_then(non_empty_owned))
        .or_else(|| env_value.and_then(non_empty_owned))
}

fn non_empty_owned(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
pub(crate) fn agent_registration_test_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DefaultHarnessReset(Option<OpenAIAgentRegistrationConfig>);

    impl Drop for DefaultHarnessReset {
        fn drop(&mut self) {
            set_default_openai_agent_registration(self.0.clone());
        }
    }

    struct EnvHarnessReset(Option<String>);

    impl Drop for EnvHarnessReset {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => unsafe { env::set_var(OPENAI_AGENT_HARNESS_ID_ENV_VAR, value) },
                None => unsafe { env::remove_var(OPENAI_AGENT_HARNESS_ID_ENV_VAR) },
            }
        }
    }

    #[test]
    fn resolves_agent_registration_with_explicit_default_and_env_precedence() {
        let _guard = agent_registration_test_lock().lock().expect("test lock");
        let _default_reset = DefaultHarnessReset(get_default_openai_agent_registration());
        let _env_reset = EnvHarnessReset(env::var(OPENAI_AGENT_HARNESS_ID_ENV_VAR).ok());

        unsafe { env::set_var(OPENAI_AGENT_HARNESS_ID_ENV_VAR, "env-harness") };
        set_default_openai_agent_registration(Some(OpenAIAgentRegistrationConfig {
            harness_id: Some("default-harness".to_owned()),
        }));

        let explicit =
            resolve_openai_agent_registration_config(Some(&OpenAIAgentRegistrationConfig {
                harness_id: Some("explicit-harness".to_owned()),
            }))
            .expect("explicit config should resolve");
        assert_eq!(explicit.harness_id, "explicit-harness");

        let defaulted =
            resolve_openai_agent_registration_config(None).expect("default config should resolve");
        assert_eq!(defaulted.harness_id, "default-harness");

        set_default_openai_agent_registration(None);
        let from_env =
            resolve_openai_agent_registration_config(None).expect("env config should resolve");
        assert_eq!(from_env.harness_id, "env-harness");
    }
}
