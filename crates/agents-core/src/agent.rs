use serde::{Deserialize, Serialize};

use crate::guardrail::{InputGuardrail, OutputGuardrail};
use crate::handoff::Handoff;
use crate::tool::StaticTool;

/// High-level agent definition.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub instructions: Option<String>,
    pub tools: Vec<StaticTool>,
    pub handoffs: Vec<Handoff>,
    pub input_guardrails: Vec<InputGuardrail>,
    pub output_guardrails: Vec<OutputGuardrail>,
    pub model: Option<String>,
}

impl Agent {
    pub fn builder(name: impl Into<String>) -> AgentBuilder {
        AgentBuilder::new(name)
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

    pub fn handoff(mut self, handoff: Handoff) -> Self {
        self.agent.handoffs.push(handoff);
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
