use std::sync::Arc;

use openai_agents::{AgentsError, MCPServerStreamableHttp, MCPServerStreamableHttpParams};

mod support;

use support::mcp_transport::{demo_tool_outputs, demo_tools, run_demo_questions};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let server = MCPServerStreamableHttp::new(
        "Streamable HTTP Python Server",
        MCPServerStreamableHttpParams {
            url: "http://127.0.0.1:18080/mcp".to_owned(),
            timeout_seconds: Some(5),
            sse_read_timeout_seconds: Some(30),
            ..MCPServerStreamableHttpParams::default()
        },
    )
    .with_tools(demo_tools())
    .with_tool_outputs(demo_tool_outputs());

    run_demo_questions(Arc::new(server)).await
}
