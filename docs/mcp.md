# MCP

Use this page when you want to discover tools and resources from Model Context Protocol servers instead of baking every tool into the process.

## What The Runtime Supports

- server lifecycle management
- dynamic tool discovery
- tool filtering
- approval-aware execution
- MCP resources and resource templates
- normalized tool/resource outputs

## Main Types

- `MCPServer`
- `MCPServerManager`
- `MCPServerStdio`
- `MCPServerSse`
- `MCPServerStreamableHttp`
- `MCPTool`
- `MCPResource`

## How To Think About MCP

MCP is not just “more tools.” It is a runtime boundary with its own lifecycle:

1. connect to servers
2. discover tools and resources
3. filter and approve what is visible
4. execute or read through normalized runtime items
5. clean up connections

## Resource Support

The runtime exposes resources and templates in addition to callable tools. That matters when a server is better treated as a structured information source than as an imperative tool executor.

## Server Selection

Use:

- stdio when the server is local and process-backed
- SSE or streamable HTTP when the server is remote

## Read Next

- [tools.md](tools.md)
- [human_in_the_loop.md](human_in_the_loop.md)
- [ref/runtime.md](ref/runtime.md)
