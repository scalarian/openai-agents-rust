use std::env;
use std::sync::Arc;

use futures::FutureExt;
use openai_agents::extensions::LitellmProvider;
use openai_agents::{
    Agent, AgentsError, ModelRetryBackoffSettings, ModelRetrySettings, ModelSettings,
    MultiProvider, MultiProviderMap, OpenAIProvider, RetryDecision, RetryPolicy,
    RetryPolicyContext, RunConfig, Runner, retry_policies, set_tracing_disabled,
};

const DEFAULT_MODEL: &str = "litellm/openai/gpt-4o-mini";
const DEFAULT_LITELLM_BASE_URL: &str = "http://localhost:4000/v1";

fn format_error(context: &RetryPolicyContext) -> String {
    context
        .error_message
        .clone()
        .unwrap_or_else(|| "unknown error".to_owned())
}

fn logging_retry_policy() -> RetryPolicy {
    let apply_policies = retry_policies::any(vec![
        retry_policies::provider_suggested(),
        retry_policies::retry_after(),
        retry_policies::network_error(),
        retry_policies::http_status([408, 409, 429, 500, 502, 503, 504]),
    ]);

    Arc::new(move |context: RetryPolicyContext| {
        let apply_policies = apply_policies.clone();
        async move {
            let decision = apply_policies(context.clone()).await;
            if decision.retry {
                let delay = decision
                    .delay
                    .map(|value| format!("waiting {value:.2}s"))
                    .unwrap_or_else(|| "using default backoff".to_owned());
                let reason = decision.reason.clone().unwrap_or_default();
                println!(
                    "[retry] retry attempt {}/{} | {delay} | reason: {reason} | error: {}",
                    context.attempt,
                    context.max_retries + 1,
                    format_error(&context)
                );
            } else {
                println!(
                    "[retry] stop after attempt {}/{}: {}",
                    context.attempt,
                    context.max_retries + 1,
                    format_error(&context)
                );
            }
            RetryDecision {
                retry: decision.retry,
                delay: decision.delay,
                reason: decision.reason,
            }
        }
        .boxed()
    })
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    set_tracing_disabled(true);

    let api_key = env::var("LITELLM_API_KEY")
        .or_else(|_| env::var("OPENAI_API_KEY"))
        .unwrap_or_else(|_| "dummy".to_owned());
    if api_key == "dummy" {
        println!("Skipping run because neither LITELLM_API_KEY nor OPENAI_API_KEY was provided.");
        return Ok(());
    }

    let base_url =
        env::var("LITELLM_BASE_URL").unwrap_or_else(|_| DEFAULT_LITELLM_BASE_URL.to_owned());
    let retry = ModelRetrySettings {
        max_retries: Some(4),
        backoff: Some(ModelRetryBackoffSettings {
            initial_delay: Some(0.5),
            max_delay: Some(5.0),
            multiplier: Some(2.0),
            jitter: Some(true),
        }),
        policy: Some(logging_retry_policy()),
    };
    let run_config = RunConfig {
        model_settings: Some(ModelSettings {
            retry: Some(retry.clone()),
            ..ModelSettings::default()
        }),
        ..RunConfig::default()
    };

    let agent = Agent::builder("Assistant")
        .instructions("You are a concise assistant. Answer in 3 short bullet points at most.")
        .model(DEFAULT_MODEL)
        .model_settings(ModelSettings {
            retry: Some(retry),
            ..ModelSettings::default()
        })
        .build();

    let mut provider_map = MultiProviderMap::default();
    provider_map.add_provider(
        "litellm",
        Arc::new(
            LitellmProvider::new()
                .with_api_key(api_key)
                .with_base_url(base_url),
        ),
    );
    let provider =
        MultiProvider::new(Arc::new(OpenAIProvider::new())).with_provider_map(provider_map);
    let result = Runner::new()
        .with_model_provider(Arc::new(provider))
        .with_config(run_config)
        .run(
            &agent,
            "Explain exponential backoff for API retries in plain English.",
        )
        .await?;

    println!("\nFinal output:\n");
    println!("{}", result.final_output.unwrap_or_default());
    Ok(())
}
