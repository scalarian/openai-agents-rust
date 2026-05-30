use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use openai_agents::{AgentsError, MCPServer, MCPServerManager, MCPTool, ToolOutput};
use serde_json::{Value, json};

#[derive(Clone)]
struct AddServer {
    name: &'static str,
    fail_connect: bool,
    connected: Arc<AtomicBool>,
}

impl AddServer {
    fn new(name: &'static str, fail_connect: bool) -> Self {
        Self {
            name,
            fail_connect,
            connected: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl MCPServer for AddServer {
    fn name(&self) -> &str {
        self.name
    }

    async fn connect(&self) -> Result<(), AgentsError> {
        if self.fail_connect {
            return Err(AgentsError::message(format!(
                "{} is intentionally unavailable",
                self.name
            )));
        }
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), AgentsError> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<MCPTool>, AgentsError> {
        Ok(vec![MCPTool {
            name: "add".to_owned(),
            description: Some("Add two numbers.".to_owned()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "a": { "type": "integer" },
                    "b": { "type": "integer" }
                },
                "required": ["a", "b"]
            })),
            ..MCPTool::default()
        }])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _meta: Option<Value>,
    ) -> Result<ToolOutput, AgentsError> {
        if tool_name != "add" {
            return Err(AgentsError::message(format!("unknown tool `{tool_name}`")));
        }
        let a = arguments
            .get("a")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let b = arguments
            .get("b")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        Ok(json!({ "sum": a + b }).into())
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let stable: Arc<dyn MCPServer> = Arc::new(AddServer::new("calculator", false));
    let inactive: Arc<dyn MCPServer> = Arc::new(AddServer::new("inactive", true));
    let mut manager = MCPServerManager::new([stable.clone(), inactive]);

    manager.connect_all().await?;
    println!("connected_servers={:?}", manager.active_server_names());
    println!("failed_servers={:?}", manager.failed_servers);

    let tools = manager.list_tools_for_active().await?;
    for (server, server_tools) in tools {
        println!(
            "server={} tools={:?}",
            server.name(),
            server_tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>()
        );
    }

    let result = stable
        .call_tool("add", json!({ "a": 4, "b": 9 }), None)
        .await?;
    println!(
        "add_result={}",
        serde_json::to_value(result).unwrap_or(Value::Null)
    );

    manager.cleanup_all().await?;
    Ok(())
}
