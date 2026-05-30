use std::io::{self, Write};

pub fn is_auto_mode() -> bool {
    std::env::var("EXAMPLES_INTERACTIVE_MODE").is_ok_and(|value| value.eq_ignore_ascii_case("auto"))
}

pub fn input_with_fallback(prompt: &str, fallback: &str) -> io::Result<String> {
    if is_auto_mode() {
        println!("[auto-input] {} -> {}", prompt.trim(), fallback);
        return Ok(fallback.to_owned());
    }

    print!("{prompt}");
    io::stdout().flush()?;
    read_line()
}

pub fn confirm_with_fallback(prompt: &str, default: bool) -> io::Result<bool> {
    if is_auto_mode() {
        let choice = if default { "yes" } else { "no" };
        println!("[auto-confirm] {} -> {}", prompt.trim(), choice);
        return Ok(default);
    }

    let answer = input_with_fallback(prompt, "")?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer.is_empty() {
        return Ok(default);
    }
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

fn read_line() -> io::Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim_end_matches(['\r', '\n']).to_owned())
}
