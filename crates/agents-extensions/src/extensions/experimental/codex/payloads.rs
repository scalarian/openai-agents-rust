use serde::Serialize;
use serde_json::Value;

/// Helper trait for types that are frequently serialized into JSON event payloads.
pub trait DictLike: Serialize {
    fn as_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

impl<T> DictLike for T where T: Serialize {}
