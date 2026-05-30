use serde::{Deserialize, Serialize};

use agents_core::{FunctionTool, Handoff, StaticTool};

use crate::config::{RealtimeSessionModelSettings, RealtimeSessionTool};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RealtimeAgent {
    pub name: String,
    pub handoff_description: Option<String>,
    pub instructions: Option<String>,
    pub model_settings: Option<RealtimeSessionModelSettings>,
    pub tools: Vec<StaticTool>,
    #[serde(skip, default)]
    pub function_tools: Vec<FunctionTool>,
    pub handoffs: Vec<Handoff>,
}

impl RealtimeAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            handoff_description: None,
            instructions: None,
            model_settings: None,
            tools: Vec::new(),
            function_tools: Vec::new(),
            handoffs: Vec::new(),
        }
    }

    pub fn with_handoff_description(mut self, description: impl Into<String>) -> Self {
        self.handoff_description = Some(description.into());
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn with_model_settings(mut self, model_settings: RealtimeSessionModelSettings) -> Self {
        self.model_settings = Some(model_settings);
        self
    }

    pub fn with_tool(mut self, tool: StaticTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_function_tool(mut self, tool: FunctionTool) -> Self {
        self.tools.push(StaticTool {
            definition: tool.definition.clone(),
        });
        self.function_tools.push(tool);
        self
    }

    pub fn with_handoff(mut self, handoff: Handoff) -> Self {
        self.handoffs.push(handoff);
        self
    }

    pub fn with_handoff_agent(mut self, agent: RealtimeAgent) -> Self {
        self.handoffs.push(crate::handoffs::realtime_handoff(agent));
        self
    }

    pub fn tool_definitions(&self) -> Vec<agents_core::ToolDefinition> {
        self.tools
            .iter()
            .map(|tool| tool.definition.clone())
            .collect()
    }

    pub fn session_tools(&self) -> Vec<RealtimeSessionTool> {
        let mut tools = self
            .tool_definitions()
            .iter()
            .map(RealtimeSessionTool::from_tool_definition)
            .collect::<Vec<_>>();

        tools.extend(
            self.handoffs
                .iter()
                .filter(|handoff| handoff.enabled)
                .map(|handoff| {
                    RealtimeSessionTool::function(
                        handoff.tool_name.clone(),
                        handoff.tool_description.clone(),
                        handoff.input_json_schema.clone(),
                    )
                }),
        );

        tools
    }
}

pub trait RealtimeAgentHooks: Send + Sync {}

pub trait RealtimeRunHooks: Send + Sync {}

#[cfg(test)]
mod tests {
    use agents_core::{Handoff, StaticTool};

    use super::*;

    #[test]
    fn realtime_agent_collects_tool_and_handoff_session_tools() {
        let agent = RealtimeAgent::new("triage")
            .with_tool(StaticTool::new("get_weather", "Get the weather."))
            .with_handoff(Handoff::new("faq").with_tool_name("transfer_to_faq_agent"));

        let tools = agent.session_tools();

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].kind, "function");
        assert_eq!(tools[0].name, "get_weather");
        assert_eq!(tools[1].name, "transfer_to_faq_agent");
    }
}
