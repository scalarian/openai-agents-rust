/// Recommended prefix for agents that rely on handoffs.
pub const RECOMMENDED_PROMPT_PREFIX: &str = concat!(
    "# System context\n",
    "You are part of a multi-agent system called the Agents SDK, designed to make agent ",
    "coordination and execution easy. Agents uses two primary abstractions: **Agents** and ",
    "**Handoffs**. An agent encompasses instructions and tools and can hand off a ",
    "conversation to another agent when appropriate. ",
    "Handoffs are achieved by calling a handoff function, generally named ",
    "`transfer_to_<agent_name>`. Transfers between agents are handled seamlessly in the ",
    "background; do not mention or draw attention to these transfers in your conversation ",
    "with the user.\n"
);

/// Prepends the recommended handoff instructions to an existing prompt.
pub fn prompt_with_handoff_instructions(prompt: &str) -> String {
    format!("{RECOMMENDED_PROMPT_PREFIX}\n\n{prompt}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_prompt() {
        let prompt = prompt_with_handoff_instructions("Help the user.");
        assert!(prompt.starts_with("# System context"));
        assert!(prompt.ends_with("Help the user."));
    }
}
