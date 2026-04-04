use crate::items::OutputItem;
use crate::tool_context::ToolCall;

#[derive(Clone, Debug, Default)]
pub(crate) struct ToolExecutionPlan {
    pub tool_calls: Vec<ToolCall>,
    pub has_handoff: bool,
}

pub(crate) fn build_tool_execution_plan(output: &[OutputItem]) -> ToolExecutionPlan {
    ToolExecutionPlan {
        tool_calls: super::tool_execution::extract_tool_calls(output),
        has_handoff: output
            .iter()
            .any(|item| matches!(item, OutputItem::Handoff { .. })),
    }
}
