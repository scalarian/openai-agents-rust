# Model Providers

Use this page when you need to decide where model resolution should live and how provider selection interacts with the runner.

## Main Types

- `ModelProvider`
- `OpenAIProvider`
- `MultiProvider`

## Default Pattern

Use one provider for the application and pass it into a shared `Runner` unless you have a real need for per-request provider switching.

## When `MultiProvider` Helps

`MultiProvider` is useful when:

- different model name prefixes map to different providers
- you want one facade with several model backends
- you want migration room without rewriting agents

## Read Next

- [settings.md](settings.md)
- [openai.md](openai.md)
