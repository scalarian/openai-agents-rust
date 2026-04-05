# Human In The Loop

Use this page when a tool, shell action, MCP call, or workflow step should require operator review before it executes or before its result is accepted.

## The Main Runtime Primitives

- approval-required tools
- MCP approval request and response items
- interruptions
- run hooks and error handlers

## Typical Uses

- approving destructive shell commands
- reviewing external tool usage
- pausing before sensitive data leaves the system
- requiring confirmation before handoffs or escalations

## Approval Design Rules

Keep approval prompts:

- specific
- reviewable
- bound to one concrete action
- traceable to a tool name and call id

The runtime already carries the call identity. Use that instead of inventing your own ad-hoc approval bookkeeping.

## Interruptions

Interruptions let you stop or branch a run without losing the run state model. That is the right surface for:

- user cancellations
- operator pauses
- “approval required” workflows

## Read Next

- [tools.md](tools.md)
- [mcp.md](mcp.md)
- [results.md](results.md)
