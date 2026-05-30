use openai_agents::{
    InputItem, MCPListToolsItem, ToolSearchCallItem, ToolSearchOutputItem, code_interpreter_tool,
    file_search_tool, image_generation_tool, tool_search_tool, web_search_tool,
};
use serde_json::json;

#[test]
fn facade_hosted_tool_helpers_are_constructible() {
    let tool_names = vec![
        code_interpreter_tool().definition.name,
        file_search_tool().definition.name,
        image_generation_tool().definition.name,
        tool_search_tool().definition.name,
        web_search_tool().definition.name,
    ];

    assert_eq!(
        tool_names,
        vec![
            "code_interpreter",
            "file_search",
            "image_generation",
            "tool_search",
            "web_search",
        ]
    );
}

#[test]
fn facade_exports_hosted_tool_search_run_items() {
    let call_item = ToolSearchCallItem {
        raw_item: InputItem::Json {
            value: json!({
                "type": "tool_search_call",
                "call_id": "search-1",
                "created_by": "server",
            }),
        },
    };
    let output_item = ToolSearchOutputItem {
        raw_item: InputItem::Json {
            value: json!({
                "type": "tool_search_output",
                "call_id": "search-1",
                "created_by": "server",
                "results": [{"name": "lookup"}],
            }),
        },
    };
    let mcp_item = MCPListToolsItem {
        raw_item: InputItem::Json {
            value: json!({
                "type": "mcp_list_tools",
                "server_label": "docs",
                "tools": [{"name": "lookup"}],
            }),
        },
    };

    assert_eq!(
        call_item.to_input_item(),
        InputItem::Json {
            value: json!({
                "type": "tool_search_call",
                "call_id": "search-1",
            })
        }
    );
    assert_eq!(
        output_item.to_input_item(),
        InputItem::Json {
            value: json!({
                "type": "tool_search_output",
                "call_id": "search-1",
                "results": [{"name": "lookup"}],
            })
        }
    );
    assert_eq!(
        mcp_item.to_input_item(),
        InputItem::Json {
            value: json!({
                "type": "mcp_list_tools",
                "server_label": "docs",
                "tools": [{"name": "lookup"}],
            })
        }
    );
}
