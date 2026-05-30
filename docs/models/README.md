# Models

Use this section when you want to understand how model selection works, what the default OpenAI path does, and where model-level settings belong.

## In This Section

- [index.md](index.md): upstream-compatible entry point for this section
- [providers.md](providers.md): provider selection and shared runtime model routing
- [settings.md](settings.md): `ModelSettings`, defaults, and overrides
- [openai.md](openai.md): OpenAI-specific model behavior, including Responses vs Chat Completions
- [litellm.md](litellm.md): LiteLLM and AnyLLM adapter setup

## The Big Idea

The runtime separates:

- the agent definition
- the model provider
- model settings
- session and continuation state

That separation lets you keep agents stable while swapping providers, request settings, and conversation-aware session behavior.

## Read Next

- [providers.md](providers.md)
- [settings.md](settings.md)
- [litellm.md](litellm.md)
- [../config.md](../config.md)
