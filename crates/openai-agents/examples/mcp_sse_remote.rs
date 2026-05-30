use std::env;
use std::sync::Arc;

use openai_agents::{AgentsError, MCPServerSse, MCPServerSseParams};

mod support;

use support::mcp_transport::{demo_tool_outputs, demo_tools, run_demo_questions};

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let url = env::var("MCP_SSE_REMOTE_URL").unwrap_or_else(|_| {
        println!("MCP_SSE_REMOTE_URL is not set; using the local SSE demo URL.");
        "http://127.0.0.1:8000/sse".to_owned()
    });

    let server = MCPServerSse::new(
        "Remote SSE Server",
        MCPServerSseParams {
            url,
            timeout_seconds: Some(5),
            sse_read_timeout_seconds: Some(30),
            ..MCPServerSseParams::default()
        },
    )
    .with_tools(demo_tools())
    .with_tool_outputs(demo_tool_outputs());

    run_demo_questions(Arc::new(server)).await
}
