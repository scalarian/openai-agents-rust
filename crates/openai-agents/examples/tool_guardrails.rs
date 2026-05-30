use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    Agent, AgentsError, InputItem, Model, ModelProvider, ModelRequest, ModelResponse, OutputItem,
    Result as AgentsResult, Runner, ToolGuardrailFunctionOutput, ToolOutput, Usage, function_tool,
    tool_input_guardrail, tool_output_guardrail,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, JsonSchema)]
struct EmailArgs {
    to: String,
    subject: String,
    body: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UserArgs {
    user_id: String,
}

#[derive(Clone, Default)]
struct GuardrailDemoModel;

#[async_trait]
impl Model for GuardrailDemoModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if let Some(tool_output) = latest_tool_output(&request.input) {
            vec![OutputItem::Text {
                text: format!("tool_output={tool_output}"),
            }]
        } else {
            vec![planned_tool_call(&request.input)]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 8,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct GuardrailDemoProvider {
    model: Arc<GuardrailDemoModel>,
}

impl ModelProvider for GuardrailDemoProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

fn planned_tool_call(input: &[InputItem]) -> OutputItem {
    let prompt = input
        .iter()
        .filter_map(|item| item.as_text())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    if prompt.contains("contact info") {
        OutputItem::ToolCall {
            call_id: "call-contact".to_owned(),
            tool_name: "get_contact_info".to_owned(),
            arguments: json!({ "user_id": "user456" }),
            namespace: None,
        }
    } else if prompt.contains("user data") || prompt.contains("data for user") {
        OutputItem::ToolCall {
            call_id: "call-user".to_owned(),
            tool_name: "get_user_data".to_owned(),
            arguments: json!({ "user_id": "user123" }),
            namespace: None,
        }
    } else {
        let body = if prompt.contains("acme") {
            "Introducing ACME corp."
        } else {
            "Welcome aboard."
        };
        OutputItem::ToolCall {
            call_id: "call-email".to_owned(),
            tool_name: "send_email".to_owned(),
            arguments: json!({
                "to": "john@example.com",
                "subject": "Welcome",
                "body": body,
            }),
            namespace: None,
        }
    }
}

fn latest_tool_output(input: &[InputItem]) -> Option<String> {
    input.iter().rev().find_map(|item| match item {
        InputItem::Json { value }
            if value.get("type").and_then(Value::as_str) == Some("tool_call_output") =>
        {
            let output = value.get("output").cloned().unwrap_or(Value::Null);
            output
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| Some(output.to_string()))
        }
        _ => None,
    })
}

fn output_contains(output: &ToolOutput, needle: &str) -> bool {
    format!("{output:?}")
        .to_lowercase()
        .contains(&needle.to_lowercase())
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let reject_sensitive_words =
        tool_input_guardrail("reject-sensitive-words", |data| async move {
            let args: Value =
                serde_json::from_str(&data.context.tool_arguments).unwrap_or(Value::Null);
            let blocked_word = args.as_object().and_then(|object| {
                object.iter().find_map(|(key, value)| {
                    let value_text = value.to_string().to_lowercase();
                    ["password", "hack", "exploit", "malware", "acme"]
                        .iter()
                        .find(|word| value_text.contains(**word))
                        .map(|word| (key.clone(), (*word).to_owned()))
                })
            });

            if let Some((argument, word)) = blocked_word {
                Ok(ToolGuardrailFunctionOutput::reject_content(
                    format!("Tool call blocked: contains '{word}'"),
                    Some(json!({
                        "blocked_word": word,
                        "argument": argument
                    })),
                ))
            } else {
                Ok(ToolGuardrailFunctionOutput::allow(Some(json!({
                    "status": "input_validated"
                }))))
            }
        });

    let block_sensitive_output =
        tool_output_guardrail("block-sensitive-output", |data| async move {
            if output_contains(&data.output, "ssn") || output_contains(&data.output, "123-45-6789")
            {
                Ok(ToolGuardrailFunctionOutput::raise_exception(Some(json!({
                    "blocked_pattern": "SSN",
                    "tool": data.context.tool_name
                }))))
            } else {
                Ok(ToolGuardrailFunctionOutput::allow(Some(json!({
                    "status": "output_validated"
                }))))
            }
        });

    let reject_phone_numbers = tool_output_guardrail("reject-phone-numbers", |data| async move {
        if output_contains(&data.output, "555-1234") {
            Ok(ToolGuardrailFunctionOutput::reject_content(
                "User data not retrieved as it contains a phone number which is restricted.",
                Some(json!({ "redacted": "phone_number" })),
            ))
        } else {
            Ok(ToolGuardrailFunctionOutput::allow(Some(json!({
                "status": "phone_number_check_passed"
            }))))
        }
    });

    let send_email = function_tool(
        "send_email",
        "Send an email to the specified recipient.",
        |_ctx, args: EmailArgs| async move {
            Ok::<_, AgentsError>(format!(
                "Email sent to {} with subject '{}' and body '{}'",
                args.to, args.subject, args.body
            ))
        },
    )?
    .with_input_guardrail(reject_sensitive_words);

    let get_user_data = function_tool(
        "get_user_data",
        "Get user data by ID.",
        |_ctx, args: UserArgs| async move {
            Ok::<_, AgentsError>(json!({
                "user_id": args.user_id,
                "name": "John Doe",
                "email": "john@example.com",
                "ssn": "123-45-6789",
                "phone": "555-1234"
            }))
        },
    )?
    .with_output_guardrail(block_sensitive_output);

    let get_contact_info = function_tool(
        "get_contact_info",
        "Get contact info by ID.",
        |_ctx, args: UserArgs| async move {
            Ok::<_, AgentsError>(json!({
                "user_id": args.user_id,
                "name": "Jane Smith",
                "email": "jane@example.com",
                "phone": "555-1234"
            }))
        },
    )?
    .with_output_guardrail(reject_phone_numbers);

    let agent = Agent::builder("Secure Assistant")
        .instructions("Use the provided tools for email and user data requests.")
        .function_tool(send_email)
        .function_tool(get_user_data)
        .function_tool(get_contact_info)
        .build();

    let runner = Runner::new().with_model_provider(Arc::new(GuardrailDemoProvider::default()));

    let result = runner
        .run(&agent, "Send a welcome email to john@example.com")
        .await?;
    println!("normal_email={}", result.final_output.unwrap_or_default());

    let result = runner
        .run(
            &agent,
            "Send an email to john@example.com introducing ACME corp.",
        )
        .await?;
    println!("rejected_input={}", result.final_output.unwrap_or_default());

    match runner.run(&agent, "Get the data for user ID user123").await {
        Ok(result) => println!(
            "unexpected_user_data={}",
            result.final_output.unwrap_or_default()
        ),
        Err(AgentsError::ToolOutputGuardrailTripwire(error)) => {
            println!("output_guardrail_tripped={}", error.guardrail_name);
            println!("details={}", error.output.output_info.unwrap_or_default());
        }
        Err(error) => return Err(error),
    }

    let result = runner.run(&agent, "Get contact info for user456").await?;
    println!(
        "rejected_output={}",
        result.final_output.unwrap_or_default()
    );

    Ok(())
}
