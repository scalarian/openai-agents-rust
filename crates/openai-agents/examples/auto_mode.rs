#[path = "support/auto_mode.rs"]
mod auto_mode;

use auto_mode::{confirm_with_fallback, input_with_fallback, is_auto_mode};
use openai_agents::AgentsError;

fn main() -> Result<(), AgentsError> {
    println!("auto_mode={}", is_auto_mode());

    let topic = input_with_fallback("Enter a topic: ", "recursion")
        .map_err(|error| AgentsError::message(error.to_string()))?;
    let approved = confirm_with_fallback("Continue? [y/N]: ", true)
        .map_err(|error| AgentsError::message(error.to_string()))?;

    println!("topic={topic}");
    println!("approved={approved}");
    Ok(())
}
