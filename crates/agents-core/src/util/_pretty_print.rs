use serde_json::Value;

use crate::result::{RunResult, RunResultStreaming};
use crate::run_error_handlers::RunErrorData;

fn indent(text: &str, indent_level: usize) -> String {
    let indent = "  ".repeat(indent_level);
    text.lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
    }
}

pub fn pretty_print_result(result: &RunResult) -> String {
    let final_output = result
        .final_output
        .clone()
        .unwrap_or_else(|| "None".to_owned());
    format!(
        "RunResult:\n- Last agent: Agent(name=\"{}\", ...)\n- Final output (String):\n{}\n- {} new item(s)\n- {} raw response(s)\n- {} input guardrail result(s)\n- {} output guardrail result(s)\n(See `RunResult` for more details)",
        result
            .last_agent
            .as_ref()
            .map(|agent| agent.name.as_str())
            .unwrap_or(&result.agent_name),
        indent(&final_output, 2),
        result.new_items.len(),
        result.raw_responses.len(),
        result.input_guardrail_results.len(),
        result.output_guardrail_results.len(),
    )
}

pub fn pretty_print_run_error_details(result: &RunErrorData) -> String {
    format!(
        "RunErrorData:\n- Last agent: Agent(name=\"{}\", ...)\n- {} new item(s)\n- {} raw response(s)\n(See `RunErrorData` for more details)",
        result.last_agent.name,
        result.new_items.len(),
        result.raw_responses.len(),
    )
}

pub fn pretty_print_run_result_streaming(result: &RunResultStreaming) -> String {
    let final_output = result
        .final_output
        .as_ref()
        .map(value_to_string)
        .unwrap_or_else(|| "None".to_owned());
    format!(
        "RunResultStreaming:\n- Current agent: Agent(name=\"{}\", ...)\n- Current turn: {}\n- Max turns: {}\n- Is complete: {}\n- Final output ({}):\n{}\n- {} new item(s)\n- {} raw response(s)\n(See `RunResultStreaming` for more details)",
        result
            .current_agent
            .as_ref()
            .map(|agent| agent.name.as_str())
            .unwrap_or("unknown"),
        result.current_turn,
        result.max_turns,
        result.is_complete,
        result
            .final_output
            .as_ref()
            .map(|value| match value {
                Value::String(_) => "String",
                _ => "Value",
            })
            .unwrap_or("Option"),
        indent(&final_output, 2),
        result.new_items.len(),
        result.raw_responses.len(),
    )
}
