use serde::{Deserialize, Serialize};

/// A patch operation for simple editor flows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyPatchOperation {
    pub path: String,
    pub replacement: String,
}

/// Result returned by editor patch application.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyPatchResult {
    pub updated: bool,
    pub path: String,
}

/// Minimal editor abstraction used by local tools.
#[derive(Clone, Debug, Default)]
pub struct Editor;

impl Editor {
    pub fn apply_patch(&self, operation: ApplyPatchOperation) -> ApplyPatchResult {
        ApplyPatchResult {
            updated: true,
            path: operation.path,
        }
    }
}

pub type ApplyPatchEditor = Editor;
