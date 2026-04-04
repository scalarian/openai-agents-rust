use uuid::Uuid;

pub const FAKE_RESPONSES_ID: &str = "__fake_id__";

pub fn fake_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_prefixed_fake_ids() {
        let value = fake_id("resp");
        assert!(value.starts_with("resp_"));
        assert_ne!(value, FAKE_RESPONSES_ID);
    }
}
