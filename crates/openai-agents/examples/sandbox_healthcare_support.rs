use std::sync::Arc;

use async_trait::async_trait;
use openai_agents::{
    AgentsError, Dir, File, InputItem, Manifest, Model, ModelProvider, ModelRequest, ModelResponse,
    OutputItem, Result as AgentsResult, RunConfig, RunInterruptionKind, Runner, SandboxAgent,
    SandboxCapability, SandboxRunConfig, StaticTool, Usage, function_tool, prepare_sandbox_run,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

const DEFAULT_PROMPT: &str = "Resolve the eligibility verification support case. Inspect the scenario and policy files, route ambiguous authorization work to the human queue, and write support artifacts into output/.";

#[derive(Debug, Deserialize, JsonSchema)]
struct HumanQueueArgs {
    patient_id: String,
    queue: String,
    reason: String,
}

#[derive(Clone, Default)]
struct HealthcareSupportModel;

#[async_trait]
impl Model for HealthcareSupportModel {
    async fn generate(&self, request: ModelRequest) -> AgentsResult<ModelResponse> {
        let output = if tool_output_text_for_call(&request.input, "read-scenario").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-scenario".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/case/scenario.json"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "read-policy").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "read-policy".to_owned(),
                tool_name: "sandbox_read_file".to_owned(),
                arguments: json!({"path": "/workspace/policies/commercial_eligibility_checklist.md"}),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "route-human").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "route-human".to_owned(),
                tool_name: "route_to_human_queue".to_owned(),
                arguments: json!({
                    "patient_id": "PX-1042",
                    "queue": "eligibility-review",
                    "reason": "Coverage is active, but the CT authorization requirement should be confirmed by a human specialist."
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-findings").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-findings".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/policy_findings.md",
                    "replacement": policy_findings()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "write-checklist").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "write-checklist".to_owned(),
                tool_name: "sandbox_apply_patch".to_owned(),
                arguments: json!({
                    "path": "/workspace/output/human_review_checklist.md",
                    "replacement": human_review_checklist()
                }),
                namespace: None,
            }]
        } else if tool_output_text_for_call(&request.input, "verify-artifacts").is_none() {
            vec![OutputItem::ToolCall {
                call_id: "verify-artifacts".to_owned(),
                tool_name: "sandbox_run_shell".to_owned(),
                arguments: json!({
                    "command": "find output -maxdepth 1 -type f | sort && grep -h '^Next step:' output/*.md"
                }),
                namespace: None,
            }]
        } else {
            let route =
                tool_output_text_for_call(&request.input, "route-human").unwrap_or_default();
            vec![OutputItem::Text {
                text: format!(
                    "Eligibility case resolved with a human queue handoff. {route} Patient-facing response: coverage appears active; a specialist will confirm the CT authorization requirement before scheduling."
                ),
            }]
        };

        Ok(ModelResponse {
            model: request.model,
            output,
            usage: Usage {
                input_tokens: 84,
                output_tokens: 38,
            },
            response_id: None,
            request_id: None,
        })
    }
}

#[derive(Clone, Default)]
struct HealthcareSupportProvider {
    model: Arc<HealthcareSupportModel>,
}

impl ModelProvider for HealthcareSupportProvider {
    fn resolve(&self, _model: Option<&str>) -> Arc<dyn Model> {
        self.model.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), AgentsError> {
    let sandbox_agent = SandboxAgent::builder("Healthcare Support Orchestrator")
        .model("gpt-5.5")
        .instructions(
            "Resolve healthcare support cases by reading the scenario, applying policy documents, routing unclear authorization work to a human queue, and writing artifacts into output/.",
        )
        .default_manifest(healthcare_manifest())
        .capabilities(vec![
            SandboxCapability::Filesystem,
            SandboxCapability::ApplyPatch,
            SandboxCapability::Shell,
        ])
        .build();

    let run_config = RunConfig {
        sandbox: Some(SandboxRunConfig::default()),
        workflow_name: "Sandbox healthcare support demo".to_owned(),
        ..RunConfig::default()
    };
    let prepared = prepare_sandbox_run(&sandbox_agent, &run_config)?;

    let route_to_human = function_tool(
        "route_to_human_queue",
        "Route a support case to a human queue for approval-gated handling.",
        |_ctx, args: HumanQueueArgs| async move {
            Ok::<_, AgentsError>(format!(
                "handoff_id=HUMAN-{} queue={} reason={}",
                args.patient_id, args.queue, args.reason
            ))
        },
    )?
    .with_needs_approval(true);

    let mut agent = prepared.agent.clone();
    agent.tools.push(StaticTool {
        definition: route_to_human.definition.clone(),
    });
    agent.function_tools.push(route_to_human);

    let runner = Runner::new().with_model_provider(Arc::new(HealthcareSupportProvider::default()));
    let mut result = runner.run(&agent, DEFAULT_PROMPT).await?;

    while !result.interruptions.is_empty() {
        println!("run_interrupted=tool_approval_required");
        let mut state = result
            .durable_state()
            .cloned()
            .ok_or_else(|| AgentsError::message("interrupted run did not include durable state"))?;
        for interruption in &result.interruptions {
            println!(
                "approval_request tool={} call_id={} reason={}",
                interruption.tool_name.as_deref().unwrap_or_default(),
                interruption.call_id.as_deref().unwrap_or_default(),
                interruption.reason.as_deref().unwrap_or_default()
            );
            if matches!(interruption.kind, Some(RunInterruptionKind::ToolApproval)) {
                state.approve_for_tool(
                    interruption.call_id.clone().unwrap_or_default(),
                    interruption.tool_name.clone(),
                    Some("approved by healthcare support lead".to_owned()),
                );
            }
        }
        result = runner.resume_with_agent(&state, &agent).await?;
    }

    println!("final_output={}", result.final_output.unwrap_or_default());
    println!(
        "policy_findings:\n{}",
        prepared
            .session
            .read_file("/workspace/output/policy_findings.md")?
            .trim()
    );
    println!(
        "human_review_checklist:\n{}",
        prepared
            .session
            .read_file("/workspace/output/human_review_checklist.md")?
            .trim()
    );
    prepared.session.cleanup()?;
    Ok(())
}

fn healthcare_manifest() -> Manifest {
    Manifest::default()
        .with_entry(
            "case",
            Dir::new()
                .with_entry(
                    "scenario.json",
                    File::from_text(
                        "{\n\
                         \"scenario_id\": \"eligibility_verification_basic\",\n\
                         \"patient_id\": \"PX-1042\",\n\
                         \"patient_name\": \"Avery Morgan\",\n\
                         \"payer\": \"Blue Cross PPO\",\n\
                         \"request\": \"Confirm active coverage and whether a CT knee scan requires prior authorization.\"\n\
                         }\n",
                    ),
                )
                .with_entry(
                    "transcript.txt",
                    File::from_text(
                        "Patient: I have a CT scan ordered for my knee and need to know if my Blue Cross PPO plan is active.\n\
                         Agent: We can check eligibility and route authorization questions for review.\n",
                    ),
                ),
        )
        .with_entry(
            "policies",
            Dir::new()
                .with_entry(
                    "commercial_eligibility_checklist.md",
                    File::from_text(
                        "# Commercial Eligibility Checklist\n\n\
                         - Confirm member identity and payer.\n\
                         - Verify active coverage date.\n\
                         - Check whether the requested imaging service requires authorization.\n",
                    ),
                )
                .with_entry(
                    "human_escalation_policy.md",
                    File::from_text(
                        "# Human Escalation Policy\n\n\
                         Route ambiguous authorization requirements to the eligibility-review queue before scheduling.\n",
                    ),
                ),
        )
        .with_entry("output", Dir::new())
}

fn policy_findings() -> &'static str {
    "# Policy Findings\n\n\
     Patient: Avery Morgan (PX-1042)\n\
     Payer: Blue Cross PPO\n\
     Coverage: Active in the synthetic scenario.\n\
     Authorization: CT knee scan authorization requirement needs human confirmation.\n\
     Next step: route_to_human_queue approved for eligibility-review.\n"
}

fn human_review_checklist() -> &'static str {
    "# Human Review Checklist\n\n\
     - Confirm Blue Cross PPO eligibility for PX-1042.\n\
     - Confirm whether CT knee imaging requires prior authorization.\n\
     - Attach the policy findings to the scheduling note.\n\
     - Send the patient a concise coverage update after review.\n\
     Next step: specialist review before scheduling.\n"
}

fn tool_output_text_for_call<'a>(input: &'a [InputItem], call_id: &str) -> Option<&'a str> {
    input.iter().find_map(|item| {
        let InputItem::Json { value } = item else {
            return None;
        };
        if value.get("type").and_then(Value::as_str) != Some("tool_call_output")
            || value.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        value
            .get("output")
            .and_then(|output| output.get("text"))
            .and_then(Value::as_str)
    })
}
