use std::fmt;
use std::sync::Arc;

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::Result;
use crate::guardrail::{InputGuardrail, OutputGuardrail};
use crate::handoff::Handoff;
use crate::run_context::{RunContext, RunContextWrapper};
use crate::stream_events::StreamEvent;
use crate::tool::{FunctionTool, FunctionToolResult, StaticTool};
use crate::tool_context::ToolCall;

pub type AgentBase = Agent;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopAtTools {
    #[serde(default)]
    pub stop_at_tool_names: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolsToFinalOutputResult {
    pub is_final_output: bool,
    pub final_output: Option<Value>,
}

impl ToolsToFinalOutputResult {
    pub fn not_final() -> Self {
        Self {
            is_final_output: false,
            final_output: None,
        }
    }

    pub fn final_output(final_output: Value) -> Self {
        Self {
            is_final_output: true,
            final_output: Some(final_output),
        }
    }
}

pub type ToolsToFinalOutputFunction = Arc<
    dyn Fn(
            RunContextWrapper<RunContext>,
            Vec<FunctionToolResult>,
        ) -> BoxFuture<'static, Result<ToolsToFinalOutputResult>>
        + Send
        + Sync,
>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentToolStreamEvent {
    pub event: StreamEvent,
    pub agent: Agent,
    pub tool_call: Option<ToolCall>,
}

#[derive(Clone, Default)]
pub enum ToolUseBehavior {
    #[default]
    RunLlmAgain,
    StopOnFirstTool,
    StopAtTools(StopAtTools),
    Custom(ToolsToFinalOutputFunction),
}

impl fmt::Debug for ToolUseBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RunLlmAgain => f.write_str("ToolUseBehavior::RunLlmAgain"),
            Self::StopOnFirstTool => f.write_str("ToolUseBehavior::StopOnFirstTool"),
            Self::StopAtTools(value) => f
                .debug_tuple("ToolUseBehavior::StopAtTools")
                .field(value)
                .finish(),
            Self::Custom(_) => f.write_str("ToolUseBehavior::Custom(<function>)"),
        }
    }
}

impl ToolUseBehavior {
    pub async fn evaluate(
        &self,
        context: &RunContextWrapper<RunContext>,
        tool_results: &[FunctionToolResult],
    ) -> Result<ToolsToFinalOutputResult> {
        if tool_results.is_empty() {
            return Ok(ToolsToFinalOutputResult::not_final());
        }

        match self {
            Self::RunLlmAgain => Ok(ToolsToFinalOutputResult::not_final()),
            Self::StopOnFirstTool => Ok(ToolsToFinalOutputResult::final_output(
                tool_results[0].final_output_value(),
            )),
            Self::StopAtTools(config) => {
                for result in tool_results {
                    if config.stop_at_tool_names.iter().any(|name| {
                        name == &result.tool_name
                            || result
                                .qualified_name
                                .as_ref()
                                .is_some_and(|qualified_name| qualified_name == name)
                    }) {
                        return Ok(ToolsToFinalOutputResult::final_output(
                            result.final_output_value(),
                        ));
                    }
                }
                Ok(ToolsToFinalOutputResult::not_final())
            }
            Self::Custom(handler) => handler(context.clone(), tool_results.to_vec()).await,
        }
    }
}

/// High-level agent definition.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub instructions: Option<String>,
    pub tools: Vec<StaticTool>,
    #[serde(skip, default)]
    pub function_tools: Vec<FunctionTool>,
    pub handoffs: Vec<Handoff>,
    pub input_guardrails: Vec<InputGuardrail>,
    pub output_guardrails: Vec<OutputGuardrail>,
    pub model: Option<String>,
    #[serde(skip, default)]
    pub tool_use_behavior: ToolUseBehavior,
}

impl Agent {
    pub fn builder(name: impl Into<String>) -> AgentBuilder {
        AgentBuilder::new(name)
    }

    pub fn tool_definitions(&self) -> Vec<crate::tool::ToolDefinition> {
        self.tools
            .iter()
            .map(|tool| tool.definition.clone())
            .collect()
    }

    pub fn find_function_tool(&self, name: &str, namespace: Option<&str>) -> Option<&FunctionTool> {
        self.function_tools.iter().find(|tool| {
            tool.definition.name == name && tool.definition.namespace.as_deref() == namespace
        })
    }

    pub fn find_handoff(&self, target: &str) -> Option<&Handoff> {
        self.handoffs
            .iter()
            .find(|handoff| handoff.target == target)
    }
}

/// Builder for [`Agent`].
#[derive(Clone, Debug)]
pub struct AgentBuilder {
    agent: Agent,
}

impl AgentBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            agent: Agent {
                name: name.into(),
                ..Agent::default()
            },
        }
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.agent.instructions = Some(instructions.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.agent.model = Some(model.into());
        self
    }

    pub fn tool(mut self, tool: StaticTool) -> Self {
        self.agent.tools.push(tool);
        self
    }

    pub fn function_tool(mut self, tool: FunctionTool) -> Self {
        self.agent.tools.push(StaticTool {
            definition: tool.definition.clone(),
        });
        self.agent.function_tools.push(tool);
        self
    }

    pub fn handoff(mut self, handoff: Handoff) -> Self {
        self.agent.handoffs.push(handoff);
        self
    }

    pub fn handoff_to_agent(mut self, agent: Agent) -> Self {
        self.agent.handoffs.push(Handoff::to_agent(agent));
        self
    }

    pub fn tool_use_behavior(mut self, tool_use_behavior: ToolUseBehavior) -> Self {
        self.agent.tool_use_behavior = tool_use_behavior;
        self
    }

    pub fn input_guardrail(mut self, guardrail: InputGuardrail) -> Self {
        self.agent.input_guardrails.push(guardrail);
        self
    }

    pub fn output_guardrail(mut self, guardrail: OutputGuardrail) -> Self {
        self.agent.output_guardrails.push(guardrail);
        self
    }

    pub fn build(self) -> Agent {
        self.agent
    }
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde::Deserialize;

    use crate::tool::function_tool;

    use super::*;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct SearchArgs {
        query: String,
    }

    #[test]
    fn stores_runtime_function_tools_and_serialized_definitions() {
        let tool = function_tool(
            "search",
            "Search documents",
            |_ctx, args: SearchArgs| async move {
                Ok::<_, crate::errors::AgentsError>(format!("result:{}", args.query))
            },
        )
        .expect("function tool should build");

        let agent = Agent::builder("assistant").function_tool(tool).build();

        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.function_tools.len(), 1);
        assert!(agent.find_function_tool("search", None).is_some());
    }

    #[tokio::test]
    async fn stop_at_tools_matches_public_and_qualified_names() {
        let behavior = ToolUseBehavior::StopAtTools(StopAtTools {
            stop_at_tool_names: vec![
                "lookup_account".to_owned(),
                "billing.lookup_account".to_owned(),
            ],
        });
        let context = RunContextWrapper::new(RunContext::default());

        let result = behavior
            .evaluate(
                &context,
                &[crate::tool::FunctionToolResult {
                    tool_name: "lookup_account".to_owned(),
                    qualified_name: Some("billing.lookup_account".to_owned()),
                    output: crate::tool::ToolOutput::from("ok"),
                    run_item: None,
                    interruptions: Vec::new(),
                    agent_run_result: None,
                }],
            )
            .await
            .expect("tool behavior should evaluate");

        assert!(result.is_final_output);
        assert_eq!(result.final_output, Some(Value::String("ok".to_owned())));
    }
}
