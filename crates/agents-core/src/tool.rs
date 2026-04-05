use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use futures::FutureExt;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::_tool_identity::tool_qualified_name;
use crate::computer::Computer;
use crate::errors::{AgentsError, Result};
use crate::function_schema::FunctionSchema;
use crate::items::{OutputItem, RunItem};
use crate::run_config::ToolErrorFormatterArgs;
use crate::run_context::{RunContext, RunContextWrapper};
use crate::tool_context::ToolContext;
use crate::tool_guardrails::{ToolInputGuardrail, ToolOutputGuardrail};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub namespace: Option<String>,
    pub strict_json_schema: bool,
    pub input_json_schema: Option<Value>,
    pub defer_loading: bool,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            namespace: None,
            strict_json_schema: true,
            input_json_schema: None,
            defer_loading: false,
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_input_json_schema(mut self, schema: Value) -> Self {
        self.input_json_schema = Some(schema);
        self
    }

    pub fn with_defer_loading(mut self, defer_loading: bool) -> Self {
        self.defer_loading = defer_loading;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolOutputText {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolOutputImage {
    pub image_url: Option<String>,
    pub file_id: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolOutputFileContent {
    pub file_data: Option<String>,
    pub file_url: Option<String>,
    pub file_id: Option<String>,
    pub filename: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolOutput {
    Text(ToolOutputText),
    Image(ToolOutputImage),
    File(ToolOutputFileContent),
    Json { value: Value },
}

impl ToolOutput {
    pub fn to_output_item(&self) -> OutputItem {
        match self {
            Self::Text(value) => OutputItem::Text {
                text: value.text.clone(),
            },
            Self::Json { value } => OutputItem::Json {
                value: value.clone(),
            },
            Self::Image(value) => OutputItem::Json {
                value: serde_json::to_value(value).unwrap_or(Value::Null),
            },
            Self::File(value) => OutputItem::Json {
                value: serde_json::to_value(value).unwrap_or(Value::Null),
            },
        }
    }
}

impl From<String> for ToolOutput {
    fn from(value: String) -> Self {
        Self::Text(ToolOutputText { text: value })
    }
}

impl From<&str> for ToolOutput {
    fn from(value: &str) -> Self {
        Self::Text(ToolOutputText {
            text: value.to_owned(),
        })
    }
}

impl From<Value> for ToolOutput {
    fn from(value: Value) -> Self {
        Self::Json { value }
    }
}

#[derive(Clone, Debug)]
pub struct FunctionToolResult {
    pub tool_name: String,
    pub qualified_name: Option<String>,
    pub output: ToolOutput,
    pub run_item: Option<RunItem>,
    pub interruptions: Vec<RunItem>,
    pub agent_run_result: Option<Value>,
}

impl FunctionToolResult {
    pub fn final_output_value(&self) -> Value {
        match &self.output {
            ToolOutput::Text(value) => Value::String(value.text.clone()),
            _ => serde_json::to_value(&self.output).unwrap_or(Value::Null),
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> &ToolDefinition;

    async fn invoke(&self, _context: ToolContext, _args: Value) -> Result<ToolOutput> {
        Err(AgentsError::message(format!(
            "tool `{}` cannot be invoked directly",
            self.definition().name
        )))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StaticTool {
    pub definition: ToolDefinition,
}

impl StaticTool {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            definition: ToolDefinition::new(name, description),
        }
    }
}

#[async_trait]
impl Tool for StaticTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}

type ToolExecutor =
    Arc<dyn Fn(ToolContext, Value) -> BoxFuture<'static, Result<ToolOutput>> + Send + Sync>;

#[derive(Clone)]
pub struct FunctionTool {
    pub definition: ToolDefinition,
    pub enabled: bool,
    pub tool_input_guardrails: Vec<ToolInputGuardrail>,
    pub tool_output_guardrails: Vec<ToolOutputGuardrail>,
    pub needs_approval: bool,
    pub timeout_seconds: Option<f64>,
    pub defer_loading: bool,
    executor: ToolExecutor,
}

impl std::fmt::Debug for FunctionTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionTool")
            .field("definition", &self.definition)
            .field("enabled", &self.enabled)
            .field("tool_input_guardrails", &self.tool_input_guardrails.len())
            .field("tool_output_guardrails", &self.tool_output_guardrails.len())
            .field("needs_approval", &self.needs_approval)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("defer_loading", &self.defer_loading)
            .finish()
    }
}

impl FunctionTool {
    pub fn new(definition: ToolDefinition, executor: ToolExecutor) -> Self {
        Self {
            definition,
            enabled: true,
            tool_input_guardrails: Vec::new(),
            tool_output_guardrails: Vec::new(),
            needs_approval: false,
            timeout_seconds: None,
            defer_loading: false,
            executor,
        }
    }

    pub fn qualified_name(&self) -> String {
        tool_qualified_name(&self.definition.name, self.definition.namespace.as_deref())
            .unwrap_or_else(|| self.definition.name.clone())
    }

    pub fn with_input_guardrail(mut self, guardrail: ToolInputGuardrail) -> Self {
        self.tool_input_guardrails.push(guardrail);
        self
    }

    pub fn with_output_guardrail(mut self, guardrail: ToolOutputGuardrail) -> Self {
        self.tool_output_guardrails.push(guardrail);
        self
    }

    pub fn with_needs_approval(mut self, needs_approval: bool) -> Self {
        self.needs_approval = needs_approval;
        self
    }

    pub fn with_timeout_seconds(mut self, timeout_seconds: f64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }

    pub fn with_defer_loading(mut self, defer_loading: bool) -> Self {
        self.defer_loading = defer_loading;
        self.definition.defer_loading = defer_loading;
        self
    }
}

pub type ApplyPatchTool = StaticTool;
pub type ComputerTool = StaticTool;
pub type HostedMCPTool = StaticTool;
pub type LocalShellTool = StaticTool;
pub type ShellTool = StaticTool;

pub type ComputerCreate = Arc<
    dyn Fn(RunContextWrapper<RunContext>) -> BoxFuture<'static, Result<Computer>> + Send + Sync,
>;
pub type ComputerDispose = Arc<
    dyn Fn(RunContextWrapper<RunContext>, Computer) -> BoxFuture<'static, Result<()>> + Send + Sync,
>;

#[derive(Clone)]
pub struct ComputerProvider {
    pub create: ComputerCreate,
    pub dispose: Option<ComputerDispose>,
}

impl std::fmt::Debug for ComputerProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputerProvider")
            .field("create", &"<function>")
            .field("dispose", &self.dispose.as_ref().map(|_| "<function>"))
            .finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LocalShellCommandRequest {
    pub command: String,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellCommandRequest {
    pub command: String,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellActionRequest {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellCallData {
    pub call_id: Option<String>,
    pub request: ShellCommandRequest,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellCallOutcome {
    pub data: ShellCallData,
    pub output: ShellCommandOutput,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellResult {
    pub output: ShellCommandOutput,
}

pub type ShellExecutor = Arc<
    dyn Fn(ShellCommandRequest) -> BoxFuture<'static, Result<ShellCommandOutput>> + Send + Sync,
>;
pub type LocalShellExecutor = Arc<
    dyn Fn(LocalShellCommandRequest) -> BoxFuture<'static, Result<ShellCommandOutput>>
        + Send
        + Sync,
>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MCPToolApprovalRequest {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub namespace: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MCPToolApprovalFunctionResult {
    pub approved: bool,
    pub reason: Option<String>,
}

pub type MCPToolApprovalFunction = Arc<
    dyn Fn(MCPToolApprovalRequest) -> BoxFuture<'static, Result<MCPToolApprovalFunctionResult>>
        + Send
        + Sync,
>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolHostedEnvironment {
    pub image: Option<String>,
    pub ephemeral: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolLocalEnvironment {
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolContainerAutoEnvironment {
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolContainerReferenceEnvironment {
    pub reference: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolContainerSkill {
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolSkillReference {
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolLocalSkill {
    pub path: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolInlineSkillSource {
    pub language: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolInlineSkill {
    pub name: String,
    pub source: ShellToolInlineSkillSource,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolContainerNetworkPolicyAllowlist {
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolContainerNetworkPolicyDisabled {
    pub disabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShellToolContainerNetworkPolicyDomainSecret {
    pub domain: String,
    pub secret_env_var: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShellToolContainerNetworkPolicy {
    Allowlist(ShellToolContainerNetworkPolicyAllowlist),
    Disabled(ShellToolContainerNetworkPolicyDisabled),
    DomainSecret(ShellToolContainerNetworkPolicyDomainSecret),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShellToolEnvironment {
    Hosted(ShellToolHostedEnvironment),
    Local(ShellToolLocalEnvironment),
    ContainerAuto(ShellToolContainerAutoEnvironment),
    ContainerReference(ShellToolContainerReferenceEnvironment),
}

pub fn tool_namespace(definition: &ToolDefinition) -> Option<&str> {
    definition.namespace.as_deref()
}

pub async fn resolve_computer(
    run_context: &RunContextWrapper<RunContext>,
    computer: Option<Computer>,
    provider: Option<&ComputerProvider>,
) -> Result<Option<Computer>> {
    match (computer, provider) {
        (Some(computer), _) => Ok(Some(computer)),
        (None, Some(provider)) => (provider.create)(run_context.clone()).await.map(Some),
        (None, None) => Ok(None),
    }
}

pub async fn dispose_resolved_computers(
    run_context: &RunContextWrapper<RunContext>,
    provider: Option<&ComputerProvider>,
    computer: Option<Computer>,
) -> Result<()> {
    if let (Some(provider), Some(computer)) = (provider, computer) {
        let Some(dispose) = provider.dispose.as_ref() else {
            return Ok(());
        };
        dispose(run_context.clone(), computer).await?;
    }
    Ok(())
}

pub fn default_tool_error_function<TContext>(args: &ToolErrorFormatterArgs<TContext>) -> String {
    format!("Tool `{}` failed: {}", args.tool_name, args.default_message)
}

#[async_trait]
impl Tool for FunctionTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn invoke(&self, context: ToolContext, args: Value) -> Result<ToolOutput> {
        (self.executor)(context, args).await
    }
}

pub fn function_tool<TArgs, TResult, F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    handler: F,
) -> Result<FunctionTool>
where
    TArgs: DeserializeOwned + JsonSchema + Send + 'static,
    TResult: Into<ToolOutput> + Send + 'static,
    F: Fn(ToolContext, TArgs) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<TResult>> + Send + 'static,
{
    let name = name.into();
    let description = description.into();
    let schema = FunctionSchema::<TArgs>::from_type(name.clone(), Some(description.clone()), true)
        .map_err(|error| AgentsError::message(error.message))?;
    let mut definition = ToolDefinition::new(name, description)
        .with_input_json_schema(schema.params_json_schema.clone());
    definition.strict_json_schema = schema.strict_json_schema;

    let handler = Arc::new(handler);
    let executor: ToolExecutor = Arc::new(move |context, args| {
        let handler = Arc::clone(&handler);
        async move {
            let parsed = serde_json::from_value::<TArgs>(args)
                .map_err(|error| AgentsError::message(error.to_string()))?;
            let result = handler(context, parsed).await?;
            Ok(result.into())
        }
        .boxed()
    });

    Ok(FunctionTool::new(definition, executor))
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

    use crate::run_context::{RunContext, RunContextWrapper};
    use crate::tool_guardrails::{ToolGuardrailFunctionOutput, tool_input_guardrail};

    use super::*;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct SearchArgs {
        query: String,
    }

    #[tokio::test]
    async fn invokes_function_tool() {
        let tool = function_tool(
            "search",
            "Search documents",
            |_ctx, args: SearchArgs| async move {
                Ok::<_, AgentsError>(format!("result:{}", args.query))
            },
        )
        .expect("tool should build");

        let output = tool
            .invoke(
                ToolContext::new(
                    RunContextWrapper::new(RunContext::default()),
                    "search",
                    "call-1",
                    "{}",
                ),
                json!({"query":"rust"}),
            )
            .await
            .expect("tool should run");

        assert_eq!(output, ToolOutput::from("result:rust"));
    }

    #[tokio::test]
    async fn configures_function_tool_runtime_settings() {
        let tool = function_tool(
            "search",
            "Search documents",
            |_ctx, args: SearchArgs| async move {
                Ok::<_, AgentsError>(format!("result:{}", args.query))
            },
        )
        .expect("tool should build")
        .with_input_guardrail(tool_input_guardrail("sanitize", |_data| async move {
            Ok(ToolGuardrailFunctionOutput::allow(None))
        }))
        .with_needs_approval(true)
        .with_timeout_seconds(5.0)
        .with_defer_loading(true);

        assert_eq!(tool.qualified_name(), "search");
        assert!(tool.needs_approval);
        assert_eq!(tool.timeout_seconds, Some(5.0));
        assert!(tool.defer_loading);
        assert_eq!(tool.tool_input_guardrails.len(), 1);
        assert!(tool.definition.defer_loading);
    }
}
