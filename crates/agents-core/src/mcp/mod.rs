//! Model Context Protocol support.

pub mod manager;
pub mod server;
pub mod util;

pub use manager::MCPServerManager;
pub use server::{
    MCPBlobResourceContents, MCPListResourceTemplatesResult, MCPListResourcesResult,
    MCPReadResourceResult, MCPResource, MCPResourceContents, MCPResourceTemplate, MCPServer,
    MCPServerSse, MCPServerSseParams, MCPServerStdio, MCPServerStdioParams,
    MCPServerStreamableHttp, MCPServerStreamableHttpParams, MCPTextResourceContents, MCPTool,
    MCPToolAnnotations, MCPTransportAuth, MCPTransportClient, MCPTransportClientBuilder,
    MCPTransportClientConfig, MCPTransportKind, RequireApprovalObject, RequireApprovalPolicy,
    RequireApprovalSetting, RequireApprovalToolList, RequireApprovalValue,
};
pub use util::{
    MCPToolMetaContext, MCPToolMetaResolver, MCPUtil, ToolFilter, ToolFilterCallable,
    ToolFilterContext, ToolFilterStatic, create_static_tool_filter,
};
