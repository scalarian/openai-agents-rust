use crate::agent::Agent;
use crate::errors::Result;
use crate::exceptions::{ModelBehaviorError, UserError};
use crate::items::{OutputItem, RunItem};
use crate::run_config::RunConfig;
use crate::run_context::RunContextWrapper;
use crate::run_state::{RunInterruption, RunInterruptionKind, RunState};
use crate::tool::{Tool, ToolOutput};
use crate::tool_context::{ToolCall, ToolContext};
use crate::tool_guardrails::{
    ToolGuardrailBehavior, ToolInputGuardrailResult, ToolOutputGuardrailResult,
};
use crate::tracing::{SpanData, function_span, get_trace_provider};

use super::approvals::append_approval_error_output;

pub(crate) struct ToolExecutionOutcome {
    pub new_items: Vec<RunItem>,
    pub input_guardrail_results: Vec<ToolInputGuardrailResult>,
    pub output_guardrail_results: Vec<ToolOutputGuardrailResult>,
    pub interruptions: Vec<RunInterruption>,
}

pub(crate) async fn execute_local_function_tools(
    agent: &Agent,
    run_config: &RunConfig,
    context: &RunContextWrapper,
    tool_calls: Vec<ToolCall>,
) -> Result<ToolExecutionOutcome> {
    let mut new_items = Vec::new();
    let mut input_guardrail_results = Vec::new();
    let mut output_guardrail_results = Vec::new();
    let mut interruptions = Vec::new();

    for tool_call in tool_calls {
        let function_tool = agent
            .find_function_tool(&tool_call.name, tool_call.namespace.as_deref())
            .ok_or_else(|| ModelBehaviorError {
                message: format!(
                    "model requested unknown local function tool `{}`",
                    tool_call.name
                ),
            })?;

        let tool_context = ToolContext::from_tool_call(context, tool_call.clone())
            .with_agent(agent.clone())
            .with_run_config(run_config.clone());
        let provider = get_trace_provider();
        let mut span = function_span(
            &tool_context.trace_name(),
            Some(tool_call.arguments.clone()),
            None,
        );
        provider.start_span(&mut span, true);

        if function_tool.needs_approval {
            match context.approvals.get(&tool_call.id) {
                None => {
                    provider.finish_span(&mut span, true);
                    interruptions.push(RunInterruption {
                        kind: Some(RunInterruptionKind::ToolApproval),
                        call_id: Some(tool_call.id.clone()),
                        tool_name: Some(tool_call.name.clone()),
                        reason: Some("tool approval required".to_owned()),
                    });
                    break;
                }
                Some(approval) if !approval.approved => {
                    append_approval_error_output(
                        &mut new_items,
                        tool_call.name.clone(),
                        tool_call.id.clone(),
                        tool_call.namespace.clone(),
                        Some(approval),
                    );
                    if let SpanData::Function(data) = &mut span.data {
                        data.output = Some("tool approval rejected".to_owned());
                    }
                    provider.finish_span(&mut span, true);
                    continue;
                }
                Some(_) => {}
            }
        }

        let mut invocation_rejected = None;
        for guardrail in &function_tool.tool_input_guardrails {
            let result = guardrail
                .run(crate::tool_guardrails::ToolInputGuardrailData {
                    context: tool_context.clone(),
                    agent: agent.clone(),
                })
                .await?;
            match &result.output.behavior {
                ToolGuardrailBehavior::Allow => {}
                ToolGuardrailBehavior::RejectContent { message } => {
                    invocation_rejected = Some(ToolOutput::from(message.as_str()));
                }
                ToolGuardrailBehavior::RaiseException => {
                    span.set_error(
                        format!("tool input guardrail `{}` triggered", result.guardrail_name),
                        None,
                    );
                    provider.finish_span(&mut span, true);
                    return Err(crate::exceptions::ToolInputGuardrailTripwireTriggered {
                        guardrail_name: result.guardrail_name.clone(),
                        output: result.output.clone(),
                    }
                    .into());
                }
            }
            input_guardrail_results.push(result);
        }

        let mut output = if let Some(rejected) = invocation_rejected {
            rejected
        } else {
            let parsed_arguments = serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                .unwrap_or(serde_json::Value::Null);
            function_tool
                .invoke(tool_context.clone(), parsed_arguments)
                .await?
        };

        for guardrail in &function_tool.tool_output_guardrails {
            let result = guardrail
                .run(crate::tool_guardrails::ToolOutputGuardrailData {
                    context: tool_context.clone(),
                    agent: agent.clone(),
                    output: output.clone(),
                })
                .await?;
            match &result.output.behavior {
                ToolGuardrailBehavior::Allow => {}
                ToolGuardrailBehavior::RejectContent { message } => {
                    output = ToolOutput::from(message.as_str());
                }
                ToolGuardrailBehavior::RaiseException => {
                    span.set_error(
                        format!(
                            "tool output guardrail `{}` triggered",
                            result.guardrail_name
                        ),
                        None,
                    );
                    provider.finish_span(&mut span, true);
                    return Err(crate::exceptions::ToolOutputGuardrailTripwireTriggered {
                        guardrail_name: result.guardrail_name.clone(),
                        output: result.output.clone(),
                    }
                    .into());
                }
            }
            output_guardrail_results.push(result);
        }

        new_items.push(RunItem::ToolCallOutput {
            tool_name: tool_call.name,
            output: output.to_output_item(),
            call_id: Some(tool_call.id),
            namespace: tool_call.namespace,
        });
        if let SpanData::Function(data) = &mut span.data {
            data.output = serde_json::to_string(&output).ok();
        }
        provider.finish_span(&mut span, true);
    }

    Ok(ToolExecutionOutcome {
        new_items,
        input_guardrail_results,
        output_guardrail_results,
        interruptions,
    })
}

pub(crate) fn extract_tool_calls(output: &[OutputItem]) -> Vec<ToolCall> {
    output
        .iter()
        .filter_map(|item| match item {
            OutputItem::ToolCall {
                call_id,
                tool_name,
                arguments,
                namespace,
            } => Some(ToolCall {
                id: call_id.clone(),
                name: tool_name.clone(),
                arguments: serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_owned()),
                namespace: namespace.clone(),
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn resolve_handoff_agent(
    current_agent: &Agent,
    output: &[OutputItem],
) -> Result<Option<Agent>> {
    let target = output.iter().find_map(|item| match item {
        OutputItem::Handoff { target_agent } => Some(target_agent.as_str()),
        _ => None,
    });

    let Some(target) = target else {
        return Ok(None);
    };

    let handoff = current_agent
        .find_handoff(target)
        .ok_or_else(|| ModelBehaviorError {
            message: format!(
                "model requested unknown handoff target `{}` from agent `{}`",
                target, current_agent.name
            ),
        })?;

    let target_agent = handoff.runtime_agent().cloned().ok_or_else(|| UserError {
        message: format!(
            "handoff target `{}` is not bound to a runtime agent instance",
            target
        ),
    })?;

    Ok(Some(target_agent))
}

pub(crate) fn find_pending_tool_call(state: &RunState, call_id: &str) -> Option<ToolCall> {
    state
        .generated_items
        .iter()
        .rev()
        .find_map(|item| match item {
            RunItem::ToolCall {
                tool_name,
                arguments,
                call_id: Some(existing_call_id),
                namespace,
            } if existing_call_id == call_id => Some(ToolCall {
                id: existing_call_id.clone(),
                name: tool_name.clone(),
                arguments: serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_owned()),
                namespace: namespace.clone(),
            }),
            _ => None,
        })
}
