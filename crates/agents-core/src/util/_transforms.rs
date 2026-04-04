pub fn transform_string_function_style(name: &str) -> String {
    let mut transformed = String::with_capacity(name.len());
    for ch in name.chars() {
        let normalized = if ch == ' ' {
            '_'
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            ch
        } else {
            '_'
        };
        transformed.push(normalized.to_ascii_lowercase());
    }
    transformed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_invalid_function_names() {
        assert_eq!(
            transform_string_function_style("Transfer To Billing!"),
            "transfer_to_billing_"
        );
    }
}
