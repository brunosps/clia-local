use crate::deploy_detect::{self, DeployProjectDetection};
use crate::deploy_plan;
use crate::deploy_repair;
use crate::deploy_scan;
use crate::store;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const DEPLOY_RUNBOOK_VERSION: &str = "2026-06-02.1";
const ENV_TEMPLATE_DEFAULT_MARKER: &str = "#dw:default";

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDeployPackageInput {
    pub workspace_id: i64,
    pub stack_name: String,
    pub project_ids: Vec<i64>,
    pub target_machine_id: Option<String>,
    pub agent_profile_id: i64,
    pub deploy_plan_path: Option<String>,
    pub include_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    pub path: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default = "default_finding_severity")]
    pub severity: String,
    #[serde(default = "default_finding_blocking")]
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DismissedReviewFinding {
    pub path: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
    pub justification: String,
    pub dismissed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_from_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_from_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReviewFindingIdentity {
    path: String,
    reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReviewAuditEvent {
    action: String,
    identity: ReviewFindingIdentity,
    justification: String,
    timestamp: String,
}

#[derive(Debug, Clone)]
struct PackagedProject {
    project: store::Project,
    detection: DeployProjectDetection,
    branch: Option<String>,
    commit_sha: Option<String>,
    dirty: bool,
    git_status_short: String,
    package_path: String,
    dockerfile_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageComposeDecision {
    mode: PackageComposeMode,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PackageComposeMode {
    Generated { path: String },
    AgentArtifact { path: String },
    SourcePassthrough { path: String },
}

impl PackageComposeMode {
    fn file_path(&self) -> &str {
        match self {
            Self::Generated { path }
            | Self::AgentArtifact { path }
            | Self::SourcePassthrough { path } => path,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Generated { .. } => "generated",
            Self::AgentArtifact { .. } => "agent_artifact",
            Self::SourcePassthrough { .. } => "source_passthrough",
        }
    }

    fn uses_source_passthrough(&self) -> bool {
        matches!(self, Self::SourcePassthrough { .. })
    }

    fn writes_generated_compose(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }
}

pub fn create_package(
    db: &store::Database,
    input: CreateDeployPackageInput,
) -> anyhow::Result<store::DeployVersion> {
    if input.stack_name.trim().is_empty() {
        anyhow::bail!("deploy stack name is required");
    }
    let workspace = db.get_workspace(input.workspace_id)?;
    let stack_slug = slugify(&input.stack_name);
    let deploy_plan_path = input
        .deploy_plan_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("deploy_plan_required: run agent planning before package creation")
        })?;
    let mut plan_bundle = deploy_plan::load_plan_bundle_from_path(
        db,
        deploy_plan::PlanDeployPackageInput {
            workspace_id: input.workspace_id,
            project_ids: input.project_ids.clone(),
            target_machine_id: input.target_machine_id.clone(),
            agent_profile_id: input.agent_profile_id,
            include_dirty: input.include_dirty,
        },
        Path::new(deploy_plan_path),
    )?;
    if deploy_plan::validation_blocks_package(&plan_bundle.validation) {
        anyhow::bail!(
            "deploy_plan_validation_failed: {}",
            deploy_plan::validation_error_summary(&plan_bundle.validation)
        );
    }
    let detection = plan_bundle.detection.clone();
    for project_detection in &detection.projects {
        let status = git_status_short(Path::new(&project_detection.path));
        if !input.include_dirty && !status.trim().is_empty() {
            anyhow::bail!(
                "project '{}' has dirty changes; enable dirty snapshot inclusion to package it",
                project_detection.name
            );
        }
    }
    let stack = db.create_deploy_stack(store::DeployStackCreate {
        workspace_id: input.workspace_id,
        name: input.stack_name.trim(),
        slug: &stack_slug,
    })?;
    let label = db.next_deploy_version_label(&stack.id)?;
    let artifact_root = Path::new(&workspace.root_path)
        .join(".dw")
        .join("deploy-packages")
        .join(input.workspace_id.to_string())
        .join(&stack_slug)
        .join(&label);
    let manifest_path = artifact_root.join("manifest.json");
    std::fs::create_dir_all(&artifact_root)
        .with_context(|| format!("failed to create {}", artifact_root.display()))?;
    let version = db.create_deploy_version(store::DeployVersionCreate {
        stack_id: &stack.id,
        workspace_id: input.workspace_id,
        label: &label,
        target_machine_id: input.target_machine_id.as_deref(),
        artifact_path: &artifact_root.display().to_string(),
        manifest_path: &manifest_path.display().to_string(),
        manifest_json: "{}",
        blocking_findings_json: "[]",
        dismissed_findings_json: "[]",
    })?;
    let mut findings = Vec::<SecretFinding>::new();
    let mut packaged_projects = Vec::<PackagedProject>::new();
    for project_detection in &detection.projects {
        let project = db.get_project(project_detection.project_id)?;
        let project_slug = slugify(&project.name);
        let relative_source = format!("projects/{project_slug}/source");
        let relative_dockerfile = format!("projects/{project_slug}/Dockerfile");
        let source_dir = artifact_root.join(&relative_source);
        std::fs::create_dir_all(&source_dir)
            .with_context(|| format!("failed to create {}", source_dir.display()))?;
        copy_source_snapshot_for_package(
            Path::new(&project.path),
            &source_dir,
            &artifact_root,
            &mut findings,
        )?;
        write_generated_dockerfile(&artifact_root.join(&relative_dockerfile), project_detection)?;
        write_generated_dockerignore(
            &artifact_root.join(format!("projects/{project_slug}/.dockerignore")),
        )?;
        let git_status = git_status_short(Path::new(&project.path));
        let packaged = PackagedProject {
            project: project.clone(),
            detection: project_detection.clone(),
            branch: git_output(Path::new(&project.path), &["branch", "--show-current"]),
            commit_sha: git_output(Path::new(&project.path), &["rev-parse", "HEAD"]),
            dirty: !git_status.trim().is_empty(),
            git_status_short: git_status,
            package_path: relative_source,
            dockerfile_path: relative_dockerfile,
        };
        db.add_deploy_version_project(store::DeployVersionProjectCreate {
            version_id: &version.id,
            project_id: project.id,
            name: &project.name,
            path: &project.path,
            branch: packaged.branch.as_deref(),
            commit_sha: packaged.commit_sha.as_deref(),
            dirty: packaged.dirty,
            package_path: &packaged.package_path,
        })?;
        if project_detection.deploy_strategy == "unsupported" {
            findings.push(SecretFinding::blocking(
                packaged.package_path.clone(),
                format!(
                    "unsupported deploy strategy: {}",
                    project_detection.strategy_reason
                ),
            ));
        }
        packaged_projects.push(packaged);
    }
    let package_strategy = package_deploy_strategy(&packaged_projects);
    let compose_decision =
        package_compose_mode(&package_strategy, &packaged_projects, &plan_bundle.plan);
    for warning in &compose_decision.warnings {
        append_validation_warning(&mut plan_bundle.validation, warning);
    }
    deploy_plan::write_analysis_artifacts(&artifact_root, &plan_bundle)?;
    let compose_mode = &compose_decision.mode;
    if compose_mode.writes_generated_compose() {
        write_compose(
            &artifact_root.join(compose_mode.file_path()),
            &packaged_projects,
        )?;
    }
    let project_env_defaults = collect_project_env_defaults(&packaged_projects);
    write_env_example(
        &artifact_root.join(".env.example"),
        &detection.services,
        &plan_bundle.plan,
        plan_has_compose_artifact(&plan_bundle.plan) || compose_mode.uses_source_passthrough(),
        &project_env_defaults,
    )?;
    std::fs::write(artifact_root.join(".dw-deploy-strategy"), &package_strategy)?;
    write_scripts(
        &artifact_root.join("scripts"),
        &package_strategy,
        &packaged_projects,
        compose_mode,
    )?;
    write_agent_plan_files(&artifact_root, &plan_bundle.plan)?;
    write_agent_plan_scripts(
        &artifact_root,
        &plan_bundle.plan,
        &package_strategy,
        compose_mode,
    )?;
    scan_package_review_files(&artifact_root, &mut findings);
    annotate_finding_occurrences(&mut findings);
    write_package_runbook(
        &artifact_root,
        &stack,
        &version,
        &input,
        &packaged_projects,
        &plan_bundle,
        &package_strategy,
    )?;
    let dismissed_findings = inherited_dismissed_findings(db, &version, &findings)?;
    let manifest = build_manifest(
        &workspace,
        &stack,
        &version,
        &input,
        &packaged_projects,
        &detection,
        &plan_bundle,
        &findings,
        &dismissed_findings,
        compose_mode,
        &compose_decision.warnings,
    )?;
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, &manifest_json)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    let findings_json = serde_json::to_string(&findings)?;
    let dismissed_findings_json = serde_json::to_string(&dismissed_findings)?;
    db.update_deploy_version_manifest(
        &version.id,
        &manifest_path.display().to_string(),
        &manifest_json,
        &findings_json,
        &dismissed_findings_json,
    )
}

pub fn read_artifact(
    version: &store::DeployVersion,
    relative_path: &str,
) -> anyhow::Result<String> {
    if relative_path == ".env" || relative_path.ends_with("/.env") {
        anyhow::bail!("runtime environment files cannot be previewed");
    }
    let root = PathBuf::from(&version.artifact_path);
    let target = scoped_existing_child(&root, relative_path)?;
    if target.metadata()?.len() > 1024 * 1024 {
        anyhow::bail!("deploy artifact is too large to preview");
    }
    std::fs::read_to_string(&target).with_context(|| format!("failed to read {}", target.display()))
}

pub fn create_repair_version_from_run(
    db: &store::Database,
    run_id: &str,
) -> anyhow::Result<store::DeployVersion> {
    let run = db.get_deploy_run(run_id)?;
    if run.orchestration_status != "repair_pending" {
        anyhow::bail!("deploy_repair_not_pending: selected run has no approved repair proposal");
    }
    let version_id = run
        .version_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("deploy_repair_not_pending: run has no version"))?;
    let source_version = db.get_deploy_version(version_id)?;
    let stack = db.get_deploy_stack(&source_version.stack_id)?;
    let workspace = db.get_workspace(source_version.workspace_id)?;
    let report_json: serde_json::Value = serde_json::from_str(&run.orchestration_report_json)
        .context("deploy_repair_not_pending: invalid orchestration report")?;
    let repair_value = report_json
        .get("repair")
        .and_then(|repair| repair.get("agent_repair"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("deploy_repair_not_pending: missing agent repair"))?;
    let repair: deploy_repair::AgentRepairReport = serde_json::from_value(repair_value)
        .context("deploy_repair_not_pending: invalid agent repair report")?;
    deploy_repair::validate_agent_repair_report(&repair)?;
    let validation = deploy_repair::validate_agent_repair_for_ade(&repair);
    if !validation.ade_safe_to_apply || repair.patch_set.is_empty() {
        anyhow::bail!(
            "deploy_repair_not_pending: agent repair patch did not pass ADE validation: {}",
            validation.validation_errors.join("; ")
        );
    }

    let label = db.next_deploy_version_label(&stack.id)?;
    let artifact_root = Path::new(&workspace.root_path)
        .join(".dw")
        .join("deploy-packages")
        .join(source_version.workspace_id.to_string())
        .join(&stack.slug)
        .join(&label);
    copy_package_dir(Path::new(&source_version.artifact_path), &artifact_root)?;
    for patch in &repair.patch_set {
        if !deploy_repair::safe_package_repair_path(&patch.path) {
            anyhow::bail!("deploy_repair_not_pending: unsafe patch path");
        }
        let path = artifact_root.join(&patch.path);
        if !path.starts_with(&artifact_root) {
            anyhow::bail!("deploy_repair_not_pending: patch path escapes package");
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &patch.body)
            .with_context(|| format!("failed to write repair patch {}", path.display()))?;
    }

    let manifest_path = artifact_root.join("manifest.json");
    let version = db.create_deploy_version(store::DeployVersionCreate {
        stack_id: &stack.id,
        workspace_id: source_version.workspace_id,
        label: &label,
        target_machine_id: source_version.target_machine_id.as_deref(),
        artifact_path: &artifact_root.display().to_string(),
        manifest_path: &manifest_path.display().to_string(),
        manifest_json: "{}",
        blocking_findings_json: &source_version.blocking_findings_json,
        dismissed_findings_json: &source_version.dismissed_findings_json,
    })?;
    for project in db.list_deploy_version_projects(&source_version.id)? {
        db.add_deploy_version_project(store::DeployVersionProjectCreate {
            version_id: &version.id,
            project_id: project.project_id,
            name: &project.name,
            path: &project.path,
            branch: project.branch.as_deref(),
            commit_sha: project.commit_sha.as_deref(),
            dirty: project.dirty,
            package_path: &project.package_path,
        })?;
    }

    let mut manifest = serde_json::from_str::<serde_json::Value>(&source_version.manifest_json)
        .unwrap_or_else(|_| json!({}));
    if let Some(object) = manifest.as_object_mut() {
        object.insert("version_id".to_string(), json!(version.id));
        object.insert("version_label".to_string(), json!(version.label));
        object.insert("approved".to_string(), json!(false));
        object.insert(
            "repair".to_string(),
            json!({
                "source_version_id": source_version.id,
                "source_version_label": source_version.label,
                "source_run_id": run.id,
                "agent": {
                    "profile_id": run.agent_profile_id,
                    "name": run.agent_name,
                    "provider": run.agent_provider,
                    "model": run.agent_model,
                },
                "patch_summary": repair.patch_summary,
                "patches": repair.patch_set.iter().map(|patch| patch.path.clone()).collect::<Vec<_>>(),
                "user_message": repair.user_message,
            }),
        );
        object.insert(
            "review".to_string(),
            json!({
                "status": "pending",
                "blocking_findings": serde_json::from_str::<serde_json::Value>(&source_version.blocking_findings_json).unwrap_or_else(|_| json!([])),
                "dismissed_findings": serde_json::from_str::<serde_json::Value>(&source_version.dismissed_findings_json).unwrap_or_else(|_| json!([])),
            }),
        );
    }
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, &manifest_json)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    db.update_deploy_version_manifest(
        &version.id,
        &manifest_path.display().to_string(),
        &manifest_json,
        &source_version.blocking_findings_json,
        &source_version.dismissed_findings_json,
    )
}

pub fn has_blocking_findings(version: &store::DeployVersion) -> bool {
    active_blocking_findings(version)
        .map(|findings| !findings.is_empty())
        .unwrap_or(true)
}

pub fn active_blocking_findings(
    version: &store::DeployVersion,
) -> anyhow::Result<Vec<SecretFinding>> {
    let findings = parse_review_findings(&version.blocking_findings_json)?;
    let dismissed = parse_dismissed_findings_lossy(&version.dismissed_findings_json);
    Ok(findings
        .into_iter()
        .filter(|finding| finding.blocking && !finding_is_dismissed(finding, &dismissed))
        .collect())
}

pub fn dismiss_review_finding(
    db: &store::Database,
    version_id: &str,
    path: &str,
    reason: &str,
    marker: Option<&str>,
    line_sha256: Option<&str>,
    justification: &str,
) -> anyhow::Result<store::DeployVersion> {
    let justification = justification.trim();
    if justification.chars().count() < 10 {
        anyhow::bail!("review finding dismissal requires a justification with at least 10 chars");
    }
    db.update_deploy_version_review_json(version_id, |version| {
        ensure_review_dismissible(version)?;
        let findings = parse_review_findings(&version.blocking_findings_json)?;
        let finding = select_blocking_finding(&findings, path, reason, marker, line_sha256)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut dismissed = parse_dismissed_findings_lossy(&version.dismissed_findings_json);
        dismissed.retain(|item| !dismissed_matches_finding(item, &finding));
        dismissed.push(DismissedReviewFinding {
            path: finding.path.clone(),
            reason: finding.reason.clone(),
            marker: finding.marker.clone(),
            line_sha256: finding.line_sha256.clone(),
            line_number: finding.line_number,
            justification: justification.to_string(),
            dismissed_at: timestamp.clone(),
            inherited_from_version_id: None,
            inherited_from_label: None,
        });
        let mut audit = parse_review_audit_events_lossy(&version.review_audit_json);
        audit.push(ReviewAuditEvent {
            action: "dismiss".to_string(),
            identity: review_identity_for_finding(&finding),
            justification: justification.to_string(),
            timestamp,
        });
        Ok((
            serde_json::to_string(&dismissed)?,
            serde_json::to_string(&audit)?,
        ))
    })
}

pub fn restore_review_finding(
    db: &store::Database,
    version_id: &str,
    path: &str,
    reason: &str,
    marker: Option<&str>,
    line_sha256: Option<&str>,
) -> anyhow::Result<store::DeployVersion> {
    db.update_deploy_version_review_json(version_id, |version| {
        ensure_review_dismissible(version)?;
        let findings = parse_review_findings(&version.blocking_findings_json)?;
        let finding = select_blocking_finding(&findings, path, reason, marker, line_sha256)?;
        let mut dismissed = parse_dismissed_findings_lossy(&version.dismissed_findings_json);
        let original_len = dismissed.len();
        dismissed.retain(|item| !dismissed_matches_finding(item, &finding));
        if dismissed.len() == original_len {
            anyhow::bail!("review finding is not dismissed for this deploy version");
        }
        let mut audit = parse_review_audit_events_lossy(&version.review_audit_json);
        audit.push(ReviewAuditEvent {
            action: "restore".to_string(),
            identity: review_identity_for_finding(&finding),
            justification: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok((
            serde_json::to_string(&dismissed)?,
            serde_json::to_string(&audit)?,
        ))
    })
}

fn inherited_dismissed_findings(
    db: &store::Database,
    version: &store::DeployVersion,
    findings: &[SecretFinding],
) -> anyhow::Result<Vec<DismissedReviewFinding>> {
    let previous_versions = db.list_deploy_versions(&version.stack_id)?;
    let mut inherited = Vec::new();
    let mut inherited_identities = Vec::<ReviewFindingIdentity>::new();
    for finding in findings.iter().filter(|finding| finding.blocking) {
        let finding_identity = review_identity_for_finding(finding);
        if inherited_identities
            .iter()
            .any(|identity| review_identities_match(identity, &finding_identity))
        {
            continue;
        }
        if latest_review_audit_action_across_stack(&previous_versions, version, finding).as_deref()
            != Some("dismiss")
        {
            continue;
        }
        let Some(match_) =
            latest_dismissed_finding_across_stack(&previous_versions, version, finding)
        else {
            continue;
        };
        let origin_version_id = match_
            .item
            .inherited_from_version_id
            .clone()
            .unwrap_or_else(|| match_.version_id.clone());
        let origin_label = match_
            .item
            .inherited_from_label
            .clone()
            .unwrap_or_else(|| match_.version_label.clone());
        inherited.push(DismissedReviewFinding {
            path: finding.path.clone(),
            reason: finding.reason.clone(),
            marker: finding.marker.clone(),
            line_sha256: finding.line_sha256.clone(),
            line_number: finding.line_number,
            justification: match_.item.justification.clone(),
            dismissed_at: match_.item.dismissed_at.clone(),
            inherited_from_version_id: Some(origin_version_id),
            inherited_from_label: Some(origin_label),
        });
        inherited_identities.push(finding_identity);
    }
    Ok(inherited)
}

#[derive(Debug, Clone)]
struct MatchedDismissedReviewFinding {
    item: DismissedReviewFinding,
    version_id: String,
    version_label: String,
    sequence: usize,
}

fn latest_dismissed_finding_across_stack(
    versions: &[store::DeployVersion],
    current: &store::DeployVersion,
    finding: &SecretFinding,
) -> Option<MatchedDismissedReviewFinding> {
    let mut latest = None::<MatchedDismissedReviewFinding>;
    let mut sequence = 0;
    for previous in versions
        .iter()
        .rev()
        .filter(|previous| previous.id != current.id)
    {
        for item in parse_dismissed_findings_lossy(&previous.dismissed_findings_json)
            .into_iter()
            .filter(|item| dismissed_matches_finding(item, finding))
        {
            sequence += 1;
            let candidate = MatchedDismissedReviewFinding {
                item,
                version_id: previous.id.clone(),
                version_label: previous.label.clone(),
                sequence,
            };
            if latest.as_ref().is_none_or(|current| {
                timestamp_is_after(
                    &candidate.item.dismissed_at,
                    candidate.sequence,
                    &current.item.dismissed_at,
                    current.sequence,
                )
            }) {
                latest = Some(candidate);
            }
        }
    }
    latest
}

#[derive(Debug, Clone)]
struct MatchedReviewAuditEvent {
    action: String,
    timestamp: String,
    sequence: usize,
}

fn latest_review_audit_action_across_stack(
    versions: &[store::DeployVersion],
    current: &store::DeployVersion,
    finding: &SecretFinding,
) -> Option<String> {
    let finding_identity = review_identity_for_finding(finding);
    let mut latest = None::<MatchedReviewAuditEvent>;
    let mut sequence = 0;
    for previous in versions
        .iter()
        .rev()
        .filter(|previous| previous.id != current.id)
    {
        for event in parse_review_audit_events_lossy(&previous.review_audit_json) {
            sequence += 1;
            if !matches!(event.action.as_str(), "dismiss" | "restore")
                || !review_identities_match(&event.identity, &finding_identity)
            {
                continue;
            }
            let candidate = MatchedReviewAuditEvent {
                action: event.action,
                timestamp: event.timestamp,
                sequence,
            };
            if latest.as_ref().is_none_or(|current| {
                timestamp_is_after(
                    &candidate.timestamp,
                    candidate.sequence,
                    &current.timestamp,
                    current.sequence,
                )
            }) {
                latest = Some(candidate);
            }
        }
    }
    latest.map(|event| event.action)
}

fn timestamp_is_after(
    left: &str,
    left_sequence: usize,
    right: &str,
    right_sequence: usize,
) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(left),
        chrono::DateTime::parse_from_rfc3339(right),
    ) {
        (Ok(left), Ok(right)) => left > right || (left == right && left_sequence > right_sequence),
        _ => left > right || (left == right && left_sequence > right_sequence),
    }
}

fn parse_review_findings(payload: &str) -> anyhow::Result<Vec<SecretFinding>> {
    serde_json::from_str::<Vec<SecretFinding>>(payload)
        .with_context(|| "invalid deploy review findings payload")
}

fn parse_dismissed_findings_lossy(payload: &str) -> Vec<DismissedReviewFinding> {
    serde_json::from_str::<Vec<DismissedReviewFinding>>(payload).unwrap_or_default()
}

fn parse_review_audit_events_lossy(payload: &str) -> Vec<ReviewAuditEvent> {
    serde_json::from_str::<Vec<ReviewAuditEvent>>(payload).unwrap_or_default()
}

fn finding_is_dismissed(finding: &SecretFinding, dismissed: &[DismissedReviewFinding]) -> bool {
    dismissed
        .iter()
        .any(|item| dismissed_matches_finding(item, finding))
}

fn ensure_review_dismissible(version: &store::DeployVersion) -> anyhow::Result<()> {
    let status = version.status.as_str();
    if matches!(status, "review_required" | "pending") && version.review_status == "pending" {
        return Ok(());
    }
    anyhow::bail!("deploy_review_not_pending: only pending review versions can dismiss findings")
}

fn select_blocking_finding(
    findings: &[SecretFinding],
    path: &str,
    reason: &str,
    marker: Option<&str>,
    line_sha256: Option<&str>,
) -> anyhow::Result<SecretFinding> {
    let matches = findings
        .iter()
        .filter(|finding| {
            finding.blocking && finding_matches_input(finding, path, reason, marker, line_sha256)
        })
        .collect::<Vec<_>>();
    let Some(first) = matches.first() else {
        anyhow::bail!("blocking review finding not found for this deploy version");
    };
    let first_identity = review_identity_for_finding(first);
    if matches.iter().any(|finding| {
        !review_identities_match(&first_identity, &review_identity_for_finding(finding))
    }) {
        anyhow::bail!(
            "review finding identity is ambiguous; refresh the package and retry the action"
        );
    }
    Ok((*first).clone())
}

fn finding_matches_input(
    finding: &SecretFinding,
    path: &str,
    reason: &str,
    marker: Option<&str>,
    line_sha256: Option<&str>,
) -> bool {
    review_identity_matches_input(
        &review_identity_for_finding(finding),
        path,
        reason,
        marker,
        line_sha256,
    )
}

fn dismissed_matches_finding(item: &DismissedReviewFinding, finding: &SecretFinding) -> bool {
    review_identities_match(
        &review_identity_for_dismissal(item),
        &review_identity_for_finding(finding),
    )
}

fn review_identity_for_finding(finding: &SecretFinding) -> ReviewFindingIdentity {
    ReviewFindingIdentity {
        path: finding.path.trim().to_string(),
        reason: finding.reason.trim().to_string(),
        marker: finding
            .marker
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        line_sha256: finding
            .line_sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    }
}

fn review_identity_for_dismissal(item: &DismissedReviewFinding) -> ReviewFindingIdentity {
    ReviewFindingIdentity {
        path: item.path.trim().to_string(),
        reason: item.reason.trim().to_string(),
        marker: item
            .marker
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        line_sha256: item
            .line_sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    }
}

fn review_identity_matches_input(
    identity: &ReviewFindingIdentity,
    path: &str,
    reason: &str,
    marker: Option<&str>,
    line_sha256: Option<&str>,
) -> bool {
    let path = path.trim();
    let reason = reason.trim();
    let marker = marker.map(str::trim).filter(|value| !value.is_empty());
    let line_sha256 = line_sha256.map(str::trim).filter(|value| !value.is_empty());
    match (
        marker,
        line_sha256,
        identity.marker.as_deref(),
        identity.line_sha256.as_deref(),
    ) {
        (Some(marker), Some(line_sha256), Some(identity_marker), Some(identity_line_sha256)) => {
            identity.path == path
                && identity_marker == marker
                && identity_line_sha256 == line_sha256
        }
        _ if !review_identity_is_content(identity) => {
            identity.path == path && identity.reason == reason
        }
        _ => false,
    }
}

fn review_identities_match(left: &ReviewFindingIdentity, right: &ReviewFindingIdentity) -> bool {
    match (
        left.marker.as_deref(),
        left.line_sha256.as_deref(),
        right.marker.as_deref(),
        right.line_sha256.as_deref(),
    ) {
        (Some(left_marker), Some(left_line), Some(right_marker), Some(right_line)) => {
            left.path == right.path && left_marker == right_marker && left_line == right_line
        }
        _ if !review_identity_is_content(left) && !review_identity_is_content(right) => {
            left.path == right.path && left.reason == right.reason
        }
        _ => false,
    }
}

fn review_identity_is_content(identity: &ReviewFindingIdentity) -> bool {
    identity.marker.is_some() || identity.reason.contains("secret-like content marker")
}

fn append_validation_warning(validation: &mut serde_json::Value, warning: &str) {
    let Some(object) = validation.as_object_mut() else {
        return;
    };
    let warnings = object
        .entry("warnings")
        .or_insert_with(|| json!([]))
        .as_array_mut();
    if let Some(warnings) = warnings {
        if !warnings.iter().any(|item| item.as_str() == Some(warning)) {
            warnings.push(json!(warning));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_manifest(
    workspace: &store::Workspace,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    input: &CreateDeployPackageInput,
    projects: &[PackagedProject],
    detection: &deploy_detect::DeployDetectionReport,
    plan_bundle: &deploy_plan::DeployPlanBundle,
    findings: &[SecretFinding],
    dismissed_findings: &[DismissedReviewFinding],
    compose_mode: &PackageComposeMode,
    package_warnings: &[String],
) -> anyhow::Result<serde_json::Value> {
    Ok(json!({
        "schema_version": "1.0",
        "workspace_id": workspace.id,
        "workspace_root": workspace.root_path,
        "stack_id": stack.id,
        "stack_name": stack.name,
        "stack_slug": stack.slug,
        "version_id": version.id,
        "version_label": version.label,
        "target_machine_id": input.target_machine_id,
        "compose_project_name": compose_project_name(&stack.slug, &version.label),
        "deploy_strategy": package_deploy_strategy(projects),
        "compose": {
            "mode": compose_mode.kind(),
            "path": compose_mode.file_path(),
            "warnings": package_warnings,
        },
        "analysis": {
            "mode": "agent_planned",
            "agent_profile_id": plan_bundle.agent.id,
            "agent_session_id": plan_bundle.agent_session_id,
            "agent_name": plan_bundle.agent.name.clone(),
            "agent_provider": plan_bundle.agent.provider.clone(),
            "agent_model": plan_bundle.agent.model.clone(),
            "deploy_plan_path": "analysis/deploy-plan.json",
            "project_context_path": "analysis/project-context.json",
            "validation_report_path": "analysis/validation-report.json",
            "confidence": plan_bundle.plan.get("confidence").and_then(|value| value.as_str()).unwrap_or("low"),
            "status": plan_bundle.validation.get("status").and_then(|value| value.as_str()).unwrap_or("blocked"),
            "summary": plan_bundle.plan.get("summary").and_then(|value| value.as_str()).unwrap_or("Deploy plan generated"),
            "guided_summary": plan_bundle.plan.clone(),
        },
        "approved": false,
        "projects": projects.iter().map(|project| json!({
            "project_id": project.project.id,
            "name": project.project.name,
            "path": project.project.path,
            "branch": project.branch,
            "commit": project.commit_sha,
            "dirty": project.dirty,
            "git_status_short": project.git_status_short,
            "package_path": project.package_path,
            "dockerfile_path": project.dockerfile_path,
            "compose_path": project.detection.compose_path,
            "language": project.detection.language,
            "framework": project.detection.framework,
            "deploy_strategy": project.detection.deploy_strategy.clone(),
            "strategy_reason": project.detection.strategy_reason.clone(),
            "runtime_commands": project.detection.runtime_commands.clone(),
            "requires_desktop_session": project.detection.requires_desktop_session,
        })).collect::<Vec<_>>(),
        "services": detection.services,
        "ports": detection.ports,
        "env": {
            "generated_example_path": ".env.example"
        },
        "runbook": {
            "version": DEPLOY_RUNBOOK_VERSION,
            "scripts": deploy_runbook_scripts()
        },
        "warnings": package_warnings,
        "review": {
            "status": "pending",
            "blocking_findings": findings,
            "dismissed_findings": dismissed_findings
        }
    }))
}

#[allow(clippy::too_many_arguments)]
fn write_package_runbook(
    root: &Path,
    stack: &store::DeployStack,
    version: &store::DeployVersion,
    input: &CreateDeployPackageInput,
    projects: &[PackagedProject],
    plan_bundle: &deploy_plan::DeployPlanBundle,
    strategy: &str,
) -> anyhow::Result<()> {
    let project_lines = projects
        .iter()
        .map(|project| {
            format!(
                "- {}: {} / {} / {}",
                project.project.name,
                project.detection.language,
                project
                    .detection
                    .framework
                    .as_deref()
                    .unwrap_or("framework unknown"),
                project.detection.deploy_strategy
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let agent_name = plan_bundle.agent.name.as_str();
    let target = input
        .target_machine_id
        .as_deref()
        .unwrap_or("target selected later");
    let content = format!(
        r#"# ADE Deploy Runbook

Stack: {stack_name}
Version: {version_label}
Strategy: {strategy}
Target: {target}
Agent: {agent_name}
Runbook version: {runbook_version}

## Projetos

{project_lines}

## Fluxo automatico na ADE

1. Revise `manifest.json`, `analysis/deploy-plan.json` e este `RUNBOOK.md`.
2. Configure as variaveis exibidas pela ADE. Valores reais ficam locais e nao entram no pacote.
3. Aprove o pacote.
4. Rode `Preparar target`.
5. Rode `Deploy`.

Durante prepare/deploy a ADE executa um Deploy Doctor:

- classifica falhas conhecidas;
- aplica receitas seguras com retry limitado a 3 tentativas;
- chama o agente selecionado quando a falha precisa de correcao de script;
- cria uma proposta de repair somente para `scripts/*` e `RUNBOOK.md`;
- exige nova versao do pacote antes de executar scripts corrigidos.

## Linux manual fallback

Use somente se precisar depurar dentro da VM.

```sh
cd ~/dw-deploy/{stack_slug}/{version_label}
sh scripts/preflight.sh
sh scripts/deploy.sh
sh scripts/healthcheck.sh
sh scripts/logs.sh
```

Para desktop dev:

```sh
cd ~/dw-deploy/{stack_slug}/{version_label}
sh scripts/prepare-dev-vm.sh
sh scripts/build-dev.sh
sh scripts/run-dev.sh
```

## Windows manual fallback

Abra PowerShell como Administrator dentro da VM:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "\\host.lan\Data\ade\bootstrap-windows.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "\\host.lan\Data\deploy-packages\{stack_slug}\{version_label}\scripts\install-deploy.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "\\host.lan\Data\deploy-packages\{stack_slug}\{version_label}\scripts\deploy.ps1"
```

Para iniciar um pacote desktop dev manualmente no Windows:

```bat
"\\host.lan\Data\deploy-packages\{stack_slug}\{version_label}\scripts\run-dev.cmd"
```

## Logs

- Linux: `~/dw-deploy/{stack_slug}/{version_label}/.dw-runbook/logs`
- Windows shared package: `\\host.lan\Data\deploy-packages\{stack_slug}\{version_label}\.dw-runbook\logs`
- Windows local copy: `C:\dw\deploy`

## Limites do agente

O agente pode corrigir scripts e este runbook no pacote. Ele nao pode alterar o codigo-fonte dos projetos, gravar secrets ou escrever fora do pacote.
"#,
        stack_name = stack.name,
        stack_slug = stack.slug,
        version_label = version.label,
        strategy = strategy,
        target = target,
        agent_name = agent_name,
        runbook_version = DEPLOY_RUNBOOK_VERSION,
        project_lines = if project_lines.trim().is_empty() {
            "- nenhum projeto empacotado".to_string()
        } else {
            project_lines
        }
    );
    std::fs::write(root.join("RUNBOOK.md"), content)?;
    Ok(())
}

#[cfg(test)]
fn copy_source_snapshot(
    source: &Path,
    destination: &Path,
    findings: &mut Vec<SecretFinding>,
) -> anyhow::Result<()> {
    copy_source_snapshot_inner(source, destination, source, destination, findings)
}

fn copy_source_snapshot_for_package(
    source: &Path,
    destination: &Path,
    package_root: &Path,
    findings: &mut Vec<SecretFinding>,
) -> anyhow::Result<()> {
    copy_source_snapshot_inner(source, destination, source, package_root, findings)
}

fn copy_source_snapshot_inner(
    source: &Path,
    destination: &Path,
    source_root: &Path,
    destination_root: &Path,
    findings: &mut Vec<SecretFinding>,
) -> anyhow::Result<()> {
    for entry in
        std::fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let target = destination.join(&name);
        if let Some(reason) = excluded_secret_reason(&name) {
            findings.push(SecretFinding::warning(
                review_path_for(&target, destination_root),
                reason,
            ));
            continue;
        }
        if should_exclude_name(&name) {
            continue;
        }
        if path.is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
            copy_source_snapshot_inner(&path, &target, source_root, destination_root, findings)?;
        } else if path.is_file() {
            if is_secret_file_name(&name) {
                findings.push(SecretFinding::warning(
                    review_path_for(&target, destination_root),
                    "secret-like filename excluded from package",
                ));
                continue;
            }
            std::fs::copy(&path, &target).with_context(|| {
                format!("failed to copy {} to {}", path.display(), target.display())
            })?;
            findings.extend(scan_secret_content(
                &path,
                &target,
                source_root,
                destination_root,
            ));
        }
    }
    Ok(())
}

fn copy_package_dir(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if destination.exists() {
        anyhow::bail!(
            "deploy_repair_not_pending: destination package already exists: {}",
            destination.display()
        );
    }
    std::fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    for entry in
        std::fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_package_dir(&source_path, &target_path)?;
        } else if source_path.is_file() {
            std::fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn should_exclude_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".dw"
            | ".agents"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".venv"
            | "__pycache__"
            | "coverage"
            | "docs"
            | "test"
            | "tests"
            | "__tests__"
            | "e2e"
            | "fixtures"
            | "README.md"
            | "DESIGN.md"
            | "TROUBLESHOOTING.md"
    )
}

fn is_secret_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.starts_with("id_rsa")
        || lower.starts_with("id_ed25519")
}

fn excluded_secret_reason(name: &str) -> Option<&'static str> {
    if name.starts_with(".env") && name != ".env.example" {
        Some("environment file excluded from package")
    } else {
        None
    }
}

fn scan_secret_content(
    source_path: &Path,
    copied_path: &Path,
    source_root: &Path,
    copied_root: &Path,
) -> Vec<SecretFinding> {
    let Ok(bytes) = std::fs::read(copied_path) else {
        return Vec::new();
    };
    if bytes.len() > 512 * 1024 || bytes.contains(&0) {
        return Vec::new();
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return Vec::new();
    };
    let path = review_path_for(copied_path, copied_root);
    let warning = source_path
        .strip_prefix(source_root)
        .ok()
        .is_some_and(is_non_runtime_path)
        || copied_path
            .strip_prefix(copied_root)
            .ok()
            .is_some_and(is_non_runtime_review_path);
    deploy_scan::blocking_secret_markers(&text)
        .into_iter()
        .map(|hit| {
            let normalized_line = normalized_line_for_index(&text, hit.index);
            let reason = format!("secret-like content marker `{}`", hit.marker);
            let line_number = line_number_for_index(&text, hit.index);
            let line_sha256 = sha256_hex(normalized_line.as_bytes());
            if warning {
                SecretFinding::secret_content_warning(
                    path.clone(),
                    reason,
                    hit.marker,
                    line_sha256,
                    line_number,
                )
            } else {
                SecretFinding::secret_content_blocking(
                    path.clone(),
                    reason,
                    hit.marker,
                    line_sha256,
                    line_number,
                )
            }
        })
        .collect()
}

fn is_non_runtime_path(path: &Path) -> bool {
    // Exact lowercase dirs are excluded before copy; this downgrade is for case variants that remain.
    path.components().any(|component| {
        let Component::Normal(value) = component else {
            return false;
        };
        matches!(
            value.to_string_lossy().to_ascii_lowercase().as_str(),
            "test" | "tests" | "__tests__" | "e2e" | "fixtures" | "docs"
        )
    })
}

fn is_non_runtime_review_path(path: &Path) -> bool {
    if let Some(source_relative) = package_source_relative_path(path) {
        return is_non_runtime_path(&source_relative);
    }
    is_non_runtime_path(path)
}

fn package_source_relative_path(path: &Path) -> Option<PathBuf> {
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if components.len() >= 3 && components[0] == "projects" && components[2] == "source" {
        let mut source_relative = PathBuf::new();
        for component in &components[3..] {
            source_relative.push(component);
        }
        return Some(source_relative);
    }
    None
}

fn review_path_for(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalized_line_for_index(text: &str, index: usize) -> String {
    let bytes = text.as_bytes();
    let mut start = index.min(bytes.len());
    while start > 0 && !matches!(bytes[start - 1], b'\n' | b'\r') {
        start -= 1;
    }
    let mut end = index.min(bytes.len());
    while end < bytes.len() && !matches!(bytes[end], b'\n' | b'\r') {
        end += 1;
    }
    text[start..end].trim().to_string()
}

fn line_number_for_index(text: &str, index: usize) -> usize {
    text.as_bytes()
        .iter()
        .take(index.min(text.len()))
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn default_finding_severity() -> String {
    "error".to_string()
}

fn default_finding_blocking() -> bool {
    true
}

impl SecretFinding {
    fn warning(path: String, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let hint = Some(review_finding_hint(&reason, false).to_string());
        Self {
            path,
            reason,
            marker: None,
            line_sha256: None,
            line_number: None,
            occurrence_index: None,
            occurrence_count: None,
            hint,
            severity: "warning".to_string(),
            blocking: false,
        }
    }

    fn blocking(path: String, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let hint = Some(review_finding_hint(&reason, true).to_string());
        Self {
            path,
            reason,
            marker: None,
            line_sha256: None,
            line_number: None,
            occurrence_index: None,
            occurrence_count: None,
            hint,
            severity: "error".to_string(),
            blocking: true,
        }
    }

    fn secret_content_warning(
        path: String,
        reason: impl Into<String>,
        marker: &str,
        line_sha256: String,
        line_number: usize,
    ) -> Self {
        let reason = reason.into();
        let hint = Some(review_finding_hint(&reason, false).to_string());
        Self {
            path,
            reason,
            marker: Some(marker.to_string()),
            line_sha256: Some(line_sha256),
            line_number: Some(line_number),
            occurrence_index: None,
            occurrence_count: None,
            hint,
            severity: "warning".to_string(),
            blocking: false,
        }
    }

    fn secret_content_blocking(
        path: String,
        reason: impl Into<String>,
        marker: &str,
        line_sha256: String,
        line_number: usize,
    ) -> Self {
        let reason = reason.into();
        let hint = Some(review_finding_hint(&reason, true).to_string());
        Self {
            path,
            reason,
            marker: Some(marker.to_string()),
            line_sha256: Some(line_sha256),
            line_number: Some(line_number),
            occurrence_index: None,
            occurrence_count: None,
            hint,
            severity: "error".to_string(),
            blocking: true,
        }
    }
}

fn annotate_finding_occurrences(findings: &mut [SecretFinding]) {
    // UI-only occurrence metadata: review identity intentionally ignores these fields so one
    // dismissal/restoration applies to every identical content occurrence in the same file.
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, finding) in findings.iter().enumerate() {
        groups
            .entry(review_occurrence_group_key(finding))
            .or_default()
            .push(index);
    }
    for indexes in groups.values() {
        if indexes.len() < 2 {
            continue;
        }
        for (offset, index) in indexes.iter().enumerate() {
            if let Some(finding) = findings.get_mut(*index) {
                finding.occurrence_index = Some(offset + 1);
                finding.occurrence_count = Some(indexes.len());
            }
        }
    }
}

fn review_occurrence_group_key(finding: &SecretFinding) -> String {
    let identity = review_identity_for_finding(finding);
    serde_json::to_string(&identity).unwrap_or_else(|_| {
        format!(
            "{}\u{1f}{}\u{1f}{:?}\u{1f}{:?}",
            identity.path, identity.reason, identity.marker, identity.line_sha256
        )
    })
}

fn review_finding_hint(reason: &str, blocking: bool) -> &'static str {
    if reason == "environment file excluded from package" {
        "Esperado: arquivos .env reais ficam fora do pacote; preencha os valores na UI de ambiente."
    } else if reason == "secret-like filename excluded from package" {
        "Esperado: chaves privadas e certificados ficam fora do pacote; configure o segredo no target ou na UI de ambiente."
    } else if reason.contains("secret-like content marker") {
        "Se for placeholder/codigo montando valor em runtime, ajuste o padrao; se for segredo real, remova do source e use env."
    } else if blocking {
        "Corrija o item indicado antes de aprovar o pacote de deploy."
    } else {
        "Revise o aviso antes de aprovar o pacote de deploy."
    }
}

fn package_deploy_strategy(projects: &[PackagedProject]) -> String {
    let mut strategies = projects
        .iter()
        .map(|project| project.detection.deploy_strategy.as_str())
        .collect::<Vec<_>>();
    strategies.sort_unstable();
    strategies.dedup();
    match strategies.as_slice() {
        [] => "unsupported".to_string(),
        [single] => (*single).to_string(),
        _ => "mixed".to_string(),
    }
}

fn package_compose_mode(
    package_strategy: &str,
    projects: &[PackagedProject],
    plan: &serde_json::Value,
) -> PackageComposeDecision {
    if let Some(path) = compose_artifact_path(plan) {
        return PackageComposeDecision {
            mode: PackageComposeMode::AgentArtifact { path },
            warnings: Vec::new(),
        };
    }
    if package_strategy == "custom_compose" {
        let passthrough_paths = projects
            .iter()
            .filter_map(source_passthrough_compose_path)
            .collect::<Vec<_>>();
        if passthrough_paths.len() == 1 {
            return PackageComposeDecision {
                mode: PackageComposeMode::SourcePassthrough {
                    path: passthrough_paths[0].clone(),
                },
                warnings: Vec::new(),
            };
        }
        if passthrough_paths.len() > 1 {
            return PackageComposeDecision {
                mode: PackageComposeMode::Generated {
                    path: "docker-compose.yml".to_string(),
                },
                warnings: vec![format!(
                    "multiple project compose files detected ({}); source passthrough disabled, generated compose selected",
                    passthrough_paths.join(", ")
                )],
            };
        }
    }
    PackageComposeDecision {
        mode: PackageComposeMode::Generated {
            path: "docker-compose.yml".to_string(),
        },
        warnings: Vec::new(),
    }
}

fn compose_artifact_path(plan: &serde_json::Value) -> Option<String> {
    let compose = plan.get("artifacts")?.get("compose")?;
    let body = compose.get("body")?.as_str()?.trim();
    if body.is_empty() {
        return None;
    }
    let path = compose
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .unwrap_or("docker-compose.yml");
    Some(path.to_string())
}

fn source_passthrough_compose_path(project: &PackagedProject) -> Option<String> {
    if project.detection.deploy_strategy != "custom_compose" {
        return None;
    }
    let compose_path = project.detection.compose_path.as_deref()?.trim();
    let path = Path::new(compose_path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(format!(
        "{}/{}",
        project.package_path,
        compose_path.replace('\\', "/")
    ))
}

fn collect_project_env_defaults(projects: &[PackagedProject]) -> BTreeMap<String, String> {
    let mut defaults = BTreeMap::new();
    for project in projects {
        for (key, value) in read_project_env_example_defaults(Path::new(&project.project.path)) {
            defaults.entry(key).or_insert(value);
        }
    }
    for project in projects {
        for (key, value) in read_project_compose_env_defaults(project) {
            defaults.insert(key, value);
        }
    }
    defaults
}

fn read_project_compose_env_defaults(project: &PackagedProject) -> BTreeMap<String, String> {
    let Some(compose_path) = safe_project_relative_path(project.detection.compose_path.as_deref())
    else {
        return BTreeMap::new();
    };
    let path = Path::new(&project.project.path).join(compose_path);
    let Ok(content) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    parse_compose_env_defaults(&content)
}

fn parse_compose_env_defaults(content: &str) -> BTreeMap<String, String> {
    let mut defaults = BTreeMap::new();
    for line in content.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let bytes = line.as_bytes();
        let mut index = 0;
        while index + 1 < bytes.len() {
            if bytes[index] != b'$' || bytes[index + 1] != b'{' {
                index += 1;
                continue;
            }
            if index > 0 && bytes[index - 1] == b'$' {
                index += 2;
                continue;
            }
            let Some((expression, end)) = compose_interpolation_expression(line, index + 2) else {
                break;
            };
            if let Some((key, value)) = compose_default_expression(expression) {
                defaults.insert(key.to_string(), value.to_string());
            }
            index = end + 1;
        }
    }
    defaults
}

fn compose_interpolation_expression(line: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = line.as_bytes();
    let mut depth = 0usize;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => depth += 1,
            b'}' if depth == 0 => return Some((&line[start..index], index)),
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        index += 1;
    }
    None
}

fn compose_default_expression(expression: &str) -> Option<(&str, &str)> {
    let key_end = expression
        .char_indices()
        .take_while(|(_, ch)| *ch == '_' || ch.is_ascii_alphanumeric())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    let key = &expression[..key_end];
    if !is_valid_env_key(key) {
        return None;
    }
    let rest = &expression[key_end..];
    let value = rest
        .strip_prefix(":-")
        .or_else(|| rest.strip_prefix('-'))?
        .trim();
    if value.is_empty() {
        return None;
    }
    Some((key, value))
}

fn read_project_env_example_defaults(project_root: &Path) -> BTreeMap<String, String> {
    let path = project_root.join(".env.example");
    let mut defaults = BTreeMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return defaults;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = unquote_env_value(value.trim());
        if is_valid_env_key(key) && !value.trim().is_empty() {
            defaults.insert(key.to_string(), value.to_string());
        }
    }
    defaults
}

fn safe_project_relative_path(value: Option<&str>) -> Option<PathBuf> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(path.to_path_buf())
}

fn write_generated_dockerfile(
    path: &Path,
    detection: &DeployProjectDetection,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = match detection.deploy_strategy.as_str() {
        "desktop_dev" => {
            "FROM ubuntu:24.04\nCMD [\"sh\", \"-c\", \"echo desktop_dev packages run directly on the ADE target VM via scripts/run-dev.sh; sleep infinity\"]\n".to_string()
        }
        "unsupported" => {
            "FROM alpine:3.20\nCMD [\"sh\", \"-c\", \"echo unsupported deploy strategy; sleep 3600\"]\n".to_string()
        }
        _ => match detection.language.as_str() {
        "typescript" => {
            let pm = detection.package_manager.as_deref().unwrap_or("npm");
            let install = if pm == "pnpm" {
                "RUN corepack enable && pnpm install --frozen-lockfile\nCMD [\"pnpm\", \"dev\", \"--host\", \"0.0.0.0\"]"
            } else {
                "RUN npm install\nCMD [\"npm\", \"run\", \"dev\"]"
            };
            format!(
                "FROM node:20-slim\nWORKDIR /app\nCOPY . .\n{install}\nEXPOSE {}\n",
                detection.ports[0].container
            )
        }
        "python" => format!("FROM python:3.12-slim\nWORKDIR /app\nCOPY . .\nRUN if [ -f requirements.txt ]; then pip install -r requirements.txt; fi\nEXPOSE {}\nCMD [\"python\", \"-m\", \"http.server\", \"{}\"]\n", detection.ports[0].container, detection.ports[0].container),
        "rust" => format!("FROM rust:1-slim\nWORKDIR /app\nCOPY . .\nRUN cargo build\nEXPOSE {}\nCMD [\"cargo\", \"run\"]\n", detection.ports[0].container),
        "dotnet" => format!("FROM mcr.microsoft.com/dotnet/sdk:8.0\nWORKDIR /app\nCOPY . .\nEXPOSE {}\nCMD [\"dotnet\", \"run\", \"--urls\", \"http://0.0.0.0:{}\"]\n", detection.ports[0].container, detection.ports[0].container),
        _ => "FROM alpine:3.20\nWORKDIR /app\nCOPY . .\nCMD [\"sh\", \"-c\", \"echo unsupported project && sleep 3600\"]\n".to_string(),
        },
    };
    std::fs::write(path, content)?;
    Ok(())
}

fn write_generated_dockerignore(path: &Path) -> anyhow::Result<()> {
    std::fs::write(
        path,
        ".git\n.dw\n.agents\nnode_modules\ntarget\ndist\nbuild\ncoverage\ndocs\ntest\ntests\n__tests__\ne2e\nfixtures\n.env\n.env.*\n*.pem\n*.key\n",
    )?;
    Ok(())
}

fn write_compose(path: &Path, projects: &[PackagedProject]) -> anyhow::Result<()> {
    let mut content = String::from("services:\n");
    let compose_projects = projects
        .iter()
        .filter(|project| {
            matches!(
                project.detection.deploy_strategy.as_str(),
                "web_service" | "custom_compose"
            )
        })
        .collect::<Vec<_>>();
    if compose_projects.is_empty() {
        content = String::from("services: {}\n");
    }
    for project in compose_projects {
        let service = slugify(&project.project.name);
        let dockerfile = if project.detection.deploy_strategy == "custom_compose" {
            select_custom_dockerfile(Path::new(&project.project.path))
                .unwrap_or_else(|| "../Dockerfile".to_string())
        } else {
            "../Dockerfile".to_string()
        };
        let port = project
            .detection
            .ports
            .first()
            .map(|port| port.container)
            .unwrap_or(8080);
        content.push_str(&format!(
            "  {service}:\n    build:\n      context: ./{}\n      dockerfile: {dockerfile}\n    env_file: .env\n    ports:\n      - \"{port}:{port}\"\n",
            project.package_path
        ));
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn write_env_example(
    path: &Path,
    services: &[deploy_detect::DeployServiceSuggestion],
    plan: &serde_json::Value,
    detector_keys_optional: bool,
    defaults: &BTreeMap<String, String>,
) -> anyhow::Result<()> {
    let mut content =
        String::from("# Generated environment template. Do not paste real secrets here.\n");
    let mut required = BTreeSet::<String>::new();
    let mut optional = BTreeSet::<String>::new();

    for key in plan_env_keys(plan, "required") {
        optional.remove(&key);
        required.insert(key);
    }
    for key in plan_env_keys(plan, "optional") {
        if !required.contains(&key) {
            optional.insert(key);
        }
    }
    for service in services {
        match service.name.as_str() {
            "postgres" => {
                insert_detected_env_key(
                    &mut required,
                    &mut optional,
                    "DATABASE_URL",
                    detector_keys_optional,
                );
            }
            "redis" => {
                insert_detected_env_key(
                    &mut required,
                    &mut optional,
                    "REDIS_URL",
                    detector_keys_optional,
                );
            }
            "smtp" => {
                insert_detected_env_key(
                    &mut required,
                    &mut optional,
                    "SMTP_URL",
                    detector_keys_optional,
                );
            }
            _ => {}
        }
    }
    for key in defaults.keys() {
        if !required.contains(key) {
            optional.insert(key.clone());
        }
    }
    optional.retain(|key| !required.contains(key));
    for key in required {
        if defaults.contains_key(&key) {
            content.push_str(ENV_TEMPLATE_DEFAULT_MARKER);
            content.push(' ');
            content.push_str(&key);
            content.push_str(" project\n");
        }
        content.push_str(&key);
        content.push('=');
        if let Some(value) = defaults.get(&key) {
            content.push_str(value);
        }
        content.push('\n');
    }
    for key in optional {
        if defaults.contains_key(&key) {
            content.push_str(ENV_TEMPLATE_DEFAULT_MARKER);
            content.push(' ');
            content.push_str(&key);
            content.push_str(" project\n");
        }
        content.push('#');
        content.push_str(&key);
        content.push('=');
        if let Some(value) = defaults.get(&key) {
            content.push_str(value);
        }
        content.push('\n');
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn unquote_env_value(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn insert_detected_env_key(
    required: &mut BTreeSet<String>,
    optional: &mut BTreeSet<String>,
    key: &str,
    detector_keys_optional: bool,
) {
    if detector_keys_optional {
        if !required.contains(key) {
            optional.insert(key.to_string());
        }
    } else {
        optional.remove(key);
        required.insert(key.to_string());
    }
}

fn plan_has_compose_artifact(plan: &serde_json::Value) -> bool {
    compose_artifact_path(plan).is_some()
}

fn select_custom_dockerfile(root: &Path) -> Option<String> {
    for name in ["Dockerfile", "Dockerfile.prod", "Dockerfile.dev"] {
        if root.join(name).is_file() {
            return Some(name.to_string());
        }
    }
    let mut candidates = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with("Dockerfile.") {
                candidates.insert(name.to_string());
            }
        }
    }
    candidates.into_iter().next()
}

fn plan_env_keys(plan: &serde_json::Value, group: &str) -> Vec<String> {
    plan.get("env")
        .and_then(|env| env.get(group))
        .and_then(serde_json::Value::as_array)
        .map(|keys| {
            keys.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|key| is_valid_env_key(key))
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn is_valid_env_key(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn write_scripts(
    dir: &Path,
    strategy: &str,
    projects: &[PackagedProject],
    compose_mode: &PackageComposeMode,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    if let Some(root) = dir.parent() {
        let mut desktop_projects = projects
            .iter()
            .filter(|project| project.detection.deploy_strategy == "desktop_dev")
            .map(|project| project.package_path.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if !desktop_projects.is_empty() {
            desktop_projects.push('\n');
        }
        std::fs::write(root.join(".dw-deploy-strategy"), strategy)?;
        std::fs::write(root.join(".dw-desktop-projects"), desktop_projects)?;
        std::fs::write(root.join(".dw-compose-file"), compose_mode.file_path())?;
    }
    std::fs::write(
        dir.join("preflight.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
compose_file="$(cat .dw-compose-file 2>/dev/null || echo docker-compose.yml)"
compose_env_args=""
if [ -f ./.env ]; then
  compose_env_args="--env-file ./.env"
fi
echo "[dw] preflight project=$project"
test -f manifest.json
if [ "$strategy" = "desktop_dev" ]; then
  test -s .dw-desktop-projects
  command -v sh >/dev/null
else
  test -f "$compose_file"
  command -v docker >/dev/null
  docker --version
  docker compose version
  docker compose $compose_env_args -f "$compose_file" -p "$project" config >/dev/null
fi
echo "[dw] preflight ok"
"#,
    )?;
    std::fs::write(
        dir.join("deploy.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
compose_file="$(cat .dw-compose-file 2>/dev/null || echo docker-compose.yml)"
compose_env_args=""
if [ -f ./.env ]; then
  compose_env_args="--env-file ./.env"
fi
if [ "$strategy" = "desktop_dev" ]; then
  chmod +x scripts/prepare-dev-vm.sh scripts/build-dev.sh
  ./scripts/prepare-dev-vm.sh
  ./scripts/build-dev.sh
else
  test -f "$compose_file"
  docker compose $compose_env_args -f "$compose_file" -p "$project" up -d --build
  docker compose $compose_env_args -f "$compose_file" -p "$project" ps
fi
"#,
    )?;
    std::fs::write(
        dir.join("stop.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
compose_file="$(cat .dw-compose-file 2>/dev/null || echo docker-compose.yml)"
compose_env_args=""
if [ -f ./.env ]; then
  compose_env_args="--env-file ./.env"
fi
if [ "$strategy" = "desktop_dev" ]; then
  echo "[dw] desktop_dev package has no managed compose service to stop"
else
  test -f "$compose_file"
  docker compose $compose_env_args -f "$compose_file" -p "$project" down
fi
"#,
    )?;
    std::fs::write(
        dir.join("healthcheck.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
compose_file="$(cat .dw-compose-file 2>/dev/null || echo docker-compose.yml)"
compose_env_args=""
if [ -f ./.env ]; then
  compose_env_args="--env-file ./.env"
fi
if [ "$strategy" = "desktop_dev" ]; then
  chmod +x scripts/verify-dev.sh
  ./scripts/verify-dev.sh
else
  test -f "$compose_file"
  docker compose $compose_env_args -f "$compose_file" -p "$project" ps
  docker compose $compose_env_args -f "$compose_file" -p "$project" ps --format json >/tmp/dw-compose-ps.json 2>/dev/null || true
  if docker compose $compose_env_args -f "$compose_file" -p "$project" ps --status exited | grep -q .; then
    echo "[dw] one or more services exited" >&2
    docker compose $compose_env_args -f "$compose_file" -p "$project" ps >&2
    exit 1
  fi
fi
echo "[dw] healthcheck ok"
"#,
    )?;
    std::fs::write(
        dir.join("logs.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
compose_file="$(cat .dw-compose-file 2>/dev/null || echo docker-compose.yml)"
compose_env_args=""
if [ -f ./.env ]; then
  compose_env_args="--env-file ./.env"
fi
if [ "$strategy" = "desktop_dev" ]; then
  find .dw-runbook/logs -type f -maxdepth 1 -print -exec sh -c 'echo "===== $1"; tail -160 "$1"' sh {} \; 2>/dev/null || true
else
  test -f "$compose_file"
  docker compose $compose_env_args -f "$compose_file" -p "$project" ps
  docker compose $compose_env_args -f "$compose_file" -p "$project" logs --tail=160
fi
"#,
    )?;
    std::fs::write(
        dir.join("prepare-dev-vm.sh"),
        r#"#!/usr/bin/env sh
set -eu

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required to prepare a desktop dev package target" >&2
  exit 1
fi

echo "[dw] preparing desktop dev VM dependencies"
apt_log_has_retryable_lock() {
  grep -Eiq 'could not get lock|unable to lock directory|is held by process|waiting for cache lock|dpkg frontend lock|dpkg lock' "$1"
}
apt_update_with_retry() {
  log="$(mktemp)"
  attempt=1
  while [ "$attempt" -le 90 ]; do
    if sudo apt-get update >"$log" 2>&1; then
      rm -f "$log"
      return 0
    fi
    if grep -qi "not valid yet" "$log"; then
      echo "[dw] apt repository metadata is newer than guest clock; waiting before retry ($attempt/90)" >&2
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    if apt_log_has_retryable_lock "$log"; then
      echo "[dw] apt is busy; waiting for package manager lock ($attempt/90)" >&2
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    cat "$log" >&2
    rm -f "$log"
    return 1
  done
  cat "$log" >&2
  rm -f "$log"
  return 1
}
apt_install_with_retry() {
  log="$(mktemp)"
  attempt=1
  while [ "$attempt" -le 90 ]; do
    if sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "$@" >"$log" 2>&1; then
      rm -f "$log"
      return 0
    fi
    if apt_log_has_retryable_lock "$log"; then
      echo "[dw] apt is busy; waiting for package manager lock ($attempt/90)" >&2
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    cat "$log" >&2
    rm -f "$log"
    return 1
  done
  cat "$log" >&2
  rm -f "$log"
  return 1
}
apt_update_with_retry
apt_install_with_retry \
  build-essential ca-certificates curl git nodejs pkg-config libssl-dev \
  cargo rustc libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev || true

node_major="$(node -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo 0)"
if [ "$node_major" -lt 20 ]; then
  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
  apt_install_with_retry nodejs
fi

if ! command -v cargo >/dev/null 2>&1; then
  apt_install_with_retry cargo rustc
fi

if find . -path '*/src-tauri/Cargo.toml' -print -quit | grep -q .; then
  apt_install_with_retry \
    libwebkit2gtk-4.1-dev libsoup-3.0-dev javascriptcoregtk-4.1-dev \
    libxdo-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev || \
  apt_install_with_retry \
    libwebkit2gtk-4.0-dev libsoup2.4-dev javascriptcoregtk-4.0-dev libxdo-dev || true

  export PATH="$HOME/.cargo/bin:$PATH"
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  fi
  rustup default stable
fi

export PATH="$HOME/.cargo/bin:$PATH"
if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi
node --version
npm --version
cargo --version
echo "[dw] desktop dev VM dependencies ready"
"#,
    )?;
    std::fs::write(
        dir.join("build-dev.sh"),
        r#"#!/usr/bin/env sh
set -eu
mkdir -p .dw-runbook/logs
while IFS= read -r project_path || [ -n "$project_path" ]; do
  [ -n "$project_path" ] || continue
  echo "[dw] verifying desktop dev project $project_path"
  if [ ! -d "$project_path" ]; then
    echo "project path not found: $project_path" >&2
    exit 1
  fi
  (
    cd "$project_path"
    if [ -f package.json ]; then
      npm install
      test -d node_modules
      if npm run | grep -q ' check:js'; then
        npm run check:js
      fi
      if [ -f src-tauri/Cargo.toml ]; then
        test -x node_modules/.bin/tauri
        node_modules/.bin/tauri --version
        if [ -f "$HOME/.cargo/env" ]; then
          . "$HOME/.cargo/env"
        fi
        export PATH="$HOME/.cargo/bin:$PATH"
        cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1 >/dev/null
      fi
    fi
  ) > ".dw-runbook/logs/$(basename "$project_path").build.log" 2>&1
done < .dw-desktop-projects
echo "[dw] desktop dev package verified"
"#,
    )?;
    std::fs::write(
        dir.join("verify-dev.sh"),
        r#"#!/usr/bin/env sh
set -eu
test -s .dw-desktop-projects
command -v node >/dev/null
command -v npm >/dev/null
export PATH="$HOME/.cargo/bin:$PATH"
if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi
while IFS= read -r project_path || [ -n "$project_path" ]; do
  [ -n "$project_path" ] || continue
  test -d "$project_path"
  if [ -f "$project_path/package.json" ]; then
    test -d "$project_path/node_modules"
  fi
  if [ -f "$project_path/src-tauri/Cargo.toml" ]; then
    command -v cargo >/dev/null
    test -x "$project_path/node_modules/.bin/tauri"
  fi
done < .dw-desktop-projects
echo "[dw] desktop dev verification ok"
"#,
    )?;
    std::fs::write(
        dir.join("run-dev.sh"),
        r#"#!/usr/bin/env sh
set -eu
export PATH="$HOME/.cargo/bin:$PATH"
if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

load_desktop_session_env() {
  if [ -n "${DISPLAY:-}" ]; then
    return 0
  fi
  for pid in $(pgrep -u "$(id -u)" -x xfce4-session 2>/dev/null || true); do
    env_file="/proc/$pid/environ"
    [ -r "$env_file" ] || continue
    display=$(tr '\0' '\n' < "$env_file" | sed -n 's/^DISPLAY=//p' | head -n 1)
    [ -n "$display" ] || continue
    dbus=$(tr '\0' '\n' < "$env_file" | sed -n 's/^DBUS_SESSION_BUS_ADDRESS=//p' | head -n 1)
    xauth=$(tr '\0' '\n' < "$env_file" | sed -n 's/^XAUTHORITY=//p' | head -n 1)
    export DISPLAY="$display"
    [ -n "$dbus" ] && export DBUS_SESSION_BUS_ADDRESS="$dbus"
    [ -n "$xauth" ] && export XAUTHORITY="$xauth"
    return 0
  done
  export DISPLAY="${DISPLAY:-:0}"
  if [ -z "${XAUTHORITY:-}" ] && [ -f "$HOME/.Xauthority" ]; then
    export XAUTHORITY="$HOME/.Xauthority"
  fi
  if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ] && [ -S "/run/user/$(id -u)/bus" ]; then
    export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
  fi
}

load_desktop_session_env
count=$(grep -cve '^[[:space:]]*$' .dw-desktop-projects || true)
if [ "$count" != "1" ]; then
  echo "Select one project and run its dev command manually:" >&2
  cat .dw-desktop-projects >&2
  exit 2
fi
project_path=$(grep -ve '^[[:space:]]*$' .dw-desktop-projects | head -n 1)
cd "$project_path"
if [ -f package.json ]; then
  exec npm run dev
fi
echo "No runnable dev command detected for $project_path" >&2
exit 1
"#,
    )?;
    std::fs::write(
        dir.join("rollback.sh"),
        r#"#!/usr/bin/env sh
set -eu
echo "[dw] rollback is orchestrated by ADE by reactivating a previous approved version." >&2
echo "[dw] This script intentionally does not select a target version on its own." >&2
exit 2
"#,
    )?;
    std::fs::write(
        dir.join("install-base-linux.sh"),
        linux_install_base_script(),
    )?;
    std::fs::write(
        dir.join("install-deploy.ps1"),
        windows_install_deploy_script(),
    )?;
    if strategy == "desktop_dev" {
        write_windows_desktop_dev_scripts(dir)?;
    }
    Ok(())
}

fn write_windows_desktop_dev_scripts(dir: &Path) -> anyhow::Result<()> {
    std::fs::write(dir.join("deploy.ps1"), windows_desktop_deploy_script())?;
    std::fs::write(
        dir.join("healthcheck.ps1"),
        windows_desktop_healthcheck_script(),
    )?;
    std::fs::write(dir.join("logs.ps1"), windows_desktop_logs_script())?;
    std::fs::write(dir.join("stop.ps1"), windows_desktop_stop_script())?;
    std::fs::write(dir.join("run-dev.ps1"), windows_desktop_run_dev_script())?;
    std::fs::write(dir.join("run-dev.cmd"), windows_desktop_run_dev_cmd())?;
    Ok(())
}

fn windows_install_deploy_script() -> &'static str {
    r#"$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PackageRoot = Resolve-Path (Join-Path $ScriptDir "..")
$StrategyFile = Join-Path $PackageRoot ".dw-deploy-strategy"
$Strategy = "web_service"
if (Test-Path $StrategyFile) {
  $Strategy = (Get-Content -Raw $StrategyFile).Trim()
}

function Assert-Admin {
  $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Run this script as Administrator."
  }
}

function Refresh-ProcessPath {
  $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
  $user = [Environment]::GetEnvironmentVariable("Path", "User")
  $env:Path = "$machine;$user"
}

function Ensure-Winget {
  if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    throw "winget is required to install Windows deploy dependencies. Install App Installer from Microsoft Store, then rerun this script."
  }
}

function Test-WingetAlreadySatisfied([string]$OutputText) {
  $lower = $OutputText.ToLowerInvariant()
  return $lower.Contains("found an existing package already installed") -and (
    $lower.Contains("no available upgrade found") -or
    $lower.Contains("no newer package versions are available")
  )
}

function Invoke-WingetInstall([string]$Id, [string[]]$ExtraArgs = @()) {
  Ensure-Winget
  $args = @(
    "install",
    "--id", $Id,
    "--exact",
    "--source", "winget",
    "--silent",
    "--accept-package-agreements",
    "--accept-source-agreements",
    "--disable-interactivity"
  ) + $ExtraArgs
  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    $output = & winget @args 2>&1 | ForEach-Object { "$_" }
    $exitCode = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
  }
  $outputText = ($output -join "`n")
  if (-not [string]::IsNullOrWhiteSpace($outputText)) {
    Write-Host $outputText
  }
  if ($exitCode -ne 0) {
    if (Test-WingetAlreadySatisfied $outputText) {
      Write-Host "[dw] $Id is already installed; winget reported no upgrade, continuing"
      return
    }
    throw "winget install failed for $Id with exit code $exitCode"
  }
}

function Wait-CommandAvailable([string]$CommandName, [string]$PackageId) {
  for ($attempt = 1; $attempt -le 24; $attempt++) {
    Refresh-ProcessPath
    if (Get-Command $CommandName -ErrorAction SilentlyContinue) {
      return
    }
    Start-Sleep -Seconds 5
  }
  throw "$CommandName was not found after installing $PackageId. Restart Windows or reopen PowerShell and run this script again."
}

function Ensure-WingetPackage([string]$Id, [string]$CommandName) {
  Refresh-ProcessPath
  if (Get-Command $CommandName -ErrorAction SilentlyContinue) {
    return
  }
  Write-Host "[dw] Installing $Id"
  Invoke-WingetInstall $Id
  Wait-CommandAvailable $CommandName $Id
}

function Ensure-WingetInstallOnly([string]$Id) {
  Write-Host "[dw] Ensuring $Id"
  Invoke-WingetInstall $Id
  Refresh-ProcessPath
}

function Test-VsBuildTools {
  $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
  if (-not (Test-Path $vswhere)) {
    return $false
  }
  $installPath = & $vswhere -latest -products Microsoft.VisualStudio.Product.BuildTools -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  return -not [string]::IsNullOrWhiteSpace($installPath)
}

function Ensure-VsBuildTools {
  Refresh-ProcessPath
  if (Test-VsBuildTools) {
    return
  }
  Write-Host "[dw] Installing Visual Studio Build Tools C++ workload"
  Invoke-WingetInstall "Microsoft.VisualStudio.2022.BuildTools" @("--override", "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended")
  Refresh-ProcessPath
}

Assert-Admin

Write-Host "[dw] Configuring OpenSSH Server for ADE deploy packages"
$capability = Get-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
if ($capability.State -ne "Installed") {
  Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
}
Set-Service -Name sshd -StartupType Automatic
Start-Service sshd
if (-not (Get-NetFirewallRule -Name OpenSSH-Server-In-TCP -ErrorAction SilentlyContinue)) {
  New-NetFirewallRule -Name OpenSSH-Server-In-TCP -DisplayName "OpenSSH Server (sshd)" -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 | Out-Null
}

New-Item -ItemType Directory -Force -Path "C:\dw", "C:\dw\deploy", "C:\dw\logs" | Out-Null

if ($Strategy -eq "desktop_dev") {
  Write-Host "[dw] Configuring Windows desktop_dev toolchain"
  Ensure-WingetPackage "Git.Git" "git"
  Ensure-WingetPackage "OpenJS.NodeJS.LTS" "node"
  Ensure-WingetPackage "Rustlang.Rustup" "rustup"
  Ensure-WingetInstallOnly "Microsoft.EdgeWebView2Runtime"
  Ensure-VsBuildTools
  Refresh-ProcessPath
  rustup default stable
  node --version
  npm --version
  cargo --version
  Write-Host "[dw] Windows desktop_dev dependencies ready. Run scripts\deploy.ps1 from this package, or retry Deploy in ADE after SSH validation."
  exit 0
}

Write-Host "[dw] Validating Docker for container deploy"
docker --version
docker compose version
"#
}

fn windows_desktop_deploy_script() -> &'static str {
    r#"param(
  [string]$ComposeProjectName = $env:DW_COMPOSE_PROJECT_NAME,
  [switch]$PrintTarget
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SharedPackageRoot = (Resolve-Path (Join-Path $ScriptDir "..")).ProviderPath
$LocalPackageName = if ([string]::IsNullOrWhiteSpace($ComposeProjectName)) {
  Split-Path -Leaf $SharedPackageRoot
} else {
  $ComposeProjectName -replace "[^A-Za-z0-9_.-]", "_"
}
$LocalPackageRoot = Join-Path "C:\dw\deploy" $LocalPackageName
$LogDir = Join-Path $SharedPackageRoot ".dw-runbook\logs"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

function Refresh-ProcessPath {
  $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
  $user = [Environment]::GetEnvironmentVariable("Path", "User")
  $env:Path = "$machine;$user"
}

function Invoke-Logged([string]$LogName, [scriptblock]$Block) {
  $logPath = Join-Path $LogDir $LogName
  $previousPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    & $Block 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath $logPath
    $exitCode = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousPreference
  }
  if ($exitCode -ne 0) {
    throw "Command failed with exit code $exitCode. See $logPath"
  }
}

function Sync-PackageToLocal {
  New-Item -ItemType Directory -Force -Path $LocalPackageRoot | Out-Null
  & robocopy $SharedPackageRoot $LocalPackageRoot /MIR /XD node_modules .git target .dw-runbook | Tee-Object -FilePath (Join-Path $LogDir "robocopy.log")
  $exitCode = $LASTEXITCODE
  if ($exitCode -gt 7) {
    throw "robocopy failed with exit code $exitCode. See $(Join-Path $LogDir "robocopy.log")"
  }
}

Refresh-ProcessPath
Sync-PackageToLocal
$PackageRoot = (Resolve-Path $LocalPackageRoot).ProviderPath
Set-Location $PackageRoot
$ProjectsFile = Join-Path $PackageRoot ".dw-desktop-projects"
if (-not (Test-Path $ProjectsFile)) {
  throw ".dw-desktop-projects not found"
}

$projects = Get-Content $ProjectsFile | Where-Object { $_.Trim().Length -gt 0 }
if ($projects.Count -eq 0) {
  throw "No desktop projects declared in .dw-desktop-projects"
}

foreach ($project in $projects) {
  $projectDir = Join-Path $PackageRoot $project
  if (-not (Test-Path $projectDir)) {
    throw "Project path not found: $project"
  }
  Push-Location $projectDir
  try {
    if (Test-Path "package.json") {
      Invoke-Logged "$((Split-Path -Leaf $project)).npm-install.log" { npm install }
      if (Test-Path "src-tauri\Cargo.toml") {
        if (Test-Path "node_modules\.bin\tauri.cmd") {
          Invoke-Logged "$((Split-Path -Leaf $project)).tauri-version.log" { & ".\node_modules\.bin\tauri.cmd" --version }
        } else {
          throw "Tauri CLI not found under node_modules\.bin after npm install"
        }
        Invoke-Logged "$((Split-Path -Leaf $project)).cargo-metadata.log" { cargo metadata --manifest-path "src-tauri\Cargo.toml" --no-deps --format-version 1 }
      }
    }
  } finally {
    Pop-Location
  }
}

Write-Host "[dw] Windows desktop_dev package verified"
"#
}

fn windows_desktop_healthcheck_script() -> &'static str {
    r#"$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PackageRoot = Resolve-Path (Join-Path $ScriptDir "..")
$ProjectsFile = Join-Path $PackageRoot ".dw-desktop-projects"
node --version
npm --version
cargo --version
Get-Content $ProjectsFile | Where-Object { $_.Trim().Length -gt 0 } | ForEach-Object {
  $projectDir = Join-Path $PackageRoot $_
  if (-not (Test-Path $projectDir)) { throw "Project path not found: $_" }
  if ((Test-Path (Join-Path $projectDir "package.json")) -and -not (Test-Path (Join-Path $projectDir "node_modules"))) {
    throw "node_modules not found for $_"
  }
  if ((Test-Path (Join-Path $projectDir "src-tauri\Cargo.toml")) -and -not (Test-Path (Join-Path $projectDir "node_modules\.bin\tauri.cmd"))) {
    throw "Tauri CLI not found for $_"
  }
}
Write-Host "[dw] Windows desktop_dev healthcheck ok"
"#
}

fn windows_desktop_logs_script() -> &'static str {
    r#"$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PackageRoot = Resolve-Path (Join-Path $ScriptDir "..")
$LogDir = Join-Path $PackageRoot ".dw-runbook\logs"
if (-not (Test-Path $LogDir)) {
  Write-Host "[dw] No runbook logs found"
  exit 0
}
Get-ChildItem $LogDir -File | ForEach-Object {
  Write-Host "===== $($_.Name)"
  Get-Content $_.FullName -Tail 160
}
"#
}

fn windows_desktop_stop_script() -> &'static str {
    r#"$ErrorActionPreference = "Stop"
Write-Host "[dw] desktop_dev package has no managed Windows service to stop"
"#
}

fn windows_desktop_run_dev_script() -> &'static str {
    r#"param(
  [string]$ComposeProjectName = $env:DW_COMPOSE_PROJECT_NAME,
  [switch]$PrintTarget
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SharedPackageRoot = (Resolve-Path (Join-Path $ScriptDir "..")).ProviderPath

function Get-ManifestComposeProjectName {
  $manifestPath = Join-Path $SharedPackageRoot "manifest.json"
  if (-not (Test-Path $manifestPath)) {
    return ""
  }
  try {
    $manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
    if ($manifest.compose_project_name) {
      return [string]$manifest.compose_project_name
    }
  } catch {
    Write-Host "[dw] Could not read compose_project_name from manifest.json: $($_.Exception.Message)"
  }
  return ""
}

function Resolve-LocalPackageRoot([string]$ProjectName) {
  if ([string]::IsNullOrWhiteSpace($ProjectName)) {
    return $null
  }
  $localPackageName = $ProjectName -replace "[^A-Za-z0-9_.-]", "_"
  $candidate = Join-Path "C:\dw\deploy" $localPackageName
  if (Test-Path $candidate) {
    return (Resolve-Path $candidate).ProviderPath
  }
  return $null
}

if ([string]::IsNullOrWhiteSpace($ComposeProjectName)) {
  $ComposeProjectName = Get-ManifestComposeProjectName
}

$PackageRoot = Resolve-LocalPackageRoot $ComposeProjectName
if (-not $PackageRoot) {
  $deployScript = Join-Path $ScriptDir "deploy.ps1"
  if (Test-Path $deployScript) {
    Write-Host "[dw] Local deploy copy not found; preparing it with deploy.ps1"
    if ([string]::IsNullOrWhiteSpace($ComposeProjectName)) {
      & $deployScript
    } else {
      & $deployScript -ComposeProjectName $ComposeProjectName
    }
    $PackageRoot = Resolve-LocalPackageRoot $ComposeProjectName
  }
}
if (-not $PackageRoot) {
  Write-Host "[dw] Local deploy copy not found; falling back to shared package"
  $PackageRoot = $SharedPackageRoot
}

$projects = @(Get-Content (Join-Path $PackageRoot ".dw-desktop-projects") | Where-Object { $_.Trim().Length -gt 0 })
if ($projects.Count -ne 1) {
  throw "Select one project and run its dev command manually."
}
$ProjectRoot = Join-Path $PackageRoot $projects[0]
if (-not (Test-Path $ProjectRoot)) {
  throw "Project path not found: $ProjectRoot"
}
if ($PrintTarget) {
  Write-Host $ProjectRoot
  exit 0
}
Set-Location $ProjectRoot
if ((Test-Path "package.json") -and -not (Test-Path "node_modules\.bin\tauri.cmd")) {
  Write-Host "[dw] Tauri CLI not found under node_modules; running npm install"
  npm install
}
npm run dev
"#
}

fn windows_desktop_run_dev_cmd() -> &'static str {
    r#"@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-dev.ps1" %*
"#
}

fn write_agent_plan_scripts(
    root: &Path,
    plan: &serde_json::Value,
    package_strategy: &str,
    compose_mode: &PackageComposeMode,
) -> anyhow::Result<()> {
    let scripts_root = root.join("scripts");
    for (relative_path, body) in deploy_plan::script_artifacts_from_plan(plan)? {
        if protected_runbook_script(&relative_path, package_strategy, compose_mode) {
            continue;
        }
        let path = root.join(&relative_path);
        if !path.starts_with(&scripts_root) {
            anyhow::bail!("deploy_plan_validation_failed: script path must stay under scripts/");
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)?;
    }
    Ok(())
}

fn protected_runbook_script(
    relative_path: &str,
    package_strategy: &str,
    compose_mode: &PackageComposeMode,
) -> bool {
    matches!(
        relative_path,
        "scripts/install-base-linux.sh" | "scripts/install-deploy.ps1"
    ) || (package_strategy == "desktop_dev"
        && deploy_runbook_scripts()
            .into_iter()
            .any(|script| script == relative_path))
        || (compose_mode.uses_source_passthrough()
            && compose_runbook_scripts()
                .into_iter()
                .any(|script| script == relative_path))
}

fn compose_runbook_scripts() -> Vec<&'static str> {
    vec![
        "scripts/preflight.sh",
        "scripts/deploy.sh",
        "scripts/healthcheck.sh",
        "scripts/logs.sh",
        "scripts/stop.sh",
    ]
}

fn write_agent_plan_files(root: &Path, plan: &serde_json::Value) -> anyhow::Result<()> {
    for (relative_path, body) in deploy_plan::file_artifacts_from_plan(plan)? {
        let path = root.join(&relative_path);
        if !path.starts_with(root) || relative_path.starts_with("scripts/") {
            anyhow::bail!("deploy_plan_validation_failed: artifact path escapes package");
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)?;
    }
    Ok(())
}

fn scan_package_review_files(root: &Path, findings: &mut Vec<SecretFinding>) {
    // The root .env.example is generated by ADE from already packaged public inputs.
    // Project .env.example files under projects/*/source are scanned during source copy.
    let paths = [root.join("docker-compose.yml"), root.join("compose.yml")];
    for path in paths {
        if path.is_file() {
            for finding in scan_secret_content(&path, &path, root, root) {
                push_unique_finding(findings, finding);
            }
        }
    }
}

fn push_unique_finding(findings: &mut Vec<SecretFinding>, finding: SecretFinding) {
    if findings.iter().any(|existing| {
        existing.path == finding.path
            && existing.reason == finding.reason
            && existing.marker == finding.marker
            && existing.line_sha256 == finding.line_sha256
            && existing.blocking == finding.blocking
    }) {
        return;
    }
    findings.push(finding);
}

pub fn deploy_runbook_scripts() -> Vec<&'static str> {
    vec![
        "scripts/preflight.sh",
        "scripts/install-base-linux.sh",
        "scripts/prepare-dev-vm.sh",
        "scripts/build-dev.sh",
        "scripts/verify-dev.sh",
        "scripts/run-dev.sh",
        "scripts/deploy.sh",
        "scripts/healthcheck.sh",
        "scripts/logs.sh",
        "scripts/stop.sh",
        "scripts/rollback.sh",
        "scripts/install-deploy.ps1",
    ]
}

pub fn linux_install_base_script() -> &'static str {
    r#"#!/usr/bin/env sh
set -eu

if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    exec sudo DW_HOST_EPOCH="${DW_HOST_EPOCH:-}" DW_SSH_PUBLIC_KEY="${DW_SSH_PUBLIC_KEY:-}" DW_SSH_USER="${DW_SSH_USER:-}" sh "$0" "$@"
  fi
  echo "Run this script as root or install sudo first." >&2
  exit 1
fi

export DEBIAN_FRONTEND=noninteractive

sync_guest_clock() {
  if command -v timedatectl >/dev/null 2>&1; then
    timedatectl set-ntp true >/dev/null 2>&1 || true
  fi
  if command -v systemctl >/dev/null 2>&1; then
    systemctl restart systemd-timesyncd >/dev/null 2>&1 || true
  fi
  if [ -n "${DW_HOST_EPOCH:-}" ] && command -v date >/dev/null 2>&1; then
    date -u -s "@$DW_HOST_EPOCH" >/dev/null 2>&1 || true
  fi
}

apt_log_has_retryable_lock() {
  grep -Eiq 'could not get lock|unable to lock directory|is held by process|waiting for cache lock|dpkg frontend lock|dpkg lock' "$1"
}

apt_get_update() {
  log_file=$(mktemp)
  attempt=1
  while [ "$attempt" -le 90 ]; do
    if apt-get update > "$log_file" 2>&1; then
      cat "$log_file"
      rm -f "$log_file"
      return 0
    fi
    cat "$log_file"
    if grep -qi "not valid yet" "$log_file"; then
      echo "[dw] apt repository metadata is newer than guest clock; syncing clock and retrying ($attempt/90)"
      sync_guest_clock
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    if apt_log_has_retryable_lock "$log_file"; then
      echo "[dw] apt is busy; waiting for package manager lock ($attempt/90)"
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    cat "$log_file" >&2
    rm -f "$log_file"
    return 1
  done
  cat "$log_file" >&2
  rm -f "$log_file"
  return 1
}

apt_install() {
  log_file=$(mktemp)
  attempt=1
  while [ "$attempt" -le 90 ]; do
    if apt-get install -y --no-install-recommends "$@" > "$log_file" 2>&1; then
      cat "$log_file"
      rm -f "$log_file"
      return 0
    fi
    cat "$log_file"
    if apt_log_has_retryable_lock "$log_file"; then
      echo "[dw] apt is busy; waiting for package manager lock ($attempt/90)"
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    cat "$log_file" >&2
    rm -f "$log_file"
    return 1
  done
  cat "$log_file" >&2
  rm -f "$log_file"
  return 1
}

start_service() {
  service_name="$1"
  if command -v systemctl >/dev/null 2>&1; then
    systemctl enable --now "$service_name" >/dev/null 2>&1 && return 0
  fi
  service "$service_name" start >/dev/null 2>&1 && return 0
  return 1
}

echo "[dw] Installing base packages"
sync_guest_clock
apt_get_update
apt_install ca-certificates curl gnupg lsb-release openssh-server rsync

if ! command -v docker >/dev/null 2>&1 || ! docker compose version >/dev/null 2>&1; then
  echo "[dw] Installing Docker Engine and Compose plugin"
  install -m 0755 -d /etc/apt/keyrings
  if [ ! -s /etc/apt/keyrings/docker.asc ]; then
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
    chmod a+r /etc/apt/keyrings/docker.asc
  fi
  . /etc/os-release
  docker_codename="${UBUNTU_CODENAME:-${VERSION_CODENAME:-}}"
  if [ -z "$docker_codename" ]; then
    echo "Could not detect Ubuntu codename for Docker repository." >&2
    exit 1
  fi
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${docker_codename} stable" > /etc/apt/sources.list.d/docker.list
  apt_get_update
  apt_install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
fi

echo "[dw] Starting services"
start_service ssh || start_service sshd || true
start_service docker || true

if [ -n "${SUDO_USER:-}" ] && id "$SUDO_USER" >/dev/null 2>&1; then
  usermod -aG docker "$SUDO_USER" || true
fi

if [ -n "${DW_SSH_PUBLIC_KEY:-}" ]; then
  target_user="${DW_SSH_USER:-${SUDO_USER:-docker}}"
  if ! id "$target_user" >/dev/null 2>&1; then
    useradd -m -s /bin/bash "$target_user"
  fi
  usermod -aG docker "$target_user" || true
  usermod -aG sudo "$target_user" || true
  home_dir=$(getent passwd "$target_user" | cut -d: -f6)
  install -d -m 700 -o "$target_user" -g "$target_user" "$home_dir/.ssh"
  touch "$home_dir/.ssh/authorized_keys"
  if ! grep -qxF "$DW_SSH_PUBLIC_KEY" "$home_dir/.ssh/authorized_keys"; then
    printf '%s\n' "$DW_SSH_PUBLIC_KEY" >> "$home_dir/.ssh/authorized_keys"
  fi
  chown "$target_user:$target_user" "$home_dir/.ssh/authorized_keys"
  chmod 600 "$home_dir/.ssh/authorized_keys"
  echo "$target_user ALL=(ALL) NOPASSWD:ALL" > "/etc/sudoers.d/dw-$target_user"
  chmod 440 "/etc/sudoers.d/dw-$target_user"
fi

echo "[dw] Verifying runtime"
docker --version
docker compose version
if command -v ss >/dev/null 2>&1; then
  ss -ltn | grep ':22 ' >/dev/null 2>&1 || echo "[dw] Warning: SSH service is not listening on port 22 yet."
fi

echo "[dw] Linux target base dependencies installed."
echo "[dw] If this is the first Docker install for your user, log out and back in or reboot before retrying ADE prepare."
"#
}

pub fn compose_project_name(stack_slug: &str, label: &str) -> String {
    format!(
        "dw_{}_{}",
        stack_slug.replace('-', "_"),
        label.replace('-', "_")
    )
}

fn slugify(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "deploy-stack".to_string()
    } else {
        slug
    }
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_status_short(root: &Path) -> String {
    git_output(root, &["status", "--short"]).unwrap_or_default()
}

fn scoped_existing_child(root: &Path, relative_path: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("deploy artifact path escapes package root");
    }
    let root = std::fs::canonicalize(root)?;
    let child = std::fs::canonicalize(root.join(relative))?;
    if !child.starts_with(&root) {
        anyhow::bail!("deploy artifact path escapes package root");
    }
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_secret_files_as_warnings_and_blocks_runtime_secret_content() {
        let root = temp_root("source");
        std::fs::write(root.join("package.json"), "{}").expect("package");
        std::fs::write(root.join(".env"), "PASSWORD=secret").expect("env");
        std::fs::write(root.join("config.txt"), "api_key=secret").expect("config");
        let dest = temp_root("dest");
        let mut findings = Vec::new();
        copy_source_snapshot(&root, &dest, &mut findings).expect("copy");
        assert!(dest.join("package.json").exists());
        assert!(!dest.join(".env").exists());
        assert!(findings.iter().any(|finding| {
            finding.path.contains(".env") && finding.severity == "warning" && !finding.blocking
        }));
        assert!(findings.iter().any(|finding| {
            finding.path.contains(".env")
                && finding
                    .hint
                    .as_deref()
                    .unwrap_or("")
                    .contains("UI de ambiente")
        }));
        assert!(findings.iter().any(|finding| {
            finding.path.contains("config.txt") && finding.severity == "error" && finding.blocking
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
        std::fs::remove_dir_all(dest).expect("cleanup");
    }

    #[test]
    fn placeholder_secret_markers_do_not_create_findings() {
        let root = temp_root("placeholder-source");
        std::fs::create_dir_all(root.join("templates")).expect("templates");
        std::fs::write(root.join("package.json"), "{}").expect("package");
        std::fs::write(
            root.join("templates/install.rs"),
            "PASSWORD={password}\nAPI_KEY=<api-key>\nSECRET=${APP_SECRET}\n",
        )
        .expect("template");
        let dest = temp_root("placeholder-dest");
        let mut findings = Vec::new();
        copy_source_snapshot(&root, &dest, &mut findings).expect("copy");
        assert!(dest.join("templates/install.rs").exists());
        assert!(findings.is_empty());
        std::fs::remove_dir_all(root).expect("cleanup");
        std::fs::remove_dir_all(dest).expect("cleanup");
    }

    #[test]
    fn review_scanner_allows_real_lettrebox_false_positives_and_blocks_real_secrets() {
        let root = temp_root("lettrebox-review-source");
        std::fs::create_dir_all(root.join("crates/mail-driver/src/stalwart")).expect("src");
        std::fs::write(root.join("package.json"), "{}").expect("package");
        std::fs::write(
            root.join("crates/mail-driver/src/stalwart/client.rs"),
            r#"let header = format!("Bearer {token}");"#,
        )
        .expect("client");
        std::fs::write(
            root.join(".env.example"),
            "SMTP_HOST=\nSMTP_USERNAME=\nSMTP_PASSWORD=\nSMTP_SECRET=${SMTP_SECRET}\n",
        )
        .expect("env example");
        let dest = temp_root("lettrebox-review-dest");
        let mut findings = Vec::new();
        copy_source_snapshot(&root, &dest, &mut findings).expect("copy");
        assert!(findings.is_empty(), "{findings:?}");

        std::fs::write(root.join("config.txt"), "password=hunter2\n").expect("real password");
        std::fs::write(
            root.join("auth.txt"),
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9\n",
        )
        .expect("real bearer");
        let blocked_dest = temp_root("lettrebox-review-blocked-dest");
        let mut blocked_findings = Vec::new();
        copy_source_snapshot(&root, &blocked_dest, &mut blocked_findings).expect("copy blocked");
        assert!(blocked_findings.iter().any(|finding| {
            finding.path.contains("config.txt")
                && finding.blocking
                && finding.reason.contains("password=")
                && finding
                    .hint
                    .as_deref()
                    .unwrap_or("")
                    .contains("segredo real")
        }));
        assert!(blocked_findings.iter().any(|finding| {
            finding.path.contains("auth.txt")
                && finding.blocking
                && finding.reason.contains("bearer ")
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
        std::fs::remove_dir_all(dest).expect("cleanup");
        std::fs::remove_dir_all(blocked_dest).expect("cleanup");
    }

    #[test]
    fn package_scanner_blocks_later_runtime_secret_after_placeholder_markers() {
        let root = temp_root("multi-secret-source");
        std::fs::write(
            root.join(".env.example"),
            "SMTP_HOST=\nSMTP_USERNAME=\nSMTP_PASSWORD=\nSMTP_SECRET=${SMTP_SECRET}\nADMIN_PASSWORD=hunter2\n",
        )
        .expect("env example");
        std::fs::write(
            root.join("auth.txt"),
            "let header = format!(\"Bearer {token}\");\nAuthorization: Bearer eyJhbGciOiJIUzI1NiJ9\n",
        )
        .expect("auth");
        let dest = temp_root("multi-secret-dest");
        let mut findings = Vec::new();
        copy_source_snapshot(&root, &dest, &mut findings).expect("copy");
        assert!(findings.iter().any(|finding| {
            finding.path.contains(".env.example")
                && finding.blocking
                && finding.severity == "error"
                && finding.reason.contains("password=")
        }));
        assert!(findings.iter().any(|finding| {
            finding.path.contains("auth.txt")
                && finding.blocking
                && finding.severity == "error"
                && finding.reason.contains("bearer ")
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
        std::fs::remove_dir_all(dest).expect("cleanup");
    }

    #[test]
    fn minimal_package_excludes_non_runtime_test_assets() {
        let root = temp_root("minimal-source");
        std::fs::create_dir_all(root.join("tests/e2e/fixtures")).expect("fixtures");
        std::fs::write(root.join("package.json"), "{}").expect("package");
        std::fs::write(root.join("tests/e2e/fixtures/seed.sh"), "PASSWORD=test\n")
            .expect("fixture");
        let dest = temp_root("minimal-dest");
        let mut findings = Vec::new();
        copy_source_snapshot(&root, &dest, &mut findings).expect("copy");
        assert!(dest.join("package.json").exists());
        assert!(!dest.join("tests").exists());
        assert!(findings.is_empty());
        std::fs::remove_dir_all(root).expect("cleanup");
        std::fs::remove_dir_all(dest).expect("cleanup");
    }

    #[test]
    fn create_package_blocks_runtime_secret_when_stack_slug_is_non_runtime_name() {
        let (root, version, findings) =
            create_secret_scan_package("stackslug", "Test", "Web", |project_root| {
                let runtime_path = project_root.join("src/config.txt");
                std::fs::create_dir_all(runtime_path.parent().expect("parent")).expect("src");
                std::fs::write(runtime_path, "password=hunter2\n").expect("runtime config");
            });

        assert!(has_blocking_findings(&version), "{findings:?}");
        assert!(findings.iter().any(|finding| {
            finding.path.contains("projects/web/source/src/config.txt")
                && finding.severity == "error"
                && finding.blocking
                && finding.reason.contains("password=")
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn create_package_downgrades_mixed_case_tests_secret_to_warning() {
        let (root, version, findings) =
            create_secret_scan_package("mixedcase", "Web deploy", "Web", |project_root| {
                let fixture_path = project_root.join("Tests/seed.sh");
                std::fs::create_dir_all(fixture_path.parent().expect("parent")).expect("fixtures");
                std::fs::write(fixture_path, "password=hunter2\n").expect("fixture");
            });

        assert!(!has_blocking_findings(&version), "{findings:?}");
        assert!(findings.iter().any(|finding| {
            finding.path.contains("projects/web/source/Tests/seed.sh")
                && finding.severity == "warning"
                && !finding.blocking
                && finding.reason.contains("password=")
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn create_package_blocks_runtime_secret_when_project_slug_is_non_runtime_name() {
        let (root, version, findings) =
            create_secret_scan_package("projectslug", "Web deploy", "Docs", |project_root| {
                let runtime_path = project_root.join("src/config.txt");
                std::fs::create_dir_all(runtime_path.parent().expect("parent")).expect("src");
                std::fs::write(runtime_path, "password=hunter2\n").expect("runtime config");
            });

        assert!(has_blocking_findings(&version), "{findings:?}");
        assert!(findings.iter().any(|finding| {
            finding.path.contains("projects/docs/source/src/config.txt")
                && finding.severity == "error"
                && finding.blocking
                && finding.reason.contains("password=")
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn blocking_finding_parser_keeps_legacy_payloads_blocking() {
        let mut version = store::DeployVersion {
            id: "v".to_string(),
            stack_id: "s".to_string(),
            workspace_id: 1,
            label: "deploy-001".to_string(),
            status: "review_required".to_string(),
            target_machine_id: None,
            artifact_path: "/tmp/package".to_string(),
            manifest_path: "/tmp/package/manifest.json".to_string(),
            manifest_json: "{}".to_string(),
            review_status: "pending".to_string(),
            reviewed_at: None,
            blocking_findings_json: r#"[{"path":".env","reason":"legacy"}]"#.to_string(),
            dismissed_findings_json: "[]".to_string(),
            review_audit_json: "[]".to_string(),
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        assert!(has_blocking_findings(&version));
        version.blocking_findings_json =
            r#"[{"path":".env","reason":"excluded","severity":"warning","blocking":false}]"#
                .to_string();
        assert!(!has_blocking_findings(&version));
    }

    #[test]
    fn dismissed_review_finding_allows_approval() {
        let fixture = review_flow_fixture(
            "dismiss-allows-approval",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let version = create_review_flow_package(&fixture);
        let finding = only_active_blocking_finding(&version);

        assert!(crate::deploy::approve_version(
            &fixture.db,
            crate::deploy::ApproveDeployVersionInput {
                version_id: version.id.clone(),
            },
        )
        .is_err());

        let dismissed = dismiss_review_finding(
            &fixture.db,
            &version.id,
            &finding.path,
            &finding.reason,
            finding.marker.as_deref(),
            finding.line_sha256.as_deref(),
            "owner accepted fake test token",
        )
        .expect("dismiss finding");
        assert!(!has_blocking_findings(&dismissed));
        assert_eq!(
            parse_review_audit_events_lossy(&dismissed.review_audit_json).len(),
            1
        );

        let approved = crate::deploy::approve_version(
            &fixture.db,
            crate::deploy::ApproveDeployVersionInput {
                version_id: version.id,
            },
        )
        .expect("approve dismissed version");
        assert_eq!(approved.review_status, "approved");
        let blocked_after_approval = dismiss_review_finding(
            &fixture.db,
            &approved.id,
            &finding.path,
            &finding.reason,
            finding.marker.as_deref(),
            finding.line_sha256.as_deref(),
            "owner accepted fake test token again",
        )
        .expect_err("approved versions cannot dismiss");
        assert!(blocked_after_approval
            .to_string()
            .contains("deploy_review_not_pending"));
        let restore_after_approval = restore_review_finding(
            &fixture.db,
            &approved.id,
            &finding.path,
            &finding.reason,
            finding.marker.as_deref(),
            finding.line_sha256.as_deref(),
        )
        .expect_err("approved versions cannot restore");
        assert!(restore_after_approval
            .to_string()
            .contains("deploy_review_not_pending"));
        std::fs::remove_dir_all(fixture.root).expect("cleanup");
    }

    #[test]
    fn dismissed_review_finding_is_inherited_by_content_and_blocks_new_occurrences() {
        let fixture = review_flow_fixture(
            "dismiss-inherit",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let first = create_review_flow_package(&fixture);
        let first_finding = only_active_blocking_finding(&first);
        dismiss_generated_finding(&fixture.db, &first, &first_finding);

        let second = create_review_flow_package(&fixture);
        assert!(!has_blocking_findings(&second));
        let second_dismissed = parse_dismissed_findings_lossy(&second.dismissed_findings_json);
        assert_eq!(second_dismissed.len(), 1);
        assert_eq!(second_dismissed[0].path, first_finding.path);
        assert_eq!(second_dismissed[0].line_sha256, first_finding.line_sha256);
        assert_eq!(
            second_dismissed[0].inherited_from_label.as_deref(),
            Some("deploy-001")
        );

        std::fs::write(
            fixture.secret_path(),
            "const fake = \"Bearer fake-token-0000\";\nconst real = \"Bearer live-token-123456\";\n",
        )
        .expect("add real token");
        let third = create_review_flow_package(&fixture);
        let third_findings =
            parse_review_findings(&third.blocking_findings_json).expect("third findings");
        assert_eq!(
            third_findings
                .iter()
                .filter(|finding| finding.blocking)
                .count(),
            2
        );
        let third_active = active_blocking_findings(&third).expect("third active findings");
        assert_eq!(third_active.len(), 1, "{third_active:?}");
        assert_eq!(third_active[0].line_number, Some(2));
        assert_ne!(third_active[0].line_sha256, first_finding.line_sha256);
        let third_dismissed = parse_dismissed_findings_lossy(&third.dismissed_findings_json);
        assert_eq!(third_dismissed.len(), 1);
        assert_eq!(third_dismissed[0].line_sha256, first_finding.line_sha256);
        std::fs::remove_dir_all(fixture.root).expect("cleanup");
    }

    #[test]
    fn dismissed_review_finding_is_not_inherited_when_path_or_line_changes() {
        let changed_line = review_flow_fixture(
            "dismiss-changed-line",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let first = create_review_flow_package(&changed_line);
        let first_finding = only_active_blocking_finding(&first);
        dismiss_generated_finding(&changed_line.db, &first, &first_finding);
        std::fs::write(
            changed_line.secret_path(),
            "const fake = \"Bearer changed-token-9999\";\n",
        )
        .expect("change dismissed line");
        let second = create_review_flow_package(&changed_line);
        assert!(has_blocking_findings(&second));
        assert!(parse_dismissed_findings_lossy(&second.dismissed_findings_json).is_empty());
        std::fs::remove_dir_all(changed_line.root).expect("cleanup");

        let renamed = review_flow_fixture(
            "dismiss-renamed-path",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let first = create_review_flow_package(&renamed);
        let first_finding = only_active_blocking_finding(&first);
        dismiss_generated_finding(&renamed.db, &first, &first_finding);
        let renamed_path = renamed.project_root.join("src/renamed.txt");
        std::fs::rename(renamed.secret_path(), &renamed_path).expect("rename dismissed file");
        let second = create_review_flow_package(&renamed);
        let active = active_blocking_findings(&second).expect("renamed active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].path, "projects/app/source/src/renamed.txt");
        assert!(parse_dismissed_findings_lossy(&second.dismissed_findings_json).is_empty());
        std::fs::remove_dir_all(renamed.root).expect("cleanup");
    }

    #[test]
    fn restored_review_finding_tombstone_blocks_future_inheritance() {
        let fixture = review_flow_fixture(
            "dismiss-restore",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let first = create_review_flow_package(&fixture);
        let first_finding = only_active_blocking_finding(&first);
        dismiss_generated_finding(&fixture.db, &first, &first_finding);

        let second = create_review_flow_package(&fixture);
        assert!(!has_blocking_findings(&second));
        let third = create_review_flow_package(&fixture);
        let third_dismissed = parse_dismissed_findings_lossy(&third.dismissed_findings_json);
        assert_eq!(
            third_dismissed[0].inherited_from_label.as_deref(),
            Some("deploy-001")
        );
        let restored = restore_review_finding(
            &fixture.db,
            &third.id,
            &first_finding.path,
            &first_finding.reason,
            first_finding.marker.as_deref(),
            first_finding.line_sha256.as_deref(),
        )
        .expect("restore finding");

        assert!(has_blocking_findings(&restored));
        assert!(restore_review_finding(
            &fixture.db,
            &third.id,
            &first_finding.path,
            &first_finding.reason,
            first_finding.marker.as_deref(),
            first_finding.line_sha256.as_deref(),
        )
        .is_err());
        let audit = parse_review_audit_events_lossy(&restored.review_audit_json);
        assert_eq!(
            audit.last().map(|event| event.action.as_str()),
            Some("restore")
        );

        let fourth = create_review_flow_package(&fixture);
        assert!(has_blocking_findings(&fourth));
        assert!(parse_dismissed_findings_lossy(&fourth.dismissed_findings_json).is_empty());
        std::fs::remove_dir_all(fixture.root).expect("cleanup");
    }

    #[test]
    fn restored_older_version_tombstone_wins_over_newer_inherited_carriers() {
        let fixture = review_flow_fixture(
            "dismiss-restore-old-version",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let first = create_review_flow_package(&fixture);
        let first_finding = only_active_blocking_finding(&first);
        dismiss_generated_finding(&fixture.db, &first, &first_finding);

        let second = create_review_flow_package(&fixture);
        assert!(!has_blocking_findings(&second));

        let restored_first = restore_review_finding(
            &fixture.db,
            &first.id,
            &first_finding.path,
            &first_finding.reason,
            first_finding.marker.as_deref(),
            first_finding.line_sha256.as_deref(),
        )
        .expect("restore original finding");
        assert!(has_blocking_findings(&restored_first));

        let third = create_review_flow_package(&fixture);
        assert!(has_blocking_findings(&third));
        assert!(parse_dismissed_findings_lossy(&third.dismissed_findings_json).is_empty());

        let third_finding = only_active_blocking_finding(&third);
        dismiss_generated_finding(&fixture.db, &third, &third_finding);
        let fourth = create_review_flow_package(&fixture);
        assert!(!has_blocking_findings(&fourth));
        let fourth_dismissed = parse_dismissed_findings_lossy(&fourth.dismissed_findings_json);
        assert_eq!(
            fourth_dismissed[0].inherited_from_label.as_deref(),
            Some("deploy-003")
        );
        std::fs::remove_dir_all(fixture.root).expect("cleanup");
    }

    #[test]
    fn identical_duplicate_content_findings_are_dismissed_and_restored_as_a_batch() {
        let fixture = review_flow_fixture(
            "dismiss-duplicate-lines",
            "const token = \"Bearer fake-token-0000\";\nconst token = \"Bearer fake-token-0000\";\n",
        );
        let version = create_review_flow_package(&fixture);
        let active = active_blocking_findings(&version).expect("active duplicate findings");
        assert_eq!(active.len(), 2, "{active:?}");
        assert_eq!(active[0].line_sha256, active[1].line_sha256);
        assert_eq!(active[0].occurrence_count, Some(2));
        assert_eq!(active[1].occurrence_count, Some(2));
        assert_eq!(active[0].occurrence_index, Some(1));
        assert_eq!(active[1].occurrence_index, Some(2));

        let dismissed = dismiss_review_finding(
            &fixture.db,
            &version.id,
            &active[0].path,
            &active[0].reason,
            active[0].marker.as_deref(),
            active[0].line_sha256.as_deref(),
            "owner accepted duplicate fake tokens",
        )
        .expect("dismiss duplicate findings");
        assert!(!has_blocking_findings(&dismissed));
        assert_eq!(
            parse_dismissed_findings_lossy(&dismissed.dismissed_findings_json).len(),
            1
        );

        let restored = restore_review_finding(
            &fixture.db,
            &version.id,
            &active[1].path,
            &active[1].reason,
            active[1].marker.as_deref(),
            active[1].line_sha256.as_deref(),
        )
        .expect("restore duplicate findings");
        let restored_active = active_blocking_findings(&restored).expect("restored active");
        assert_eq!(restored_active.len(), 2, "{restored_active:?}");
        std::fs::remove_dir_all(fixture.root).expect("cleanup");
    }

    #[test]
    fn secret_content_identity_without_hash_does_not_fall_back_to_path_reason() {
        let finding = SecretFinding::secret_content_blocking(
            "projects/app/source/src/config.txt".to_string(),
            "secret-like content marker `Bearer `",
            "Bearer ",
            sha256_hex(b"const fake = \"Bearer fake-token-0000\";"),
            1,
        );
        let legacy_secret_dismissal = DismissedReviewFinding {
            path: finding.path.clone(),
            reason: finding.reason.clone(),
            marker: None,
            line_sha256: None,
            line_number: None,
            justification: "legacy accepted fake token".to_string(),
            dismissed_at: "2026-07-13T12:00:00Z".to_string(),
            inherited_from_version_id: None,
            inherited_from_label: None,
        };
        assert!(select_blocking_finding(
            std::slice::from_ref(&finding),
            &finding.path,
            &finding.reason,
            None,
            None
        )
        .is_err());
        assert!(!finding_is_dismissed(&finding, &[legacy_secret_dismissal]));

        let strategy = SecretFinding::blocking(
            "projects/app/source".to_string(),
            "unsupported deploy strategy: unknown project shape",
        );
        let strategy_dismissal = DismissedReviewFinding {
            path: strategy.path.clone(),
            reason: strategy.reason.clone(),
            marker: None,
            line_sha256: None,
            line_number: None,
            justification: "owner accepts unsupported strategy".to_string(),
            dismissed_at: "2026-07-13T12:00:00Z".to_string(),
            inherited_from_version_id: None,
            inherited_from_label: None,
        };
        assert!(select_blocking_finding(
            std::slice::from_ref(&strategy),
            &strategy.path,
            &strategy.reason,
            None,
            None
        )
        .is_ok());
        assert!(finding_is_dismissed(&strategy, &[strategy_dismissal]));
    }

    #[test]
    fn unsupported_strategy_review_finding_uses_package_relative_path() {
        let root = temp_root("unsupported-strategy-path");
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join("mystery");
        std::fs::create_dir_all(&project_root).expect("project root");
        std::fs::write(project_root.join("notes.txt"), "no deploy runtime here\n")
            .expect("project note");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "Mystery",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");
        let version = create_package(
            &db,
            CreateDeployPackageInput {
                workspace_id: workspace.id,
                stack_name: "Mystery deploy".to_string(),
                project_ids: vec![project.id],
                target_machine_id: None,
                agent_profile_id: agent.id,
                deploy_plan_path: Some(write_test_plan(&workspace_root, project.id, "unsupported")),
                include_dirty: true,
            },
        )
        .expect("create unsupported package");
        let findings = parse_review_findings(&version.blocking_findings_json).expect("findings");
        let finding = findings
            .iter()
            .find(|finding| finding.reason.starts_with("unsupported deploy strategy:"))
            .expect("unsupported finding");

        assert_eq!(finding.path, "projects/mystery/source");
        assert!(!Path::new(&finding.path).is_absolute());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn review_finding_audit_is_append_only() {
        let fixture = review_flow_fixture(
            "dismiss-audit",
            "const fake = \"Bearer fake-token-0000\";\n",
        );
        let version = create_review_flow_package(&fixture);
        let finding = only_active_blocking_finding(&version);
        let first = dismiss_generated_finding(&fixture.db, &version, &finding);
        let second = dismiss_review_finding(
            &fixture.db,
            &version.id,
            &finding.path,
            &finding.reason,
            finding.marker.as_deref(),
            finding.line_sha256.as_deref(),
            "owner accepted fake test token twice",
        )
        .expect("dismiss duplicate");
        assert_eq!(
            parse_review_audit_events_lossy(&first.review_audit_json).len(),
            1
        );
        let audit = parse_review_audit_events_lossy(&second.review_audit_json);
        assert_eq!(audit.len(), 2);
        assert!(audit.iter().all(|event| event.action == "dismiss"));
        assert_eq!(
            parse_dismissed_findings_lossy(&second.dismissed_findings_json).len(),
            1
        );

        let restored = restore_review_finding(
            &fixture.db,
            &version.id,
            &finding.path,
            &finding.reason,
            finding.marker.as_deref(),
            finding.line_sha256.as_deref(),
        )
        .expect("restore");
        let audit = parse_review_audit_events_lossy(&restored.review_audit_json);
        assert_eq!(audit.len(), 3);
        assert_eq!(
            audit.last().map(|event| event.action.as_str()),
            Some("restore")
        );
        assert!(parse_dismissed_findings_lossy(&restored.dismissed_findings_json).is_empty());
        std::fs::remove_dir_all(fixture.root).expect("cleanup");
    }

    #[test]
    fn artifact_reader_rejects_path_escape() {
        let root = temp_root("artifact");
        std::fs::write(root.join("manifest.json"), "{}").expect("manifest");
        let version = store::DeployVersion {
            id: "v".to_string(),
            stack_id: "s".to_string(),
            workspace_id: 1,
            label: "deploy-001".to_string(),
            status: "review_required".to_string(),
            target_machine_id: None,
            artifact_path: root.display().to_string(),
            manifest_path: root.join("manifest.json").display().to_string(),
            manifest_json: "{}".to_string(),
            review_status: "pending".to_string(),
            reviewed_at: None,
            blocking_findings_json: "[]".to_string(),
            dismissed_findings_json: "[]".to_string(),
            review_audit_json: "[]".to_string(),
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        assert!(read_artifact(&version, "manifest.json").is_ok());
        assert!(read_artifact(&version, "../manifest.json").is_err());
        std::fs::write(root.join(".env"), "SECRET=value").expect("env");
        assert!(read_artifact(&version, ".env").is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn generated_dockerfiles_do_not_copy_optional_globs() {
        let root = temp_root("dockerfile");
        let node_path = root.join("Node.Dockerfile");
        write_generated_dockerfile(
            &node_path,
            &DeployProjectDetection {
                project_id: 1,
                name: "web".to_string(),
                path: root.display().to_string(),
                language: "typescript".to_string(),
                framework: Some("vite".to_string()),
                package_manager: Some("npm".to_string()),
                has_dockerfile: false,
                has_compose: false,
                compose_path: None,
                services: vec![],
                ports: vec![deploy_detect::DeployPortSuggestion {
                    container: 3000,
                    host: 3000,
                    confidence: "default".to_string(),
                }],
                healthcheck: None,
                deploy_strategy: "web_service".to_string(),
                strategy_reason: "test web service".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
        )
        .expect("write node dockerfile");
        let content = std::fs::read_to_string(&node_path).expect("read node dockerfile");
        assert!(content.contains("COPY . ."));
        assert!(!content.contains("pnpm-lock.yaml*"));

        let python_path = root.join("Python.Dockerfile");
        write_generated_dockerfile(
            &python_path,
            &DeployProjectDetection {
                project_id: 2,
                name: "api".to_string(),
                path: root.display().to_string(),
                language: "python".to_string(),
                framework: Some("fastapi".to_string()),
                package_manager: Some("pip".to_string()),
                has_dockerfile: false,
                has_compose: false,
                compose_path: None,
                services: vec![],
                ports: vec![deploy_detect::DeployPortSuggestion {
                    container: 8000,
                    host: 8000,
                    confidence: "default".to_string(),
                }],
                healthcheck: None,
                deploy_strategy: "web_service".to_string(),
                strategy_reason: "test web service".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
        )
        .expect("write python dockerfile");
        let content = std::fs::read_to_string(&python_path).expect("read python dockerfile");
        assert!(content.contains("COPY . ."));
        assert!(!content.contains("requirements*.txt"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn generated_compose_uses_runtime_env_file() {
        let root = temp_root("compose");
        let project = PackagedProject {
            project: store::Project {
                id: 1,
                workspace_id: 1,
                name: "web".to_string(),
                path: root.display().to_string(),
                remote_url: None,
                parent_project_id: None,
                is_submodule: false,
                submodule_path: None,
                created_at: "now".to_string(),
            },
            detection: DeployProjectDetection {
                project_id: 1,
                name: "web".to_string(),
                path: root.display().to_string(),
                language: "typescript".to_string(),
                framework: Some("vite".to_string()),
                package_manager: Some("npm".to_string()),
                has_dockerfile: false,
                has_compose: false,
                compose_path: None,
                services: vec![],
                ports: vec![deploy_detect::DeployPortSuggestion {
                    container: 3000,
                    host: 3000,
                    confidence: "default".to_string(),
                }],
                healthcheck: None,
                deploy_strategy: "web_service".to_string(),
                strategy_reason: "test web service".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
            branch: None,
            commit_sha: None,
            dirty: false,
            git_status_short: String::new(),
            package_path: "projects/web/source".to_string(),
            dockerfile_path: "projects/web/Dockerfile".to_string(),
        };
        let compose = root.join("docker-compose.yml");
        write_compose(&compose, &[project]).expect("compose");
        let content = std::fs::read_to_string(compose).expect("read compose");
        assert!(content.contains("env_file: .env"));
        assert!(!content.contains(".env.example"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn custom_compose_prefers_prod_dockerfile_from_source_contract() {
        let root = temp_root("compose-prod-dockerfile");
        std::fs::write(root.join("Dockerfile.dev"), "FROM alpine\n").expect("dev dockerfile");
        std::fs::write(root.join("Dockerfile.prod"), "FROM alpine\n").expect("prod dockerfile");
        let project = PackagedProject {
            project: store::Project {
                id: 1,
                workspace_id: 1,
                name: "api".to_string(),
                path: root.display().to_string(),
                remote_url: None,
                parent_project_id: None,
                is_submodule: false,
                submodule_path: None,
                created_at: "now".to_string(),
            },
            detection: DeployProjectDetection {
                project_id: 1,
                name: "api".to_string(),
                path: root.display().to_string(),
                language: "rust".to_string(),
                framework: None,
                package_manager: Some("cargo".to_string()),
                has_dockerfile: true,
                has_compose: true,
                compose_path: Some("docker-compose.yml".to_string()),
                services: vec![],
                ports: vec![deploy_detect::DeployPortSuggestion {
                    container: 8080,
                    host: 8080,
                    confidence: "default".to_string(),
                }],
                healthcheck: None,
                deploy_strategy: "custom_compose".to_string(),
                strategy_reason: "custom docker contract".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
            branch: None,
            commit_sha: None,
            dirty: false,
            git_status_short: String::new(),
            package_path: "projects/api/source".to_string(),
            dockerfile_path: "projects/api/Dockerfile".to_string(),
        };
        let compose = root.join("docker-compose.yml");
        write_compose(&compose, &[project]).expect("compose");
        let content = std::fs::read_to_string(compose).expect("read compose");
        assert!(content.contains("dockerfile: Dockerfile.prod"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn env_example_merges_plan_required_and_optional_keys() {
        let root = temp_root("env-example");
        let plan = serde_json::json!({
            "env": {
                "required": ["APP_SECRET", "DATABASE_URL"],
                "optional": ["RUST_LOG", "APP_SECRET", "REDIS_URL"]
            }
        });
        write_env_example(
            &root.join(".env.example"),
            &[
                deploy_detect::DeployServiceSuggestion {
                    name: "postgres".to_string(),
                    reason: "detected pg".to_string(),
                },
                deploy_detect::DeployServiceSuggestion {
                    name: "redis".to_string(),
                    reason: "detected redis".to_string(),
                },
            ],
            &plan,
            false,
            &BTreeMap::from([(
                "DATABASE_URL".to_string(),
                "postgres://user:password@postgres:5432/app".to_string(),
            )]),
        )
        .expect("env example");
        let content = std::fs::read_to_string(root.join(".env.example")).expect("read env");
        assert!(content.contains("\nAPP_SECRET=\n"));
        assert!(content.contains("\nDATABASE_URL=postgres://user:password@postgres:5432/app\n"));
        assert!(content.contains("\nREDIS_URL=\n"));
        assert!(content.contains("\n#RUST_LOG=\n"));
        assert_eq!(content.matches("APP_SECRET=").count(), 1);
        assert_eq!(content.matches("DATABASE_URL=").count(), 1);
        assert_eq!(content.matches("REDIS_URL=").count(), 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn env_example_marks_detector_keys_optional_with_agent_compose() {
        let root = temp_root("env-example-compose");
        let plan = serde_json::json!({
            "env": {
                "required": ["APP_SECRET"],
                "optional": ["RUST_LOG"]
            },
            "artifacts": {
                "compose": {
                    "path": "docker-compose.yml",
                    "body": "services:\n  app:\n    image: app\n"
                }
            }
        });
        assert!(plan_has_compose_artifact(&plan));
        write_env_example(
            &root.join(".env.example"),
            &[
                deploy_detect::DeployServiceSuggestion {
                    name: "postgres".to_string(),
                    reason: "detected pg".to_string(),
                },
                deploy_detect::DeployServiceSuggestion {
                    name: "smtp".to_string(),
                    reason: "detected smtp".to_string(),
                },
            ],
            &plan,
            true,
            &BTreeMap::new(),
        )
        .expect("env example");
        let content = std::fs::read_to_string(root.join(".env.example")).expect("read env");
        assert!(content.contains("\nAPP_SECRET=\n"));
        assert!(content.contains("\n#DATABASE_URL=\n"));
        assert!(content.contains("\n#SMTP_URL=\n"));
        assert!(content.contains("\n#RUST_LOG=\n"));
        assert!(!content.contains("\nDATABASE_URL=postgres://"));
        assert!(!content.contains("\nSMTP_URL=smtp://"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn env_example_prefills_project_defaults_before_empty_values() {
        let root = temp_root("env-example-defaults");
        let plan = serde_json::json!({
            "env": {
                "required": ["POSTGRES_PASSWORD", "SMTP_HOST", "API_KEY"],
                "optional": ["LOG_LEVEL"]
            }
        });
        write_env_example(
            &root.join(".env.example"),
            &[],
            &plan,
            false,
            &BTreeMap::from([
                ("LOG_LEVEL".to_string(), "debug".to_string()),
                ("POSTGRES_PASSWORD".to_string(), "rwfw".to_string()),
                ("SMTP_HOST".to_string(), "mailhog".to_string()),
            ]),
        )
        .expect("env example");

        let content = std::fs::read_to_string(root.join(".env.example")).expect("read env");
        assert!(content.contains("\nPOSTGRES_PASSWORD=rwfw\n"));
        assert!(content.contains("\nSMTP_HOST=mailhog\n"));
        assert!(content.contains("\nAPI_KEY=\n"));
        assert!(content.contains("\n#LOG_LEVEL=debug\n"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_env_defaults_prefer_compose_interpolation_over_env_example() {
        let root = temp_root("project-defaults");
        std::fs::write(
            root.join("docker-compose.yml"),
            "services:\n  db:\n    image: postgres\n    environment:\n      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-rwfw}\n      SMTP_HOST: ${SMTP_HOST:-mailhog}\n",
        )
        .expect("compose");
        std::fs::write(
            root.join(".env.example"),
            "POSTGRES_PASSWORD=from-example\nAPI_KEY=public-test-key\n#COMMENTED_DEFAULT=ignored\n",
        )
        .expect("env example");
        let project = PackagedProject {
            project: store::Project {
                id: 1,
                workspace_id: 1,
                name: "lettrebox".to_string(),
                path: root.display().to_string(),
                remote_url: None,
                parent_project_id: None,
                is_submodule: false,
                submodule_path: None,
                created_at: "now".to_string(),
            },
            detection: DeployProjectDetection {
                project_id: 1,
                name: "lettrebox".to_string(),
                path: root.display().to_string(),
                language: "typescript".to_string(),
                framework: None,
                package_manager: Some("pnpm".to_string()),
                has_dockerfile: false,
                has_compose: true,
                compose_path: Some("docker-compose.yml".to_string()),
                services: vec![],
                ports: vec![],
                healthcheck: None,
                deploy_strategy: "custom_compose".to_string(),
                strategy_reason: "custom compose".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
            branch: None,
            commit_sha: None,
            dirty: false,
            git_status_short: String::new(),
            package_path: "projects/lettrebox/source".to_string(),
            dockerfile_path: "projects/lettrebox/Dockerfile".to_string(),
        };

        let defaults = collect_project_env_defaults(&[project]);
        assert_eq!(defaults.get("POSTGRES_PASSWORD"), Some(&"rwfw".to_string()));
        assert_eq!(defaults.get("SMTP_HOST"), Some(&"mailhog".to_string()));
        assert_eq!(
            defaults.get("API_KEY"),
            Some(&"public-test-key".to_string())
        );
        assert_eq!(defaults.get("COMMENTED_DEFAULT"), None);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn compose_env_defaults_ignore_comments_escapes_and_parse_compose_forms() {
        let defaults = parse_compose_env_defaults(
            r#"
# COMMENTED=${COMMENTED:-ignored}
services:
  app:
    environment:
      DASH_DEFAULT: ${DASH_DEFAULT-fallback}
      COLON_DEFAULT: ${COLON_DEFAULT:-fallback}
      ESCAPED: $${ESCAPED:-ignored}
      JSON_DEFAULT: ${JSON_DEFAULT:-{"nested":"ok"}}
"#,
        );

        assert_eq!(defaults.get("COMMENTED"), None);
        assert_eq!(defaults.get("ESCAPED"), None);
        assert_eq!(defaults.get("DASH_DEFAULT"), Some(&"fallback".to_string()));
        assert_eq!(defaults.get("COLON_DEFAULT"), Some(&"fallback".to_string()));
        assert_eq!(
            defaults.get("JSON_DEFAULT"),
            Some(&r#"{"nested":"ok"}"#.to_string())
        );
    }

    #[test]
    fn runbook_scripts_are_generated_for_every_package() {
        let root = temp_root("runbook");
        let scripts_dir = root.join("scripts");
        let compose_mode = PackageComposeMode::Generated {
            path: "docker-compose.yml".to_string(),
        };
        write_scripts(&scripts_dir, "web_service", &[], &compose_mode).expect("write scripts");

        for script in deploy_runbook_scripts() {
            assert!(root.join(script).is_file(), "missing {script}");
        }
        let preflight =
            std::fs::read_to_string(scripts_dir.join("preflight.sh")).expect("read preflight");
        assert!(preflight.contains(
            "docker compose $compose_env_args -f \"$compose_file\" -p \"$project\" config"
        ));
        let rollback =
            std::fs::read_to_string(scripts_dir.join("rollback.sh")).expect("read rollback");
        assert!(rollback.contains("reactivating a previous approved version"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn custom_compose_passthrough_resolves_relative_paths_from_copied_source() {
        let root = temp_root("compose-passthrough");
        let source = root.join("projects/lettrebox/source");
        std::fs::create_dir_all(source.join("config")).expect("config dir");
        std::fs::write(source.join("config/x.json"), "{}\n").expect("config");
        std::fs::write(source.join("Dockerfile.prod"), "FROM alpine:3.20\n").expect("dockerfile");
        std::fs::write(
            source.join("docker-compose.yml"),
            r#"services:
  app:
    build:
      context: ${SRC:-.}
      dockerfile: Dockerfile.prod
    volumes:
      - ${SRC:-.}/config/x.json:/app/config/x.json:ro
  postgres:
    image: postgres:16-alpine
  mailhog:
    image: mailhog/mailhog:latest
  stalwart:
    image: stalwartlabs/mail-server:latest
"#,
        )
        .expect("compose");
        std::fs::write(root.join(".env"), "").expect("env");
        std::fs::write(root.join("manifest.json"), "{}\n").expect("manifest");

        let project = PackagedProject {
            project: store::Project {
                id: 1,
                workspace_id: 1,
                name: "lettrebox".to_string(),
                path: source.display().to_string(),
                remote_url: None,
                parent_project_id: None,
                is_submodule: false,
                submodule_path: None,
                created_at: "now".to_string(),
            },
            detection: DeployProjectDetection {
                project_id: 1,
                name: "lettrebox".to_string(),
                path: source.display().to_string(),
                language: "rust".to_string(),
                framework: None,
                package_manager: Some("cargo".to_string()),
                has_dockerfile: true,
                has_compose: true,
                compose_path: Some("docker-compose.yml".to_string()),
                services: vec![],
                ports: vec![deploy_detect::DeployPortSuggestion {
                    container: 5000,
                    host: 5000,
                    confidence: "detected".to_string(),
                }],
                healthcheck: Some("http://127.0.0.1:5000/".to_string()),
                deploy_strategy: "custom_compose".to_string(),
                strategy_reason: "project compose".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
            branch: None,
            commit_sha: None,
            dirty: false,
            git_status_short: String::new(),
            package_path: "projects/lettrebox/source".to_string(),
            dockerfile_path: "projects/lettrebox/Dockerfile".to_string(),
        };
        let plan = serde_json::json!({
            "artifacts": {
                "compose": null
            }
        });
        let decision =
            package_compose_mode("custom_compose", std::slice::from_ref(&project), &plan);
        let mode = decision.mode;
        assert!(decision.warnings.is_empty());
        assert_eq!(
            mode,
            PackageComposeMode::SourcePassthrough {
                path: "projects/lettrebox/source/docker-compose.yml".to_string()
            }
        );
        write_scripts(&root.join("scripts"), "custom_compose", &[project], &mode).expect("scripts");
        assert!(!root.join("docker-compose.yml").exists());

        let output = std::process::Command::new("docker")
            .args([
                "compose",
                "--env-file",
                "./.env",
                "-f",
                "projects/lettrebox/source/docker-compose.yml",
                "-p",
                "lettrebox_fixture",
                "config",
            ])
            .current_dir(&root)
            .output()
            .expect("docker compose config");
        assert!(
            output.status.success(),
            "docker compose config failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let config = String::from_utf8_lossy(&output.stdout);
        for service in ["app:", "postgres:", "mailhog:", "stalwart:"] {
            assert!(config.contains(service), "missing {service}\n{config}");
        }
        let expected_source = source.display().to_string();
        assert!(
            config.contains(&format!("context: {expected_source}")),
            "compose did not resolve build context to source dir\n{config}"
        );
        assert!(
            config.contains(&format!("source: {expected_source}/config/x.json"))
                && config.contains("target: /app/config/x.json")
                && config.contains("read_only: true"),
            "compose did not resolve bind mount to source dir\n{config}"
        );

        let preflight = std::process::Command::new("sh")
            .args(["scripts/preflight.sh"])
            .current_dir(&root)
            .output()
            .expect("preflight");
        assert!(
            preflight.status.success(),
            "preflight failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&preflight.stdout),
            String::from_utf8_lossy(&preflight.stderr)
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn custom_compose_passthrough_falls_back_when_multiple_projects_have_compose() {
        let root = temp_root("compose-passthrough-multi");
        let project_a = custom_compose_project_fixture(
            1,
            "api",
            &root.join("api"),
            "projects/api/source",
            "compose.deploy.yaml",
        );
        let project_b = custom_compose_project_fixture(
            2,
            "worker",
            &root.join("worker"),
            "projects/worker/source",
            "docker-compose.yml",
        );
        let plan = serde_json::json!({
            "artifacts": {
                "compose": null
            }
        });

        let decision = package_compose_mode("custom_compose", &[project_a, project_b], &plan);

        assert_eq!(
            decision.mode,
            PackageComposeMode::Generated {
                path: "docker-compose.yml".to_string()
            }
        );
        assert_eq!(decision.warnings.len(), 1);
        assert!(decision.warnings[0].contains("multiple project compose files detected"));
        assert!(decision.warnings[0].contains("projects/api/source/compose.deploy.yaml"));
        assert!(decision.warnings[0].contains("projects/worker/source/docker-compose.yml"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn desktop_dev_package_uses_vm_runbook_without_web_compose_service() {
        let root = temp_root("desktop-runbook");
        let project = PackagedProject {
            project: store::Project {
                id: 1,
                workspace_id: 1,
                name: "desktop".to_string(),
                path: root.display().to_string(),
                remote_url: None,
                parent_project_id: None,
                is_submodule: false,
                submodule_path: None,
                created_at: "now".to_string(),
            },
            detection: DeployProjectDetection {
                project_id: 1,
                name: "desktop".to_string(),
                path: root.display().to_string(),
                language: "typescript".to_string(),
                framework: Some("tauri".to_string()),
                package_manager: Some("npm".to_string()),
                has_dockerfile: false,
                has_compose: false,
                compose_path: None,
                services: vec![],
                ports: vec![],
                healthcheck: None,
                deploy_strategy: "desktop_dev".to_string(),
                strategy_reason: "Tauri desktop project detected".to_string(),
                runtime_commands: vec!["npm run dev".to_string()],
                requires_desktop_session: true,
                warnings: vec![],
            },
            branch: None,
            commit_sha: None,
            dirty: false,
            git_status_short: String::new(),
            package_path: "projects/desktop/source".to_string(),
            dockerfile_path: "projects/desktop/Dockerfile".to_string(),
        };
        let compose = root.join("docker-compose.yml");
        write_compose(&compose, std::slice::from_ref(&project)).expect("compose");
        let compose_mode = PackageComposeMode::Generated {
            path: "docker-compose.yml".to_string(),
        };
        write_scripts(
            &root.join("scripts"),
            "desktop_dev",
            &[project],
            &compose_mode,
        )
        .expect("scripts");

        let compose_content = std::fs::read_to_string(compose).expect("read compose");
        assert_eq!(compose_content, "services: {}\n");
        let deploy = std::fs::read_to_string(root.join("scripts/deploy.sh")).expect("deploy");
        assert!(deploy.contains("./scripts/prepare-dev-vm.sh"));
        assert!(deploy.contains("./scripts/build-dev.sh"));
        let desktop_projects =
            std::fs::read_to_string(root.join(".dw-desktop-projects")).expect("desktop projects");
        assert_eq!(desktop_projects, "projects/desktop/source\n");
        let build_dev =
            std::fs::read_to_string(root.join("scripts/build-dev.sh")).expect("build-dev");
        assert!(build_dev.contains("read -r project_path || [ -n \"$project_path\" ]"));
        assert!(build_dev.contains("test -x node_modules/.bin/tauri"));
        let prepare_dev =
            std::fs::read_to_string(root.join("scripts/prepare-dev-vm.sh")).expect("prepare-dev");
        assert!(prepare_dev.contains("https://deb.nodesource.com/setup_22.x"));
        assert!(prepare_dev.contains("rustup default stable"));
        assert!(prepare_dev.contains("apt_log_has_retryable_lock"));
        assert!(prepare_dev.contains("apt_update_with_retry"));
        assert!(prepare_dev.contains("apt_install_with_retry"));
        assert!(prepare_dev.contains("waiting for package manager lock"));
        let verify_dev =
            std::fs::read_to_string(root.join("scripts/verify-dev.sh")).expect("verify-dev");
        assert!(verify_dev.contains("test -d \"$project_path/node_modules\""));
        assert!(verify_dev.contains("test -x \"$project_path/node_modules/.bin/tauri\""));
        let healthcheck =
            std::fs::read_to_string(root.join("scripts/healthcheck.sh")).expect("healthcheck");
        assert!(healthcheck.contains("./scripts/verify-dev.sh"));
        let run_dev = std::fs::read_to_string(root.join("scripts/run-dev.sh")).expect("run-dev");
        assert!(run_dev.contains("npm run dev"));
        let windows_install =
            std::fs::read_to_string(root.join("scripts/install-deploy.ps1")).expect("install ps1");
        assert!(windows_install.contains(r#"$Strategy -eq "desktop_dev""#));
        assert!(windows_install.contains(r#""--source", "winget""#));
        assert!(windows_install.contains(r#""--disable-interactivity""#));
        assert!(windows_install.contains("function Test-WingetAlreadySatisfied"));
        assert!(windows_install
            .contains("$Id is already installed; winget reported no upgrade, continuing"));
        assert!(windows_install.contains("Wait-CommandAvailable $CommandName $Id"));
        assert!(windows_install.contains("function Test-VsBuildTools"));
        assert!(windows_install.contains("Microsoft.VisualStudio.Component.VC.Tools.x86.x64"));
        assert!(windows_install.contains(r#"Ensure-WingetPackage "OpenJS.NodeJS.LTS" "node""#));
        assert!(windows_install.contains(r#"Ensure-WingetPackage "Rustlang.Rustup" "rustup""#));
        assert!(windows_install.contains("Validating Docker for container deploy"));
        let docker_check = windows_install
            .find("Validating Docker for container deploy")
            .expect("docker branch marker");
        let desktop_exit = windows_install
            .find("Windows desktop_dev dependencies ready")
            .expect("desktop branch marker");
        assert!(desktop_exit < docker_check);
        let windows_deploy =
            std::fs::read_to_string(root.join("scripts/deploy.ps1")).expect("deploy ps1");
        assert!(windows_deploy.contains(".ProviderPath"));
        assert!(windows_deploy.contains(r#"Join-Path "C:\dw\deploy" $LocalPackageName"#));
        assert!(windows_deploy.contains("function Sync-PackageToLocal"));
        assert!(windows_deploy.contains("robocopy $SharedPackageRoot $LocalPackageRoot"));
        assert!(windows_deploy.contains("npm install"));
        assert!(windows_deploy.contains(r#"$ErrorActionPreference = "Continue""#));
        assert!(windows_deploy.contains("ForEach-Object { \"$_\" }"));
        assert!(windows_deploy.contains("Command failed with exit code $exitCode"));
        assert!(windows_deploy.contains("cargo metadata"));
        let windows_healthcheck =
            std::fs::read_to_string(root.join("scripts/healthcheck.ps1")).expect("health ps1");
        assert!(windows_healthcheck.contains("Windows desktop_dev healthcheck ok"));
        let windows_run_dev =
            std::fs::read_to_string(root.join("scripts/run-dev.ps1")).expect("run-dev ps1");
        assert!(windows_run_dev.contains("Get-ManifestComposeProjectName"));
        assert!(windows_run_dev.contains("ConvertFrom-Json"));
        assert!(windows_run_dev.contains(r#"Join-Path "C:\dw\deploy" $localPackageName"#));
        assert!(windows_run_dev.contains("[switch]$PrintTarget"));
        assert!(windows_run_dev.contains("if ($PrintTarget)"));
        assert!(windows_run_dev.contains("Set-Location $ProjectRoot"));
        assert!(windows_run_dev.contains("npm install"));
        let windows_run_dev_cmd =
            std::fs::read_to_string(root.join("scripts/run-dev.cmd")).expect("run-dev cmd");
        assert!(windows_run_dev_cmd.contains("ExecutionPolicy Bypass"));
        assert!(windows_run_dev_cmd.contains(r#""%~dp0run-dev.ps1""#));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn desktop_dev_package_ignores_agent_overrides_for_core_runbook_scripts() {
        let root = temp_root("desktop-runbook-agent-overrides");
        let compose_mode = PackageComposeMode::Generated {
            path: "docker-compose.yml".to_string(),
        };
        write_scripts(&root.join("scripts"), "desktop_dev", &[], &compose_mode).expect("scripts");
        let plan = serde_json::json!({
            "artifacts": {
                "scripts": [
                    {
                        "path": "scripts/preflight.sh",
                        "body": "#!/usr/bin/env bash\nset -euo pipefail\necho agent preflight\n"
                    },
                    {
                        "path": "scripts/deploy.sh",
                        "body": "#!/usr/bin/env sh\nset -eu\necho agent deploy\n"
                    },
                    {
                        "path": "scripts/custom-agent.sh",
                        "body": "#!/usr/bin/env sh\nset -eu\necho custom\n"
                    }
                ]
            }
        });
        write_agent_plan_scripts(&root, &plan, "desktop_dev", &compose_mode)
            .expect("agent scripts");

        let preflight =
            std::fs::read_to_string(root.join("scripts/preflight.sh")).expect("preflight");
        assert!(preflight.contains("preflight project="));
        assert!(!preflight.contains("agent preflight"));
        let deploy = std::fs::read_to_string(root.join("scripts/deploy.sh")).expect("deploy");
        assert!(deploy.contains("./scripts/prepare-dev-vm.sh"));
        assert!(!deploy.contains("agent deploy"));
        assert!(root.join("scripts/custom-agent.sh").is_file());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stale_project_selection_does_not_create_empty_stack() {
        let root = temp_root("stale-selection-db");
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: None,
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");

        let error = create_package(
            &db,
            CreateDeployPackageInput {
                workspace_id: workspace.id,
                stack_name: "Winbox deploy".to_string(),
                project_ids: vec![3],
                target_machine_id: None,
                agent_profile_id: agent.id,
                deploy_plan_path: Some(write_test_plan(&workspace_root, 3, "web_service")),
                include_dirty: true,
            },
        )
        .expect_err("stale project should fail")
        .to_string();

        assert!(error.contains("deploy_project_selection_stale"));
        assert!(db
            .list_deploy_stacks(workspace.id)
            .expect("list stacks")
            .is_empty());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn create_package_requires_agent_plan_and_writes_analysis_artifacts() {
        let root = temp_root("agent-plan-package");
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join("web");
        std::fs::create_dir_all(&project_root).expect("project root");
        std::fs::create_dir_all(project_root.join("crates/mail-driver/src/stalwart"))
            .expect("mail driver root");
        std::fs::write(
            project_root.join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0"},"dependencies":{"vite":"latest"}}"#,
        )
        .expect("package");
        std::fs::write(
            project_root.join("crates/mail-driver/src/stalwart/client.rs"),
            r#"let auth = format!("Bearer {token}");"#,
        )
        .expect("client");
        std::fs::write(project_root.join(".env.example"), "SMTP_PASSWORD=\n").expect("env example");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "Web",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");

        let version = create_package(
            &db,
            CreateDeployPackageInput {
                workspace_id: workspace.id,
                stack_name: "Web deploy".to_string(),
                project_ids: vec![project.id],
                target_machine_id: None,
                agent_profile_id: agent.id,
                deploy_plan_path: Some(write_test_plan(&workspace_root, project.id, "web_service")),
                include_dirty: true,
            },
        )
        .expect("create package");
        let artifact = PathBuf::from(&version.artifact_path);
        assert!(artifact.join("analysis/project-context.json").is_file());
        assert!(artifact.join("analysis/deploy-plan.json").is_file());
        assert!(artifact.join("analysis/validation-report.json").is_file());
        assert!(version
            .manifest_json
            .contains("\"mode\": \"agent_planned\""));
        assert!(version
            .manifest_json
            .contains("\"agent_name\": \"Codex Deploy\""));
        let review_findings =
            serde_json::from_str::<Vec<SecretFinding>>(&version.blocking_findings_json)
                .expect("review findings");
        assert!(
            review_findings.iter().all(|finding| !finding.blocking),
            "{review_findings:?}"
        );
        let deploy_script =
            std::fs::read_to_string(artifact.join("scripts/deploy.sh")).expect("deploy script");
        assert!(deploy_script.contains("agent generated deploy"));
        let linux_base = std::fs::read_to_string(artifact.join("scripts/install-base-linux.sh"))
            .expect("linux base script");
        assert!(linux_base.contains("Installing base packages"));
        assert!(linux_base.contains("DW_HOST_EPOCH"));
        assert!(linux_base.contains("apt_log_has_retryable_lock"));
        assert!(linux_base.contains("waiting for package manager lock"));
        assert!(!linux_base.contains("agent generated linux base"));
        let windows_base = std::fs::read_to_string(artifact.join("scripts/install-deploy.ps1"))
            .expect("windows base script");
        assert!(windows_base.contains("OpenSSH Server"));
        assert!(!windows_base.contains("agent generated windows base"));
        let compose =
            std::fs::read_to_string(artifact.join("docker-compose.yml")).expect("compose");
        assert!(compose.contains("image: nginx:alpine"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn create_package_passthroughs_detected_custom_compose_when_agent_omits_compose() {
        let root = temp_root("agent-plan-compose-passthrough");
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join("lettrebox");
        std::fs::create_dir_all(&project_root).expect("project root");
        std::fs::write(
            project_root.join("package.json"),
            r#"{"dependencies":{"pg":"latest"}}"#,
        )
        .expect("package");
        std::fs::write(
            project_root.join("docker-compose.yml"),
            "services:\n  app:\n    image: nginx:alpine\n  postgres:\n    image: postgres:16-alpine\n",
        )
        .expect("compose");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "lettrebox",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");

        let version = create_package(
            &db,
            CreateDeployPackageInput {
                workspace_id: workspace.id,
                stack_name: "lettrebox deploy".to_string(),
                project_ids: vec![project.id],
                target_machine_id: None,
                agent_profile_id: agent.id,
                deploy_plan_path: Some(write_passthrough_plan(&workspace_root, project.id)),
                include_dirty: true,
            },
        )
        .expect("create package");

        let artifact = PathBuf::from(&version.artifact_path);
        assert!(!artifact.join("docker-compose.yml").exists());
        assert_eq!(
            std::fs::read_to_string(artifact.join(".dw-compose-file")).expect("compose file"),
            "projects/lettrebox/source/docker-compose.yml"
        );
        let manifest: serde_json::Value =
            serde_json::from_str(&version.manifest_json).expect("manifest");
        assert_eq!(
            manifest
                .get("compose")
                .and_then(|compose| compose.get("mode"))
                .and_then(serde_json::Value::as_str),
            Some("source_passthrough")
        );
        assert_eq!(
            manifest
                .get("compose")
                .and_then(|compose| compose.get("path"))
                .and_then(serde_json::Value::as_str),
            Some("projects/lettrebox/source/docker-compose.yml")
        );
        let env_example =
            std::fs::read_to_string(artifact.join(".env.example")).expect("env example");
        assert!(env_example.contains("\n#DATABASE_URL=\n"));
        assert!(!env_example.contains("DATABASE_URL=postgres://"));
        let deploy = std::fs::read_to_string(artifact.join("scripts/deploy.sh")).expect("deploy");
        assert!(deploy.contains("-f \"$compose_file\" -p \"$project\" up -d --build"));
        assert!(!deploy.contains("agent generated deploy"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn create_package_with_real_lettrebox_compose_prefills_without_blocking_review() {
        let root = temp_root("real-lettrebox-compose-prefill");
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join("lettrebox");
        std::fs::create_dir_all(project_root.join("config/stalwart")).expect("project dirs");
        std::fs::write(
            project_root.join("Cargo.toml"),
            "[package]\nname = \"lettrebox\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("cargo");
        std::fs::write(project_root.join("Dockerfile.prod"), "FROM alpine:3.20\n")
            .expect("dockerfile");
        std::fs::write(project_root.join("config/stalwart/config.json"), "{}\n")
            .expect("stalwart config");
        std::fs::write(
            project_root.join(".env.example"),
            "POSTGRES_PASSWORD=\nBULWARK_SESSION_SECRET=\n",
        )
        .expect("project env example");
        std::fs::write(
            project_root.join("compose.deploy.yaml"),
            include_str!("../fixtures/deploy/lettrebox/compose.deploy.yaml"),
        )
        .expect("compose");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "lettrebox",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");

        let version = create_package(
            &db,
            CreateDeployPackageInput {
                workspace_id: workspace.id,
                stack_name: "lettrebox deploy".to_string(),
                project_ids: vec![project.id],
                target_machine_id: None,
                agent_profile_id: agent.id,
                deploy_plan_path: Some(write_passthrough_plan(&workspace_root, project.id)),
                include_dirty: true,
            },
        )
        .expect("create package");

        let artifact = PathBuf::from(&version.artifact_path);
        let env_example =
            std::fs::read_to_string(artifact.join(".env.example")).expect("env example");
        assert!(env_example.contains("#dw:default POSTGRES_PASSWORD project\n"));
        assert!(env_example.contains("POSTGRES_PASSWORD=rwfw"));
        assert!(env_example.contains("STALWART_ADMIN_PASSWORD=lettrebox-deploy-admin"));
        assert!(
            env_example.contains("BULWARK_SESSION_SECRET=change-me-change-me-change-me-32chars")
        );
        assert!(env_example.contains("BULWARK_ADMIN_PASSWORD=lettrebox-deploy-bulwark-admin"));
        let findings = serde_json::from_str::<Vec<SecretFinding>>(&version.blocking_findings_json)
            .expect("review findings");
        assert!(
            findings.iter().all(|finding| !finding.blocking),
            "{findings:?}"
        );
        crate::deploy::approve_version(
            &db,
            crate::deploy::ApproveDeployVersionInput {
                version_id: version.id.clone(),
            },
        )
        .expect("approve version");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn create_secret_scan_package<F>(
        label: &str,
        stack_name: &str,
        project_name: &str,
        populate_project: F,
    ) -> (PathBuf, store::DeployVersion, Vec<SecretFinding>)
    where
        F: FnOnce(&Path),
    {
        let root = temp_root(label);
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join(slugify(project_name));
        std::fs::create_dir_all(&project_root).expect("project root");
        std::fs::write(
            project_root.join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0"},"dependencies":{"vite":"latest"}}"#,
        )
        .expect("package");
        populate_project(&project_root);
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                project_name,
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");
        let version = create_package(
            &db,
            CreateDeployPackageInput {
                workspace_id: workspace.id,
                stack_name: stack_name.to_string(),
                project_ids: vec![project.id],
                target_machine_id: None,
                agent_profile_id: agent.id,
                deploy_plan_path: Some(write_test_plan(&workspace_root, project.id, "web_service")),
                include_dirty: true,
            },
        )
        .expect("create package");
        let findings = serde_json::from_str::<Vec<SecretFinding>>(&version.blocking_findings_json)
            .expect("review findings");
        (root, version, findings)
    }

    struct ReviewFlowFixture {
        root: PathBuf,
        db: store::Database,
        workspace_root: PathBuf,
        project_root: PathBuf,
        workspace_id: i64,
        project_id: i64,
        agent_id: i64,
        stack_name: String,
    }

    impl ReviewFlowFixture {
        fn secret_path(&self) -> PathBuf {
            self.project_root.join("src/config.txt")
        }
    }

    fn review_flow_fixture(label: &str, secret_source: &str) -> ReviewFlowFixture {
        let root = temp_root(label);
        let db = store::Database::open(&root).expect("open db");
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join("app");
        std::fs::create_dir_all(project_root.join("src")).expect("project src");
        std::fs::write(
            project_root.join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0"},"dependencies":{"vite":"latest"}}"#,
        )
        .expect("package");
        std::fs::write(project_root.join("src/config.txt"), secret_source).expect("secret source");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "App",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let agent = db
            .create_agent_profile(store::AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create agent");
        ReviewFlowFixture {
            root,
            db,
            workspace_root,
            project_root,
            workspace_id: workspace.id,
            project_id: project.id,
            agent_id: agent.id,
            stack_name: "App deploy".to_string(),
        }
    }

    fn create_review_flow_package(fixture: &ReviewFlowFixture) -> store::DeployVersion {
        create_package(
            &fixture.db,
            CreateDeployPackageInput {
                workspace_id: fixture.workspace_id,
                stack_name: fixture.stack_name.clone(),
                project_ids: vec![fixture.project_id],
                target_machine_id: None,
                agent_profile_id: fixture.agent_id,
                deploy_plan_path: Some(write_test_plan(
                    &fixture.workspace_root,
                    fixture.project_id,
                    "web_service",
                )),
                include_dirty: true,
            },
        )
        .expect("create package")
    }

    fn only_active_blocking_finding(version: &store::DeployVersion) -> SecretFinding {
        let active = active_blocking_findings(version).expect("active findings");
        assert_eq!(active.len(), 1, "{active:?}");
        active.into_iter().next().expect("active finding")
    }

    fn dismiss_generated_finding(
        db: &store::Database,
        version: &store::DeployVersion,
        finding: &SecretFinding,
    ) -> store::DeployVersion {
        dismiss_review_finding(
            db,
            &version.id,
            &finding.path,
            &finding.reason,
            finding.marker.as_deref(),
            finding.line_sha256.as_deref(),
            "owner accepted fake test token",
        )
        .expect("dismiss generated finding")
    }

    fn write_test_plan(workspace_root: &Path, project_id: i64, strategy: &str) -> String {
        let analysis_dir = workspace_root
            .join(".dw")
            .join("deploy-plans")
            .join(format!("plan-{project_id}-{strategy}"))
            .join("analysis");
        std::fs::create_dir_all(&analysis_dir).expect("analysis dir");
        let plan = serde_json::json!({
            "schema_version": "1.0",
            "strategy": strategy,
            "confidence": "high",
            "summary": "agent generated test plan",
            "projects": [{
                "project_id": project_id,
                "name": "Web",
                "kind": "node",
                "package_manager": "npm",
                "runtime": "container",
                "install": ["npm install"],
                "verify": ["npm test -- --runInBand"],
                "run": "npm run dev",
                "requires": {
                    "system_packages": [],
                    "desktop_session": false,
                    "docker": true
                },
                "ports": [{"container": 3000, "host": 3000, "confidence": "suggested"}],
                "healthcheck": "curl -fsS http://127.0.0.1:3000",
                "risks": []
            }],
            "services": [],
            "ports": [{"container": 3000, "host": 3000, "confidence": "suggested"}],
            "env": {"required": [], "optional": []},
            "artifacts": {
                "compose": {
                    "path": "docker-compose.yml",
                    "body": "services:\n  web:\n    image: nginx:alpine\n"
                },
                "dockerfiles": [],
                "scripts": [
                    {"path": "scripts/preflight.sh", "purpose": "preflight", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated preflight\n"},
                    {"path": "scripts/deploy.sh", "purpose": "deploy", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated deploy\n"},
                    {"path": "scripts/healthcheck.sh", "purpose": "health", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated health\n"},
                    {"path": "scripts/logs.sh", "purpose": "logs", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated logs\n"},
                    {"path": "scripts/stop.sh", "purpose": "stop", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated stop\n"},
                    {"path": "scripts/rollback.sh", "purpose": "rollback", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated rollback\n"},
                    {"path": "scripts/install-base-linux.sh", "purpose": "linux base", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated linux base\n"},
                    {"path": "scripts/install-deploy.ps1", "purpose": "windows base", "body": "Write-Host 'agent generated windows base'\n"}
                ]
            },
            "risks": []
        });
        let path = analysis_dir.join("deploy-plan.json");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&plan).expect("plan json"),
        )
        .expect("write plan");
        path.display().to_string()
    }

    fn write_passthrough_plan(workspace_root: &Path, project_id: i64) -> String {
        let analysis_dir = workspace_root
            .join(".dw")
            .join("deploy-plans")
            .join(format!("plan-{project_id}-custom-compose-passthrough"))
            .join("analysis");
        std::fs::create_dir_all(&analysis_dir).expect("analysis dir");
        let plan = serde_json::json!({
            "schema_version": "1.0",
            "strategy": "custom_compose",
            "confidence": "high",
            "summary": "agent omits compose so ADE passthroughs source compose",
            "projects": [{
                "project_id": project_id,
                "name": "lettrebox",
                "kind": "compose",
                "package_manager": "none",
                "runtime": "compose",
                "install": [],
                "verify": [],
                "run": null,
                "requires": {
                    "system_packages": [],
                    "desktop_session": false,
                    "docker": true
                },
                "ports": [{"container": 5000, "host": 5000, "confidence": "suggested"}],
                "healthcheck": "curl -fsS http://127.0.0.1:5000/",
                "risks": []
            }],
            "services": [],
            "ports": [{"container": 5000, "host": 5000, "confidence": "suggested"}],
            "env": {"required": [], "optional": []},
            "artifacts": {
                "compose": null,
                "dockerfiles": [],
                "scripts": [
                    {"path": "scripts/deploy.sh", "purpose": "deploy", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated deploy\n"},
                    {"path": "scripts/healthcheck.sh", "purpose": "health", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated health\n"},
                    {"path": "scripts/logs.sh", "purpose": "logs", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated logs\n"},
                    {"path": "scripts/stop.sh", "purpose": "stop", "body": "#!/usr/bin/env sh\nset -eu\necho agent generated stop\n"}
                ]
            },
            "risks": []
        });
        let path = analysis_dir.join("deploy-plan.json");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&plan).expect("plan json"),
        )
        .expect("write plan");
        path.display().to_string()
    }

    fn custom_compose_project_fixture(
        id: i64,
        name: &str,
        path: &Path,
        package_path: &str,
        compose_path: &str,
    ) -> PackagedProject {
        PackagedProject {
            project: store::Project {
                id,
                workspace_id: 1,
                name: name.to_string(),
                path: path.display().to_string(),
                remote_url: None,
                parent_project_id: None,
                is_submodule: false,
                submodule_path: None,
                created_at: "now".to_string(),
            },
            detection: DeployProjectDetection {
                project_id: id,
                name: name.to_string(),
                path: path.display().to_string(),
                language: "rust".to_string(),
                framework: None,
                package_manager: Some("cargo".to_string()),
                has_dockerfile: true,
                has_compose: true,
                compose_path: Some(compose_path.to_string()),
                services: vec![],
                ports: vec![deploy_detect::DeployPortSuggestion {
                    container: 8080,
                    host: 8080,
                    confidence: "suggested".to_string(),
                }],
                healthcheck: Some("http://127.0.0.1:8080/health".to_string()),
                deploy_strategy: "custom_compose".to_string(),
                strategy_reason: "project compose".to_string(),
                runtime_commands: vec![],
                requires_desktop_session: false,
                warnings: vec![],
            },
            branch: None,
            commit_sha: None,
            dirty: false,
            git_status_short: String::new(),
            package_path: package_path.to_string(),
            dockerfile_path: format!("projects/{name}/Dockerfile"),
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "dw-deploy-package-{}-{}",
            label,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).expect("mkdir");
        root
    }
}
