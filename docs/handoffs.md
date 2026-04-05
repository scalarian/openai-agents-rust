# Handoffs

Use this page when you want control to move from one agent to another instead of staying inside one agent’s tool call.

## What A Handoff Does

A handoff transfers control from one agent to another while preserving the runtime’s understanding of:

- history
- run state
- tracing
- session continuity

## Why Handoffs Exist

Use a handoff when:

- the next agent should become the active agent
- the new agent should answer directly
- the history shape needs explicit filtering or nesting

Use an agent-as-tool instead when the parent agent should remain the one in charge.

## History Control

The handoff path supports:

- handoff input filters
- nested handoff history
- handoff history mappers
- default handoff history mapping helpers

That matters when you want to avoid blindly forwarding the entire transcript into every downstream specialist.

## A Healthy Default

Start with explicit handoffs only when ownership of the conversation truly changes. Otherwise prefer a specialist tool.

## Read Next

- [multi_agent.md](multi_agent.md)
- [config.md](config.md)
- [sessions/README.md](sessions/README.md)
