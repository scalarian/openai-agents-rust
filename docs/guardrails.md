# Guardrails

Use this page when you need runtime checks that can stop or rewrite unsafe inputs, outputs, or tool traffic.

## Guardrail Families

| Guardrail | Runs against |
| --- | --- |
| input guardrail | user input and prepared model input |
| output guardrail | final model output |
| tool input guardrail | tool arguments before execution |
| tool output guardrail | tool result before it becomes model-visible |

## What Guardrails Are Good At

- blocking unsafe requests
- sanitizing tool input
- stopping sensitive outputs
- enforcing business or compliance checks
- turning policy into reusable runtime code

## What Guardrails Are Not

Guardrails are not a substitute for:

- secure tool implementations
- proper authorization
- application-level validation
- model-independent business rules that belong before the runtime

## Tripwires

Guardrail failures surface as explicit runtime results and errors rather than being hidden inside free-form model text.

That makes them suitable for:

- audit trails
- test assertions
- operator intervention
- retry or fallback logic

## Read Next

- [tools.md](tools.md)
- [human_in_the_loop.md](human_in_the_loop.md)
- [tracing.md](tracing.md)
