use crate::deploy_package::{
    compose_project_name, deploy_runbook_scripts, has_blocking_findings, DEPLOY_RUNBOOK_VERSION,
};
use crate::{agent, deploy_env, deploy_repair, store, winbox_provider};
use anyhow::{Context, Error};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize)]
pub struct DeployProgressEvent {
    pub run_id: String,
    pub stack_id: String,
    pub version_id: Option<String>,
    pub machine_id: Option<String>,
    pub step_key: String,
    pub status: String,
    pub message: String,
    pub percent: Option<u8>,
    pub timestamp: String,
}

pub fn prepare_target(
    app: &tauri::AppHandle,
    db: &store::Database,
    version_id: &str,
    machine_id: &str,
    agent_profile_id: Option<i64>,
) -> anyhow::Result<store::DeployRun> {
    let version = db.get_deploy_version(version_id)?;
    if version.review_status != "approved" || has_blocking_findings(&version) {
        anyhow::bail!("deploy version must be approved and secret-clean before target prepare");
    }
    let stack = db.get_deploy_stack(&version.stack_id)?;
    let machine = db.get_workspace_machine(machine_id)?;
    let env = deploy_env::require_environment_ready(db, &version, &stack, machine_id)?;
    let agent = deploy_agent_profile(db, stack.workspace_id, agent_profile_id)?;
    let run = db.create_deploy_run(
        &stack.id,
        Some(version_id),
        Some(machine_id),
        "prepare",
        Some(&agent),
    )?;
    if machine.image_family == "windows" {
        let result = orchestrated_step(
            app,
            db,
            &run,
            &stack,
            &version,
            &machine,
            &agent,
            &env,
            "prepare",
            || prepare_windows_shared_folder(app, db, &run, &stack, &version, &machine),
        );
        finish_run(db, &run.id, result, "Windows target prepared")
    } else {
        let result = orchestrated_step(
            app,
            db,
            &run,
            &stack,
            &version,
            &machine,
            &agent,
            &env,
            "prepare",
            || prepare_linux_ssh(app, db, &run, &version, &machine),
        );
        finish_run(db, &run.id, result, "Linux target prepared")
    }
}

pub fn deploy_version(
    app: &tauri::AppHandle,
    db: &store::Database,
    version_id: &str,
    machine_id: &str,
    agent_profile_id: Option<i64>,
) -> anyhow::Result<store::DeployRun> {
    let version = db.get_deploy_version(version_id)?;
    if version.review_status != "approved" || has_blocking_findings(&version) {
        anyhow::bail!("deploy version must be approved and secret-clean before execution");
    }
    let stack = db.get_deploy_stack(&version.stack_id)?;
    let machine = db.get_workspace_machine(machine_id)?;
    let env = deploy_env::require_environment_ready(db, &version, &stack, machine_id)?;
    let agent = deploy_agent_profile(db, stack.workspace_id, agent_profile_id)?;
    let run = db.create_deploy_run(
        &stack.id,
        Some(version_id),
        Some(machine_id),
        "deploy",
        Some(&agent),
    )?;
    let result = orchestrated_step(
        app,
        db,
        &run,
        &stack,
        &version,
        &machine,
        &agent,
        &env,
        "deploy",
        || {
            stop_current_active_deploy(app, db, &run, &stack, &version, &machine).and_then(|_| {
                if machine.image_family == "windows" {
                    deploy_over_ssh(app, db, &run, &stack, &version, &machine, true)
                } else {
                    deploy_over_ssh(app, db, &run, &stack, &version, &machine, false)
                }
            })
        },
    );
    let run = finish_run(db, &run.id, result, "Deploy finished")?;
    if run.status == "passed" {
        db.set_active_deploy_version(&stack.id, version_id, machine_id)?;
    }
    Ok(run)
}

pub fn stop_stack(
    app: &tauri::AppHandle,
    db: &store::Database,
    stack_id: &str,
    machine_id: &str,
) -> anyhow::Result<store::DeployRun> {
    let stack = db.get_deploy_stack(stack_id)?;
    let machine = db.get_workspace_machine(machine_id)?;
    let run = db.create_deploy_run(
        &stack.id,
        stack.active_version_id.as_deref(),
        Some(machine_id),
        "stop",
        None,
    )?;
    let result = if let Some(version_id) = stack.active_version_id.as_deref() {
        let version = db.get_deploy_version(version_id)?;
        run_remote_compose_down(app, db, &run, &stack, &version, &machine)
    } else {
        record_step(
            app,
            db,
            &run,
            "noop",
            "passed",
            "No active deploy to stop",
            None,
        )?;
        Ok(())
    };
    finish_run(db, &run.id, result, "Stop finished")
}

pub fn reactivate_version(
    app: &tauri::AppHandle,
    db: &store::Database,
    version_id: &str,
    machine_id: &str,
    agent_profile_id: Option<i64>,
) -> anyhow::Result<store::DeployRun> {
    deploy_version(app, db, version_id, machine_id, agent_profile_id)
}

fn deploy_agent_profile(
    db: &store::Database,
    workspace_id: i64,
    agent_profile_id: Option<i64>,
) -> anyhow::Result<store::AgentProfile> {
    let Some(profile_id) = agent_profile_id else {
        anyhow::bail!("deploy_agent_required: select an agent profile before Prepare or Deploy");
    };
    let profile = db.get_agent_profile(profile_id)?;
    if profile.workspace_id != workspace_id {
        anyhow::bail!(
            "deploy_agent_workspace_mismatch: selected agent does not belong to this workspace"
        );
    }
    Ok(profile)
}

#[allow(clippy::too_many_arguments)]
fn orchestrated_step<F>(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    agent: &store::AgentProfile,
    env: &deploy_env::DeployEnvironment,
    operation: &str,
    mut execute: F,
) -> anyhow::Result<()>
where
    F: FnMut() -> anyhow::Result<()>,
{
    agent_precheck(app, db, run, stack, version, machine, agent, env, operation)?;
    let mut attempts = Vec::<Value>::new();
    for attempt in 1..=deploy_repair::MAX_REPAIR_ATTEMPTS {
        match execute() {
            Ok(()) => {
                let message = if attempts.is_empty() {
                    "Agent-assisted verification passed with redacted run evidence".to_string()
                } else {
                    format!(
                        "Agent-assisted verification passed after {} repair attempt(s)",
                        attempts.len()
                    )
                };
                agent_postcheck(
                    app, db, run, stack, version, machine, agent, operation, "passed", &message,
                )?;
                return Ok(());
            }
            Err(error) => {
                let message = redact_error_chain(&error);
                let diagnosis = deploy_repair::classify_failure(&message);
                let step_status = if diagnosis.safe_recipe.is_some()
                    && attempt < deploy_repair::MAX_REPAIR_ATTEMPTS
                {
                    "retrying"
                } else {
                    "blocked"
                };
                record_step(
                    app,
                    db,
                    run,
                    "deploy-doctor",
                    step_status,
                    &format!(
                        "Attempt {attempt}/{}: {} ({}) - {}",
                        deploy_repair::MAX_REPAIR_ATTEMPTS,
                        diagnosis.title,
                        diagnosis.code,
                        diagnosis.next_step
                    ),
                    Some(version),
                )?;
                attempts.push(json!({
                    "attempt": attempt,
                    "error": message,
                    "diagnosis": diagnosis,
                }));

                if let Some(recipe) = attempts
                    .last()
                    .and_then(|attempt| attempt.get("diagnosis"))
                    .and_then(|diagnosis| diagnosis.get("safe_recipe"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                {
                    if attempt < deploy_repair::MAX_REPAIR_ATTEMPTS {
                        update_repair_report(
                            db, run, stack, version, machine, agent, operation, "retrying",
                            &attempts, None, None,
                        )?;
                        apply_safe_repair_recipe(app, db, run, version, &recipe, attempt)?;
                        continue;
                    }
                }

                match request_agent_repair(
                    app, db, run, stack, version, machine, agent, operation, &attempts,
                ) {
                    Ok(Some(report)) => {
                        let report_value = deploy_repair::report_to_value(&report);
                        let validation = deploy_repair::validate_agent_repair_for_ade(&report);
                        let repair_pending =
                            validation.ade_safe_to_apply && !report.patch_set.is_empty();
                        update_repair_report(
                            db,
                            run,
                            stack,
                            version,
                            machine,
                            agent,
                            operation,
                            if repair_pending {
                                "repair_pending"
                            } else {
                                "blocked"
                            },
                            &attempts,
                            Some(report_value),
                            Some(json!(validation.clone())),
                        )?;
                        let status = if repair_pending {
                            "repair_pending"
                        } else {
                            "blocked"
                        };
                        let user_message = if report.user_message.trim().is_empty() {
                            "Agente analisou a falha, mas nao devolveu uma correcao aplicavel."
                                .to_string()
                        } else {
                            report.user_message.clone()
                        };
                        record_step(
                            app,
                            db,
                            run,
                            "agent-repair",
                            status,
                            &redact(&user_message),
                            Some(version),
                        )?;
                        if status == "repair_pending" {
                            anyhow::bail!("deploy_repair_pending: {user_message}");
                        }
                        let validation_message = if validation.validation_errors.is_empty() {
                            user_message
                        } else {
                            format!(
                                "{} Validation errors: {}",
                                user_message,
                                validation.validation_errors.join("; ")
                            )
                        };
                        anyhow::bail!("deploy_agent_repair_blocked: {validation_message}");
                    }
                    Ok(None) => {
                        update_repair_report(
                            db, run, stack, version, machine, agent, operation, "blocked",
                            &attempts, None, None,
                        )?;
                        let _ = agent_postcheck(
                            app,
                            db,
                            run,
                            stack,
                            version,
                            machine,
                            agent,
                            operation,
                            "blocked",
                            &format!("Agent-assisted verification blocked the run: {message}"),
                        );
                        return Err(error);
                    }
                    Err(agent_error) => {
                        record_step(
                            app,
                            db,
                            run,
                            "agent-repair",
                            "blocked",
                            &format!(
                                "Agent repair failed; original deploy error remains: {}",
                                redact_error_chain(&agent_error)
                            ),
                            Some(version),
                        )?;
                        update_repair_report(
                            db, run, stack, version, machine, agent, operation, "blocked",
                            &attempts, None, None,
                        )?;
                        return Err(error);
                    }
                }
            }
        }
    }
    anyhow::bail!("deploy_repair_exhausted: repair loop exhausted without a final deploy result")
}

#[allow(clippy::too_many_arguments)]
fn agent_precheck(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    agent: &store::AgentProfile,
    env: &deploy_env::DeployEnvironment,
    operation: &str,
) -> anyhow::Result<()> {
    let missing_scripts = missing_runbook_scripts(version);
    let target_ok = assisted_deploy_target_ok(machine);
    let running_ok = machine.status == "running";
    let status = if missing_scripts.is_empty() && target_ok && running_ok {
        "passed"
    } else {
        "blocked"
    };
    let message = if status == "passed" {
        format!(
            "Orchestration profile {} passed {operation} precheck for {} using runbook {}",
            agent.name, machine.display_name, DEPLOY_RUNBOOK_VERSION
        )
    } else if !target_ok {
        "deploy_agent_target_scope: v2 assisted deploy supports Ubuntu Server Deploy VM, Ubuntu Desktop Deploy VM, or Windows 11"
            .to_string()
    } else if !running_ok {
        "deploy_target_not_running: target VM must be running before assisted deploy".to_string()
    } else {
        format!(
            "deploy_runbook_incomplete: missing scripts {}",
            missing_scripts.join(", ")
        )
    };
    let report = json!({
        "schema_version": "1.0",
        "operation": operation,
        "agent": agent_report(agent),
        "stack": {
            "id": stack.id,
            "slug": stack.slug,
            "version": version.label,
        },
        "target": {
            "machine_id": machine.id,
            "display_name": machine.display_name,
            "preset_id": machine.preset_id,
            "status": machine.status,
        },
        "runbook": {
            "version": DEPLOY_RUNBOOK_VERSION,
            "missing_scripts": missing_scripts,
        },
        "environment": {
            "required_count": env.required_count,
            "saved_count": env.saved_count,
            "missing_keys": env.missing_keys,
            "secret_values": "redacted",
        },
        "decision": status,
        "message": message.clone(),
    });
    db.update_deploy_run_orchestration(run.id.as_str(), status, &report.to_string())?;
    record_step(
        app,
        db,
        run,
        "agent-precheck",
        status,
        &message,
        Some(version),
    )?;
    if status == "passed" {
        Ok(())
    } else {
        anyhow::bail!(message)
    }
}

#[allow(clippy::too_many_arguments)]
fn agent_postcheck(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    agent: &store::AgentProfile,
    operation: &str,
    status: &str,
    message: &str,
) -> anyhow::Result<()> {
    let report = json!({
        "schema_version": "1.0",
        "operation": operation,
        "agent": agent_report(agent),
        "stack": {
            "id": stack.id,
            "slug": stack.slug,
            "version": version.label,
        },
        "target": {
            "machine_id": machine.id,
            "display_name": machine.display_name,
            "preset_id": machine.preset_id,
        },
        "decision": status,
        "message": redact(message),
        "secret_values": "redacted",
    });
    db.update_deploy_run_orchestration(run.id.as_str(), status, &report.to_string())?;
    record_step(
        app,
        db,
        run,
        "agent-postcheck",
        status,
        &redact(message),
        Some(version),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_repair_report(
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    agent: &store::AgentProfile,
    operation: &str,
    status: &str,
    attempts: &[Value],
    agent_repair: Option<Value>,
    repair_validation: Option<Value>,
) -> anyhow::Result<()> {
    let agent_safe_to_apply = agent_repair
        .as_ref()
        .and_then(|value| value.get("safe_to_apply"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ade_safe_to_apply = repair_validation
        .as_ref()
        .and_then(|value| value.get("ade_safe_to_apply"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let latest_error = attempts
        .last()
        .and_then(|attempt| attempt.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let report = json!({
        "schema_version": "1.0",
        "operation": operation,
        "agent": agent_report(agent),
        "stack": {
            "id": stack.id,
            "slug": stack.slug,
            "version": version.label,
        },
        "target": {
            "machine_id": machine.id,
            "display_name": machine.display_name,
            "preset_id": machine.preset_id,
            "status": machine.status,
        },
        "decision": status,
        "failure": {
            "full_error_chain": latest_error,
        },
        "repair": {
            "mode": "deploy_doctor",
            "max_attempts": deploy_repair::MAX_REPAIR_ATTEMPTS,
            "attempts": attempts,
            "agent_repair": agent_repair,
            "validation": repair_validation,
            "agent_safe_to_apply": agent_safe_to_apply,
            "ade_safe_to_apply": ade_safe_to_apply,
            "patch_policy": "package_scripts_and_runbook_only",
            "versioning": "agent patches create a new package version",
            "secret_values": "redacted",
        }
    });
    db.update_deploy_run_orchestration(run.id.as_str(), status, &report.to_string())?;
    Ok(())
}

fn apply_safe_repair_recipe(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    version: &store::DeployVersion,
    recipe: &str,
    attempt: usize,
) -> anyhow::Result<()> {
    let wait_seconds = match recipe {
        "wait_and_retry_package_manager" | "wait_and_retry_apt_clock" => 10,
        "wait_and_retry_ssh" => 6,
        _ => 3,
    };
    record_step(
        app,
        db,
        run,
        "repair-recipe",
        "retrying",
        &format!(
            "Applying safe recipe {recipe}; waiting {wait_seconds}s before retry {}",
            attempt + 1
        ),
        Some(version),
    )?;
    thread::sleep(Duration::from_secs(wait_seconds));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn request_agent_repair(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    agent_profile: &store::AgentProfile,
    operation: &str,
    attempts: &[Value],
) -> anyhow::Result<Option<deploy_repair::AgentRepairReport>> {
    record_step(
        app,
        db,
        run,
        "agent-repair",
        "running",
        "Known safe recipes did not finish the deploy; asking the selected agent for a package-local repair plan",
        Some(version),
    )?;
    let context = repair_context(db, run, stack, version, machine, operation, attempts)?;
    let prompt = deploy_repair::repair_prompt(&context)?;
    let workspace = db.get_workspace(stack.workspace_id)?;
    let result = agent::run_agent_prompt_blocking(
        app,
        db,
        agent_profile,
        None,
        &workspace.root_path,
        "Deploy repair",
        &prompt,
        Some(json!({
            "kind": "deploy_repair",
            "run_id": run.id,
            "version_id": version.id,
            "machine_id": machine.id,
            "operation": operation,
        })),
        Duration::from_secs(180),
    )?;
    let report = deploy_repair::parse_agent_repair_report(&result.assistant_output)?;
    Ok(Some(report))
}

fn repair_context(
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    operation: &str,
    attempts: &[Value],
) -> anyhow::Result<Value> {
    let steps = db
        .list_deploy_run_steps(&run.id)?
        .into_iter()
        .map(|step| {
            json!({
                "step_key": step.step_key,
                "status": step.status,
                "message": redact(&step.message),
                "error_code": step.error_code,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema_version": "1.0",
        "operation": operation,
        "run": {
            "id": run.id,
            "status": run.status,
            "summary": redact(&run.summary),
            "steps": steps,
        },
        "stack": {
            "id": stack.id,
            "name": stack.name,
            "slug": stack.slug,
        },
        "version": {
            "id": version.id,
            "label": version.label,
            "artifact_path": version.artifact_path,
            "manifest": read_artifact_preview(version, "manifest.json"),
            "manifest_summary": package_manifest_summary(version),
            "deploy_plan": read_artifact_preview(version, "analysis/deploy-plan.json"),
            "runbook": read_artifact_preview(version, "RUNBOOK.md"),
            "scripts": package_script_inventory(version),
            "script_previews": {
                "install_deploy_ps1": read_artifact_preview(version, "scripts/install-deploy.ps1"),
                "deploy_ps1": read_artifact_preview(version, "scripts/deploy.ps1"),
                "preflight_sh": read_artifact_preview(version, "scripts/preflight.sh"),
                "deploy_sh": read_artifact_preview(version, "scripts/deploy.sh"),
                "build_dev_sh": read_artifact_preview(version, "scripts/build-dev.sh"),
            },
        },
        "target": {
            "machine_id": machine.id,
            "display_name": machine.display_name,
            "preset_id": machine.preset_id,
            "image_family": machine.image_family,
            "status": machine.status,
            "ssh_port": machine.ssh_port,
            "access_user": machine.access_user,
        },
        "repair_attempts": attempts,
        "constraints": {
            "max_attempts": deploy_repair::MAX_REPAIR_ATTEMPTS,
            "allowed_patch_paths": ["scripts/*", "RUNBOOK.md"],
            "forbidden": ["source repository edits", "secret values", "absolute paths", "parent directory traversal"],
            "patch_versioning": "approved patches create a new deploy package version",
        },
        "secret_values": "redacted",
    }))
}

fn read_artifact_preview(version: &store::DeployVersion, relative_path: &str) -> Option<String> {
    let path = Path::new(&version.artifact_path).join(relative_path);
    let text = std::fs::read_to_string(path).ok()?;
    let redacted = redact(&text);
    Some(redacted.chars().take(24 * 1024).collect())
}

fn package_manifest_summary(version: &store::DeployVersion) -> Value {
    let Ok(manifest) = serde_json::from_str::<Value>(&version.manifest_json) else {
        return json!({ "error": "manifest_json is not valid JSON" });
    };
    let projects = manifest
        .get("projects")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|project| {
                    json!({
                        "name": project.get("name").and_then(Value::as_str),
                        "package_path": project.get("package_path").and_then(Value::as_str),
                        "deploy_strategy": project.get("deploy_strategy").and_then(Value::as_str),
                        "language": project.get("language").and_then(Value::as_str),
                        "framework": project.get("framework").and_then(Value::as_str),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "deploy_strategy": manifest.get("deploy_strategy").and_then(Value::as_str),
        "projects": projects,
        "runbook_scripts": manifest
            .get("runbook")
            .and_then(|runbook| runbook.get("scripts"))
            .cloned()
            .unwrap_or_else(|| json!([])),
    })
}

fn package_script_inventory(version: &store::DeployVersion) -> Value {
    let scripts_dir = Path::new(&version.artifact_path).join("scripts");
    let mut scripts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(scripts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            scripts.push(json!({
                "path": format!("scripts/{name}"),
                "bytes": path.metadata().map(|metadata| metadata.len()).unwrap_or_default(),
            }));
        }
    }
    scripts.sort_by(|left, right| {
        left.get("path")
            .and_then(Value::as_str)
            .cmp(&right.get("path").and_then(Value::as_str))
    });
    json!(scripts)
}

fn assisted_deploy_target_ok(machine: &store::WorkspaceMachine) -> bool {
    matches!(
        machine.preset_id.as_str(),
        "ubuntu_deploy_vm" | "ubuntu_desktop_deploy_vm" | "windows_11"
    )
}

fn is_linux_deploy_target(machine: &store::WorkspaceMachine) -> bool {
    machine.image_family == "linux_cloud"
        || matches!(
            machine.preset_id.as_str(),
            "ubuntu_deploy_vm" | "ubuntu_desktop_deploy_vm"
        )
}

fn agent_report(agent: &store::AgentProfile) -> serde_json::Value {
    json!({
        "profile_id": agent.id,
        "name": agent.name.clone(),
        "provider": agent.provider.clone(),
        "model": agent.model.clone(),
        "context_mode": agent.context_mode.clone(),
        "sandbox": agent.sandbox.clone(),
    })
}

fn missing_runbook_scripts(version: &store::DeployVersion) -> Vec<String> {
    deploy_runbook_scripts()
        .into_iter()
        .filter(|script| !Path::new(&version.artifact_path).join(script).is_file())
        .map(ToOwned::to_owned)
        .collect()
}

fn prepare_linux_ssh(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
) -> anyhow::Result<()> {
    if !is_linux_deploy_target(machine) {
        let message = "unsupported_deploy_target: create or select an Ubuntu Server or Desktop Deploy VM for automatic deploy";
        record_step(
            app,
            db,
            run,
            "linux-target",
            "failed",
            message,
            Some(version),
        )?;
        anyhow::bail!(message);
    }
    let Some(port) = machine.ssh_port else {
        let message = "ssh_port_missing: WinBox did not expose an SSH port";
        record_step(
            app,
            db,
            run,
            "linux-preflight",
            "failed",
            message,
            Some(version),
        )?;
        anyhow::bail!(message);
    };
    match run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        "printf 'ssh-ok\\n'",
    ))
    .context("linux target SSH check failed")
    {
        Ok(output) => record_step(
            app,
            db,
            run,
            "linux-ssh",
            "passed",
            &redact(&output),
            Some(version),
        )?,
        Err(error) => {
            let message = format!(
                "ssh_unavailable: WinBox exposed SSH port {port}, but the target did not accept a non-interactive SSH command: {error}"
            );
            record_step(
                app,
                db,
                run,
                "linux-ssh",
                "failed",
                &redact(&message),
                Some(version),
            )?;
            anyhow::bail!(message);
        }
    };

    if let Ok(output) = linux_docker_preflight(port, machine.access_user.as_deref()) {
        record_step(
            app,
            db,
            run,
            "linux-preflight",
            "passed",
            &redact(&output),
            Some(version),
        )?;
        return Ok(());
    }

    record_step(
        app,
        db,
        run,
        "linux-base",
        "running",
        "Docker/Compose not ready on target; installing base dependencies from package runbook",
        Some(version),
    )?;
    install_linux_base_from_package(app, db, run, version, machine, port)?;
    match linux_docker_preflight_with_retry(port, machine.access_user.as_deref()) {
        Ok(output) => {
            record_step(
                app,
                db,
                run,
                "linux-preflight",
                "passed",
                &redact(&output),
                Some(version),
            )?;
            Ok(())
        }
        Err(error) => {
            let message = format!(
                "linux_base_missing: Prepare installed/repaired the Linux base, but Docker/Compose still did not pass preflight: {error}"
            );
            record_step(
                app,
                db,
                run,
                "linux-preflight",
                "failed",
                &redact(&message),
                Some(version),
            )?;
            anyhow::bail!(message)
        }
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn linux_docker_preflight(port: i64, user: Option<&str>) -> anyhow::Result<String> {
    run_command(ssh_command(
        port,
        user,
        "docker --version && docker compose version",
    ))
    .context("linux Docker/Compose preflight failed")
}

fn linux_docker_preflight_with_retry(port: i64, user: Option<&str>) -> anyhow::Result<String> {
    let mut last_error = None;
    for attempt in 1..=3 {
        match linux_docker_preflight(port, user) {
            Ok(output) => return Ok(output),
            Err(error) => {
                last_error = Some(error);
                if attempt < 3 {
                    let _ = run_command(ssh_command(
                        port,
                        user,
                        "sudo systemctl restart docker >/dev/null 2>&1 || sudo service docker start >/dev/null 2>&1 || true; sleep 5",
                    ));
                }
            }
        }
    }
    Err(last_error.expect("linux preflight retry loop must run at least once"))
}

fn install_linux_base_from_package(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    port: i64,
) -> anyhow::Result<()> {
    let stack = db.get_deploy_stack(&version.stack_id)?;
    let remote_dir = linux_remote_deploy_dir(&stack, version);
    let mkdir_output = run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        &format!("mkdir -p {remote_dir}"),
    ))
    .context("linux base remote mkdir failed")?;
    record_step(
        app,
        db,
        run,
        "linux-base-mkdir",
        "passed",
        &redact(&mkdir_output),
        Some(version),
    )?;

    let staged = stage_artifact_with_environment(db, version, &stack, &machine.id)?;
    let rsync_output = run_command(rsync_command(
        port,
        machine.access_user.as_deref(),
        &staged.path().display().to_string(),
        &remote_dir,
    ))
    .context("linux base package transfer failed")?;
    record_step(
        app,
        db,
        run,
        "linux-base-transfer",
        "passed",
        &redact(&rsync_output),
        Some(version),
    )?;

    let host_epoch = chrono::Utc::now().timestamp().to_string();
    let install_command = format!(
        "cd {remote_dir} && DW_HOST_EPOCH={} DW_SSH_USER={} sh scripts/install-base-linux.sh",
        shell_quote(&host_epoch),
        shell_quote(machine.access_user.as_deref().unwrap_or("")),
    );
    let install_output = run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        &install_command,
    ))
    .context("linux base install failed")?;
    record_step(
        app,
        db,
        run,
        "linux-base",
        "passed",
        &redact(&install_output),
        Some(version),
    )?;
    Ok(())
}

fn prepare_windows_shared_folder(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
) -> anyhow::Result<()> {
    let provider = winbox_provider::discover()
        .ok_or_else(|| anyhow::anyhow!("winbox_not_found: WinBox CLI is required"))?;
    let profile = provider
        .profile(&machine.provider_profile)
        .map_err(|error| anyhow::anyhow!("profile_not_found: {}", error.message))?;
    let shared_dir = profile
        .shared_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("shared_dir_missing: WinBox profile did not expose SHARED_DIR")
        })?;
    let destination = Path::new(shared_dir)
        .join("deploy-packages")
        .join(&stack.slug)
        .join(&version.label);
    let staged = stage_artifact_with_environment(db, version, stack, &machine.id)?;
    copy_dir(staged.path(), &destination)?;
    let bootstrap_script = windows_shared_guest_path(&["ade", "bootstrap-windows.ps1"]);
    let install_script = windows_shared_guest_path(&[
        "deploy-packages",
        &stack.slug,
        &version.label,
        "scripts",
        "install-deploy.ps1",
    ]);
    let message = format!(
        "Package copied to the WinBox shared folder. Inside Windows, run PowerShell as Administrator: powershell -NoProfile -ExecutionPolicy Bypass -File \"{bootstrap_script}\"; powershell -NoProfile -ExecutionPolicy Bypass -File \"{install_script}\". Then validate SSH and retry Deploy."
    );
    record_step(
        app,
        db,
        run,
        "windows-shared-folder",
        "passed",
        &message,
        Some(version),
    )?;
    Ok(())
}

fn deploy_over_ssh(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    windows: bool,
) -> anyhow::Result<()> {
    if !windows && machine.image_family != "linux_cloud" && machine.preset_id != "ubuntu_deploy_vm"
    {
        anyhow::bail!(
            "unsupported_deploy_target: create or select an Ubuntu Server or Desktop Deploy VM for automatic deploy"
        );
    }
    let port = machine.ssh_port.ok_or_else(|| {
        anyhow::anyhow!("ssh_port_missing: target has not been bootstrapped for SSH")
    })?;
    ensure_deploy_ssh_ready(
        app,
        db,
        run,
        version,
        port,
        machine.access_user.as_deref(),
        windows,
    )?;
    if windows && !windows_artifact_has_remote_runbook(version) {
        let message = windows_runbook_manual_required_message(stack, version);
        record_step(
            app,
            db,
            run,
            "windows-runbook",
            "failed",
            &message,
            Some(version),
        )?;
        anyhow::bail!("{message}");
    }
    if windows {
        return deploy_windows_shared_runbook(app, db, run, stack, version, machine, port);
    }

    let remote_dir = linux_remote_deploy_dir(stack, version);
    let mkdir = format!("mkdir -p {remote_dir}");
    let mkdir_output = run_command(ssh_command(port, machine.access_user.as_deref(), &mkdir))
        .context("remote mkdir failed")?;
    record_step(
        app,
        db,
        run,
        "remote-mkdir",
        "passed",
        &redact(&mkdir_output),
        Some(version),
    )?;

    let staged = stage_artifact_with_environment(db, version, stack, &machine.id)?;
    let rsync_output = run_command(rsync_command(
        port,
        machine.access_user.as_deref(),
        &staged.path().display().to_string(),
        &remote_dir,
    ))
    .context("rsync transfer failed")?;
    record_step(
        app,
        db,
        run,
        "transfer",
        "passed",
        &redact(&rsync_output),
        Some(version),
    )?;

    let compose_project = compose_project_name(&stack.slug, &version.label);
    run_remote_script_with_retry(
        app,
        db,
        run,
        version,
        port,
        machine.access_user.as_deref(),
        &remote_dir,
        &compose_project,
        "scripts/preflight.sh",
        "runbook-preflight",
    )?;
    let deploy_command = format!(
        "cd {remote_dir} && chmod +x scripts/deploy.sh && DW_COMPOSE_PROJECT_NAME={} ./scripts/deploy.sh",
        shell_quote(&compose_project)
    );
    let deploy_output = run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        &deploy_command,
    ))
    .context("runbook deploy failed")?;
    record_step(
        app,
        db,
        run,
        "runbook-deploy",
        "passed",
        &redact(&deploy_output),
        Some(version),
    )?;
    run_remote_script_with_retry(
        app,
        db,
        run,
        version,
        port,
        machine.access_user.as_deref(),
        &remote_dir,
        &compose_project,
        "scripts/healthcheck.sh",
        "runbook-healthcheck",
    )?;
    run_remote_script_with_retry(
        app,
        db,
        run,
        version,
        port,
        machine.access_user.as_deref(),
        &remote_dir,
        &compose_project,
        "scripts/logs.sh",
        "runbook-logs",
    )?;
    Ok(())
}

fn windows_shared_guest_path(parts: &[&str]) -> String {
    let suffix = parts
        .iter()
        .map(|part| part.trim_matches(['\\', '/']))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\\");
    if suffix.is_empty() {
        "\\\\host.lan\\Data".to_string()
    } else {
        format!("\\\\host.lan\\Data\\{suffix}")
    }
}

fn linux_remote_deploy_dir(stack: &store::DeployStack, version: &store::DeployVersion) -> String {
    format!("~/dw-deploy/{}/{}", stack.slug, version.label)
}

fn ensure_deploy_ssh_ready(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    version: &store::DeployVersion,
    port: i64,
    user: Option<&str>,
    windows: bool,
) -> anyhow::Result<()> {
    match run_command(ssh_command(
        port,
        user,
        deploy_ssh_preflight_command(windows),
    )) {
        Ok(output) => {
            record_step(
                app,
                db,
                run,
                "ssh-preflight",
                "passed",
                &redact(&output),
                Some(version),
            )?;
            Ok(())
        }
        Err(error) => {
            let message = deploy_ssh_unavailable_message(port, windows, &error.to_string());
            record_step(
                app,
                db,
                run,
                "ssh-preflight",
                "failed",
                &message,
                Some(version),
            )?;
            anyhow::bail!("{message}")
        }
    }
}

fn deploy_ssh_preflight_command(_windows: bool) -> &'static str {
    "echo ssh-ok"
}

fn deploy_ssh_unavailable_message(port: i64, windows: bool, error: &str) -> String {
    let details = redact(error);
    if windows {
        format!(
            "windows_ssh_bootstrap_required: OpenSSH is not reachable on 127.0.0.1:{port}. Run PowerShell as Administrator inside Windows: powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\". Then validate SSH and retry Deploy. Details: {details}",
            windows_shared_guest_path(&["ade", "bootstrap-windows.ps1"])
        )
    } else {
        format!(
            "ssh_unavailable: target did not accept a non-interactive SSH command on 127.0.0.1:{port}. Details: {details}"
        )
    }
}

fn windows_artifact_has_remote_runbook(version: &store::DeployVersion) -> bool {
    windows_artifact_script_exists(version, "deploy.ps1")
}

fn windows_pre_deploy_scripts(version: &store::DeployVersion) -> Vec<&'static str> {
    if windows_artifact_script_exists(version, "install-deploy.ps1") {
        vec!["install-deploy.ps1"]
    } else {
        Vec::new()
    }
}

fn windows_artifact_script_exists(version: &store::DeployVersion, script_name: &str) -> bool {
    Path::new(&version.artifact_path)
        .join("scripts")
        .join(script_name)
        .is_file()
}

fn windows_runbook_manual_required_message(
    stack: &store::DeployStack,
    version: &store::DeployVersion,
) -> String {
    let install_script = windows_shared_guest_path(&[
        "deploy-packages",
        &stack.slug,
        &version.label,
        "scripts",
        "install-deploy.ps1",
    ]);
    format!(
        "windows_runbook_manual_required: this package does not include scripts/deploy.ps1, so ADE will not run the Unix shell runbook over Windows SSH. Run inside the Windows desktop session with PowerShell as Administrator: powershell -NoProfile -ExecutionPolicy Bypass -File \"{install_script}\"."
    )
}

fn deploy_windows_shared_runbook(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    port: i64,
) -> anyhow::Result<()> {
    for script_name in windows_pre_deploy_scripts(version) {
        run_windows_shared_script(
            app,
            db,
            run,
            stack,
            version,
            machine,
            port,
            script_name,
            "runbook-install",
            &[],
        )?;
    }

    let compose_project = compose_project_name(&stack.slug, &version.label);
    let deploy_script = windows_shared_guest_path(&[
        "deploy-packages",
        &stack.slug,
        &version.label,
        "scripts",
        "deploy.ps1",
    ]);
    let deploy_output = run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        &windows_powershell_file_command(
            &deploy_script,
            &[("ComposeProjectName", compose_project.as_str())],
        ),
    ))
    .context("windows runbook deploy failed")?;
    record_step(
        app,
        db,
        run,
        "runbook-deploy",
        "passed",
        &redact(&deploy_output),
        Some(version),
    )?;

    if windows_artifact_script_exists(version, "healthcheck.ps1") {
        run_windows_shared_script(
            app,
            db,
            run,
            stack,
            version,
            machine,
            port,
            "healthcheck.ps1",
            "runbook-healthcheck",
            &[],
        )?;
    }
    if windows_artifact_script_exists(version, "logs.ps1") {
        run_windows_shared_script(
            app,
            db,
            run,
            stack,
            version,
            machine,
            port,
            "logs.ps1",
            "runbook-logs",
            &[],
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_windows_shared_script(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
    port: i64,
    script_name: &str,
    step_key: &str,
    args: &[(&str, &str)],
) -> anyhow::Result<()> {
    let script = windows_shared_guest_path(&[
        "deploy-packages",
        &stack.slug,
        &version.label,
        "scripts",
        script_name,
    ]);
    let output = match run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        &windows_powershell_file_command(&script, args),
    )) {
        Ok(output) => output,
        Err(error) => {
            let message = format!(
                "{step_key} failed while running {script_name}: {}",
                redact_error_chain(&error)
            );
            record_step(app, db, run, step_key, "failed", &message, Some(version))?;
            return Err(error).with_context(|| format!("{step_key} failed"));
        }
    };
    record_step(
        app,
        db,
        run,
        step_key,
        "passed",
        &redact(&output),
        Some(version),
    )?;
    Ok(())
}

fn windows_powershell_file_command(script_path: &str, args: &[(&str, &str)]) -> String {
    let mut command = format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -File {}",
        windows_command_arg(script_path)
    );
    for (key, value) in args {
        command.push_str(&format!(" -{} {}", key, windows_command_arg(value)));
    }
    command
}

fn windows_command_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[allow(clippy::too_many_arguments)]
fn run_remote_script_with_retry(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    version: &store::DeployVersion,
    port: i64,
    user: Option<&str>,
    remote_dir: &str,
    compose_project: &str,
    script: &str,
    step_key: &str,
) -> anyhow::Result<()> {
    let command = format!(
        "cd {remote_dir} && chmod +x {} && DW_COMPOSE_PROJECT_NAME={} ./{}",
        shell_quote(script),
        shell_quote(compose_project),
        shell_quote(script)
    );
    let first = run_command(ssh_command(port, user, &command));
    match first {
        Ok(output) => {
            record_step(
                app,
                db,
                run,
                step_key,
                "passed",
                &redact(&output),
                Some(version),
            )?;
            Ok(())
        }
        Err(error) => {
            record_step(
                app,
                db,
                run,
                step_key,
                "retrying",
                &format!("{} failed once; retrying once", redact(&error.to_string())),
                Some(version),
            )?;
            let output = run_command(ssh_command(port, user, &command))
                .with_context(|| format!("{step_key} failed after one retry"))?;
            record_step(
                app,
                db,
                run,
                step_key,
                "passed",
                &redact(&output),
                Some(version),
            )?;
            Ok(())
        }
    }
}

fn stop_current_active_deploy(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
) -> anyhow::Result<()> {
    let Some(active_stack) = db.active_deploy_for_machine(stack.workspace_id, &machine.id)? else {
        return Ok(());
    };
    let Some(active_version_id) = active_stack.active_version_id.as_deref() else {
        return Ok(());
    };
    if active_stack.id == stack.id && active_version_id == version.id {
        return Ok(());
    }

    let active_version = db.get_deploy_version(active_version_id)?;
    record_step(
        app,
        db,
        run,
        "stop-active",
        "running",
        &format!(
            "Stopping active stack {} {} before activating {}",
            active_stack.name, active_version.label, version.label
        ),
        Some(version),
    )?;
    run_remote_compose_down(app, db, run, &active_stack, &active_version, machine)?;
    Ok(())
}

fn run_remote_compose_down(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    machine: &store::WorkspaceMachine,
) -> anyhow::Result<()> {
    let port = machine
        .ssh_port
        .ok_or_else(|| anyhow::anyhow!("ssh_port_missing: target has no SSH port"))?;
    if machine.image_family == "windows" {
        if windows_artifact_script_exists(version, "stop.ps1") {
            run_windows_shared_script(
                app,
                db,
                run,
                stack,
                version,
                machine,
                port,
                "stop.ps1",
                "compose-down",
                &[],
            )?;
        } else {
            record_step(
                app,
                db,
                run,
                "compose-down",
                "passed",
                "No Windows stop.ps1 found; nothing managed to stop",
                Some(version),
            )?;
        }
        db.update_deploy_version_status(&version.id, "stopped")?;
        return Ok(());
    }
    let remote_dir = format!("~/dw-deploy/{}/{}", stack.slug, version.label);
    let output = run_command(ssh_command(
        port,
        machine.access_user.as_deref(),
        &format!(
            "cd {remote_dir} && if [ -f scripts/stop.sh ]; then chmod +x scripts/stop.sh && DW_COMPOSE_PROJECT_NAME={} ./scripts/stop.sh; else docker compose --project-name {} down; fi",
            compose_project_name(&stack.slug, &version.label),
            compose_project_name(&stack.slug, &version.label)
        ),
    ))
    .context("compose stop failed")?;
    record_step(
        app,
        db,
        run,
        "compose-down",
        "passed",
        &redact(&output),
        Some(version),
    )?;
    db.update_deploy_version_status(&version.id, "stopped")?;
    Ok(())
}

fn finish_run(
    db: &store::Database,
    run_id: &str,
    result: anyhow::Result<()>,
    success_summary: &str,
) -> anyhow::Result<store::DeployRun> {
    match result {
        Ok(()) => db.complete_deploy_run(run_id, "passed", success_summary),
        Err(error) => db.complete_deploy_run(run_id, "failed", &redact_error_chain(&error)),
    }
}

fn record_step(
    app: &tauri::AppHandle,
    db: &store::Database,
    run: &store::DeployRun,
    step_key: &str,
    status: &str,
    message: &str,
    version: Option<&store::DeployVersion>,
) -> anyhow::Result<store::DeployRunStep> {
    let step = db.add_deploy_run_step(run.id.as_str(), step_key, status, message, None, None)?;
    let _ = app.emit(
        "deploy://progress",
        DeployProgressEvent {
            run_id: run.id.clone(),
            stack_id: run.stack_id.clone(),
            version_id: version
                .map(|item| item.id.clone())
                .or_else(|| run.version_id.clone()),
            machine_id: run.machine_id.clone(),
            step_key: step_key.to_string(),
            status: status.to_string(),
            message: message.to_string(),
            percent: None,
            timestamp: step.started_at.clone(),
        },
    );
    Ok(step)
}

pub fn ssh_command(port: i64, user: Option<&str>, remote_command: &str) -> Command {
    let mut command = Command::new("ssh");
    let target = ssh_target(user);
    command
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=8")
        .arg("-o")
        .arg("ConnectionAttempts=1")
        .arg("-o")
        .arg("ServerAliveInterval=15")
        .arg("-o")
        .arg("ServerAliveCountMax=4")
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-o")
        .arg("LogLevel=ERROR")
        .arg("-p")
        .arg(port.to_string())
        .arg(target)
        .arg(remote_command);
    command
}

pub fn rsync_command(port: i64, user: Option<&str>, source: &str, remote_dir: &str) -> Command {
    let mut command = Command::new("rsync");
    let target = ssh_target(user);
    command
        .arg("-az")
        .arg("-e")
        .arg(format!(
            "ssh -p {port} -o BatchMode=yes -o ConnectTimeout=8 -o ConnectionAttempts=1 -o ServerAliveInterval=15 -o ServerAliveCountMax=4 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"
        ))
        .arg(format!("{}/", source.trim_end_matches('/')))
        .arg(format!("{target}:{remote_dir}/"));
    command
}

fn ssh_target(user: Option<&str>) -> String {
    match user.map(str::trim).filter(|value| !value.is_empty()) {
        Some(user) => format!("{user}@127.0.0.1"),
        None => "127.0.0.1".to_string(),
    }
}

fn run_command(mut command: Command) -> anyhow::Result<String> {
    let program = command.get_program().to_string_lossy().to_string();
    let output = command
        .output()
        .with_context(|| format!("failed to execute {program}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}").trim().to_string();
    if output.status.success() {
        Ok(combined)
    } else {
        anyhow::bail!(
            "{program} failed with {}: {}",
            output.status,
            redact(&combined)
        )
    }
}

fn error_chain(error: &Error) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

fn redact_error_chain(error: &Error) -> String {
    redact(&error_chain(error))
}

fn copy_dir(source: &Path, destination: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    for entry in
        std::fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &dest_path)?;
        } else if source_path.is_file() {
            std::fs::copy(&source_path, &dest_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    dest_path.display()
                )
            })?;
        }
    }
    Ok(())
}

struct StagedDeployPackage {
    path: PathBuf,
}

impl StagedDeployPackage {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StagedDeployPackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn stage_artifact_with_environment(
    db: &store::Database,
    version: &store::DeployVersion,
    stack: &store::DeployStack,
    machine_id: &str,
) -> anyhow::Result<StagedDeployPackage> {
    let root = std::env::temp_dir().join(format!(
        "dw-deploy-runtime-{}-{}",
        version.id,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    copy_dir(Path::new(&version.artifact_path), &root)?;
    deploy_env::write_runtime_env(db, version, stack, machine_id, &root)?;
    Ok(StagedDeployPackage { path: root })
}

fn redact(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            if lower.contains("password")
                || lower.contains("secret")
                || lower.contains("token")
                || lower.contains("api_key")
            {
                "***"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_command_uses_explicit_args() {
        let command = ssh_command(2222, Some("bruno"), "docker --version");
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(command.get_program().to_string_lossy(), "ssh");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"ConnectTimeout=8".to_string()));
        assert!(args.contains(&"ConnectionAttempts=1".to_string()));
        assert!(args.contains(&"StrictHostKeyChecking=no".to_string()));
        assert!(args.contains(&"UserKnownHostsFile=/dev/null".to_string()));
        assert!(args.contains(&"bruno@127.0.0.1".to_string()));
        assert!(args.contains(&"docker --version".to_string()));
    }

    #[test]
    fn rsync_command_uses_ssh_transport_arg() {
        let command = rsync_command(2222, Some("bruno"), "/tmp/package", "~/dw-deploy/app");
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(command.get_program().to_string_lossy(), "rsync");
        assert!(args.iter().any(|arg| arg.contains("ssh -p 2222")));
        assert!(args.iter().any(|arg| arg.contains("ConnectTimeout=8")));
        assert!(args.iter().any(|arg| arg.contains("ConnectionAttempts=1")));
        assert!(args
            .iter()
            .any(|arg| arg.contains("UserKnownHostsFile=/dev/null")));
        assert!(args.iter().any(|arg| arg == "/tmp/package/"));
        assert!(args
            .iter()
            .any(|arg| arg == "bruno@127.0.0.1:~/dw-deploy/app/"));
    }

    #[test]
    fn windows_shared_guest_path_uses_guest_unc_path() {
        assert_eq!(
            windows_shared_guest_path(&[
                "deploy-packages",
                "winbox-gui-deploy",
                "deploy-002",
                "scripts",
                "install-deploy.ps1"
            ]),
            "\\\\host.lan\\Data\\deploy-packages\\winbox-gui-deploy\\deploy-002\\scripts\\install-deploy.ps1"
        );
        assert_eq!(
            windows_shared_guest_path(&["ade", "bootstrap-windows.ps1"]),
            "\\\\host.lan\\Data\\ade\\bootstrap-windows.ps1"
        );
    }

    #[test]
    fn windows_ssh_preflight_error_points_to_bootstrap() {
        let message = deploy_ssh_unavailable_message(
            2223,
            true,
            "ssh failed with exit status 255: connect to host 127.0.0.1 port 2223: Connection refused",
        );

        assert!(message.starts_with("windows_ssh_bootstrap_required:"));
        assert!(message.contains("127.0.0.1:2223"));
        assert!(message.contains("\\\\host.lan\\Data\\ade\\bootstrap-windows.ps1"));
        assert!(message.contains("validate SSH and retry Deploy"));
    }

    #[test]
    fn windows_manual_runbook_message_points_to_shared_install_script() {
        let stack = store::DeployStack {
            id: "stack-1".to_string(),
            workspace_id: 3,
            name: "WinBox GUI".to_string(),
            slug: "winbox-gui-deploy".to_string(),
            status: "idle".to_string(),
            active_version_id: None,
            active_machine_id: None,
            created_at: "2026-06-02T00:00:00Z".to_string(),
            updated_at: "2026-06-02T00:00:00Z".to_string(),
        };
        let version = store::DeployVersion {
            id: "version-1".to_string(),
            stack_id: stack.id.clone(),
            workspace_id: stack.workspace_id,
            label: "deploy-002".to_string(),
            status: "approved".to_string(),
            target_machine_id: None,
            artifact_path: "/tmp/package".to_string(),
            manifest_path: "/tmp/package/manifest.json".to_string(),
            manifest_json: "{}".to_string(),
            review_status: "approved".to_string(),
            reviewed_at: None,
            blocking_findings_json: "[]".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            updated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let message = windows_runbook_manual_required_message(&stack, &version);

        assert!(message.starts_with("windows_runbook_manual_required:"));
        assert!(message.contains("scripts/deploy.ps1"));
        assert!(message.contains(
            "\\\\host.lan\\Data\\deploy-packages\\winbox-gui-deploy\\deploy-002\\scripts\\install-deploy.ps1"
        ));
    }

    #[test]
    fn windows_pre_deploy_scripts_include_install_when_packaged() {
        let root = std::env::temp_dir().join(format!(
            "dw-gui-windows-predeploy-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(root.join("scripts")).expect("create scripts");
        std::fs::write(
            root.join("scripts/install-deploy.ps1"),
            "Write-Host install",
        )
        .expect("write install script");
        let version = store::DeployVersion {
            id: "version-1".to_string(),
            stack_id: "stack-1".to_string(),
            workspace_id: 3,
            label: "deploy-002".to_string(),
            status: "approved".to_string(),
            target_machine_id: None,
            artifact_path: root.display().to_string(),
            manifest_path: root.join("manifest.json").display().to_string(),
            manifest_json: "{}".to_string(),
            review_status: "approved".to_string(),
            reviewed_at: None,
            blocking_findings_json: "[]".to_string(),
            created_at: "2026-06-02T00:00:00Z".to_string(),
            updated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        assert_eq!(
            windows_pre_deploy_scripts(&version),
            vec!["install-deploy.ps1"]
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn windows_powershell_file_command_uses_shared_script_without_unix_shell() {
        let command = windows_powershell_file_command(
            "\\\\host.lan\\Data\\deploy-packages\\winbox-gui-deploy\\deploy-002\\scripts\\deploy.ps1",
            &[("ComposeProjectName", "dw_winbox_gui_deploy_deploy_002")],
        );

        assert!(command.starts_with("powershell -NoProfile -ExecutionPolicy Bypass -File "));
        assert!(command.contains(
            "\"\\\\host.lan\\Data\\deploy-packages\\winbox-gui-deploy\\deploy-002\\scripts\\deploy.ps1\""
        ));
        assert!(command.contains("-ComposeProjectName \"dw_winbox_gui_deploy_deploy_002\""));
        assert!(!command.contains(" sh "));
        assert!(!command.contains("rsync"));
    }

    #[test]
    fn redacted_error_chain_preserves_runbook_root_cause_for_classification() {
        let error =
            anyhow::anyhow!("docker : O termo 'docker' nao e reconhecido como nome de cmdlet")
                .context("runbook-install failed");

        let chain = redact_error_chain(&error);

        assert!(chain.contains("runbook-install failed"));
        assert!(chain.contains("docker"));
        assert_eq!(
            crate::deploy_repair::classify_failure(&chain).code,
            "docker_missing"
        );
    }
}
