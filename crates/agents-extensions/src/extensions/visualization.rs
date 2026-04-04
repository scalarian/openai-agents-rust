use std::collections::BTreeSet;

use agents_core::{Agent, Handoff};

/// Generates the full graph in DOT format for an agent and its runtime handoffs.
pub fn get_main_graph(agent: &Agent) -> String {
    let parts = vec![
        "digraph G {\n    graph [splines=true];\n    node [fontname=\"Arial\"];\n    edge [penwidth=1.5];\n".to_owned(),
        get_all_nodes(agent),
        get_all_edges(agent),
        "}\n".to_owned(),
    ];
    parts.join("")
}

/// Generates DOT nodes for the agent graph.
pub fn get_all_nodes(agent: &Agent) -> String {
    let mut visited = BTreeSet::new();
    let mut parts = Vec::new();
    collect_nodes(agent, None, &mut visited, &mut parts);
    parts.join("")
}

/// Generates DOT edges for the agent graph.
pub fn get_all_edges(agent: &Agent) -> String {
    let mut visited = BTreeSet::new();
    let mut parts = Vec::new();
    collect_edges(agent, None, &mut visited, &mut parts);
    parts.join("")
}

/// Returns the rendered DOT text.
pub fn draw_graph(agent: &Agent) -> String {
    get_main_graph(agent)
}

fn collect_nodes(
    agent: &Agent,
    parent: Option<&Agent>,
    visited: &mut BTreeSet<String>,
    parts: &mut Vec<String>,
) {
    if !visited.insert(agent.name.clone()) {
        return;
    }

    if parent.is_none() {
        parts.push(
            "\"__start__\" [label=\"__start__\", shape=ellipse, style=filled, fillcolor=lightblue, width=0.5, height=0.3];\
             \"__end__\" [label=\"__end__\", shape=ellipse, style=filled, fillcolor=lightblue, width=0.5, height=0.3];"
                .to_owned(),
        );
        parts.push(format!(
            "\"{}\" [label=\"{}\", shape=box, style=filled, fillcolor=lightyellow, width=1.5, height=0.8];",
            agent.name, agent.name
        ));
    }

    for tool in &agent.tools {
        parts.push(format!(
            "\"{}\" [label=\"{}\", shape=ellipse, style=filled, fillcolor=lightgreen, width=0.5, height=0.3];",
            tool.definition.name, tool.definition.name
        ));
    }

    for handoff in &agent.handoffs {
        add_handoff_node(handoff, visited, parts);
    }
}

fn add_handoff_node(handoff: &Handoff, visited: &mut BTreeSet<String>, parts: &mut Vec<String>) {
    parts.push(format!(
        "\"{}\" [label=\"{}\", shape=box, style=\"filled,rounded\", fillcolor=lightyellow, width=1.5, height=0.8];",
        handoff.target, handoff.target
    ));
    if let Some(agent) = handoff.runtime_agent() {
        collect_nodes(agent, Some(agent), visited, parts);
    }
}

fn collect_edges(
    agent: &Agent,
    parent: Option<&Agent>,
    visited: &mut BTreeSet<String>,
    parts: &mut Vec<String>,
) {
    if !visited.insert(agent.name.clone()) {
        return;
    }

    if parent.is_none() {
        parts.push(format!("\"__start__\" -> \"{}\";", agent.name));
    }

    for tool in &agent.tools {
        parts.push(format!(
            "\"{}\" -> \"{}\" [style=dotted, penwidth=1.5];\"{}\" -> \"{}\" [style=dotted, penwidth=1.5];",
            agent.name, tool.definition.name, tool.definition.name, agent.name
        ));
    }

    if agent.handoffs.is_empty() {
        parts.push(format!("\"{}\" -> \"__end__\";", agent.name));
    }

    for handoff in &agent.handoffs {
        parts.push(format!("\"{}\" -> \"{}\";", agent.name, handoff.target));
        if let Some(target) = handoff.runtime_agent() {
            collect_edges(target, Some(agent), visited, parts);
        }
    }
}

#[cfg(test)]
mod tests {
    use agents_core::Agent;

    use super::*;

    #[test]
    fn renders_handoffs_and_tools() {
        let specialist = Agent::builder("specialist").build();
        let root = Agent::builder("root")
            .handoff_to_agent(specialist)
            .tool(agents_core::StaticTool::new("search", "Search"))
            .build();

        let graph = get_main_graph(&root);
        assert!(graph.contains("\"root\" -> \"specialist\";"));
        assert!(graph.contains("\"search\""));
    }
}
