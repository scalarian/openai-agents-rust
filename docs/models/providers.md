# Model Providers

Use this page when you need to decide where model resolution should live and how provider selection interacts with the runner.

## Main Types

- `ModelProvider`
- `OpenAIProvider`
- `MultiProvider`

## Default Pattern

Use one provider for the application and pass it into a shared `Runner` unless you have a real need for per-request provider switching.

Use `RunOptions::model_provider` when a single run should resolve models from a custom provider. See [custom_model_provider.rs](../../crates/openai-agents/examples/custom_model_provider.rs).
Use `Agent::model(...)` when one agent should request a specific model name from the provider. See [custom_agent_model.rs](../../crates/openai-agents/examples/custom_agent_model.rs).
Use `set_default_agent_runner(...)` when facade calls such as `run(...)` should share a custom provider. See [default_model_provider.rs](../../crates/openai-agents/examples/default_model_provider.rs).

## When `MultiProvider` Helps

`MultiProvider` is useful when:

- different model name prefixes map to different providers
- you want one facade with several model backends
- you want migration room without rewriting agents

## Read Next

- [settings.md](settings.md)
- [openai.md](openai.md)
