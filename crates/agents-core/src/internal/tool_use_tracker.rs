use std::collections::{HashMap, HashSet};

use crate::agent::Agent;
use crate::model_settings::ModelSettings;

#[derive(Clone, Debug, Default)]
pub(crate) struct AgentToolUseTracker {
    agent_to_tools: HashMap<String, HashSet<String>>,
}

impl AgentToolUseTracker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_tool_use(
        &mut self,
        agent: &Agent,
        tool_names: impl IntoIterator<Item = String>,
    ) {
        let tool_names = tool_names.into_iter().collect::<Vec<_>>();
        if tool_names.is_empty() {
            return;
        }
        let entry = self
            .agent_to_tools
            .entry(agent_tracker_key(agent))
            .or_default();
        entry.extend(tool_names);
    }

    pub(crate) fn has_used_tools(&self, agent: &Agent) -> bool {
        self.agent_to_tools
            .get(&agent_tracker_key(agent))
            .map(|tools| !tools.is_empty())
            .unwrap_or(false)
    }

    pub(crate) fn maybe_reset_tool_choice(
        &self,
        agent: &Agent,
        mut settings: ModelSettings,
    ) -> ModelSettings {
        if agent.reset_tool_choice && self.has_used_tools(agent) {
            settings.tool_choice = None;
        }
        settings
    }

    pub(crate) fn as_serializable(&self) -> HashMap<String, Vec<String>> {
        self.agent_to_tools
            .iter()
            .map(|(name, tools)| {
                let mut values = tools.iter().cloned().collect::<Vec<_>>();
                values.sort();
                (name.clone(), values)
            })
            .collect()
    }

    pub(crate) fn from_serializable(snapshot: HashMap<String, Vec<String>>) -> Self {
        let agent_to_tools = snapshot
            .into_iter()
            .map(|(name, tools)| (name, tools.into_iter().collect()))
            .collect();
        Self { agent_to_tools }
    }
}

fn agent_tracker_key(agent: &Agent) -> String {
    agent
        .sandbox_identity_key
        .clone()
        .unwrap_or_else(|| agent.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_tools_by_agent_name() {
        let mut tracker = AgentToolUseTracker::new();
        let agent = Agent::builder("router").build();

        tracker.add_tool_use(&agent, ["search".to_owned(), "lookup".to_owned()]);

        assert!(tracker.has_used_tools(&agent));
        assert_eq!(
            tracker.as_serializable().get("router"),
            Some(&vec!["lookup".to_owned(), "search".to_owned()])
        );
    }

    #[test]
    fn tracks_tools_by_runtime_identity_key() {
        let mut tracker = AgentToolUseTracker::new();
        let first = Agent::builder("duplicate").build();
        let second = Agent::builder("duplicate").build().clone_with(|agent| {
            agent.sandbox_identity_key = Some("duplicate#2".to_owned());
        });

        tracker.add_tool_use(&second, ["approval_tool".to_owned()]);

        assert!(!tracker.has_used_tools(&first));
        assert!(tracker.has_used_tools(&second));
        assert_eq!(
            tracker.as_serializable().get("duplicate#2"),
            Some(&vec!["approval_tool".to_owned()])
        );
    }
}
