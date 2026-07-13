use crate::agent;
use crate::deploy_detect::{self, DeployDetectionReport, DeployProjectDetection};
use crate::deploy_scan;
use crate::store;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Component, Path};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

const PLAN_SCHEMA_VERSION: &str = "1.0";
const MAX_CONTEXT_FILE_BYTES: u64 = 48 * 1024;
const MAX_CONTEXT_FILES: usize = 40;

#[derive(Debug, Clone, Deserialize)]
pub struct PlanDeployPackageInput {
    pub workspace_id: i64,
    pub project_ids: Vec<i64>,
    pub target_machine_id: Option<String>,
    pub agent_profile_id: i64,
    pub include_dirty: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployPlanReport {
    pub workspace_id: i64,
    pub project_ids: Vec<i64>,
    pub target_machine_id: Option<String>,
    pub agent_profile_id: i64,
    pub agent_session_id: Option<i64>,
    pub agent_name: String,
    pub mode: String,
    pub planning_status: String,
    pub status: String,
    pub confidence: String,
    pub summary: String,
    pub guided_summary: Value,
    pub project_context_path: Option<String>,
    pub deploy_plan_path: Option<String>,
    pub validation_report_path: Option<String>,
    pub project_context_json: String,
    pub deploy_plan_json: String,
    pub validation_report_json: String,
    pub validation_findings: Vec<DeployPlanFinding>,
    pub validation_errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeployPlanFinding {
    pub path: String,
    pub reason: String,
    pub hint: String,
    pub severity: String,
    pub blocking: bool,
}

#[derive(Debug, Clone)]
pub struct DeployPlanBundle {
    pub input: PlanDeployPackageInput,
    pub detection: DeployDetectionReport,
    pub agent: store::AgentProfile,
    pub agent_session_id: Option<i64>,
    pub context: Value,
    pub plan: Value,
    pub validation: Value,
}

pub fn plan_package(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: PlanDeployPackageInput,
) -> anyhow::Result<DeployPlanReport> {
    let (workspace, detection, agent, target, context) = build_context_components(db, &input)?;
    let prompt = deploy_plan_prompt(&context)?;
    let run = agent::run_agent_prompt_blocking(
        app,
        db,
        &agent,
        None,
        &workspace.root_path,
        "Deploy planning",
        &prompt,
        Some(json!({
            "kind": "deploy_plan",
            "workspace_id": input.workspace_id,
            "project_ids": input.project_ids,
            "target_machine_id": input.target_machine_id,
        })),
        Duration::from_secs(180),
    )?;
    let mut plan = parse_agent_deploy_plan(&run.assistant_output)?;
    attach_agent_metadata(&mut plan, &agent, run.session.id);
    let validation = validate_plan(&plan, &detection, target.as_ref());
    let bundle = DeployPlanBundle {
        input,
        detection,
        agent,
        agent_session_id: Some(run.session.id),
        context,
        plan,
        validation,
    };
    let draft_root = Path::new(&workspace.root_path)
        .join(".dw")
        .join("deploy-plans")
        .join(new_plan_id());
    write_analysis_artifacts(&draft_root, &bundle)?;
    report_from_bundle(
        &bundle,
        Some(&draft_root.join("analysis").join("project-context.json")),
        Some(&draft_root.join("analysis").join("deploy-plan.json")),
        Some(&draft_root.join("analysis").join("validation-report.json")),
    )
}

pub fn load_plan_bundle_from_path(
    db: &store::Database,
    input: PlanDeployPackageInput,
    deploy_plan_path: &Path,
) -> anyhow::Result<DeployPlanBundle> {
    let (workspace, detection, agent, target, context) = build_context_components(db, &input)?;
    let scoped_path = scoped_existing_plan_path(&workspace.root_path, deploy_plan_path)?;
    let plan_text = std::fs::read_to_string(&scoped_path)
        .with_context(|| format!("failed to read {}", scoped_path.display()))?;
    let plan = serde_json::from_str::<Value>(&plan_text)
        .with_context(|| format!("deploy_agent_invalid_json: {}", scoped_path.display()))?;
    let validation = validate_plan(&plan, &detection, target.as_ref());
    Ok(DeployPlanBundle {
        input,
        detection,
        agent,
        agent_session_id: agent_session_id_from_plan(&plan),
        context,
        plan,
        validation,
    })
}

pub fn write_analysis_artifacts(root: &Path, bundle: &DeployPlanBundle) -> anyhow::Result<()> {
    let analysis_dir = root.join("analysis");
    std::fs::create_dir_all(&analysis_dir)
        .with_context(|| format!("failed to create {}", analysis_dir.display()))?;
    std::fs::write(
        analysis_dir.join("project-context.json"),
        serde_json::to_string_pretty(&bundle.context)?,
    )?;
    std::fs::write(
        analysis_dir.join("deploy-plan.json"),
        serde_json::to_string_pretty(&bundle.plan)?,
    )?;
    std::fs::write(
        analysis_dir.join("validation-report.json"),
        serde_json::to_string_pretty(&bundle.validation)?,
    )?;
    Ok(())
}

pub fn report_from_bundle(
    bundle: &DeployPlanBundle,
    context_path: Option<&Path>,
    plan_path: Option<&Path>,
    validation_path: Option<&Path>,
) -> anyhow::Result<DeployPlanReport> {
    let status = bundle
        .validation
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("failed")
        .to_string();
    let confidence = bundle
        .plan
        .get("confidence")
        .and_then(Value::as_str)
        .unwrap_or("low")
        .to_string();
    let summary = bundle
        .plan
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("Deploy plan generated from selected project context")
        .to_string();
    let warnings = bundle
        .validation
        .get("warnings")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let validation_findings = validation_findings(&bundle.validation);
    Ok(DeployPlanReport {
        workspace_id: bundle.input.workspace_id,
        project_ids: bundle
            .detection
            .projects
            .iter()
            .map(|project| project.project_id)
            .collect(),
        target_machine_id: bundle.input.target_machine_id.clone(),
        agent_profile_id: bundle.agent.id,
        agent_session_id: bundle.agent_session_id,
        agent_name: bundle.agent.name.clone(),
        mode: "agent_planned".to_string(),
        planning_status: if status == "passed" {
            "planned".to_string()
        } else {
            "validation_failed".to_string()
        },
        status,
        confidence,
        summary,
        guided_summary: guided_summary(&bundle.plan, &bundle.validation),
        project_context_path: context_path.map(|path| path.display().to_string()),
        deploy_plan_path: plan_path.map(|path| path.display().to_string()),
        validation_report_path: validation_path.map(|path| path.display().to_string()),
        project_context_json: serde_json::to_string_pretty(&bundle.context)?,
        deploy_plan_json: serde_json::to_string_pretty(&bundle.plan)?,
        validation_report_json: serde_json::to_string_pretty(&bundle.validation)?,
        validation_findings,
        validation_errors: validation_errors(&bundle.validation),
        warnings,
    })
}

pub fn validation_blocks_package(validation: &Value) -> bool {
    validation
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status != "passed")
        .unwrap_or(true)
}

pub fn validation_error_summary(validation: &Value) -> String {
    let errors = validation_errors(validation);
    if errors.is_empty() {
        "deploy plan validation failed".to_string()
    } else {
        errors.join("; ")
    }
}

pub fn script_artifacts_from_plan(plan: &Value) -> anyhow::Result<Vec<(String, String)>> {
    let scripts = plan
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("scripts"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("deploy_plan_validation_failed: missing artifacts.scripts")
        })?;
    let mut out = Vec::new();
    for script in scripts {
        let path = script
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("deploy_plan_validation_failed: script missing path"))?;
        let body = script
            .get("body")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("deploy_plan_validation_failed: script missing body"))?;
        if !safe_script_path(path) {
            anyhow::bail!("deploy_plan_validation_failed: script path must stay under scripts/");
        }
        out.push((path.to_string(), body.to_string()));
    }
    Ok(out)
}

pub fn file_artifacts_from_plan(plan: &Value) -> anyhow::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    if let Some(compose) = plan
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("compose"))
    {
        if let Some((path, body)) = artifact_file(compose)? {
            out.push((path, body));
        }
    }
    if let Some(dockerfiles) = plan
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("dockerfiles"))
        .and_then(Value::as_array)
    {
        for dockerfile in dockerfiles {
            if let Some((path, body)) = artifact_file(dockerfile)? {
                out.push((path, body));
            }
        }
    }
    Ok(out)
}

fn artifact_file(value: &Value) -> anyhow::Result<Option<(String, String)>> {
    if value.is_null() {
        return Ok(None);
    }
    if value.as_str().is_some() {
        return Ok(None);
    }
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("deploy_plan_validation_failed: artifact missing path"))?;
    let body = value
        .get("body")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("deploy_plan_validation_failed: artifact missing body"))?;
    if !safe_package_artifact_path(path) {
        anyhow::bail!("deploy_plan_validation_failed: artifact path escapes package");
    }
    Ok(Some((path.to_string(), body.to_string())))
}

fn build_context_components(
    db: &store::Database,
    input: &PlanDeployPackageInput,
) -> anyhow::Result<(
    store::Workspace,
    DeployDetectionReport,
    store::AgentProfile,
    Option<store::WorkspaceMachine>,
    Value,
)> {
    let workspace = db.get_workspace(input.workspace_id)?;
    let agent = db.get_agent_profile(input.agent_profile_id)?;
    if agent.workspace_id != input.workspace_id {
        anyhow::bail!(
            "deploy_agent_workspace_mismatch: selected agent does not belong to this workspace"
        );
    }
    let detection = deploy_detect::detect_projects(db, input.workspace_id, &input.project_ids)?;
    let target = input
        .target_machine_id
        .as_deref()
        .and_then(|machine_id| db.get_workspace_machine(machine_id).ok());
    let context = build_project_context(&agent, &detection, target.as_ref(), input.include_dirty);
    Ok((workspace, detection, agent, target, context))
}

fn deploy_plan_prompt(context: &Value) -> anyhow::Result<String> {
    let context_json = serde_json::to_string_pretty(context)?;
    Ok(format!(
        r##"You are the deploy planning agent for ADE.

Return ONLY one strict JSON object. Do not use Markdown fences.
You are not allowed to edit source repositories or execute commands.
Plan package-local deploy artifacts only.

Required JSON shape:
{{
  "schema_version": "1.0",
  "strategy": "web_service | custom_compose | desktop_dev | mixed | unsupported",
  "confidence": "high | medium | low",
  "summary": "short rationale",
  "projects": [
    {{
      "project_id": 1,
      "name": "project",
      "kind": "node | tauri_desktop | python | compose | unknown",
      "package_manager": "npm | pnpm | cargo | pip | none",
      "runtime": "container | compose | desktop_session | unsupported",
      "install": ["commands run inside packaged project"],
      "verify": ["commands that prove dependencies/build readiness"],
      "run": "command to start the app or null",
      "requires": {{
        "system_packages": ["packages"],
        "desktop_session": false,
        "docker": true
      }},
      "ports": [],
      "healthcheck": "curl -fsS http://127.0.0.1:8080/health" or "http://127.0.0.1:8080/health" or {{"type":"http","url":"http://127.0.0.1:8080/health","interval_seconds":10}} or null,
      "risks": []
    }}
  ],
  "services": [],
  "ports": [],
  "env": {{"required": [], "optional": []}},
  "artifacts": {{
    "compose": {{"path": "docker-compose.yml", "body": "services:\n  app:\n    ..."}} or null,
    "dockerfiles": [{{"project_id": 1, "path": "projects/project/Dockerfile", "body": "FROM ..."}}],
    "scripts": [
      {{"path": "scripts/preflight.sh", "purpose": "validate prerequisites", "body": "#!/usr/bin/env bash\nset -euo pipefail\n..."}}
    ]
  }},
  "risks": [{{"level": "medium", "message": "risk"}}]
}}

Rules:
- Use only paths below scripts/ for scripts.
- Do not include secret values.
- Do not write outside the deploy package or copied project directories.
- For web_service/custom_compose/mixed, include a healthcheck as a non-empty string command/URL or an object with non-empty url, command, or test.
- For desktop_dev, include build-dev.sh, verify-dev.sh, and run-dev.sh scripts.
- For Windows targets, include install-deploy.ps1 guidance when relevant.

Project context:
{context_json}
"##
    ))
}

fn parse_agent_deploy_plan(output: &str) -> anyhow::Result<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(output.trim()) {
        return Ok(value);
    }
    let candidate = json_candidate(output)
        .ok_or_else(|| anyhow::anyhow!("deploy_agent_invalid_json: no JSON object in output"))?;
    serde_json::from_str::<Value>(candidate)
        .with_context(|| "deploy_agent_invalid_json: assistant output is not valid JSON")
}

fn json_candidate(output: &str) -> Option<&str> {
    if let Some(start) = output.find("```json") {
        let body_start = output[start..]
            .find('\n')
            .map(|offset| start + offset + 1)?;
        let body_end = output[body_start..]
            .find("```")
            .map(|offset| body_start + offset)?;
        return Some(output[body_start..body_end].trim());
    }
    let start = output.find('{')?;
    let end = output.rfind('}')?;
    (end > start).then_some(output[start..=end].trim())
}

fn scoped_existing_plan_path(
    workspace_root: &str,
    deploy_plan_path: &Path,
) -> anyhow::Result<std::path::PathBuf> {
    let workspace_root = Path::new(workspace_root);
    let plan_root = workspace_root.join(".dw").join("deploy-plans");
    let path = if deploy_plan_path.is_absolute() {
        deploy_plan_path.to_path_buf()
    } else {
        workspace_root.join(deploy_plan_path)
    };
    let canonical = path
        .canonicalize()
        .with_context(|| format!("deploy_plan_required: {}", path.display()))?;
    let canonical_plan_root = plan_root
        .canonicalize()
        .with_context(|| format!("deploy_plan_required: {}", plan_root.display()))?;
    if !canonical.starts_with(&canonical_plan_root) {
        anyhow::bail!("deploy_plan_validation_failed: plan path is outside workspace deploy plans");
    }
    if canonical.file_name().and_then(|name| name.to_str()) != Some("deploy-plan.json") {
        anyhow::bail!("deploy_plan_validation_failed: expected analysis/deploy-plan.json");
    }
    Ok(canonical)
}

fn agent_session_id_from_plan(plan: &Value) -> Option<i64> {
    plan.get("planner")
        .and_then(|planner| planner.get("agent_session_id"))
        .and_then(Value::as_i64)
}

fn attach_agent_metadata(plan: &mut Value, agent: &store::AgentProfile, session_id: i64) {
    let planner = json!({
        "mode": "agent_required",
        "agent": agent_json(agent),
        "producer": "selected-agent",
        "agent_session_id": session_id,
    });
    if let Some(object) = plan.as_object_mut() {
        object.insert("planner".to_string(), planner);
    }
}

fn validation_errors(validation: &Value) -> Vec<String> {
    validation_findings(validation)
        .into_iter()
        .filter(|finding| finding.blocking)
        .map(|finding| format!("{}: {} — {}", finding.path, finding.reason, finding.hint))
        .collect()
}

fn validation_findings(validation: &Value) -> Vec<DeployPlanFinding> {
    validation
        .get("findings")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|finding| {
                    let path = finding
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("analysis/deploy-plan.json")
                        .trim();
                    let reason = finding.get("reason").and_then(Value::as_str)?.trim();
                    let hint = finding
                        .get("hint")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|hint| !hint.is_empty())
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| finding_hint(path, reason));
                    Some(DeployPlanFinding {
                        path: if path.is_empty() {
                            "analysis/deploy-plan.json".to_string()
                        } else {
                            path.to_string()
                        },
                        reason: reason.to_string(),
                        hint,
                        severity: finding
                            .get("severity")
                            .and_then(Value::as_str)
                            .unwrap_or("error")
                            .to_string(),
                        blocking: finding
                            .get("blocking")
                            .and_then(Value::as_bool)
                            .unwrap_or(true),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn guided_summary(plan: &Value, validation: &Value) -> Value {
    json!({
        "strategy": plan.get("strategy").cloned().unwrap_or(Value::Null),
        "confidence": plan.get("confidence").cloned().unwrap_or(Value::Null),
        "summary": plan.get("summary").cloned().unwrap_or(Value::Null),
        "projects": plan.get("projects").cloned().unwrap_or_else(|| json!([])),
        "env": plan.get("env").cloned().unwrap_or_else(|| json!({ "required": [], "optional": [] })),
        "ports": plan.get("ports").cloned().unwrap_or_else(|| json!([])),
        "risks": plan.get("risks").cloned().unwrap_or_else(|| json!([])),
        "validation": {
            "status": validation.get("status").cloned().unwrap_or_else(|| json!("blocked")),
            "errors": validation_errors(validation),
        }
    })
}

fn build_project_context(
    agent: &store::AgentProfile,
    detection: &DeployDetectionReport,
    target: Option<&store::WorkspaceMachine>,
    include_dirty: bool,
) -> Value {
    json!({
        "schema_version": PLAN_SCHEMA_VERSION,
        "mode": "agent_planned",
        "agent": agent_json(agent),
        "target": target.map(target_json),
        "include_dirty": include_dirty,
        "detector": {
            "workspace_id": detection.workspace_id,
            "warnings": detection.warnings,
            "services": detection.services,
            "ports": detection.ports,
        },
        "projects": detection.projects.iter().map(|project| json!({
            "project_id": project.project_id,
            "name": project.name,
            "path": project.path,
            "language": project.language,
            "framework": project.framework,
            "package_manager": project.package_manager,
            "has_dockerfile": project.has_dockerfile,
            "has_compose": project.has_compose,
            "deploy_strategy": project.deploy_strategy,
            "strategy_reason": project.strategy_reason,
            "runtime_commands": project.runtime_commands,
            "requires_desktop_session": project.requires_desktop_session,
            "warnings": project.warnings,
            "evidence_files": safe_project_files(Path::new(&project.path)),
        })).collect::<Vec<_>>(),
        "instruction": "Plan the deploy before any package is generated. Return strict JSON matching analysis/deploy-plan.json; ADE validates and renders package-local artifacts only.",
    })
}

fn validate_plan(
    plan: &Value,
    detection: &DeployDetectionReport,
    target: Option<&store::WorkspaceMachine>,
) -> Value {
    let mut findings = Vec::new();
    let mut warnings = Vec::new();
    if plan.get("schema_version").and_then(Value::as_str) != Some(PLAN_SCHEMA_VERSION) {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "unsupported deploy plan schema_version",
            true,
            "Defina `schema_version` como `1.0` e regenere o plano no formato atual do ADE.",
        ));
    }
    let strategy = plan.get("strategy").and_then(Value::as_str).unwrap_or("");
    let detected_strategy = package_strategy(&detection.projects);
    if strategy != detected_strategy {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "deploy plan strategy does not match detected project set",
            true,
            &format!(
                "A estratégia detectada para esta seleção é `{detected_strategy}`; regenere o plano ou ajuste a seleção de projetos."
            ),
        ));
    }
    let detected_project_ids = detection
        .projects
        .iter()
        .map(|project| project.project_id)
        .collect::<Vec<_>>();
    let planned_project_ids = plan
        .get("projects")
        .and_then(Value::as_array)
        .map(|projects| {
            projects
                .iter()
                .filter_map(|project| project.get("project_id").and_then(Value::as_i64))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if planned_project_ids.is_empty()
        || !same_project_ids(&detected_project_ids, &planned_project_ids)
    {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "deploy plan projects do not match selected projects",
            true,
            "Liste exatamente os `project_id` selecionados em `projects[]` ou ajuste a seleção de projetos antes de replanejar.",
        ));
    }
    if plan.get("confidence").and_then(Value::as_str) == Some("low") {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "agent deploy plan confidence is low",
            true,
            "O agente declarou confiança baixa; revise o contexto do projeto (evidence files) e replaneje.",
        ));
    }
    if strategy == "desktop_dev" && target.map(is_invalid_desktop_dev_target).unwrap_or(false) {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "desktop_dev deploy requires an Ubuntu Desktop Deploy VM or Windows 11 target",
            true,
            "Selecione uma Ubuntu Desktop Deploy VM ou um alvo Windows 11 antes de executar este plano desktop_dev.",
        ));
    }
    if matches!(strategy, "web_service" | "custom_compose" | "mixed")
        && plan_healthchecks(plan).is_empty()
    {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "web or compose deploy plans must include at least one healthcheck",
            true,
            "Inclua `projects[].healthcheck` (string de comando/URL ou objeto {url, interval_seconds, ...}) ou um healthcheck top-level no plano.",
        ));
    }
    match file_artifacts_from_plan(plan) {
        Ok(artifacts) => {
            for (path, body) in artifacts {
                if contains_secret_marker(&body) {
                    findings.push(finding(
                        &path,
                        "artifact contains secret-like marker",
                        true,
                        SECRET_MARKER_HINT,
                    ));
                }
                if contains_dangerous_command(&body) {
                    findings.push(finding(
                        &path,
                        "artifact contains dangerous host command",
                        true,
                        DANGEROUS_COMMAND_HINT,
                    ));
                }
            }
        }
        Err(error) => {
            findings.push(finding(
                "analysis/deploy-plan.json",
                &error.to_string(),
                true,
                artifact_error_hint(&error.to_string()),
            ));
        }
    }
    let scripts = plan
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("scripts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if scripts.is_empty() {
        findings.push(finding(
            "analysis/deploy-plan.json",
            "deploy plan must include package-local scripts",
            true,
            "Inclua ao menos um item em `artifacts.scripts[]` com `path` em `scripts/` e `body` executável.",
        ));
    }
    for script in scripts {
        let path = script.get("path").and_then(Value::as_str).unwrap_or("");
        if !safe_script_path(path) {
            findings.push(finding(
                path,
                "script path must stay under scripts/",
                true,
                "Mova o script para `scripts/<nome>.sh` — só esse diretório é permitido.",
            ));
        }
        if script
            .get("body")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            findings.push(finding(
                path,
                "script body is required",
                true,
                "Preencha `artifacts.scripts[].body` com o conteúdo do script ou remova o item vazio.",
            ));
        }
        if let Some(body) = script.get("body").and_then(Value::as_str) {
            if contains_secret_marker(body) {
                findings.push(finding(
                    path,
                    "script contains secret-like marker",
                    true,
                    SECRET_MARKER_HINT,
                ));
            }
            if contains_dangerous_command(body) {
                findings.push(finding(
                    path,
                    "script contains dangerous host command",
                    true,
                    DANGEROUS_COMMAND_HINT,
                ));
            }
        }
    }
    if detection
        .projects
        .iter()
        .any(|project| project.deploy_strategy == "unsupported")
    {
        warnings
            .push("One or more projects are unsupported by the current deploy planner".to_string());
    }
    let status = if findings.iter().any(|finding| {
        finding
            .get("blocking")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }) {
        "blocked"
    } else {
        "passed"
    };
    json!({
        "schema_version": PLAN_SCHEMA_VERSION,
        "status": status,
        "findings": findings,
        "warnings": warnings,
    })
}

fn safe_project_files(root: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for relative in key_project_files()
        .into_iter()
        .chain(root_deploy_contract_files(root))
    {
        if out.len() >= MAX_CONTEXT_FILES {
            break;
        }
        if !seen.insert(relative.clone()) {
            continue;
        }
        let path = root.join(&relative);
        if path.is_file() {
            out.push(context_file(root, &path, &relative));
        }
    }
    for dir in ["scripts", ".github/workflows"] {
        if out.len() >= MAX_CONTEXT_FILES {
            break;
        }
        collect_context_dir(root, &root.join(dir), &mut out);
    }
    out
}

fn key_project_files() -> Vec<String> {
    [
        "package.json",
        "pnpm-lock.yaml",
        "package-lock.json",
        "yarn.lock",
        "Cargo.toml",
        "src-tauri/Cargo.toml",
        "pyproject.toml",
        "requirements.txt",
        ".dockerignore",
        "README.md",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

fn root_deploy_contract_files(root: &Path) -> Vec<String> {
    let mut files = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name == "Dockerfile"
            || name.starts_with("Dockerfile.")
            || is_root_compose_file_name(name)
        {
            files.insert(name.to_string());
        }
    }
    files.into_iter().collect()
}

fn is_root_compose_file_name(name: &str) -> bool {
    (name.starts_with("compose.") || name.starts_with("docker-compose."))
        && (name.ends_with(".yml") || name.ends_with(".yaml"))
}

fn collect_context_dir(root: &Path, dir: &Path, out: &mut Vec<Value>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= MAX_CONTEXT_FILES {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_context_dir(root, &path, out);
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .ok()
                .and_then(|path| path.to_str())
                .unwrap_or("unknown");
            out.push(context_file(root, &path, relative));
        }
    }
}

fn context_file(_root: &Path, path: &Path, relative: &str) -> Value {
    let metadata = std::fs::metadata(path).ok();
    let bytes = metadata
        .as_ref()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let content = if bytes <= MAX_CONTEXT_FILE_BYTES {
        std::fs::read_to_string(path)
            .ok()
            .map(|content| redact_text(&content))
    } else {
        None
    };
    json!({
        "path": relative,
        "bytes": bytes,
        "content": content,
    })
}

fn same_project_ids(detected: &[i64], planned: &[i64]) -> bool {
    let mut detected = detected.to_vec();
    let mut planned = planned.to_vec();
    detected.sort_unstable();
    planned.sort_unstable();
    detected == planned
}

fn plan_healthchecks(plan: &Value) -> Vec<String> {
    let mut healthchecks = Vec::new();
    if let Some(value) = plan.get("healthcheck").and_then(healthcheck_repr) {
        healthchecks.push(value);
    }
    if let Some(projects) = plan.get("projects").and_then(Value::as_array) {
        for project in projects {
            if let Some(value) = project.get("healthcheck").and_then(healthcheck_repr) {
                healthchecks.push(value);
            }
        }
    }
    healthchecks
}

fn healthcheck_repr(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return non_empty_string(value);
    }
    let object = value.as_object()?;
    ["url", "command", "test"]
        .into_iter()
        .find_map(|key| object.get(key).and_then(healthcheck_field_repr))
}

fn healthcheck_field_repr(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return non_empty_string(value);
    }
    if let Some(values) = value.as_array() {
        let joined = values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return non_empty_string(&joined);
    }
    None
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn package_strategy(projects: &[DeployProjectDetection]) -> String {
    let mut strategies = projects
        .iter()
        .map(|project| project.deploy_strategy.as_str())
        .collect::<Vec<_>>();
    strategies.sort_unstable();
    strategies.dedup();
    match strategies.as_slice() {
        [] => "unsupported".to_string(),
        [single] => (*single).to_string(),
        _ => "mixed".to_string(),
    }
}

fn is_invalid_desktop_dev_target(machine: &store::WorkspaceMachine) -> bool {
    !matches!(
        machine.preset_id.as_str(),
        "ubuntu_desktop_deploy_vm" | "windows_11"
    )
}

fn agent_json(agent: &store::AgentProfile) -> Value {
    json!({
        "profile_id": agent.id,
        "name": agent.name,
        "provider": agent.provider,
        "model": agent.model,
        "context_mode": agent.context_mode,
        "sandbox": agent.sandbox,
    })
}

fn target_json(machine: &store::WorkspaceMachine) -> Value {
    json!({
        "machine_id": machine.id,
        "display_name": machine.display_name,
        "preset_id": machine.preset_id,
        "image_family": machine.image_family,
        "status": machine.status,
    })
}

const SECRET_MARKER_HINT: &str = "Troque o valor literal por placeholder `${VAR}` e declare a chave em `env.required` para preenchê-la na UI de deploy.";
const DANGEROUS_COMMAND_HINT: &str = "Remova ou ajuste o comando apontado no artefato; alvos na raiz (`rm -rf /`) são bloqueados — use paths específicos.";

fn finding(path: &str, reason: &str, blocking: bool, hint: &str) -> Value {
    json!({
        "path": if path.trim().is_empty() { "analysis/deploy-plan.json" } else { path },
        "reason": reason,
        "hint": hint,
        "severity": if blocking { "error" } else { "warning" },
        "blocking": blocking,
    })
}

fn finding_hint(_path: &str, reason: &str) -> String {
    match reason {
        "unsupported deploy plan schema_version" => {
            "Defina `schema_version` como `1.0` e regenere o plano no formato atual do ADE."
        }
        "deploy plan strategy does not match detected project set" => {
            "Regenere o plano com a estratégia detectada para a seleção atual ou ajuste a seleção de projetos."
        }
        "deploy plan projects do not match selected projects" => {
            "Liste exatamente os `project_id` selecionados em `projects[]` ou ajuste a seleção de projetos antes de replanejar."
        }
        "agent deploy plan confidence is low" => {
            "O agente declarou confiança baixa; revise o contexto do projeto (evidence files) e replaneje."
        }
        "desktop_dev deploy requires an Ubuntu Desktop Deploy VM or Windows 11 target" => {
            "Selecione uma Ubuntu Desktop Deploy VM ou um alvo Windows 11 antes de executar este plano desktop_dev."
        }
        "web or compose deploy plans must include at least one healthcheck" => {
            "Inclua `projects[].healthcheck` (string de comando/URL ou objeto {url, interval_seconds, ...}) ou um healthcheck top-level no plano."
        }
        "artifact contains secret-like marker" | "script contains secret-like marker" => {
            SECRET_MARKER_HINT
        }
        "artifact contains dangerous host command" | "script contains dangerous host command" => {
            DANGEROUS_COMMAND_HINT
        }
        "script path must stay under scripts/" => {
            "Mova o script para `scripts/<nome>.sh` — só esse diretório é permitido."
        }
        "script body is required" => {
            "Preencha `artifacts.scripts[].body` com o conteúdo do script ou remova o item vazio."
        }
        "deploy plan must include package-local scripts" => {
            "Inclua ao menos um item em `artifacts.scripts[]` com `path` em `scripts/` e `body` executável."
        }
        _ => artifact_error_hint(reason),
    }
    .to_string()
}

fn artifact_error_hint(reason: &str) -> &'static str {
    if reason.contains("artifact path escapes package") {
        "Mantenha artefatos dentro do pacote ou `projects/<nome>/...`; paths absolutos e `..` são bloqueados."
    } else if reason.contains("artifact missing path") {
        "Defina `path` no artefato de compose/Dockerfile dentro do pacote antes de replanejar."
    } else if reason.contains("artifact missing body") {
        "Defina `body` no artefato de compose/Dockerfile com o conteúdo que será empacotado."
    } else {
        "Corrija o artefato indicado pelo validador e regenere o plano antes de empacotar."
    }
}

fn safe_script_path(path: &str) -> bool {
    let path = Path::new(path);
    if path.is_absolute() {
        return false;
    }
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    if first != "scripts" {
        return false;
    }
    !path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn safe_package_artifact_path(path: &str) -> bool {
    let path = Path::new(path);
    if path.is_absolute() {
        return false;
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return false;
    }
    path == Path::new("docker-compose.yml")
        || path == Path::new("compose.yml")
        || path.components().next() == Some(Component::Normal(std::ffi::OsStr::new("projects")))
}

fn contains_secret_marker(text: &str) -> bool {
    deploy_scan::contains_blocking_secret_marker(text)
}

fn contains_dangerous_command(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("mkfs.")
        || lower.contains("dd if=")
        || lower.contains("/etc/shadow")
        || has_root_target_command(&lower, "rm -rf /")
        || has_root_target_command(&lower, "chmod -r 777 /")
}

fn has_root_target_command(lower: &str, marker: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = lower[offset..].find(marker) {
        let after = offset + index + marker.len();
        let Some(next) = lower[after..].chars().next() else {
            return true;
        };
        if !is_path_component_start(next) {
            return true;
        }
        offset = after;
    }
    false
}

fn is_path_component_start(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '~' | '-')
}

fn redact_text(text: &str) -> String {
    text.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if contains_secret_marker(&lower) || lower.trim_start().starts_with(".env") {
                "[redacted secret-like line]".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn new_plan_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("plan-{nanos}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desktop_detection() -> DeployDetectionReport {
        DeployDetectionReport {
            workspace_id: 1,
            projects: vec![DeployProjectDetection {
                project_id: 1,
                name: "desktop".to_string(),
                path: "/tmp/desktop".to_string(),
                language: "typescript".to_string(),
                framework: Some("tauri".to_string()),
                package_manager: Some("npm".to_string()),
                has_dockerfile: false,
                has_compose: false,
                services: vec![],
                ports: vec![],
                healthcheck: None,
                deploy_strategy: "desktop_dev".to_string(),
                strategy_reason: "Tauri desktop project detected".to_string(),
                runtime_commands: vec!["npm run dev".to_string()],
                requires_desktop_session: true,
                warnings: vec![],
            }],
            services: vec![],
            ports: vec![],
            warnings: vec![],
        }
    }

    fn desktop_plan() -> Value {
        json!({
            "schema_version": PLAN_SCHEMA_VERSION,
            "strategy": "desktop_dev",
            "confidence": "high",
            "projects": [{"project_id": 1}],
            "artifacts": {
                "scripts": [
                    {"path": "scripts/build-dev.sh", "body": "echo build"},
                    {"path": "scripts/verify-dev.sh", "body": "echo verify"},
                    {"path": "scripts/run-dev.sh", "body": "echo run"}
                ]
            }
        })
    }

    fn web_detection(root: &str) -> DeployDetectionReport {
        DeployDetectionReport {
            workspace_id: 1,
            projects: vec![DeployProjectDetection {
                project_id: 1,
                name: "lettrebox".to_string(),
                path: root.to_string(),
                language: "rust".to_string(),
                framework: None,
                package_manager: Some("cargo".to_string()),
                has_dockerfile: false,
                has_compose: false,
                services: vec![],
                ports: vec![],
                healthcheck: Some("http://127.0.0.1:5000/".to_string()),
                deploy_strategy: "web_service".to_string(),
                strategy_reason: "Rust project detected".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            }],
            services: vec![],
            ports: vec![],
            warnings: vec![],
        }
    }

    fn custom_compose_detection(root: &str) -> DeployDetectionReport {
        let mut detection = web_detection(root);
        detection.projects[0].has_compose = true;
        detection.projects[0].deploy_strategy = "custom_compose".to_string();
        detection.projects[0].strategy_reason = "Compose runtime contract detected".to_string();
        detection
    }

    fn web_plan_with_dockerfile(body: &str) -> Value {
        json!({
            "schema_version": PLAN_SCHEMA_VERSION,
            "strategy": "web_service",
            "confidence": "high",
            "summary": "test web plan",
            "projects": [{
                "project_id": 1,
                "healthcheck": "curl -fsS http://127.0.0.1:5000/"
            }],
            "artifacts": {
                "compose": {
                    "path": "docker-compose.yml",
                    "body": "services:\n  app:\n    build:\n      context: ./projects/lettrebox\n      dockerfile: Dockerfile\n"
                },
                "dockerfiles": [{
                    "project_id": 1,
                    "path": "projects/lettrebox/Dockerfile",
                    "body": body
                }],
                "scripts": [
                    {"path": "scripts/preflight.sh", "body": "echo preflight"},
                    {"path": "scripts/deploy.sh", "body": "echo deploy"},
                    {"path": "scripts/healthcheck.sh", "body": "echo health"}
                ]
            }
        })
    }

    fn target_machine(preset_id: &str, image_family: &str) -> store::WorkspaceMachine {
        store::WorkspaceMachine {
            id: "machine-1".to_string(),
            workspace_id: 1,
            project_id: None,
            provider: "winbox".to_string(),
            provider_runtime: "native".to_string(),
            provider_profile: format!("dw-1-{preset_id}"),
            display_name: preset_id.to_string(),
            preset_id: preset_id.to_string(),
            image_family: image_family.to_string(),
            access_user: Some("bruno".to_string()),
            status: "running".to_string(),
            web_port: Some(8007),
            rdp_port: Some(3390),
            ssh_port: Some(2223),
            last_health_status: None,
            last_health_summary: None,
            last_error_code: None,
            last_error_message: None,
            created_at: "2026-06-02T00:00:00Z".to_string(),
            updated_at: "2026-06-02T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn desktop_dev_plan_accepts_windows_target() {
        let report = validate_plan(
            &desktop_plan(),
            &desktop_detection(),
            Some(&target_machine("windows_11", "windows")),
        );

        assert_eq!(report.get("status").and_then(Value::as_str), Some("passed"));
        assert!(!validation_blocks_package(&report));
    }

    #[test]
    fn desktop_dev_plan_rejects_ubuntu_server_target() {
        let report = validate_plan(
            &desktop_plan(),
            &desktop_detection(),
            Some(&target_machine("ubuntu_deploy_vm", "linux_cloud")),
        );

        assert_eq!(
            report.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert!(validation_error_summary(&report).contains("Windows 11"));
    }

    #[test]
    fn validates_script_paths_and_low_confidence() {
        let detection = DeployDetectionReport {
            workspace_id: 1,
            projects: vec![DeployProjectDetection {
                project_id: 1,
                name: "cli".to_string(),
                path: "/tmp/cli".to_string(),
                language: "unknown".to_string(),
                framework: None,
                package_manager: None,
                has_dockerfile: false,
                has_compose: false,
                services: vec![],
                ports: vec![],
                healthcheck: None,
                deploy_strategy: "unsupported".to_string(),
                strategy_reason: "none".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec!["unsupported".to_string()],
            }],
            services: vec![],
            ports: vec![],
            warnings: vec!["unsupported".to_string()],
        };
        let plan = json!({
            "schema_version": PLAN_SCHEMA_VERSION,
            "strategy": "unsupported",
            "confidence": "low",
            "artifacts": {
                "scripts": [{"path": "../bad.sh", "body": "echo ok"}]
            }
        });
        let report = validate_plan(&plan, &detection, None);
        assert_eq!(
            report.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert!(validation_blocks_package(&report));
    }

    #[test]
    fn plan_healthchecks_accepts_string_and_object_shapes() {
        let plan = json!({
            "healthcheck": " http://127.0.0.1:8080/health ",
            "projects": [
                {"healthcheck": {"type": "http", "url": "http://127.0.0.1:5000/"}},
                {"healthcheck": {"command": "curl -fsS http://127.0.0.1:5001/health"}},
                {"healthcheck": {"test": ["CMD", "curl", "-fsS", "http://127.0.0.1:5002/health"]}}
            ]
        });

        assert_eq!(
            plan_healthchecks(&plan),
            vec![
                "http://127.0.0.1:8080/health".to_string(),
                "http://127.0.0.1:5000/".to_string(),
                "curl -fsS http://127.0.0.1:5001/health".to_string(),
                "CMD curl -fsS http://127.0.0.1:5002/health".to_string(),
            ]
        );
    }

    #[test]
    fn plan_healthchecks_ignores_empty_null_and_missing_object_fields() {
        let plan = json!({
            "healthcheck": null,
            "projects": [
                {"healthcheck": ""},
                {"healthcheck": {}},
                {"healthcheck": {"url": "  "}},
                {"healthcheck": {"command": null}},
                {"healthcheck": {"test": []}},
                {"healthcheck": {"type": "http", "interval_seconds": 10}}
            ]
        });

        assert!(plan_healthchecks(&plan).is_empty());
    }

    #[test]
    fn web_plan_with_empty_object_healthcheck_is_blocked() {
        let mut plan = web_plan_with_dockerfile("FROM alpine\n");
        plan["projects"][0]["healthcheck"] = json!({});

        let report = validate_plan(&plan, &web_detection("/tmp/lettrebox"), None);

        assert_eq!(
            report.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert!(validation_error_summary(&report).contains("must include at least one healthcheck"));
    }

    fn assert_all_findings_have_hints(report: &Value) {
        let findings = validation_findings(report);
        assert!(!findings.is_empty(), "expected validation findings");
        for finding in findings {
            assert!(
                !finding.hint.trim().is_empty(),
                "missing hint for {}: {}",
                finding.path,
                finding.reason
            );
            assert!(
                validation_errors(report)
                    .iter()
                    .any(|error| !finding.blocking || error.contains(&finding.hint)),
                "blocking hint must be included in validation_errors"
            );
        }
    }

    #[test]
    fn blocking_validation_findings_include_actionable_hints() {
        let schema_report = validate_plan(&json!({}), &web_detection("/tmp/lettrebox"), None);
        assert_all_findings_have_hints(&schema_report);

        let strategy_report =
            validate_plan(&desktop_plan(), &web_detection("/tmp/lettrebox"), None);
        assert_all_findings_have_hints(&strategy_report);
        assert!(validation_error_summary(&strategy_report).contains("web_service"));

        let mut project_plan = desktop_plan();
        project_plan["projects"] = json!([{"project_id": 999}]);
        let project_report = validate_plan(&project_plan, &desktop_detection(), None);
        assert_all_findings_have_hints(&project_report);

        let mut confidence_plan = desktop_plan();
        confidence_plan["confidence"] = json!("low");
        let confidence_report = validate_plan(&confidence_plan, &desktop_detection(), None);
        assert_all_findings_have_hints(&confidence_report);

        let desktop_target_report = validate_plan(
            &desktop_plan(),
            &desktop_detection(),
            Some(&target_machine("ubuntu_deploy_vm", "linux_cloud")),
        );
        assert_all_findings_have_hints(&desktop_target_report);

        let mut healthcheck_plan = web_plan_with_dockerfile("FROM alpine\n");
        healthcheck_plan["projects"][0]["healthcheck"] = Value::Null;
        let healthcheck_report =
            validate_plan(&healthcheck_plan, &web_detection("/tmp/lettrebox"), None);
        assert_all_findings_have_hints(&healthcheck_report);

        let artifact_report = validate_plan(
            &web_plan_with_dockerfile("FROM alpine\nRUN rm -rf /\nENV password=hunter2\n"),
            &web_detection("/tmp/lettrebox"),
            None,
        );
        assert_all_findings_have_hints(&artifact_report);

        let artifact_schema_report = validate_plan(
            &json!({
                "schema_version": PLAN_SCHEMA_VERSION,
                "strategy": "web_service",
                "confidence": "high",
                "projects": [{"project_id": 1, "healthcheck": "curl -fsS http://127.0.0.1:5000/"}],
                "artifacts": {
                    "compose": {"body": "services: {}"},
                    "scripts": [{"path": "scripts/deploy.sh", "body": "echo ok"}]
                }
            }),
            &web_detection("/tmp/lettrebox"),
            None,
        );
        assert_all_findings_have_hints(&artifact_schema_report);

        let script_report = validate_plan(
            &json!({
                "schema_version": PLAN_SCHEMA_VERSION,
                "strategy": "web_service",
                "confidence": "high",
                "projects": [{"project_id": 1, "healthcheck": "curl -fsS http://127.0.0.1:5000/"}],
                "artifacts": {
                    "compose": null,
                    "dockerfiles": [],
                    "scripts": [
                        {"path": "../deploy.sh", "body": ""},
                        {"path": "scripts/unsafe.sh", "body": "password=hunter2\nrm -rf /"}
                    ]
                }
            }),
            &web_detection("/tmp/lettrebox"),
            None,
        );
        assert_all_findings_have_hints(&script_report);
    }

    #[test]
    fn dangerous_command_detection_is_root_target_aware() {
        assert!(!contains_dangerous_command(
            "RUN apt-get update && rm -rf /var/lib/apt/lists/*"
        ));
        assert!(!contains_dangerous_command("rm -rf /tmp/build"));
        assert!(contains_dangerous_command("rm -rf /"));
        assert!(contains_dangerous_command("rm -rf //"));
        assert!(contains_dangerous_command("rm -rf /*"));
        assert!(contains_dangerous_command("rm -rf / "));
        assert!(contains_dangerous_command("$(rm -rf /)"));
        assert!(contains_dangerous_command("`rm -rf /`"));
        assert!(contains_dangerous_command("rm -rf />/dev/null"));
        assert!(contains_dangerous_command("rm -rf /<x"));
        assert!(!contains_dangerous_command("chmod -R 777 /var/cache/app"));
        assert!(contains_dangerous_command("chmod -R 777 /"));
        assert!(contains_dangerous_command("chmod -R 777 //"));
        assert!(contains_dangerous_command("chmod -r 777 /*"));
        assert!(contains_dangerous_command("$(chmod -R 777 /)"));
        assert!(contains_dangerous_command("mkfs.ext4 /dev/sda"));
        assert!(contains_dangerous_command("dd if=/dev/zero of=/dev/sda"));
        assert!(contains_dangerous_command("cat /etc/shadow"));
    }

    #[test]
    fn secret_marker_detection_allows_explicit_placeholders_only() {
        for content in [
            "PASSWORD=",
            "password=   \n",
            "secret=${APP_SECRET}",
            "api_key=$API_KEY",
            "password=\"$PASSWORD\"",
            "apikey=<placeholder>",
            "password=xxx",
            "secret=ChangeMe",
        ] {
            assert!(!contains_secret_marker(content), "{content}");
        }
        for content in [
            "password=hunter2",
            "password='$ecret123'",
            "secret=real-value",
            "api_key=sk-live",
            "apikey=abc123",
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9",
        ] {
            assert!(contains_secret_marker(content), "{content}");
        }
    }

    #[test]
    fn validation_errors_include_paths_and_report_findings_are_typed() {
        let validation = json!({
            "status": "blocked",
            "findings": [{
                "path": "projects/app/Dockerfile",
                "reason": "artifact contains dangerous host command",
                "severity": "error",
                "blocking": true
            }],
            "warnings": []
        });
        assert_eq!(
            validation_errors(&validation),
            vec![format!(
                "projects/app/Dockerfile: artifact contains dangerous host command — {DANGEROUS_COMMAND_HINT}"
            )]
        );
        assert_eq!(
            validation_findings(&validation),
            vec![DeployPlanFinding {
                path: "projects/app/Dockerfile".to_string(),
                reason: "artifact contains dangerous host command".to_string(),
                hint: DANGEROUS_COMMAND_HINT.to_string(),
                severity: "error".to_string(),
                blocking: true,
            }]
        );
    }

    #[test]
    fn real_lettrebox_blocked_dockerfile_now_validates() {
        let dockerfile = r#"# syntax=docker/dockerfile:1.7
FROM rust:1-bookworm AS builder
WORKDIR /src
ENV CARGO_TERM_COLOR=always
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --bin lettrebox && \
    cp /src/target/release/lettrebox /usr/local/bin/lettrebox

FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/local/bin/lettrebox /usr/local/bin/lettrebox
COPY config /app/config
ENV RUST_LOG=info
EXPOSE 5000
HEALTHCHECK --interval=30s --timeout=5s --start-period=40s --retries=5 \
  CMD curl -fsS http://127.0.0.1:5000/ || exit 1
CMD ["/usr/local/bin/lettrebox"]
"#;
        let report = validate_plan(
            &web_plan_with_dockerfile(dockerfile),
            &web_detection("/tmp/lettrebox"),
            None,
        );
        assert_eq!(report.get("status").and_then(Value::as_str), Some("passed"));
        assert!(!validation_blocks_package(&report));
    }

    #[test]
    fn real_owner_lettrebox_deploy_plan_fixture_validates() {
        let plan = include_str!("../tests/fixtures/lettrebox-deploy-plan.json");
        let plan = serde_json::from_str::<Value>(plan).expect("parse owner deploy plan");
        let report = validate_plan(&plan, &web_detection("/home/bruno/wks/letrebox"), None);
        assert_eq!(report.get("status").and_then(Value::as_str), Some("passed"));
        assert!(!validation_blocks_package(&report));
    }

    #[test]
    fn real_owner_lettrebox_object_healthcheck_fixture_validates_as_custom_compose() {
        let plan = include_str!("../tests/fixtures/lettrebox-deploy-plan-object-healthcheck.json");
        let plan = serde_json::from_str::<Value>(plan)
            .expect("parse owner deploy plan with object healthcheck");
        let report = validate_plan(
            &plan,
            &custom_compose_detection("/home/bruno/wks/letrebox"),
            None,
        );
        assert_eq!(report.get("status").and_then(Value::as_str), Some("passed"));
        assert_eq!(validation_findings(&report), Vec::new());
        assert!(!validation_blocks_package(&report));
    }

    #[test]
    fn project_context_includes_extended_docker_contract_files() {
        let root = std::env::temp_dir().join(format!(
            "dw-deploy-plan-evidence-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).expect("mkdir");
        std::fs::write(root.join("Dockerfile.prod"), "FROM alpine\n").expect("dockerfile");
        std::fs::write(root.join("compose.deploy.yaml"), "services: {}\n").expect("compose");

        let files = safe_project_files(&root);
        let paths = files
            .iter()
            .filter_map(|file| file.get("path").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(paths.contains(&"Dockerfile.prod"));
        assert!(paths.contains(&"compose.deploy.yaml"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn redacts_context_secret_like_lines() {
        let redacted = redact_text("DATABASE_URL=postgres://local\npassword=abc\nSAFE=1");
        assert!(redacted.contains("DATABASE_URL=postgres://local"));
        assert!(redacted.contains("[redacted secret-like line]"));
        assert!(!redacted.contains("password=abc"));
    }
}
