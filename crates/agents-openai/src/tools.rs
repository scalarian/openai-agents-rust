use agents_core::StaticTool;

pub fn web_search_tool() -> StaticTool {
    StaticTool::new("web_search", "Search the public web.")
}

pub fn file_search_tool() -> StaticTool {
    StaticTool::new("file_search", "Search indexed files.")
}

pub fn code_interpreter_tool() -> StaticTool {
    StaticTool::new("code_interpreter", "Run short code snippets in a sandbox.")
}

pub fn tool_search_tool() -> StaticTool {
    StaticTool::new("tool_search", "Search tools available to the runtime.")
}

pub fn image_generation_tool() -> StaticTool {
    StaticTool::new("image_generation", "Generate or edit images.")
}
