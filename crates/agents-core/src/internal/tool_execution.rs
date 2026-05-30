use crate::agent::Agent;
use crate::errors::Result;
use crate::exceptions::{ModelBehaviorError, UserError};
use crate::items::{OutputItem, RunItem};
use crate::run_config::{RunConfig, ToolErrorFormatterArgs, ToolNotFoundBehavior};
use crate::run_context::{ApprovalRecord, RunContextWrapper};
use crate::run_state::{RunInterruption, RunInterruptionKind, RunState};
use crate::tool::{
    FunctionTool, FunctionToolResult, Tool, ToolOutput, default_tool_error_function,
    get_function_tool_origin,
};
use crate::tool_context::{ToolCall, ToolContext};
use crate::tool_guardrails::{
    ToolGuardrailBehavior, ToolInputGuardrailResult, ToolOutputGuardrailResult,
};
use crate::tracing::{Span, SpanData, function_span, get_trace_provider};
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use std::collections::{BTreeMap, VecDeque};
use uuid::Uuid;

use super::approvals::append_approval_error_output;
use super::streaming::StreamRecorder;

pub(crate) struct ToolExecutionOutcome {
    pub new_items: Vec<RunItem>,
    pub tool_results: Vec<FunctionToolResult>,
    pub input_guardrail_results: Vec<ToolInputGuardrailResult>,
    pub output_guardrail_results: Vec<ToolOutputGuardrailResult>,
    pub interruptions: Vec<RunInterruption>,
}

enum ToolExecutionPlan {
    Execute {
        order: usize,
        tool_call: ToolCall,
        function_tool: crate::tool::FunctionTool,
    },
    ReturnErrorToModel {
        order: usize,
        run_item: RunItem,
    },
}

#[derive(Default)]
struct SingleToolExecutionOutcome {
    new_items: Vec<RunItem>,
    tool_results: Vec<FunctionToolResult>,
    input_guardrail_results: Vec<ToolInputGuardrailResult>,
    output_guardrail_results: Vec<ToolOutputGuardrailResult>,
    interruptions: Vec<RunInterruption>,
}

pub(crate) async fn execute_local_function_tools(
    agent: &Agent,
    run_config: &RunConfig,
    context: &RunContextWrapper,
    tool_calls: Vec<ToolCall>,
    stream_recorder: Option<&StreamRecorder>,
    approved_execution: Option<(&RunInterruption, &ApprovalRecord)>,
) -> Result<ToolExecutionOutcome> {
    let runtime_tools = agent.get_all_function_tools(context).await?;
    execute_local_function_tools_with_runtime_tools(
        agent,
        run_config,
        context,
        &runtime_tools,
        tool_calls,
        stream_recorder,
        approved_execution,
    )
    .await
}

pub(crate) async fn execute_local_function_tools_with_runtime_tools(
    agent: &Agent,
    run_config: &RunConfig,
    context: &RunContextWrapper,
    runtime_tools: &[FunctionTool],
    tool_calls: Vec<ToolCall>,
    stream_recorder: Option<&StreamRecorder>,
    approved_execution: Option<(&RunInterruption, &ApprovalRecord)>,
) -> Result<ToolExecutionOutcome> {
    if let Some(tool_execution) = &run_config.tool_execution {
        tool_execution.validate()?;
    }

    reject_disabled_function_tool_calls(agent, runtime_tools, &tool_calls)?;
    let plans = build_tool_execution_plans(run_config, context, runtime_tools, tool_calls).await?;
    let must_execute_sequentially = approved_execution.is_some()
        || plans.iter().any(|plan| {
            matches!(
                plan,
                ToolExecutionPlan::Execute {
                    function_tool,
                    ..
                } if function_tool.needs_approval
            )
        })
        || run_config
            .tool_execution
            .as_ref()
            .and_then(|config| config.max_function_tool_concurrency)
            == Some(1);

    if must_execute_sequentially {
        return execute_tool_plans_sequentially(
            agent,
            run_config,
            context,
            plans,
            stream_recorder,
            approved_execution,
        )
        .await;
    }

    execute_tool_plans_concurrently(agent, run_config, context, plans, stream_recorder).await
}

async fn build_tool_execution_plans(
    run_config: &RunConfig,
    context: &RunContextWrapper,
    runtime_tools: &[crate::tool::FunctionTool],
    tool_calls: Vec<ToolCall>,
) -> Result<Vec<ToolExecutionPlan>> {
    let mut plans = Vec::new();

    for (order, tool_call) in tool_calls.into_iter().enumerate() {
        let Some(function_tool) = runtime_tools.iter().find(|tool| {
            tool.definition.name == tool_call.name
                && tool.definition.namespace.as_deref() == tool_call.namespace.as_deref()
        }) else {
            if run_config.tool_not_found_behavior == ToolNotFoundBehavior::ReturnErrorToModel {
                let message =
                    resolve_tool_not_found_message(context, run_config, &tool_call).await?;
                plans.push(ToolExecutionPlan::ReturnErrorToModel {
                    order,
                    run_item: RunItem::ToolCallOutput {
                        tool_name: tool_call.name,
                        output: OutputItem::Text { text: message },
                        call_id: Some(tool_call.id),
                        namespace: tool_call.namespace,
                        tool_origin: None,
                    },
                });
                continue;
            }

            return Err(ModelBehaviorError {
                message: format!(
                    "model requested unknown local function tool `{}`",
                    tool_call.name
                ),
            }
            .into());
        };

        plans.push(ToolExecutionPlan::Execute {
            order,
            tool_call,
            function_tool: function_tool.clone(),
        });
    }

    Ok(plans)
}

async fn execute_tool_plans_sequentially(
    agent: &Agent,
    run_config: &RunConfig,
    context: &RunContextWrapper,
    plans: Vec<ToolExecutionPlan>,
    stream_recorder: Option<&StreamRecorder>,
    approved_execution: Option<(&RunInterruption, &ApprovalRecord)>,
) -> Result<ToolExecutionOutcome> {
    let mut new_items = Vec::new();
    let mut tool_results = Vec::new();
    let mut input_guardrail_results = Vec::new();
    let mut output_guardrail_results = Vec::new();
    let mut interruptions = Vec::new();

    for plan in plans {
        let outcome = match plan {
            ToolExecutionPlan::ReturnErrorToModel { run_item, .. } => SingleToolExecutionOutcome {
                new_items: vec![run_item],
                ..SingleToolExecutionOutcome::default()
            },
            ToolExecutionPlan::Execute {
                tool_call,
                function_tool,
                ..
            } => {
                execute_single_function_tool(
                    agent,
                    run_config,
                    context,
                    tool_call,
                    function_tool,
                    stream_recorder,
                    approved_execution,
                )
                .await?
            }
        };

        let interrupted = !outcome.interruptions.is_empty();
        new_items.extend(outcome.new_items);
        tool_results.extend(outcome.tool_results);
        input_guardrail_results.extend(outcome.input_guardrail_results);
        output_guardrail_results.extend(outcome.output_guardrail_results);
        interruptions.extend(outcome.interruptions);
        if interrupted {
            break;
        }
    }

    Ok(ToolExecutionOutcome {
        new_items,
        tool_results,
        input_guardrail_results,
        output_guardrail_results,
        interruptions,
    })
}

async fn execute_tool_plans_concurrently(
    agent: &Agent,
    run_config: &RunConfig,
    context: &RunContextWrapper,
    plans: Vec<ToolExecutionPlan>,
    stream_recorder: Option<&StreamRecorder>,
) -> Result<ToolExecutionOutcome> {
    let max_concurrency = run_config
        .tool_execution
        .as_ref()
        .and_then(|config| config.max_function_tool_concurrency);
    let mut completed = BTreeMap::<usize, SingleToolExecutionOutcome>::new();
    let mut pending = VecDeque::new();

    for plan in plans {
        match plan {
            ToolExecutionPlan::ReturnErrorToModel { order, run_item } => {
                completed.insert(
                    order,
                    SingleToolExecutionOutcome {
                        new_items: vec![run_item],
                        ..SingleToolExecutionOutcome::default()
                    },
                );
            }
            ToolExecutionPlan::Execute {
                order,
                tool_call,
                function_tool,
            } => pending.push_back((order, tool_call, function_tool)),
        }
    }

    let mut active = futures::stream::FuturesUnordered::new();
    fill_tool_task_slots(
        agent,
        run_config,
        context,
        stream_recorder,
        max_concurrency,
        &mut pending,
        &mut active,
    );

    while let Some(result) = active.next().await {
        let (order, outcome) = result?;
        completed.insert(order, outcome);
        fill_tool_task_slots(
            agent,
            run_config,
            context,
            stream_recorder,
            max_concurrency,
            &mut pending,
            &mut active,
        );
    }

    let mut outcome = ToolExecutionOutcome {
        new_items: Vec::new(),
        tool_results: Vec::new(),
        input_guardrail_results: Vec::new(),
        output_guardrail_results: Vec::new(),
        interruptions: Vec::new(),
    };

    for item in completed.into_values() {
        outcome.new_items.extend(item.new_items);
        outcome.tool_results.extend(item.tool_results);
        outcome
            .input_guardrail_results
            .extend(item.input_guardrail_results);
        outcome
            .output_guardrail_results
            .extend(item.output_guardrail_results);
        outcome.interruptions.extend(item.interruptions);
    }

    Ok(outcome)
}

type ToolTaskFuture<'a> = BoxFuture<'a, Result<(usize, SingleToolExecutionOutcome)>>;

fn fill_tool_task_slots<'a>(
    agent: &'a Agent,
    run_config: &'a RunConfig,
    context: &'a RunContextWrapper,
    stream_recorder: Option<&'a StreamRecorder>,
    max_concurrency: Option<usize>,
    pending: &mut VecDeque<(usize, ToolCall, crate::tool::FunctionTool)>,
    active: &mut futures::stream::FuturesUnordered<ToolTaskFuture<'a>>,
) {
    let available_slots = max_concurrency
        .map(|max| max.saturating_sub(active.len()))
        .unwrap_or(pending.len());

    for _ in 0..available_slots {
        let Some((order, tool_call, function_tool)) = pending.pop_front() else {
            break;
        };
        active.push(
            async move {
                execute_single_function_tool(
                    agent,
                    run_config,
                    context,
                    tool_call,
                    function_tool,
                    stream_recorder,
                    None,
                )
                .await
                .map(|outcome| (order, outcome))
            }
            .boxed(),
        );
    }
}

async fn execute_single_function_tool(
    agent: &Agent,
    run_config: &RunConfig,
    context: &RunContextWrapper,
    tool_call: ToolCall,
    function_tool: crate::tool::FunctionTool,
    stream_recorder: Option<&StreamRecorder>,
    approved_execution: Option<(&RunInterruption, &ApprovalRecord)>,
) -> Result<SingleToolExecutionOutcome> {
    let mut new_items = Vec::new();
    let mut tool_results = Vec::new();
    let mut input_guardrail_results = Vec::new();
    let mut output_guardrail_results = Vec::new();
    let mut interruptions = Vec::new();

    let tool_context = ToolContext::from_tool_call(context, tool_call.clone())
        .with_agent(agent.clone())
        .with_run_config(run_config.clone());
    let provider = get_trace_provider();
    let trace_sensitive = !run_config.tracing_disabled && run_config.trace_include_sensitive_data;
    let mut span = function_span(
        &tool_context.trace_name(),
        trace_sensitive.then(|| tool_call.arguments.clone()),
        None,
    );
    if let Some(recorder) = stream_recorder {
        recorder
            .push_lifecycle(
                "tool_start",
                Some(serde_json::json!({
                    "tool_name": tool_call.name.clone(),
                    "call_id": tool_call.id.clone(),
                    "namespace": tool_call.namespace.clone(),
                })),
            )
            .await;
    }
    if let Some(hooks) = &run_config.run_hooks {
        hooks
            .on_tool_start(&tool_context, agent, &function_tool.definition)
            .await;
    }
    if let Some(hooks) = &agent.hooks {
        hooks
            .on_tool_start(&tool_context, agent, &function_tool.definition)
            .await;
    }
    provider.start_span(&mut span, true);

    let approval_resolved = if let Some((interruption, approval)) = approved_execution {
        if approval_decision_matches_tool_call(interruption, approval, &tool_call) {
            if !approval.approved {
                append_approval_error_output(
                    &mut new_items,
                    tool_call.name.clone(),
                    tool_call.id.clone(),
                    tool_call.namespace.clone(),
                    Some(approval),
                    get_function_tool_origin(&function_tool),
                );
                if let SpanData::Function(data) = &mut span.data {
                    data.output = Some("tool approval rejected".to_owned());
                }
                tool_results.push(FunctionToolResult {
                    tool_name: tool_call.name.clone(),
                    call_id: Some(tool_call.id.clone()),
                    tool_arguments: Some(tool_call.arguments.clone()),
                    qualified_name: Some(function_tool.qualified_name()),
                    output: ToolOutput::from(
                        approval
                            .reason
                            .as_deref()
                            .unwrap_or(super::approvals::REJECTION_MESSAGE),
                    ),
                    run_item: new_items.last().cloned(),
                    interruptions: Vec::new(),
                    agent_run_result: None,
                });
                provider.finish_span(&mut span, true);
                return Ok(SingleToolExecutionOutcome {
                    new_items,
                    tool_results,
                    input_guardrail_results,
                    output_guardrail_results,
                    interruptions,
                });
            }
            true
        } else {
            false
        }
    } else {
        false
    };

    if function_tool.needs_approval && !approval_resolved {
        let approval_id = Uuid::new_v4().to_string();
        provider.finish_span(&mut span, true);
        if let Some(recorder) = stream_recorder {
            recorder
                .push_lifecycle(
                    "tool_approval_required",
                    Some(serde_json::json!({
                        "approval_id": approval_id,
                        "tool_name": tool_call.name.clone(),
                        "call_id": tool_call.id.clone(),
                        "namespace": tool_call.namespace.clone(),
                    })),
                )
                .await;
        }
        interruptions.push(RunInterruption {
            kind: Some(RunInterruptionKind::ToolApproval),
            approval_id: Some(approval_id),
            call_id: Some(tool_call.id.clone()),
            tool_name: Some(tool_call.name.clone()),
            namespace: tool_call.namespace.clone(),
            tool_origin: get_function_tool_origin(&function_tool),
            reason: Some("tool approval required".to_owned()),
        });
        return Ok(SingleToolExecutionOutcome {
            new_items,
            tool_results,
            input_guardrail_results,
            output_guardrail_results,
            interruptions,
        });
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
        match parse_function_tool_json_input(&tool_call.name, &tool_call.arguments) {
            Err(error) => {
                let error_message = error.to_string();
                set_tool_error_on_span(
                    &mut span,
                    &function_tool.definition.name,
                    "Error parsing tool arguments",
                    trace_sensitive,
                    &error_message,
                );
                if let Some(output) = resolve_function_tool_error_output(
                    context,
                    run_config,
                    &tool_call,
                    "invalid_json_input",
                    error_message,
                )
                .await?
                {
                    output
                } else {
                    provider.finish_span(&mut span, true);
                    return Err(error);
                }
            }
            Ok(parsed_arguments) => match function_tool
                .invoke(tool_context.clone(), parsed_arguments)
                .await
            {
                Ok(output) => output,
                Err(error) => {
                    let error_message = error.to_string();
                    set_tool_error_on_span(
                        &mut span,
                        &function_tool.definition.name,
                        "Error running tool",
                        trace_sensitive,
                        &error_message,
                    );
                    if let Some(output) = resolve_function_tool_error_output(
                        context,
                        run_config,
                        &tool_call,
                        "invoke_error",
                        "Tool execution failed.".to_owned(),
                    )
                    .await?
                    {
                        output
                    } else {
                        provider.finish_span(&mut span, true);
                        return Err(error);
                    }
                }
            },
        }
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

    let run_item = RunItem::ToolCallOutput {
        tool_name: tool_call.name,
        output: output.to_output_item(),
        call_id: Some(tool_call.id),
        namespace: tool_call.namespace,
        tool_origin: get_function_tool_origin(&function_tool),
    };
    let lifecycle_tool_name = match &run_item {
        RunItem::ToolCallOutput { tool_name, .. } => tool_name.clone(),
        _ => String::new(),
    };
    let lifecycle_call_id = match &run_item {
        RunItem::ToolCallOutput { call_id, .. } => call_id.clone(),
        _ => None,
    };
    let output_text = serde_json::to_string(&output).ok();
    new_items.push(run_item.clone());
    tool_results.push(FunctionToolResult {
        tool_name: match &run_item {
            RunItem::ToolCallOutput { tool_name, .. } => tool_name.clone(),
            _ => String::new(),
        },
        call_id: match &run_item {
            RunItem::ToolCallOutput { call_id, .. } => call_id.clone(),
            _ => None,
        },
        tool_arguments: tool_context
            .tool_call
            .as_ref()
            .map(|call| call.arguments.clone()),
        qualified_name: Some(function_tool.qualified_name()),
        output: output.clone(),
        run_item: Some(run_item),
        interruptions: Vec::new(),
        agent_run_result: None,
    });
    if let Some(hooks) = &run_config.run_hooks {
        hooks
            .on_tool_end(&tool_context, agent, &function_tool.definition, &output)
            .await;
    }
    if let Some(hooks) = &agent.hooks {
        hooks
            .on_tool_end(&tool_context, agent, &function_tool.definition, &output)
            .await;
    }
    if let SpanData::Function(data) = &mut span.data {
        data.output = trace_sensitive.then(|| {
            output_text
                .clone()
                .unwrap_or_else(|| "[tool output omitted]".to_owned())
        });
    }
    provider.finish_span(&mut span, true);
    if let Some(recorder) = stream_recorder {
        recorder
            .push_lifecycle(
                "tool_end",
                Some(serde_json::json!({
                    "tool_name": lifecycle_tool_name,
                    "call_id": lifecycle_call_id,
                })),
            )
            .await;
    }

    Ok(SingleToolExecutionOutcome {
        new_items,
        tool_results,
        input_guardrail_results,
        output_guardrail_results,
        interruptions,
    })
}

fn approval_decision_matches_tool_call(
    interruption: &RunInterruption,
    approval: &ApprovalRecord,
    tool_call: &ToolCall,
) -> bool {
    approval.approval_id == interruption.approval_id
        && approval.call_id.as_deref() == Some(tool_call.id.as_str())
        && approval.tool_name.as_deref() == Some(tool_call.name.as_str())
        && approval.namespace == tool_call.namespace
        && interruption.call_id.as_deref() == Some(tool_call.id.as_str())
        && interruption.tool_name.as_deref() == Some(tool_call.name.as_str())
        && interruption.namespace == tool_call.namespace
}

fn reject_disabled_function_tool_calls(
    agent: &Agent,
    runtime_tools: &[crate::tool::FunctionTool],
    tool_calls: &[ToolCall],
) -> Result<()> {
    for tool_call in tool_calls {
        if runtime_tools
            .iter()
            .any(|tool| tool_matches_call(tool, tool_call))
        {
            continue;
        }

        if agent
            .function_tools
            .iter()
            .any(|tool| tool_matches_call(tool, tool_call))
        {
            return Err(ModelBehaviorError {
                message: format!(
                    "Tool {} is currently disabled for agent {}.",
                    tool_call.name, agent.name
                ),
            }
            .into());
        }
    }

    Ok(())
}

fn tool_matches_call(tool: &crate::tool::FunctionTool, tool_call: &ToolCall) -> bool {
    tool.definition.name == tool_call.name
        && tool.definition.namespace.as_deref() == tool_call.namespace.as_deref()
}

fn set_tool_error_on_span(
    span: &mut Span,
    tool_name: &str,
    message: &str,
    trace_include_sensitive_data: bool,
    error_message: &str,
) {
    span.set_error(
        message.to_owned(),
        Some(serde_json::json!({
            "tool_name": tool_name,
            "error": trace_tool_error(trace_include_sensitive_data, error_message),
        })),
    );
}

fn trace_tool_error(trace_include_sensitive_data: bool, error_message: &str) -> String {
    if trace_include_sensitive_data {
        error_message.to_owned()
    } else {
        "Tool execution failed. Error details are redacted.".to_owned()
    }
}

async fn resolve_tool_not_found_message(
    context: &RunContextWrapper,
    run_config: &RunConfig,
    tool_call: &ToolCall,
) -> Result<String> {
    let default_message = format!("Tool '{}' not found.", tool_call.name);
    let Some(formatter) = &run_config.tool_error_formatter else {
        return Ok(default_message);
    };

    let formatted = formatter(ToolErrorFormatterArgs {
        kind: "tool_not_found",
        tool_type: "function",
        tool_name: tool_call.name.clone(),
        call_id: tool_call.id.clone(),
        default_message: default_message.clone(),
        run_context: context.clone(),
    })
    .await?;

    Ok(formatted.unwrap_or(default_message))
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
                arguments: raw_tool_arguments(arguments),
                namespace: namespace.clone(),
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn apply_tool_origins_to_run_items(
    items: &mut [RunItem],
    runtime_tools: &[FunctionTool],
) {
    for item in items {
        let RunItem::ToolCall {
            tool_name,
            namespace,
            tool_origin,
            ..
        } = item
        else {
            continue;
        };
        if tool_origin.is_some() {
            continue;
        }
        *tool_origin = runtime_tools
            .iter()
            .find(|tool| {
                tool.definition.name == *tool_name
                    && tool.definition.namespace.as_deref() == namespace.as_deref()
            })
            .and_then(get_function_tool_origin);
    }
}

fn raw_tool_arguments(arguments: &serde_json::Value) -> String {
    arguments
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_owned()))
}

fn parse_function_tool_json_input(tool_name: &str, input_json: &str) -> Result<serde_json::Value> {
    let parsed = if input_json.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str::<serde_json::Value>(input_json).map_err(|_| {
            let base_message = format!("Invalid JSON input for tool {tool_name}");
            let message = if crate::_debug::dont_log_tool_data() {
                base_message
            } else {
                format!("{base_message}: {input_json}")
            };
            ModelBehaviorError { message }
        })?
    };

    if !parsed.is_object() {
        return Err(ModelBehaviorError {
            message: format!("Invalid JSON input for tool {tool_name}: expected a JSON object"),
        }
        .into());
    }

    Ok(parsed)
}

async fn resolve_function_tool_error_output(
    context: &RunContextWrapper,
    run_config: &RunConfig,
    tool_call: &ToolCall,
    kind: &'static str,
    default_message: String,
) -> Result<Option<ToolOutput>> {
    if let Some(formatter) = &run_config.tool_error_formatter {
        return formatter(ToolErrorFormatterArgs {
            kind,
            tool_type: "function",
            tool_name: tool_call.name.clone(),
            call_id: tool_call.id.clone(),
            default_message,
            run_context: context.clone(),
        })
        .await
        .map(|message| message.map(ToolOutput::from));
    }

    let args = ToolErrorFormatterArgs {
        kind,
        tool_type: "function",
        tool_name: tool_call.name.clone(),
        call_id: tool_call.id.clone(),
        default_message,
        run_context: context.clone(),
    };
    Ok(Some(ToolOutput::from(default_tool_error_function(&args))))
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
                ..
            } if existing_call_id == call_id => Some(ToolCall {
                id: existing_call_id.clone(),
                name: tool_name.clone(),
                arguments: raw_tool_arguments(arguments),
                namespace: namespace.clone(),
            }),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use futures::FutureExt;
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

    use crate::errors::AgentsError;
    use crate::run_context::RunContext;
    use crate::tool::function_tool;

    use super::*;

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn tool_logging_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn invalid_tool_json_redacts_payload_when_tool_logging_is_disabled() {
        let _lock = tool_logging_env_lock()
            .lock()
            .expect("tool logging env lock");
        let _env = EnvVarGuard::set("OPENAI_AGENTS_DONT_LOG_TOOL_DATA", "true");
        let bad_json = "{\"secret\":\"SECRET_TOKEN_123\"";

        let error = parse_function_tool_json_input("echo_tool", bad_json)
            .expect_err("invalid JSON should fail");

        assert_eq!(error.to_string(), "Invalid JSON input for tool echo_tool");
        assert!(!error.to_string().contains("SECRET_TOKEN_123"));
    }

    #[test]
    fn invalid_tool_json_includes_payload_when_tool_logging_is_enabled() {
        let _lock = tool_logging_env_lock()
            .lock()
            .expect("tool logging env lock");
        let _env = EnvVarGuard::set("OPENAI_AGENTS_DONT_LOG_TOOL_DATA", "false");
        let bad_json = "{\"secret\":\"SECRET_TOKEN_123\"";

        let error = parse_function_tool_json_input("echo_tool", bad_json)
            .expect_err("invalid JSON should fail");

        assert_eq!(
            error.to_string(),
            format!("Invalid JSON input for tool echo_tool: {bad_json}")
        );
        assert!(error.to_string().contains("SECRET_TOKEN_123"));
    }

    #[test]
    fn function_tool_json_input_must_be_object() {
        let error = parse_function_tool_json_input("echo_tool", "[1,2,3]")
            .expect_err("non-object JSON should fail");

        assert_eq!(
            error.to_string(),
            "Invalid JSON input for tool echo_tool: expected a JSON object"
        );
    }

    #[test]
    fn tool_call_extraction_preserves_raw_string_arguments() {
        let bad_json = "{\"secret\":\"SECRET_TOKEN_123\"";

        let calls = extract_tool_calls(&[OutputItem::ToolCall {
            call_id: "call-1".to_owned(),
            tool_name: "echo_tool".to_owned(),
            arguments: json!(bad_json),
            namespace: None,
        }]);

        assert_eq!(calls[0].arguments, bad_json);
    }

    #[test]
    fn tool_call_extraction_serializes_structured_arguments() {
        let calls = extract_tool_calls(&[OutputItem::ToolCall {
            call_id: "call-1".to_owned(),
            tool_name: "echo_tool".to_owned(),
            arguments: json!({"query": "rust"}),
            namespace: None,
        }]);

        assert_eq!(calls[0].arguments, "{\"query\":\"rust\"}");
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    struct EchoArgs {
        value: String,
    }

    #[tokio::test]
    async fn invalid_tool_json_default_error_output_redacts_payload() {
        let _lock = tool_logging_env_lock()
            .lock()
            .expect("tool logging env lock");
        let _env = EnvVarGuard::set("OPENAI_AGENTS_DONT_LOG_TOOL_DATA", "true");
        let tool = function_tool(
            "echo_tool",
            "Echo input",
            |_ctx, args: EchoArgs| async move { Ok::<_, AgentsError>(args.value) },
        )
        .expect("function tool should build");
        let agent = Agent::builder("assistant").function_tool(tool).build();
        let bad_json = "{\"secret\":\"SECRET_TOKEN_123\"";

        let outcome = execute_local_function_tools(
            &agent,
            &RunConfig::default(),
            &RunContextWrapper::new(RunContext::default()),
            vec![ToolCall {
                id: "call-1".to_owned(),
                name: "echo_tool".to_owned(),
                arguments: bad_json.to_owned(),
                namespace: None,
            }],
            None,
            None,
        )
        .await
        .expect("invalid JSON should be returned to the model by default");

        let RunItem::ToolCallOutput {
            output: OutputItem::Text { text },
            ..
        } = &outcome.new_items[0]
        else {
            panic!("expected text tool-call output");
        };
        assert_eq!(
            text,
            "Tool `echo_tool` failed: Invalid JSON input for tool echo_tool"
        );
        assert!(!text.contains("SECRET_TOKEN_123"));
    }

    #[tokio::test]
    async fn disabled_function_tool_call_fails_before_siblings_execute() {
        let disabled_invocations = Arc::new(AtomicUsize::new(0));
        let sibling_invocations = Arc::new(AtomicUsize::new(0));
        let disabled_invocations_for_tool = disabled_invocations.clone();
        let sibling_invocations_for_tool = sibling_invocations.clone();

        let disabled_tool = function_tool(
            "disabled_tool",
            "Disabled tool",
            move |_ctx, _args: serde_json::Value| {
                let disabled_invocations = disabled_invocations_for_tool.clone();
                async move {
                    disabled_invocations.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, AgentsError>("disabled result")
                }
            },
        )
        .expect("disabled function tool should build")
        .with_is_enabled(Arc::new(|_, _| async move { false }.boxed()));
        let sibling_tool = function_tool(
            "sibling_tool",
            "Sibling tool",
            move |_ctx, _args: serde_json::Value| {
                let sibling_invocations = sibling_invocations_for_tool.clone();
                async move {
                    sibling_invocations.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, AgentsError>("sibling result")
                }
            },
        )
        .expect("sibling function tool should build");
        let agent = Agent::builder("assistant")
            .function_tool(disabled_tool)
            .function_tool(sibling_tool)
            .build();

        let error = match execute_local_function_tools(
            &agent,
            &RunConfig::default(),
            &RunContextWrapper::new(RunContext::default()),
            vec![
                ToolCall {
                    id: "call-disabled".to_owned(),
                    name: "disabled_tool".to_owned(),
                    arguments: "{}".to_owned(),
                    namespace: None,
                },
                ToolCall {
                    id: "call-sibling".to_owned(),
                    name: "sibling_tool".to_owned(),
                    arguments: "{}".to_owned(),
                    namespace: None,
                },
            ],
            None,
            None,
        )
        .await
        {
            Ok(_) => panic!("disabled function tool should fail before execution"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("Tool disabled_tool is currently disabled for agent assistant.")
        );
        assert_eq!(disabled_invocations.load(Ordering::SeqCst), 0);
        assert_eq!(sibling_invocations.load(Ordering::SeqCst), 0);
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    struct TrackedArgs {
        value: usize,
    }

    #[tokio::test]
    async fn function_tool_concurrency_default_starts_all_calls() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let active_for_tool = active.clone();
        let max_seen_for_tool = max_seen.clone();
        let tool = function_tool(
            "tracked_tool",
            "Tracked tool",
            move |_ctx, args: TrackedArgs| {
                let active = active_for_tool.clone();
                let max_seen = max_seen_for_tool.clone();
                async move {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, AgentsError>(format!("ok-{}", args.value))
                }
            },
        )
        .expect("function tool should build");
        let agent = Agent::builder("assistant").function_tool(tool).build();

        let outcome = execute_local_function_tools(
            &agent,
            &RunConfig::default(),
            &RunContextWrapper::new(RunContext::default()),
            vec![
                ToolCall {
                    id: "call-1".to_owned(),
                    name: "tracked_tool".to_owned(),
                    arguments: json!({"value":1}).to_string(),
                    namespace: None,
                },
                ToolCall {
                    id: "call-2".to_owned(),
                    name: "tracked_tool".to_owned(),
                    arguments: json!({"value":2}).to_string(),
                    namespace: None,
                },
                ToolCall {
                    id: "call-3".to_owned(),
                    name: "tracked_tool".to_owned(),
                    arguments: json!({"value":3}).to_string(),
                    namespace: None,
                },
            ],
            None,
            None,
        )
        .await
        .expect("tools should execute");

        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(max_seen.load(Ordering::SeqCst), 3);
        assert_tool_text_outputs(&outcome.new_items, &["ok-1", "ok-2", "ok-3"]);
    }

    #[tokio::test]
    async fn function_tool_concurrency_cap_limits_calls_and_preserves_output_order() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let active_for_tool = active.clone();
        let max_seen_for_tool = max_seen.clone();
        let tool = function_tool(
            "tracked_tool",
            "Tracked tool",
            move |_ctx, args: TrackedArgs| {
                let active = active_for_tool.clone();
                let max_seen = max_seen_for_tool.clone();
                async move {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(now, Ordering::SeqCst);
                    let delay = if args.value == 1 { 30 } else { 1 };
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, AgentsError>(format!("ok-{}", args.value))
                }
            },
        )
        .expect("function tool should build");
        let agent = Agent::builder("assistant").function_tool(tool).build();
        let run_config = RunConfig {
            tool_execution: Some(crate::run_config::ToolExecutionConfig {
                max_function_tool_concurrency: Some(2),
            }),
            ..RunConfig::default()
        };

        let outcome = execute_local_function_tools(
            &agent,
            &run_config,
            &RunContextWrapper::new(RunContext::default()),
            vec![
                ToolCall {
                    id: "call-1".to_owned(),
                    name: "tracked_tool".to_owned(),
                    arguments: json!({"value":1}).to_string(),
                    namespace: None,
                },
                ToolCall {
                    id: "call-2".to_owned(),
                    name: "tracked_tool".to_owned(),
                    arguments: json!({"value":2}).to_string(),
                    namespace: None,
                },
                ToolCall {
                    id: "call-3".to_owned(),
                    name: "tracked_tool".to_owned(),
                    arguments: json!({"value":3}).to_string(),
                    namespace: None,
                },
            ],
            None,
            None,
        )
        .await
        .expect("tools should execute");

        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(max_seen.load(Ordering::SeqCst), 2);
        assert_tool_text_outputs(&outcome.new_items, &["ok-1", "ok-2", "ok-3"]);
    }

    #[tokio::test]
    async fn function_tool_concurrency_cap_leaves_queued_calls_unstarted_after_failure() {
        let failing_started = Arc::new(AtomicUsize::new(0));
        let queued_started = Arc::new(AtomicUsize::new(0));
        let failing_started_for_tool = failing_started.clone();
        let queued_started_for_tool = queued_started.clone();
        let failing_tool = function_tool(
            "failing_tool",
            "Failing tool",
            move |_ctx, _args: serde_json::Value| {
                let failing_started = failing_started_for_tool.clone();
                async move {
                    failing_started.fetch_add(1, Ordering::SeqCst);
                    Err::<String, _>(AgentsError::message("boom"))
                }
            },
        )
        .expect("function tool should build");
        let queued_tool = function_tool(
            "queued_tool",
            "Queued tool",
            move |_ctx, _args: serde_json::Value| {
                let queued_started = queued_started_for_tool.clone();
                async move {
                    queued_started.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, AgentsError>("should-not-run")
                }
            },
        )
        .expect("function tool should build");
        let agent = Agent::builder("assistant")
            .function_tool(failing_tool)
            .function_tool(queued_tool)
            .build();
        let run_config = RunConfig {
            tool_execution: Some(crate::run_config::ToolExecutionConfig {
                max_function_tool_concurrency: Some(1),
            }),
            tool_error_formatter: Some(Arc::new(|_| async { Ok(None) }.boxed())),
            ..RunConfig::default()
        };

        let error = match execute_local_function_tools(
            &agent,
            &run_config,
            &RunContextWrapper::new(RunContext::default()),
            vec![
                ToolCall {
                    id: "call-1".to_owned(),
                    name: "failing_tool".to_owned(),
                    arguments: "{}".to_owned(),
                    namespace: None,
                },
                ToolCall {
                    id: "call-2".to_owned(),
                    name: "queued_tool".to_owned(),
                    arguments: "{}".to_owned(),
                    namespace: None,
                },
            ],
            None,
            None,
        )
        .await
        {
            Ok(_) => panic!("failure should propagate"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("boom"));
        assert_eq!(failing_started.load(Ordering::SeqCst), 1);
        assert_eq!(queued_started.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn tool_execution_config_rejects_zero_function_tool_concurrency() {
        let config = crate::run_config::ToolExecutionConfig {
            max_function_tool_concurrency: Some(0),
        };

        let error = config.validate().expect_err("zero concurrency should fail");

        assert!(
            error
                .to_string()
                .contains("tool_execution.max_function_tool_concurrency must be at least 1")
        );
    }

    fn assert_tool_text_outputs(items: &[RunItem], expected: &[&str]) {
        let actual = items
            .iter()
            .map(|item| match item {
                RunItem::ToolCallOutput {
                    output: OutputItem::Text { text },
                    ..
                } => text.as_str(),
                _ => panic!("expected tool call output"),
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }
}
