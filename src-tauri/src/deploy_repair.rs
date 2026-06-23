use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Component, Path};

pub const MAX_REPAIR_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Serialize)]
pub struct RepairDiagnosis {
    pub code: String,
    pub title: String,
    pub safe_recipe: Option<String>,
    pub next_step: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentRepairReport {
    pub schema_version: String,
    pub status: String,
    pub diagnosis: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub safe_to_apply: bool,
    #[serde(default)]
    pub patch_summary: String,
    #[serde(default)]
    pub patch_set: Vec<AgentRepairPatch>,
    #[serde(default)]
    pub rerun_steps: Vec<String>,
    #[serde(default)]
    pub user_message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentRepairPatch {
    pub path: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairValidation {
    pub ade_safe_to_apply: bool,
    pub validation_status: String,
    pub validation_errors: Vec<String>,
    pub patch_paths: Vec<String>,
}

pub fn classify_failure(message: &str) -> RepairDiagnosis {
    let lower = message.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "could not get lock",
            "unable to lock directory",
            "is held by process",
            "waiting for cache lock",
            "dpkg frontend lock",
            "/var/lib/apt/lists/lock",
        ],
    ) {
        return diagnosis(
            "apt_lock",
            "APT is busy",
            Some("wait_and_retry_package_manager"),
            "ADE will wait for the package manager lock and retry the same deploy phase.",
            "high",
        );
    }
    if lower.contains("not valid yet") && lower.contains("release file") {
        return diagnosis(
            "apt_clock_skew",
            "Guest clock is behind apt metadata",
            Some("wait_and_retry_apt_clock"),
            "ADE will wait and retry because the guest clock should catch up during boot.",
            "high",
        );
    }
    if contains_any(
        &lower,
        &[
            "connection refused",
            "connection reset",
            "kex_exchange_identification",
            "operation timed out",
            "connection timed out",
        ],
    ) && lower.contains("ssh")
    {
        return diagnosis(
            "ssh_transient",
            "SSH is not ready yet",
            Some("wait_and_retry_ssh"),
            "ADE will wait for the WinBox SSH port and retry the command.",
            "medium",
        );
    }
    if contains_any(
        &lower,
        &[
            "docker: command not found",
            "docker : o termo 'docker'",
            "docker is not recognized",
            "docker --version",
        ],
    ) {
        return diagnosis(
            "docker_missing",
            "Docker is missing on the target",
            None,
            "Use a target profile that installs Docker or let the agent propose a package runbook correction.",
            "high",
        );
    }
    if contains_any(
        &lower,
        &[
            "tauri: not found",
            "tauri not found",
            "node_modules/.bin/tauri",
            "test -x node_modules/.bin/tauri",
        ],
    ) {
        return diagnosis(
            "tauri_missing",
            "Tauri CLI dependency is missing",
            None,
            "The agent must correct the package scripts so the project installs the right dev dependencies before verification.",
            "high",
        );
    }
    if contains_any(
        &lower,
        &[
            "winget install failed",
            "no available upgrade found",
            "no newer package versions are available",
        ],
    ) {
        return diagnosis(
            "winget_install_issue",
            "Windows package install reported a non-fatal winget state",
            None,
            "The Windows runbook should treat already-installed packages with no upgrade as satisfied and continue.",
            "high",
        );
    }
    if lower.contains("remote mkdir failed") {
        return diagnosis(
            "remote_mkdir_failed",
            "Remote deploy directory could not be created",
            None,
            "Validate SSH credentials, target user permissions, and the selected VM before retrying.",
            "high",
        );
    }
    if contains_any(
        &lower,
        &["\\\\host.lan\\data", "unc", "shared folder", "shared_dir"],
    ) {
        return diagnosis(
            "shared_folder_issue",
            "Shared folder is unavailable or too slow",
            None,
            "The package should run from a local target copy or the WinBox shared folder must be fixed.",
            "medium",
        );
    }
    if contains_any(
        &lower,
        &[
            "nativecommanderror",
            "fullyqualifiederrorid",
            "categoryinfo",
            "parentcontainserrorrecordexception",
        ],
    ) {
        return diagnosis(
            "powershell_native_stderr",
            "PowerShell treated native command output as an error",
            None,
            "The agent should make the PowerShell runbook check native exit codes instead of failing on warnings.",
            "high",
        );
    }
    diagnosis(
        "unknown_failure",
        "Unknown deploy failure",
        None,
        "The selected agent must inspect redacted evidence and propose a package-local correction.",
        "low",
    )
}

pub fn safe_package_repair_path(path: &str) -> bool {
    let trimmed = path.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed.starts_with('/') || trimmed.contains('\0') {
        return false;
    }
    if trimmed == "RUNBOOK.md" {
        return true;
    }
    if !trimmed.starts_with("scripts/") {
        return false;
    }
    Path::new(&trimmed).components().all(|component| {
        matches!(component, Component::Normal(_))
            || matches!(component, Component::CurDir)
            || matches!(component, Component::RootDir)
    }) && !Path::new(&trimmed)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
}

pub fn parse_agent_repair_report(output: &str) -> anyhow::Result<AgentRepairReport> {
    let candidate = if let Ok(value) = serde_json::from_str::<Value>(output.trim()) {
        value
    } else {
        let slice = json_candidate(output)
            .ok_or_else(|| anyhow::anyhow!("deploy_repair_invalid_json: no JSON object found"))?;
        serde_json::from_str::<Value>(slice)
            .context("deploy_repair_invalid_json: assistant output is not valid JSON")?
    };
    let report: AgentRepairReport = serde_json::from_value(candidate)
        .context("deploy_repair_invalid_json: repair report has invalid shape")?;
    validate_agent_repair_report(&report)?;
    Ok(report)
}

pub fn validate_agent_repair_report(report: &AgentRepairReport) -> anyhow::Result<()> {
    if report.schema_version != "1.0" {
        anyhow::bail!("deploy_repair_invalid_json: unsupported schema_version");
    }
    if !matches!(
        report.status.as_str(),
        "blocked" | "patch_proposed" | "no_patch"
    ) {
        anyhow::bail!("deploy_repair_invalid_json: invalid status");
    }
    if report.safe_to_apply && report.patch_set.is_empty() {
        anyhow::bail!("deploy_repair_invalid_json: safe_to_apply requires patch_set");
    }
    for patch in &report.patch_set {
        if !safe_package_repair_path(&patch.path) {
            anyhow::bail!(
                "deploy_repair_invalid_json: patch path must stay under scripts/ or RUNBOOK.md"
            );
        }
        if patch.body.trim().is_empty() {
            anyhow::bail!("deploy_repair_invalid_json: patch body is empty");
        }
    }
    Ok(())
}

pub fn validate_agent_repair_for_ade(report: &AgentRepairReport) -> RepairValidation {
    let mut errors = Vec::new();
    let patch_paths = report
        .patch_set
        .iter()
        .map(|patch| patch.path.trim().replace('\\', "/"))
        .collect::<Vec<_>>();
    if report.status != "patch_proposed" {
        errors.push("repair status is not patch_proposed".to_string());
    }
    if report.patch_set.is_empty() {
        errors.push("repair patch_set is empty".to_string());
    }
    for patch in &report.patch_set {
        if !safe_package_repair_path(&patch.path) {
            errors.push(format!(
                "patch path escapes allowed package scope: {}",
                patch.path
            ));
        }
        if patch.body.trim().is_empty() {
            errors.push(format!("patch body is empty: {}", patch.path));
        }
        if contains_secret_marker(&patch.body) {
            errors.push(format!("patch contains secret-like marker: {}", patch.path));
        }
    }
    let ade_safe_to_apply = errors.is_empty();
    RepairValidation {
        ade_safe_to_apply,
        validation_status: if ade_safe_to_apply {
            "passed".to_string()
        } else {
            "blocked".to_string()
        },
        validation_errors: errors,
        patch_paths,
    }
}

pub fn repair_prompt(context: &Value) -> anyhow::Result<String> {
    let context_json = serde_json::to_string_pretty(context)?;
    Ok(format!(
        r##"You are the selected ADE deploy repair agent.

Return ONLY one strict JSON object. Do not use Markdown fences.
You may diagnose the failed deploy and propose package-local corrections only.
Never edit source repositories. Never include secret values.
Allowed patch paths: scripts/* and RUNBOOK.md.
Use manifest.projects[].package_path as the source of truth for packaged project locations.
safe_to_apply is your recommendation; ADE will still validate paths, secrets, and package scope deterministically.

Required JSON shape:
{{
  "schema_version": "1.0",
  "status": "patch_proposed | no_patch | blocked",
  "diagnosis": "specific failure cause",
  "confidence": "high | medium | low",
  "safe_to_apply": false,
  "patch_summary": "what changes in the deploy package",
  "patch_set": [
    {{"path": "scripts/deploy.sh", "body": "#!/usr/bin/env sh\nset -eu\n..."}}
  ],
  "rerun_steps": ["what ADE or the user should run next"],
  "user_message": "short guided message in Portuguese"
}}

Context:
{context_json}
"##
    ))
}

pub fn report_to_value(report: &AgentRepairReport) -> Value {
    json!(report)
}

fn diagnosis(
    code: &str,
    title: &str,
    safe_recipe: Option<&str>,
    next_step: &str,
    confidence: &str,
) -> RepairDiagnosis {
    RepairDiagnosis {
        code: code.to_string(),
        title: title.to_string(),
        safe_recipe: safe_recipe.map(ToOwned::to_owned),
        next_step: next_step.to_string(),
        confidence: confidence.to_string(),
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn contains_secret_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["api_key=", "apikey=", "secret=", "password=", "bearer "]
        .iter()
        .any(|marker| lower.contains(marker))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifier_maps_real_deploy_failures() {
        assert_eq!(
            classify_failure(
                "E: Could not get lock /var/lib/apt/lists/lock. It is held by process 1163"
            )
            .code,
            "apt_lock"
        );
        assert_eq!(
            classify_failure(
                "E: Release file for http://archive.ubuntu.com/ubuntu/dists/noble-updates/InRelease is not valid yet"
            )
            .code,
            "apt_clock_skew"
        );
        assert_eq!(
            classify_failure("docker : O termo 'docker' nao e reconhecido como nome de cmdlet")
                .code,
            "docker_missing"
        );
        assert_eq!(
            classify_failure("test -x node_modules/.bin/tauri failed").code,
            "tauri_missing"
        );
        assert_eq!(
            classify_failure(
                "winget install failed for Microsoft.EdgeWebView2Runtime with exit code -1978335189. Found an existing package already installed. No available upgrade found."
            )
            .code,
            "winget_install_issue"
        );
        assert_eq!(
            classify_failure("remote mkdir failed: ssh failed with exit status 255").code,
            "remote_mkdir_failed"
        );
    }

    #[test]
    fn validates_agent_repair_patch_paths() {
        assert!(safe_package_repair_path("scripts/deploy.sh"));
        assert!(safe_package_repair_path("RUNBOOK.md"));
        assert!(!safe_package_repair_path("../src/main.rs"));
        assert!(!safe_package_repair_path(
            "projects/app/source/package.json"
        ));
        assert!(!safe_package_repair_path("/tmp/script.sh"));
    }

    #[test]
    fn parses_strict_agent_repair_report() {
        let report = parse_agent_repair_report(
            r##"{
              "schema_version": "1.0",
              "status": "patch_proposed",
              "diagnosis": "npm script misses tauri dependency install",
              "confidence": "high",
              "safe_to_apply": true,
              "patch_summary": "install npm dependencies before tauri check",
              "patch_set": [
                {"path": "scripts/build-dev.sh", "body": "#!/usr/bin/env sh\nset -eu\nnpm install\n"}
              ],
              "rerun_steps": ["create repaired package version"],
              "user_message": "Crie a versao corrigida e rode Prepare novamente."
            }"##,
        )
        .expect("valid repair report");

        assert_eq!(report.status, "patch_proposed");
        assert_eq!(report.patch_set[0].path, "scripts/build-dev.sh");
    }

    #[test]
    fn ade_validation_accepts_package_patch_even_when_agent_marks_not_safe() {
        let report = AgentRepairReport {
            schema_version: "1.0".to_string(),
            status: "patch_proposed".to_string(),
            diagnosis: "script points at wrong package path".to_string(),
            confidence: "high".to_string(),
            safe_to_apply: false,
            patch_summary: "fix package script".to_string(),
            patch_set: vec![AgentRepairPatch {
                path: "scripts/deploy.sh".to_string(),
                body: "#!/usr/bin/env sh\nset -eu\necho ok\n".to_string(),
            }],
            rerun_steps: vec![],
            user_message: "Crie a versao corrigida.".to_string(),
        };

        let validation = validate_agent_repair_for_ade(&report);

        assert!(validation.ade_safe_to_apply);
        assert_eq!(validation.validation_status, "passed");
        assert_eq!(validation.patch_paths, vec!["scripts/deploy.sh"]);
    }

    #[test]
    fn ade_validation_blocks_unsafe_repair_patch() {
        let report = AgentRepairReport {
            schema_version: "1.0".to_string(),
            status: "patch_proposed".to_string(),
            diagnosis: "unsafe".to_string(),
            confidence: "high".to_string(),
            safe_to_apply: true,
            patch_summary: "bad".to_string(),
            patch_set: vec![
                AgentRepairPatch {
                    path: "../src/main.rs".to_string(),
                    body: "fn main() {}".to_string(),
                },
                AgentRepairPatch {
                    path: "RUNBOOK.md".to_string(),
                    body: "password=real-value".to_string(),
                },
            ],
            rerun_steps: vec![],
            user_message: "bad".to_string(),
        };

        let validation = validate_agent_repair_for_ade(&report);

        assert!(!validation.ade_safe_to_apply);
        assert!(validation
            .validation_errors
            .iter()
            .any(|error| error.contains("escapes allowed package scope")));
        assert!(validation
            .validation_errors
            .iter()
            .any(|error| error.contains("secret-like marker")));
    }
}
