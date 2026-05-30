pub fn transform_string_function_style(name: &str) -> String {
    let mut transformed = String::with_capacity(name.len());
    let mut replaced_invalid_chars = false;

    for ch in name.chars() {
        let normalized = if ch == ' ' {
            replaced_invalid_chars = true;
            '_'
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            ch
        } else {
            replaced_invalid_chars = true;
            '_'
        };
        transformed.push(normalized.to_ascii_lowercase());
    }

    if replaced_invalid_chars {
        log::warn!(
            target: crate::LOGGER_TARGET,
            "Tool name {name:?} contains invalid characters for function calling and has been \
             transformed to {transformed:?}. Please use only letters, digits, and underscores to \
             avoid potential naming conflicts."
        );
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

    #[test]
    fn normalizes_case_without_replacement() {
        assert_eq!(transform_string_function_style("MyTool"), "mytool");
        assert_eq!(
            transform_string_function_style("transfer_to_Agent"),
            "transfer_to_agent"
        );
        assert_eq!(transform_string_function_style("snake_case"), "snake_case");
    }
}
