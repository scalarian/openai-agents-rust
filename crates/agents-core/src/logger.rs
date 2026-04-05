use std::io::{self, Write};
use std::sync::Once;

use log::{LevelFilter, Log, Metadata, Record};

/// Crate-wide logger target.
pub const LOGGER_TARGET: &str = "openai_agents";

struct VerboseStdoutLogger;

impl Log for VerboseStdoutLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let _ = writeln!(
            io::stdout(),
            "[{} {}] {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {
        let _ = io::stdout().flush();
    }
}

static VERBOSE_STDOUT_LOGGER: VerboseStdoutLogger = VerboseStdoutLogger;
static INIT_VERBOSE_STDOUT_LOGGER: Once = Once::new();

/// Enables verbose logging to stdout for any log-based diagnostics emitted by the SDK.
pub fn enable_verbose_stdout_logging() {
    INIT_VERBOSE_STDOUT_LOGGER.call_once(|| {
        let _ = log::set_logger(&VERBOSE_STDOUT_LOGGER);
    });
    log::set_max_level(LevelFilter::Debug);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enables_verbose_stdout_logging_idempotently() {
        let previous_level = log::max_level();

        enable_verbose_stdout_logging();
        enable_verbose_stdout_logging();

        assert_eq!(log::max_level(), LevelFilter::Debug);

        log::set_max_level(previous_level);
    }
}
