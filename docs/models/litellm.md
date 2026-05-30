# LiteLLM And AnyLLM

Use this page when you want to route model calls through a LiteLLM-compatible or any-llm-compatible gateway.

The Rust adapters live in `openai_agents::extensions`:

- `LitellmModel` and `LitellmProvider`
- `AnyLLMModel`, `AnyLLMProvider`, and `AnyLLMApi`

Both providers implement the shared `ModelProvider` trait, so you can attach them to a `Runner` without changing your agents.

## LiteLLM Provider

`LitellmProvider` routes through the Chat Completions request path using an OpenAI-compatible base URL.

```rust,no_run
use std::sync::Arc;

use openai_agents::extensions::LitellmProvider;
use openai_agents::{Agent, AgentsError, Runner};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .model("openrouter/openai/gpt-5.4-mini")
        .instructions("Reply briefly.")
        .build();

    let provider = LitellmProvider::new()
        .with_base_url("https://openrouter.ai/api/v1")
        .with_api_key(std::env::var("OPENROUTER_API_KEY").unwrap_or_default());
    let runner = Runner::new().with_model_provider(Arc::new(provider));

    let result = runner.run(&agent, "Hello").await?;
    println!("{}", result.final_output_text().unwrap_or_default());
    Ok(())
}
```

Runnable examples:

- [litellm_provider.rs](../../crates/openai-agents/examples/litellm_provider.rs)
- [litellm_auto.rs](../../crates/openai-agents/examples/litellm_auto.rs)
- [retry_litellm.rs](../../crates/openai-agents/examples/retry_litellm.rs)

The examples skip the live request when no `OPENROUTER_API_KEY` is configured.

## AnyLLM Provider

`AnyLLMProvider` can route through either Chat Completions or Responses-compatible APIs.

```rust,no_run
use std::sync::Arc;

use openai_agents::extensions::{AnyLLMApi, AnyLLMProvider};
use openai_agents::{Agent, AgentsError, Runner};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let agent = Agent::builder("Assistant")
        .model("openrouter/openai/gpt-5.4-mini")
        .build();

    let provider = AnyLLMProvider::new()
        .with_base_url("https://openrouter.ai/api/v1")
        .with_api_key(std::env::var("OPENROUTER_API_KEY").unwrap_or_default())
        .with_api(AnyLLMApi::ChatCompletions);
    let runner = Runner::new().with_model_provider(Arc::new(provider));

    let result = runner.run(&agent, "Hello").await?;
    println!("{}", result.final_output_text().unwrap_or_default());
    Ok(())
}
```

Runnable examples:

- [any_llm_provider.rs](../../crates/openai-agents/examples/any_llm_provider.rs)
- [any_llm_auto.rs](../../crates/openai-agents/examples/any_llm_auto.rs)

## Choosing An Adapter

| Use case | Adapter |
| --- | --- |
| LiteLLM gateway with Chat Completions semantics | `LitellmProvider` |
| OpenAI-compatible gateway where you choose Responses or Chat Completions | `AnyLLMProvider` |
| Multiple providers behind prefixes in one runner | `MultiProvider` from the core model surface |

Hosted OpenAI tools and Responses-only surfaces depend on the selected backend. Validate tool support against the gateway you plan to run in production.

## Read Next

- [README.md](README.md)
- [providers.md](providers.md)
- [settings.md](settings.md)
- [../examples.md](../examples.md)
