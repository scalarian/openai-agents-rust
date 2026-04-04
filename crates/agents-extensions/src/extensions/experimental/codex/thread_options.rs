use serde::{Deserialize, Serialize};

/// Approval policy for Codex tool usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

/// Filesystem sandbox level used for Codex execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

/// Reasoning effort used by Codex models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

/// Web search mode exposed by Codex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchMode {
    Disabled,
    Cached,
    Live,
}

/// Per-thread execution options for Codex.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadOptions {
    pub model: Option<String>,
    pub sandbox_mode: Option<SandboxMode>,
    pub working_directory: Option<String>,
    pub skip_git_repo_check: Option<bool>,
    pub model_reasoning_effort: Option<ModelReasoningEffort>,
    pub network_access_enabled: Option<bool>,
    pub web_search_mode: Option<WebSearchMode>,
    pub web_search_enabled: Option<bool>,
    pub approval_policy: Option<ApprovalMode>,
    pub additional_directories: Option<Vec<String>>,
}

pub fn coerce_thread_options(options: Option<ThreadOptions>) -> Option<ThreadOptions> {
    options
}
