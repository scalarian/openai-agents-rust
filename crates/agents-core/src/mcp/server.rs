use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::errors::{AgentsError, Result};
use crate::exceptions::UserError;
use crate::tool::ToolOutput;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RequireApprovalToolList {
    #[serde(default)]
    pub tool_names: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RequireApprovalObject {
    pub always: Option<RequireApprovalToolList>,
    pub never: Option<RequireApprovalToolList>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MCPToolAnnotations {
    pub title: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
    pub title: Option<String>,
    pub annotations: Option<MCPToolAnnotations>,
    pub meta: Option<Value>,
    pub namespace: Option<String>,
    pub requires_approval: bool,
}

impl MCPTool {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub meta: Option<Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPResourceTemplate {
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub meta: Option<Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPTextResourceContents {
    pub uri: String,
    pub text: String,
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPBlobResourceContents {
    pub uri: String,
    pub blob: String,
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MCPResourceContents {
    Text(MCPTextResourceContents),
    Blob(MCPBlobResourceContents),
    Json {
        uri: String,
        value: Value,
        mime_type: Option<String>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPListResourcesResult {
    pub resources: Vec<MCPResource>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPListResourceTemplatesResult {
    pub resource_templates: Vec<MCPResourceTemplate>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPReadResourceResult {
    pub contents: Vec<MCPResourceContents>,
}

#[async_trait]
pub trait MCPServer: Send + Sync {
    fn name(&self) -> &str;

    async fn connect(&self) -> Result<()>;

    async fn cleanup(&self) -> Result<()>;

    async fn list_tools(&self) -> Result<Vec<MCPTool>>;

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        meta: Option<Value>,
    ) -> Result<ToolOutput>;

    async fn list_resources(&self, _cursor: Option<String>) -> Result<MCPListResourcesResult> {
        Err(AgentsError::User(UserError {
            message: "list_resources is not implemented".to_owned(),
        }))
    }

    async fn list_resource_templates(
        &self,
        _cursor: Option<String>,
    ) -> Result<MCPListResourceTemplatesResult> {
        Err(AgentsError::User(UserError {
            message: "list_resource_templates is not implemented".to_owned(),
        }))
    }

    async fn read_resource(&self, _uri: &str) -> Result<MCPReadResourceResult> {
        Err(AgentsError::User(UserError {
            message: "read_resource is not implemented".to_owned(),
        }))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MCPServerStdioParams {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MCPServerSseParams {
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MCPServerStreamableHttpParams {
    pub url: String,
}

#[derive(Clone)]
pub struct MCPServerStdio {
    name: String,
    pub params: MCPServerStdioParams,
    connected: Arc<AtomicBool>,
    resources: Arc<Mutex<Vec<MCPResource>>>,
    resource_templates: Arc<Mutex<Vec<MCPResourceTemplate>>>,
    resource_contents: Arc<Mutex<HashMap<String, MCPReadResourceResult>>>,
}

impl fmt::Debug for MCPServerStdio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MCPServerStdio")
            .field("name", &self.name)
            .field("params", &self.params)
            .finish()
    }
}

impl MCPServerStdio {
    pub fn new(name: impl Into<String>, params: MCPServerStdioParams) -> Self {
        Self {
            name: name.into(),
            params,
            connected: Arc::new(AtomicBool::new(false)),
            resources: Arc::new(Mutex::new(Vec::new())),
            resource_templates: Arc::new(Mutex::new(Vec::new())),
            resource_contents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_resources(mut self, resources: Vec<MCPResource>) -> Self {
        self.resources = Arc::new(Mutex::new(resources));
        self
    }

    pub fn with_resource_templates(mut self, resource_templates: Vec<MCPResourceTemplate>) -> Self {
        self.resource_templates = Arc::new(Mutex::new(resource_templates));
        self
    }

    pub fn with_resource_content(
        mut self,
        uri: impl Into<String>,
        result: MCPReadResourceResult,
    ) -> Self {
        self.resource_contents = Arc::new(Mutex::new(HashMap::from([(uri.into(), result)])));
        self
    }

    fn ensure_connected(&self) -> Result<()> {
        if self.connected.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(AgentsError::User(UserError {
                message: "Server not initialized".to_owned(),
            }))
        }
    }
}

#[async_trait]
impl MCPServer for MCPServerStdio {
    fn name(&self) -> &str {
        &self.name
    }

    async fn connect(&self) -> Result<()> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<MCPTool>> {
        Ok(Vec::new())
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        _arguments: Value,
        _meta: Option<Value>,
    ) -> Result<ToolOutput> {
        Ok(ToolOutput::from(format!("mcp:{tool_name}")))
    }

    async fn list_resources(&self, _cursor: Option<String>) -> Result<MCPListResourcesResult> {
        self.ensure_connected()?;
        Ok(MCPListResourcesResult {
            resources: self.resources.lock().await.clone(),
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _cursor: Option<String>,
    ) -> Result<MCPListResourceTemplatesResult> {
        self.ensure_connected()?;
        Ok(MCPListResourceTemplatesResult {
            resource_templates: self.resource_templates.lock().await.clone(),
            next_cursor: None,
        })
    }

    async fn read_resource(&self, uri: &str) -> Result<MCPReadResourceResult> {
        self.ensure_connected()?;
        self.resource_contents
            .lock()
            .await
            .get(uri)
            .cloned()
            .ok_or_else(|| {
                AgentsError::User(UserError {
                    message: format!("resource `{uri}` not found"),
                })
            })
    }
}

#[derive(Clone)]
pub struct MCPServerStreamableHttp {
    name: String,
    pub params: MCPServerStreamableHttpParams,
    connected: Arc<AtomicBool>,
    resources: Arc<Mutex<Vec<MCPResource>>>,
    resource_templates: Arc<Mutex<Vec<MCPResourceTemplate>>>,
    resource_contents: Arc<Mutex<HashMap<String, MCPReadResourceResult>>>,
}

impl fmt::Debug for MCPServerStreamableHttp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MCPServerStreamableHttp")
            .field("name", &self.name)
            .field("params", &self.params)
            .finish()
    }
}

impl MCPServerStreamableHttp {
    pub fn new(name: impl Into<String>, params: MCPServerStreamableHttpParams) -> Self {
        Self {
            name: name.into(),
            params,
            connected: Arc::new(AtomicBool::new(false)),
            resources: Arc::new(Mutex::new(Vec::new())),
            resource_templates: Arc::new(Mutex::new(Vec::new())),
            resource_contents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_resources(mut self, resources: Vec<MCPResource>) -> Self {
        self.resources = Arc::new(Mutex::new(resources));
        self
    }

    pub fn with_resource_templates(mut self, resource_templates: Vec<MCPResourceTemplate>) -> Self {
        self.resource_templates = Arc::new(Mutex::new(resource_templates));
        self
    }

    pub fn with_resource_content(
        mut self,
        uri: impl Into<String>,
        result: MCPReadResourceResult,
    ) -> Self {
        self.resource_contents = Arc::new(Mutex::new(HashMap::from([(uri.into(), result)])));
        self
    }

    fn ensure_connected(&self) -> Result<()> {
        if self.connected.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(AgentsError::User(UserError {
                message: "Server not initialized".to_owned(),
            }))
        }
    }
}

#[async_trait]
impl MCPServer for MCPServerStreamableHttp {
    fn name(&self) -> &str {
        &self.name
    }

    async fn connect(&self) -> Result<()> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<MCPTool>> {
        Ok(Vec::new())
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        _arguments: Value,
        _meta: Option<Value>,
    ) -> Result<ToolOutput> {
        Ok(ToolOutput::from(format!("mcp:{tool_name}")))
    }

    async fn list_resources(&self, _cursor: Option<String>) -> Result<MCPListResourcesResult> {
        self.ensure_connected()?;
        Ok(MCPListResourcesResult {
            resources: self.resources.lock().await.clone(),
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _cursor: Option<String>,
    ) -> Result<MCPListResourceTemplatesResult> {
        self.ensure_connected()?;
        Ok(MCPListResourceTemplatesResult {
            resource_templates: self.resource_templates.lock().await.clone(),
            next_cursor: None,
        })
    }

    async fn read_resource(&self, uri: &str) -> Result<MCPReadResourceResult> {
        self.ensure_connected()?;
        self.resource_contents
            .lock()
            .await
            .get(uri)
            .cloned()
            .ok_or_else(|| {
                AgentsError::User(UserError {
                    message: format!("resource `{uri}` not found"),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn streamable_http_resources_require_connection_and_roundtrip() {
        let server = MCPServerStreamableHttp::new(
            "docs",
            MCPServerStreamableHttpParams {
                url: "http://localhost:8000/mcp".to_owned(),
            },
        )
        .with_resources(vec![MCPResource {
            uri: "file:///readme.md".to_owned(),
            name: "readme.md".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            ..MCPResource::default()
        }])
        .with_resource_templates(vec![MCPResourceTemplate {
            uri_template: "file:///{path}".to_owned(),
            name: "file".to_owned(),
            ..MCPResourceTemplate::default()
        }])
        .with_resource_content(
            "file:///readme.md",
            MCPReadResourceResult {
                contents: vec![MCPResourceContents::Text(MCPTextResourceContents {
                    uri: "file:///readme.md".to_owned(),
                    text: "# Hello".to_owned(),
                    mime_type: Some("text/markdown".to_owned()),
                })],
            },
        );

        let error = server
            .list_resources(None)
            .await
            .expect_err("unconnected resource listing should fail");
        assert!(matches!(error, AgentsError::User(_)));

        server.connect().await.expect("connect should succeed");

        let resources = server
            .list_resources(Some("cursor".to_owned()))
            .await
            .expect("resources should load");
        let templates = server
            .list_resource_templates(None)
            .await
            .expect("resource templates should load");
        let content = server
            .read_resource("file:///readme.md")
            .await
            .expect("resource contents should load");

        assert_eq!(resources.resources.len(), 1);
        assert_eq!(templates.resource_templates.len(), 1);
        assert_eq!(content.contents.len(), 1);
    }
}
