#![allow(unused_imports, unused_macros)]

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_E2B_WORKSPACE_ROOT: &str = "/workspace";
pub const DEFAULT_MODAL_WORKSPACE_ROOT: &str = "/workspace";
pub const DEFAULT_DAYTONA_WORKSPACE_ROOT: &str = "/home/daytona/workspace";
pub const DEFAULT_BLAXEL_WORKSPACE_ROOT: &str = "/workspace";
pub const DEFAULT_CLOUDFLARE_WORKSPACE_ROOT: &str = "/workspace";
pub const DEFAULT_RUNLOOP_WORKSPACE_ROOT: &str = "/home/user";
pub const DEFAULT_VERCEL_WORKSPACE_ROOT: &str = "/vercel/sandbox";

static NEXT_HOSTED_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostedAuthKind {
    ApiKey,
    Token,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceRootPolicy {
    Strict(&'static str),
    Defaulted(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HostedProviderSpec {
    provider_name: &'static str,
    auth_kind: HostedAuthKind,
    auth_env_var: &'static str,
    workspace_root_policy: WorkspaceRootPolicy,
    supports_exposed_ports: bool,
    supports_pty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedSandboxError {
    message: String,
}

impl HostedSandboxError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HostedSandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HostedSandboxError {}

type HostedSandboxResult<T> = std::result::Result<T, HostedSandboxError>;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HostedSandboxClientOptionsBase {
    pub workspace_root: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub client_timeout_s: Option<u64>,
    pub exposed_ports: Vec<u16>,
    pub interactive_pty: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HostedSandboxSessionStateBase {
    pub session_id: String,
    pub workspace_root: String,
    pub base_url: Option<String>,
    pub exposed_ports: Vec<u16>,
    pub interactive_pty: bool,
    pub start_state_preserved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HostedResolvedAuth {
    value: String,
    source: &'static str,
}

fn next_hosted_session_id(provider_name: &str) -> String {
    let id = NEXT_HOSTED_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    format!("{provider_name}-session-{id}")
}

fn normalize_workspace_root(
    provider_name: &str,
    policy: WorkspaceRootPolicy,
    requested_root: Option<&str>,
) -> HostedSandboxResult<String> {
    match policy {
        WorkspaceRootPolicy::Strict(required_root) => match requested_root {
            Some(root) if root != required_root => Err(HostedSandboxError::new(format!(
                "{provider_name} sandboxes require workspace_root={required_root:?}, got {root:?}"
            ))),
            Some(root) => Ok(root.to_owned()),
            None => Ok(required_root.to_owned()),
        },
        WorkspaceRootPolicy::Defaulted(default_root) => {
            Ok(requested_root.unwrap_or(default_root).to_owned())
        }
    }
}

fn resolve_auth(
    provider_name: &str,
    auth_kind: HostedAuthKind,
    auth_env_var: &str,
    api_key: Option<&str>,
    token: Option<&str>,
) -> HostedSandboxResult<HostedResolvedAuth> {
    let explicit = match auth_kind {
        HostedAuthKind::ApiKey => api_key.filter(|value| !value.is_empty()),
        HostedAuthKind::Token => token.filter(|value| !value.is_empty()),
    };
    if let Some(value) = explicit {
        return Ok(HostedResolvedAuth {
            value: value.to_owned(),
            source: "explicit",
        });
    }

    if let Ok(value) = std::env::var(auth_env_var) {
        if !value.is_empty() {
            return Ok(HostedResolvedAuth {
                value,
                source: "env",
            });
        }
    }

    let auth_label = match auth_kind {
        HostedAuthKind::ApiKey => "api_key",
        HostedAuthKind::Token => "token",
    };
    Err(HostedSandboxError::new(format!(
        "{provider_name} sandboxes require {auth_label} or {auth_env_var}"
    )))
}

fn validate_capabilities(
    provider_name: &str,
    supports_exposed_ports: bool,
    supports_pty: bool,
    exposed_ports: &[u16],
    interactive_pty: bool,
) -> HostedSandboxResult<()> {
    if !supports_exposed_ports && !exposed_ports.is_empty() {
        return Err(HostedSandboxError::new(format!(
            "{provider_name} sandboxes do not support exposed ports"
        )));
    }
    if interactive_pty && !supports_pty {
        return Err(HostedSandboxError::new(format!(
            "{provider_name} sandboxes do not support interactive PTY sessions"
        )));
    }
    Ok(())
}

macro_rules! define_hosted_sandbox_provider {
    ($mod_name:ident, $client:ident, $options:ident, $session:ident, $state:ident, $spec:expr) => {
        pub mod $mod_name {
            use super::*;

            const SPEC: HostedProviderSpec = $spec;

            #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
            pub struct $options {
                pub workspace_root: Option<String>,
                pub base_url: Option<String>,
                pub api_key: Option<String>,
                pub token: Option<String>,
                pub client_timeout_s: Option<u64>,
                pub exposed_ports: Vec<u16>,
                pub interactive_pty: bool,
            }

            #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
            pub struct $state {
                pub session_id: String,
                pub workspace_root: String,
                pub base_url: Option<String>,
                pub exposed_ports: Vec<u16>,
                pub interactive_pty: bool,
                pub start_state_preserved: bool,
            }

            impl Default for $state {
                fn default() -> Self {
                    Self {
                        session_id: String::new(),
                        workspace_root: normalize_workspace_root(
                            SPEC.provider_name,
                            SPEC.workspace_root_policy,
                            None,
                        )
                        .expect("default workspace root should be valid"),
                        base_url: None,
                        exposed_ports: Vec::new(),
                        interactive_pty: false,
                        start_state_preserved: false,
                    }
                }
            }

            #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
            pub struct $client {
                pub options: $options,
            }

            impl $client {
                pub fn new(options: $options) -> Self {
                    Self { options }
                }

                pub fn options(&self) -> &$options {
                    &self.options
                }

                pub fn create(&self) -> HostedSandboxResult<$session> {
                    let auth = resolve_auth(
                        SPEC.provider_name,
                        SPEC.auth_kind,
                        SPEC.auth_env_var,
                        self.options.api_key.as_deref(),
                        self.options.token.as_deref(),
                    )?;
                    validate_capabilities(
                        SPEC.provider_name,
                        SPEC.supports_exposed_ports,
                        SPEC.supports_pty,
                        &self.options.exposed_ports,
                        self.options.interactive_pty,
                    )?;

                    let workspace_root = normalize_workspace_root(
                        SPEC.provider_name,
                        SPEC.workspace_root_policy,
                        self.options.workspace_root.as_deref(),
                    )?;

                    Ok($session {
                        state: $state {
                            session_id: next_hosted_session_id(SPEC.provider_name),
                            workspace_root,
                            base_url: self.options.base_url.clone(),
                            exposed_ports: self.options.exposed_ports.clone(),
                            interactive_pty: self.options.interactive_pty,
                            start_state_preserved: false,
                        },
                        resolved_auth: auth,
                    })
                }

                pub fn resume(&self, state: $state) -> HostedSandboxResult<$session> {
                    let auth = resolve_auth(
                        SPEC.provider_name,
                        SPEC.auth_kind,
                        SPEC.auth_env_var,
                        self.options.api_key.as_deref(),
                        self.options.token.as_deref(),
                    )?;
                    validate_capabilities(
                        SPEC.provider_name,
                        SPEC.supports_exposed_ports,
                        SPEC.supports_pty,
                        &state.exposed_ports,
                        state.interactive_pty,
                    )?;

                    let workspace_root = normalize_workspace_root(
                        SPEC.provider_name,
                        SPEC.workspace_root_policy,
                        Some(&state.workspace_root),
                    )?;

                    Ok($session {
                        state: $state {
                            session_id: state.session_id,
                            workspace_root,
                            base_url: state.base_url,
                            exposed_ports: state.exposed_ports,
                            interactive_pty: state.interactive_pty,
                            start_state_preserved: true,
                        },
                        resolved_auth: auth,
                    })
                }

                pub fn serialize_session_state(
                    &self,
                    state: &$state,
                ) -> HostedSandboxResult<serde_json::Value> {
                    serde_json::to_value(state)
                        .map_err(|error| HostedSandboxError::new(error.to_string()))
                }

                pub fn deserialize_session_state(
                    &self,
                    payload: serde_json::Value,
                ) -> HostedSandboxResult<$state> {
                    serde_json::from_value(payload)
                        .map_err(|error| HostedSandboxError::new(error.to_string()))
                }
            }

            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct $session {
                pub state: $state,
                resolved_auth: HostedResolvedAuth,
            }

            impl Default for $session {
                fn default() -> Self {
                    Self {
                        state: $state::default(),
                        resolved_auth: HostedResolvedAuth {
                            value: String::new(),
                            source: "explicit",
                        },
                    }
                }
            }

            impl $session {
                pub fn new(state: $state) -> Self {
                    Self {
                        state,
                        resolved_auth: HostedResolvedAuth {
                            value: String::new(),
                            source: "explicit",
                        },
                    }
                }

                pub fn state(&self) -> &$state {
                    &self.state
                }

                pub fn resolved_auth_source(&self) -> &'static str {
                    self.resolved_auth.source
                }

                pub fn resolved_auth_value(&self) -> &str {
                    &self.resolved_auth.value
                }

                pub fn supports_pty(&self) -> bool {
                    SPEC.supports_pty && self.state.interactive_pty
                }
            }
        }

        pub use $mod_name::{$client, $options, $session, $state};
    };
}

define_hosted_sandbox_provider!(
    e2b,
    E2BSandboxClient,
    E2BSandboxClientOptions,
    E2BSandboxSession,
    E2BSandboxSessionState,
    HostedProviderSpec {
        provider_name: "e2b",
        auth_kind: HostedAuthKind::ApiKey,
        auth_env_var: "E2B_API_KEY",
        workspace_root_policy: WorkspaceRootPolicy::Defaulted(DEFAULT_E2B_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: true,
    }
);
define_hosted_sandbox_provider!(
    modal,
    ModalSandboxClient,
    ModalSandboxClientOptions,
    ModalSandboxSession,
    ModalSandboxSessionState,
    HostedProviderSpec {
        provider_name: "modal",
        auth_kind: HostedAuthKind::Token,
        auth_env_var: "MODAL_TOKEN_ID",
        workspace_root_policy: WorkspaceRootPolicy::Defaulted(DEFAULT_MODAL_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: false,
    }
);
define_hosted_sandbox_provider!(
    daytona,
    DaytonaSandboxClient,
    DaytonaSandboxClientOptions,
    DaytonaSandboxSession,
    DaytonaSandboxSessionState,
    HostedProviderSpec {
        provider_name: "daytona",
        auth_kind: HostedAuthKind::ApiKey,
        auth_env_var: "DAYTONA_API_KEY",
        workspace_root_policy: WorkspaceRootPolicy::Defaulted(DEFAULT_DAYTONA_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: true,
    }
);
define_hosted_sandbox_provider!(
    blaxel,
    BlaxelSandboxClient,
    BlaxelSandboxClientOptions,
    BlaxelSandboxSession,
    BlaxelSandboxSessionState,
    HostedProviderSpec {
        provider_name: "blaxel",
        auth_kind: HostedAuthKind::Token,
        auth_env_var: "BL_API_KEY",
        workspace_root_policy: WorkspaceRootPolicy::Defaulted(DEFAULT_BLAXEL_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: true,
    }
);
define_hosted_sandbox_provider!(
    cloudflare,
    CloudflareSandboxClient,
    CloudflareSandboxClientOptions,
    CloudflareSandboxSession,
    CloudflareSandboxSessionState,
    HostedProviderSpec {
        provider_name: "cloudflare",
        auth_kind: HostedAuthKind::ApiKey,
        auth_env_var: "CLOUDFLARE_SANDBOX_API_KEY",
        workspace_root_policy: WorkspaceRootPolicy::Strict(DEFAULT_CLOUDFLARE_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: true,
    }
);
define_hosted_sandbox_provider!(
    runloop,
    RunloopSandboxClient,
    RunloopSandboxClientOptions,
    RunloopSandboxSession,
    RunloopSandboxSessionState,
    HostedProviderSpec {
        provider_name: "runloop",
        auth_kind: HostedAuthKind::ApiKey,
        auth_env_var: "RUNLOOP_API_KEY",
        workspace_root_policy: WorkspaceRootPolicy::Defaulted(DEFAULT_RUNLOOP_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: false,
    }
);
define_hosted_sandbox_provider!(
    vercel,
    VercelSandboxClient,
    VercelSandboxClientOptions,
    VercelSandboxSession,
    VercelSandboxSessionState,
    HostedProviderSpec {
        provider_name: "vercel",
        auth_kind: HostedAuthKind::Token,
        auth_env_var: "VERCEL_TOKEN",
        workspace_root_policy: WorkspaceRootPolicy::Defaulted(DEFAULT_VERCEL_WORKSPACE_ROOT),
        supports_exposed_ports: true,
        supports_pty: false,
    }
);
