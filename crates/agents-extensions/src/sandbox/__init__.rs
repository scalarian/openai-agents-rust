#![allow(unused_imports, unused_macros)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! define_hosted_sandbox_provider {
    ($client:ident, $options:ident, $session:ident, $state:ident) => {
        #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub struct $options {
            pub workspace_root: Option<String>,
            pub base_url: Option<String>,
        }

        #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub struct $state {
            pub session_id: Option<String>,
            pub workspace_root: Option<String>,
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
        }

        #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub struct $session {
            pub state: $state,
        }

        impl $session {
            pub fn new(state: $state) -> Self {
                Self { state }
            }

            pub fn state(&self) -> &$state {
                &self.state
            }
        }
    };
}

#[cfg(feature = "e2b")]
pub mod e2b {
    use super::*;

    define_hosted_sandbox_provider!(
        E2BSandboxClient,
        E2BSandboxClientOptions,
        E2BSandboxSession,
        E2BSandboxSessionState
    );
}

#[cfg(feature = "modal")]
pub mod modal {
    use super::*;

    define_hosted_sandbox_provider!(
        ModalSandboxClient,
        ModalSandboxClientOptions,
        ModalSandboxSession,
        ModalSandboxSessionState
    );
}

#[cfg(feature = "daytona")]
pub mod daytona {
    use super::*;

    define_hosted_sandbox_provider!(
        DaytonaSandboxClient,
        DaytonaSandboxClientOptions,
        DaytonaSandboxSession,
        DaytonaSandboxSessionState
    );
}

#[cfg(feature = "blaxel")]
pub mod blaxel {
    use super::*;

    define_hosted_sandbox_provider!(
        BlaxelSandboxClient,
        BlaxelSandboxClientOptions,
        BlaxelSandboxSession,
        BlaxelSandboxSessionState
    );
}

#[cfg(feature = "cloudflare")]
pub mod cloudflare {
    use super::*;

    define_hosted_sandbox_provider!(
        CloudflareSandboxClient,
        CloudflareSandboxClientOptions,
        CloudflareSandboxSession,
        CloudflareSandboxSessionState
    );
}

#[cfg(feature = "runloop")]
pub mod runloop {
    use super::*;

    define_hosted_sandbox_provider!(
        RunloopSandboxClient,
        RunloopSandboxClientOptions,
        RunloopSandboxSession,
        RunloopSandboxSessionState
    );
}

#[cfg(feature = "vercel")]
pub mod vercel {
    use super::*;

    define_hosted_sandbox_provider!(
        VercelSandboxClient,
        VercelSandboxClientOptions,
        VercelSandboxSession,
        VercelSandboxSessionState
    );
}

#[cfg(feature = "blaxel")]
pub use blaxel::{
    BlaxelSandboxClient, BlaxelSandboxClientOptions, BlaxelSandboxSession,
    BlaxelSandboxSessionState,
};
#[cfg(feature = "cloudflare")]
pub use cloudflare::{
    CloudflareSandboxClient, CloudflareSandboxClientOptions, CloudflareSandboxSession,
    CloudflareSandboxSessionState,
};
#[cfg(feature = "daytona")]
pub use daytona::{
    DaytonaSandboxClient, DaytonaSandboxClientOptions, DaytonaSandboxSession,
    DaytonaSandboxSessionState,
};
#[cfg(feature = "e2b")]
pub use e2b::{
    E2BSandboxClient, E2BSandboxClientOptions, E2BSandboxSession, E2BSandboxSessionState,
};
#[cfg(feature = "modal")]
pub use modal::{
    ModalSandboxClient, ModalSandboxClientOptions, ModalSandboxSession, ModalSandboxSessionState,
};
#[cfg(feature = "runloop")]
pub use runloop::{
    RunloopSandboxClient, RunloopSandboxClientOptions, RunloopSandboxSession,
    RunloopSandboxSessionState,
};
#[cfg(feature = "vercel")]
pub use vercel::{
    VercelSandboxClient, VercelSandboxClientOptions, VercelSandboxSession,
    VercelSandboxSessionState,
};
