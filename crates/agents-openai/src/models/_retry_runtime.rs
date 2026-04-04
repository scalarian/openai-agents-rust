use std::cell::Cell;

thread_local! {
    static DISABLE_PROVIDER_MANAGED_RETRIES: Cell<bool> = const { Cell::new(false) };
    static DISABLE_WEBSOCKET_PRE_EVENT_RETRIES: Cell<bool> = const { Cell::new(false) };
}

pub struct RetryFlagGuard {
    previous: bool,
    setter: fn(bool),
}

impl Drop for RetryFlagGuard {
    fn drop(&mut self) {
        (self.setter)(self.previous);
    }
}

fn set_provider_managed_retries_disabled(disabled: bool) {
    DISABLE_PROVIDER_MANAGED_RETRIES.with(|value| value.set(disabled));
}

fn set_websocket_pre_event_retries_disabled(disabled: bool) {
    DISABLE_WEBSOCKET_PRE_EVENT_RETRIES.with(|value| value.set(disabled));
}

pub fn provider_managed_retries_disabled(disabled: bool) -> RetryFlagGuard {
    let previous = DISABLE_PROVIDER_MANAGED_RETRIES.with(|value| {
        let previous = value.get();
        value.set(disabled);
        previous
    });
    RetryFlagGuard {
        previous,
        setter: set_provider_managed_retries_disabled,
    }
}

pub fn should_disable_provider_managed_retries() -> bool {
    DISABLE_PROVIDER_MANAGED_RETRIES.with(Cell::get)
}

pub fn websocket_pre_event_retries_disabled(disabled: bool) -> RetryFlagGuard {
    let previous = DISABLE_WEBSOCKET_PRE_EVENT_RETRIES.with(|value| {
        let previous = value.get();
        value.set(disabled);
        previous
    });
    RetryFlagGuard {
        previous,
        setter: set_websocket_pre_event_retries_disabled,
    }
}

pub fn should_disable_websocket_pre_event_retries() -> bool {
    DISABLE_WEBSOCKET_PRE_EVENT_RETRIES.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resets_provider_retry_flag_after_scope() {
        assert!(!should_disable_provider_managed_retries());
        {
            let _guard = provider_managed_retries_disabled(true);
            assert!(should_disable_provider_managed_retries());
        }
        assert!(!should_disable_provider_managed_retries());
    }
}
