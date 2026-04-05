use std::sync::OnceLock;

use openai_agents::{
    Agent, AgentAsToolInput, AgentAsToolOptions, AgentRunner, RunConfig, RunContext,
    RunContextWrapper, Runner, Tool, ToolContext, ToolOutput, drop_agent_tool_run_result,
    get_default_agent_runner, run, run_sync, set_default_agent_runner,
};
use serde_json::json;

fn default_runner_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

struct DefaultRunnerReset(AgentRunner);

impl Drop for DefaultRunnerReset {
    fn drop(&mut self) {
        set_default_agent_runner(Some(self.0.clone()));
    }
}

#[tokio::test]
async fn facade_free_run_uses_configured_default_runner() {
    let _guard = default_runner_lock().lock().await;
    let original_runner = get_default_agent_runner();
    let _reset = DefaultRunnerReset(original_runner.clone());
    set_default_agent_runner(Some(Runner::new().with_config(RunConfig {
        model: Some("gpt-facade-default".to_owned()),
        ..RunConfig::default()
    })));

    let agent = Agent::builder("assistant").build();
    let result = run(&agent, "hello")
        .await
        .expect("facade run should succeed");

    assert_eq!(
        result
            .raw_responses
            .first()
            .and_then(|response| response.model.as_deref()),
        Some("gpt-facade-default")
    );
}

#[tokio::test]
async fn facade_run_sync_rejects_active_runtime() {
    let agent = Agent::builder("assistant").build();

    let error = run_sync(&agent, "hello").expect_err("run_sync should reject active runtimes");

    assert!(error.to_string().contains("event loop is already running"));
}

#[tokio::test]
async fn facade_agent_as_tool_runs_nested_agent() {
    let agent = Agent::builder("nested").build();
    let tool = agent
        .as_tool::<AgentAsToolInput>(
            Some("nested_tool"),
            Some("Invoke the nested agent"),
            AgentAsToolOptions::default(),
        )
        .expect("agent tool should build");

    let call_id = "call-facade-nested";
    let output = tool
        .invoke(
            ToolContext::new(
                RunContextWrapper::new(RunContext::default()),
                "nested_tool",
                call_id,
                "{\"input\":\"hello\"}",
            ),
            json!({"input":"hello"}),
        )
        .await
        .expect("agent tool should execute");

    assert_eq!(output, ToolOutput::from("hello"));
    let stored = openai_agents::peek_agent_tool_run_result(call_id, None)
        .expect("nested run result should be recorded");
    assert_eq!(stored.final_output.as_deref(), Some("hello"));
    drop_agent_tool_run_result(call_id, None);
}
