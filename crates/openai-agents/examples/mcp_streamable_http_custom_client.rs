use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use openai_agents::{
    AgentsError, MCPServerStreamableHttp, MCPServerStreamableHttpParams, MCPTransportClient,
    MCPTransportClientConfig, Result as AgentsResult,
};

mod support;

use support::mcp_transport::{demo_tool_outputs, demo_tools, run_demo_questions};

#[derive(Debug)]
struct CustomTransportClient {
    config: MCPTransportClientConfig,
}

#[async_trait]
impl MCPTransportClient for CustomTransportClient {
    fn config(&self) -> &MCPTransportClientConfig {
        &self.config
    }

    async fn connect(&self) -> AgentsResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> AgentsResult<()> {
        Ok(())
    }

    fn session_id(&self) -> Option<String> {
        Some("custom-client-session".to_owned())
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let captured_configs = Arc::new(Mutex::new(Vec::new()));
    let captured_for_builder = captured_configs.clone();

    let server = MCPServerStreamableHttp::new(
        "Streamable HTTP with Custom Client",
        MCPServerStreamableHttpParams {
            url: "http://127.0.0.1:18080/mcp".to_owned(),
            headers: BTreeMap::from([
                (
                    "X-Custom-Client".to_owned(),
                    "agents-mcp-example".to_owned(),
                ),
                ("User-Agent".to_owned(), "OpenAI-Agents-MCP/1.0".to_owned()),
            ]),
            timeout_seconds: Some(60),
            sse_read_timeout_seconds: Some(120),
            ..MCPServerStreamableHttpParams::default()
        },
    )
    .with_client_builder(Arc::new(move |config| {
        captured_for_builder
            .lock()
            .expect("captured config mutex")
            .push(config.clone());
        Arc::new(CustomTransportClient { config })
    }))
    .with_tools(demo_tools())
    .with_tool_outputs(demo_tool_outputs());

    run_demo_questions(Arc::new(server)).await?;

    let configs = captured_configs.lock().expect("captured config mutex");
    if let Some(first) = configs.first() {
        println!(
            "custom_client headers={:?} timeout={:?} read_timeout={:?}",
            first.headers, first.timeout_seconds, first.sse_read_timeout_seconds
        );
    }

    Ok(())
}
