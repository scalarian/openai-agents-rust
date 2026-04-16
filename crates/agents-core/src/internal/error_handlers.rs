use serde_json::Value;

use crate::OutputSchemaDefinition;
use crate::exceptions::ModelBehaviorError;
use crate::items::OutputItem;
use crate::run_error_handlers::RunErrorHandlerResult;

pub(crate) fn format_final_output_text(final_output: &Value) -> String {
    match final_output {
        Value::String(text) => text.clone(),
        _ => {
            serde_json::to_string_pretty(final_output).unwrap_or_else(|_| final_output.to_string())
        }
    }
}

pub(crate) fn create_message_output_item(final_output: &Value) -> OutputItem {
    match final_output {
        Value::String(text) => OutputItem::Text { text: text.clone() },
        _ => OutputItem::Json {
            value: final_output.clone(),
        },
    }
}

pub(crate) fn resolve_run_error_handler_result(
    result: Option<RunErrorHandlerResult>,
) -> Option<(String, OutputItem, bool)> {
    result.map(|result| {
        let final_output_text = format_final_output_text(&result.final_output);
        let output_item = create_message_output_item(&result.final_output);
        (final_output_text, output_item, result.include_in_history)
    })
}

pub(crate) fn validate_handler_final_output(
    result: &Value,
    output_schema: Option<&OutputSchemaDefinition>,
) -> crate::errors::Result<()> {
    if matches!(result, Value::Null) {
        return Err(crate::errors::AgentsError::message(
            "run error handler must return a non-null final_output",
        ));
    }

    if let Some(output_schema) = output_schema {
        let validator = jsonschema::validator_for(&output_schema.schema).map_err(|error| {
            crate::errors::AgentsError::from(ModelBehaviorError {
                message: format!(
                    "run error handler final_output could not compile structured output schema `{}`: {error}",
                    output_schema.name
                ),
            })
        })?;
        validator.validate(result).map_err(|error| {
            crate::errors::AgentsError::from(ModelBehaviorError {
                message: format!(
                    "run error handler final_output did not match structured output schema `{}`: {error}",
                    output_schema.name
                ),
            })
        })?;
    }

    Ok(())
}
