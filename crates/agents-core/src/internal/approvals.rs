use crate::items::{OutputItem, RunItem};
use crate::run_context::ApprovalRecord;
use crate::run_state::{RunInterruption, RunInterruptionKind};

pub(crate) const REJECTION_MESSAGE: &str = "Tool execution was not approved.";

pub(crate) fn append_approval_error_output(
    items: &mut Vec<RunItem>,
    tool_name: String,
    call_id: String,
    namespace: Option<String>,
    approval: Option<&ApprovalRecord>,
) {
    let message = approval
        .and_then(|approval| approval.reason.as_deref())
        .unwrap_or(REJECTION_MESSAGE);
    items.push(RunItem::ToolCallOutput {
        tool_name,
        output: OutputItem::Text {
            text: message.to_owned(),
        },
        call_id: Some(call_id),
        namespace,
    });
}

pub(crate) fn approvals_from_step(step: &Option<RunInterruption>) -> Vec<String> {
    step.as_ref()
        .filter(|step| matches!(step.kind, Some(RunInterruptionKind::ToolApproval)))
        .and_then(|step| step.call_id.clone())
        .into_iter()
        .collect()
}

pub(crate) fn filter_tool_approvals(items: &[RunItem], approvals: &[String]) -> Vec<RunItem> {
    items
        .iter()
        .filter(|item| match item {
            RunItem::ToolCall { call_id, .. } | RunItem::ToolCallOutput { call_id, .. } => !call_id
                .as_deref()
                .is_some_and(|call_id| approvals.iter().any(|candidate| candidate == call_id)),
            _ => true,
        })
        .cloned()
        .collect()
}
