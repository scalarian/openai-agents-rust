use std::collections::BTreeMap;
use std::sync::Arc;

use agents_core::Result;

use crate::extensions::experimental::codex::codex_options::CodexOptions;
use crate::extensions::experimental::codex::exec::CodexExec;
use crate::extensions::experimental::codex::thread::Thread;
use crate::extensions::experimental::codex::thread_options::ThreadOptions;

/// Entry point for the experimental Codex CLI integration.
#[derive(Clone, Debug)]
pub struct Codex {
    exec: Arc<CodexExec>,
    options: CodexOptions,
}

impl Codex {
    pub fn new(options: Option<CodexOptions>) -> Result<Self> {
        let options = options.unwrap_or_default();
        let exec = CodexExec::new(
            options.codex_path_override.clone(),
            normalize_env(&options),
            options.codex_subprocess_stream_limit_bytes,
        )?;
        Ok(Self {
            exec: Arc::new(exec),
            options,
        })
    }

    pub fn start_thread(&self, options: Option<ThreadOptions>) -> Thread {
        Thread::new(
            Arc::clone(&self.exec),
            self.options.clone(),
            options.unwrap_or_default(),
            None,
        )
    }

    pub fn resume_thread(
        &self,
        thread_id: impl Into<String>,
        options: Option<ThreadOptions>,
    ) -> Thread {
        Thread::new(
            Arc::clone(&self.exec),
            self.options.clone(),
            options.unwrap_or_default(),
            Some(thread_id.into()),
        )
    }
}

fn normalize_env(options: &CodexOptions) -> Option<BTreeMap<String, String>> {
    options.env.clone()
}
