# Model Settings

Use this page when you need to shape provider requests without hardcoding those choices inside the agent instructions.

## What `ModelSettings` Is For

`ModelSettings` carries model request knobs such as:

- reasoning settings
- verbosity
- tool-choice-related behavior
- truncation and retry settings
- provider request extras such as headers, query params, and body fields

## Good Defaults

- keep stable defaults on the runner
- add request-specific overrides in `RunConfig`
- avoid storing provider request details in your agent instructions

Use `ModelSettings::retry` with `ModelRetrySettings` and `retry_policies` when the runner should retry transient model failures instead of leaving recovery to the model provider.

## Read Next

- [providers.md](providers.md)
- [openai.md](openai.md)
- [../config.md](../config.md)
