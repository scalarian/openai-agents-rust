# Multi-Agent Patterns

Use this page when a single agent stops being enough and you need delegation, specialization, or routing.

## The Three Main Patterns

| Pattern | Use it when |
| --- | --- |
| nested agent as tool | the parent should decide when to delegate |
| explicit handoff | control should move from one agent to another |
| router and specialists | one agent chooses between several domain roles |

## Start Simple

Do not begin with five agents because the architecture diagram looks impressive. Start with:

1. one agent
2. a small tool set
3. clear output expectations

Add another agent only when a role genuinely needs:

- different instructions
- different tool visibility
- different output constraints
- different approval or session behavior

## Nested Agent Tools

`Agent::as_tool()` is the cleanest option when the parent agent should stay in charge and simply call a specialist as one tool in its toolkit.

This is usually the right choice for:

- code review specialists
- search or retrieval specialists
- translation or formatting specialists

## Handoffs

Use a handoff when control should actually move to another agent instead of appearing as a tool call in the parent run.

Read [handoffs.md](handoffs.md) for history filtering and handoff-specific settings.

## Session Guidance

If multiple agents participate in one broader workflow, decide early whether they should share session history or keep independent state. This affects:

- replay
- conversation continuity
- auditability
- prompt size over time

## Read Next

- [agents.md](agents.md)
- [handoffs.md](handoffs.md)
- [tools.md](tools.md)
