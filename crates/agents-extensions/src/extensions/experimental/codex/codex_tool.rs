use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

use agents_core::{AgentsError, FunctionTool, Result, ToolContext, function_tool};

use crate::extensions::experimental::codex::codex::Codex;
use crate::extensions::experimental::codex::codex_options::CodexOptions;
use crate::extensions::experimental::codex::events::{ThreadEvent, Usage};
use crate::extensions::experimental::codex::thread::{Input, Thread, UserInput};
use crate::extensions::experimental::codex::thread_options::ThreadOptions;
use crate::extensions::experimental::codex::turn_options::TurnOptions;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexToolInputItem {
    Text { text: String },
    LocalImage { path: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CodexToolParameters {
    pub inputs: Vec<CodexToolInputItem>,
    pub thread_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CodexToolRunContextParameters {
    pub inputs: Vec<CodexToolInputItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaPrimitive {
    #[serde(rename = "type")]
    pub type_name: String,
    pub description: Option<String>,
    pub r#enum: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaArray {
    #[serde(rename = "type")]
    pub type_name: String,
    pub description: Option<String>,
    pub items: Option<Box<OutputSchemaPrimitive>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaPropertyDescriptor {
    pub name: String,
    pub description: Option<String>,
    pub schema: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchemaDescriptor {
    pub title: Option<String>,
    pub description: Option<String>,
    pub properties: Option<Vec<OutputSchemaPropertyDescriptor>>,
    pub required: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexToolResult {
    pub thread_id: Option<String>,
    pub response: String,
    pub usage: Option<Usage>,
}

impl CodexToolResult {
    pub fn as_dict(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[derive(Clone, Debug)]
pub struct CodexToolStreamEvent {
    pub event: ThreadEvent,
    pub thread: Thread,
    pub tool_call: Value,
}

#[derive(Clone, Debug, Default)]
pub struct CodexToolOptions {
    pub name: Option<String>,
    pub description: Option<String>,
    pub output_schema: Option<OutputSchemaDescriptor>,
    pub codex: Option<Codex>,
    pub codex_options: Option<CodexOptions>,
    pub default_thread_options: Option<ThreadOptions>,
    pub thread_id: Option<String>,
    pub default_turn_options: Option<TurnOptions>,
    pub persist_session: bool,
}

pub fn codex_tool(options: CodexToolOptions) -> Result<FunctionTool> {
    let tool_name = options.name.clone().unwrap_or_else(|| "codex".to_owned());
    let description = options
        .description
        .clone()
        .unwrap_or_else(|| "Run a Codex CLI task".to_owned());
    let output_schema = options
        .output_schema
        .as_ref()
        .map(build_codex_output_schema)
        .transpose()?;
    let codex = options.codex.clone();
    let codex_options = options.codex_options.clone();
    let default_thread_options = options.default_thread_options.clone();
    let default_turn_options =
        build_turn_options(options.default_turn_options.clone(), output_schema);
    let pinned_thread_id = options.thread_id.clone();

    function_tool(
        tool_name,
        description,
        move |_ctx: ToolContext, args: CodexToolParameters| {
            let codex = codex.clone();
            let codex_options = codex_options.clone();
            let default_thread_options = default_thread_options.clone();
            let default_turn_options = default_turn_options.clone();
            let pinned_thread_id = pinned_thread_id.clone();
            async move {
                let codex = match codex.clone() {
                    Some(codex) => codex,
                    None => Codex::new(codex_options.clone())?,
                };

                let thread_id = args.thread_id.clone().or(pinned_thread_id.clone());
                let mut thread = match thread_id {
                    Some(thread_id) => {
                        codex.resume_thread(thread_id, default_thread_options.clone())
                    }
                    None => codex.start_thread(default_thread_options.clone()),
                };

                let turn = thread
                    .run(
                        Input::Items(
                            args.inputs
                                .into_iter()
                                .map(|item| match item {
                                    CodexToolInputItem::Text { text } => UserInput::Text { text },
                                    CodexToolInputItem::LocalImage { path } => {
                                        UserInput::LocalImage { path }
                                    }
                                })
                                .collect(),
                        ),
                        default_turn_options.clone(),
                    )
                    .await?;

                let result = CodexToolResult {
                    thread_id: thread.id.clone(),
                    response: turn.final_response,
                    usage: turn.usage,
                };
                Ok::<_, agents_core::AgentsError>(json!(result))
            }
        },
    )
}

fn build_turn_options(
    defaults: Option<TurnOptions>,
    output_schema: Option<Value>,
) -> Option<TurnOptions> {
    match (defaults, output_schema) {
        (None, None) => None,
        (None, Some(output_schema)) => Some(TurnOptions {
            output_schema: Some(output_schema),
            ..Default::default()
        }),
        (Some(mut defaults), output_schema) => {
            if output_schema.is_some() {
                defaults.output_schema = output_schema;
            }
            Some(defaults)
        }
    }
}

fn build_codex_output_schema(descriptor: &OutputSchemaDescriptor) -> Result<Value> {
    let properties = descriptor
        .properties
        .as_ref()
        .filter(|props| !props.is_empty())
        .ok_or_else(|| {
            AgentsError::message("Codex output schema descriptor must include properties")
        })?;

    let mut seen = BTreeSet::new();
    let mut schema_properties = Map::new();
    for property in properties {
        let name = property.name.trim();
        if name.is_empty() {
            return Err(AgentsError::message(
                "Codex output schema properties must include non-empty names",
            ));
        }
        if !seen.insert(name.to_owned()) {
            return Err(AgentsError::message(format!(
                "Duplicate property name `{name}` in output_schema"
            )));
        }

        let field = property.schema.as_ref().ok_or_else(|| {
            AgentsError::message(format!("Invalid schema for output property `{name}`"))
        })?;
        let mut property_schema = build_codex_output_schema_field(field).map_err(|_| {
            AgentsError::message(format!("Invalid schema for output property `{name}`"))
        })?;
        if let Some(description) = property
            .description
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            if let Some(object) = property_schema.as_object_mut() {
                object.insert("description".to_owned(), Value::String(description.clone()));
            }
        }
        schema_properties.insert(name.to_owned(), property_schema);
    }

    let required = descriptor.required.clone().unwrap_or_default();
    for name in &required {
        if !seen.contains(name) {
            return Err(AgentsError::message(format!(
                "Required property `{name}` must also be defined in properties"
            )));
        }
    }

    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    schema.insert("additionalProperties".to_owned(), Value::Bool(false));
    schema.insert("properties".to_owned(), Value::Object(schema_properties));
    schema.insert(
        "required".to_owned(),
        Value::Array(required.into_iter().map(Value::String).collect()),
    );
    if let Some(title) = descriptor.title.as_ref().filter(|value| !value.is_empty()) {
        schema.insert("title".to_owned(), Value::String(title.clone()));
    }
    if let Some(description) = descriptor
        .description
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        schema.insert("description".to_owned(), Value::String(description.clone()));
    }

    Ok(Value::Object(schema))
}

fn build_codex_output_schema_field(field: &Value) -> Result<Value> {
    let object = field
        .as_object()
        .ok_or_else(|| AgentsError::message("Codex output schema fields must be JSON objects"))?;
    match object.get("type").and_then(Value::as_str) {
        Some("array") => {
            let items = object.get("items").ok_or_else(|| {
                AgentsError::message("Codex output schema arrays must include items")
            })?;
            let mut schema = Map::new();
            schema.insert("type".to_owned(), Value::String("array".to_owned()));
            schema.insert("items".to_owned(), build_codex_output_schema_field(items)?);
            if let Some(description) = object
                .get("description")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                schema.insert(
                    "description".to_owned(),
                    Value::String(description.to_owned()),
                );
            }
            Ok(Value::Object(schema))
        }
        Some("string" | "number" | "integer" | "boolean") => {
            let mut schema = Map::new();
            let type_name = object
                .get("type")
                .and_then(Value::as_str)
                .expect("primitive type checked");
            schema.insert("type".to_owned(), Value::String(type_name.to_owned()));
            if let Some(description) = object
                .get("description")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                schema.insert(
                    "description".to_owned(),
                    Value::String(description.to_owned()),
                );
            }
            if let Some(enum_value) = object.get("enum") {
                let values = enum_value.as_array().ok_or_else(|| {
                    AgentsError::message("Codex output schema enum must be a string array")
                })?;
                if !values.iter().all(|value| value.as_str().is_some()) {
                    return Err(AgentsError::message(
                        "Codex output schema enum must be a string array",
                    ));
                }
                schema.insert("enum".to_owned(), Value::Array(values.clone()));
            }
            Ok(Value::Object(schema))
        }
        _ => Err(AgentsError::message(
            "Codex output schema fields must use primitive or array types",
        )),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn codex_tool_output_schema_descriptor_builds_turn_schema_without_mutating_input() {
        let nested = json!({
            "type": "array",
            "description": "Tags array",
            "items": {
                "type": "string",
                "description": "Tag value"
            }
        });
        let descriptor = OutputSchemaDescriptor {
            title: Some("Report".to_owned()),
            description: Some("Structured output".to_owned()),
            properties: Some(vec![OutputSchemaPropertyDescriptor {
                name: "tags".to_owned(),
                description: Some("Tag list".to_owned()),
                schema: Some(nested.clone()),
            }]),
            required: Some(vec!["tags".to_owned()]),
        };

        let schema = build_codex_output_schema(&descriptor).expect("schema should build");

        assert_eq!(
            descriptor.properties.as_ref().unwrap()[0].schema,
            Some(nested)
        );
        assert_eq!(schema["title"], json!("Report"));
        assert_eq!(schema["description"], json!("Structured output"));
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(schema["required"], json!(["tags"]));
        assert_eq!(
            schema["properties"]["tags"]["description"],
            json!("Tag list")
        );
        assert_eq!(
            schema["properties"]["tags"]["items"]["description"],
            json!("Tag value")
        );
    }

    #[test]
    fn codex_tool_output_schema_overrides_default_turn_schema() {
        let default_schema = json!({
            "type": "object",
            "properties": {
                "old": { "type": "string" }
            }
        });
        let output_schema = json!({
            "type": "object",
            "properties": {
                "new": { "type": "string" }
            }
        });
        let defaults = TurnOptions {
            output_schema: Some(default_schema.clone()),
            idle_timeout_seconds: Some(2.0),
            ..Default::default()
        };

        let merged = build_turn_options(Some(defaults.clone()), Some(output_schema.clone()))
            .expect("merged options should exist");

        assert_eq!(defaults.output_schema, Some(default_schema));
        assert_eq!(merged.output_schema, Some(output_schema));
        assert_eq!(merged.idle_timeout_seconds, Some(2.0));
    }

    #[test]
    fn codex_tool_output_schema_rejects_invalid_required_property() {
        let descriptor = OutputSchemaDescriptor {
            title: None,
            description: None,
            properties: Some(vec![OutputSchemaPropertyDescriptor {
                name: "name".to_owned(),
                description: None,
                schema: Some(json!({ "type": "string" })),
            }]),
            required: Some(vec!["missing".to_owned()]),
        };

        let error = build_codex_output_schema(&descriptor).expect_err("schema should be rejected");

        assert!(
            error
                .to_string()
                .contains("Required property `missing` must also be defined")
        );
    }
}
