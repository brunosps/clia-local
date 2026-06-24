use crate::deploy_detect::{self, DeployProjectDetection};
use crate::deploy_plan;
use crate::deploy_repair;
use crate::store;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const DEPLOY_RUNBOOK_VERSION: &str = "2026-06-02.1";

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
    #[serde(default = "default_finding_severity")]
    pub severity: String,
    #[serde(default = "default_finding_blocking")]
    pub blocking: bool,
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
    let plan_bundle = deploy_plan::load_plan_bundle_from_path(
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
    })?;
    let mut findings = Vec::<SecretFinding>::new();
    deploy_plan::write_analysis_artifacts(&artifact_root, &plan_bundle)?;
    let mut packaged_projects = Vec::<PackagedProject>::new();
    for project_detection in &detection.projects {
        let project = db.get_project(project_detection.project_id)?;
        let project_slug = slugify(&project.name);
        let relative_source = format!("projects/{project_slug}/source");
        let relative_dockerfile = format!("projects/{project_slug}/Dockerfile");
        let source_dir = artifact_root.join(&relative_source);
        std::fs::create_dir_all(&source_dir)
            .with_context(|| format!("failed to create {}", source_dir.display()))?;
        copy_source_snapshot(Path::new(&project.path), &source_dir, &mut findings)?;
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
                project.path.clone(),
                format!(
                    "unsupported deploy strategy: {}",
                    project_detection.strategy_reason
                ),
            ));
        }
        packaged_projects.push(packaged);
    }
    write_compose(
        &artifact_root.join("docker-compose.yml"),
        &packaged_projects,
    )?;
    write_env_example(&artifact_root.join(".env.example"), &detection.services)?;
    let package_strategy = package_deploy_strategy(&packaged_projects);
    std::fs::write(artifact_root.join(".dw-deploy-strategy"), &package_strategy)?;
    write_scripts(
        &artifact_root.join("scripts"),
        &package_strategy,
        &packaged_projects,
    )?;
    write_agent_plan_files(&artifact_root, &plan_bundle.plan)?;
    write_agent_plan_scripts(&artifact_root, &plan_bundle.plan, &package_strategy)?;
    write_package_runbook(
        &artifact_root,
        &stack,
        &version,
        &input,
        &packaged_projects,
        &plan_bundle,
        &package_strategy,
    )?;
    let manifest = build_manifest(
        &workspace,
        &stack,
        &version,
        &input,
        &packaged_projects,
        &detection,
        &plan_bundle,
        &findings,
    )?;
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, &manifest_json)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    let findings_json = serde_json::to_string(&findings)?;
    db.update_deploy_version_manifest(
        &version.id,
        &manifest_path.display().to_string(),
        &manifest_json,
        &findings_json,
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
    )
}

pub fn has_blocking_findings(version: &store::DeployVersion) -> bool {
    serde_json::from_str::<Vec<SecretFinding>>(&version.blocking_findings_json)
        .map(|findings| findings.iter().any(|finding| finding.blocking))
        .unwrap_or(true)
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
        "review": {
            "status": "pending",
            "blocking_findings": findings
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

fn copy_source_snapshot(
    source: &Path,
    destination: &Path,
    findings: &mut Vec<SecretFinding>,
) -> anyhow::Result<()> {
    for entry in
        std::fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(reason) = excluded_secret_reason(&name) {
            findings.push(SecretFinding::warning(path.display().to_string(), reason));
            continue;
        }
        if should_exclude_name(&name) {
            continue;
        }
        let target = destination.join(&name);
        if path.is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
            copy_source_snapshot(&path, &target, findings)?;
        } else if path.is_file() {
            if is_secret_file_name(&name) {
                findings.push(SecretFinding::warning(
                    path.display().to_string(),
                    "secret-like filename excluded from package",
                ));
                continue;
            }
            std::fs::copy(&path, &target).with_context(|| {
                format!("failed to copy {} to {}", path.display(), target.display())
            })?;
            if let Some(finding) = scan_secret_content(&path, &target) {
                findings.push(finding);
            }
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

fn scan_secret_content(source_path: &Path, copied_path: &Path) -> Option<SecretFinding> {
    let bytes = std::fs::read(copied_path).ok()?;
    if bytes.len() > 512 * 1024 || bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    let lower = text.to_ascii_lowercase();
    let non_runtime_context = is_non_runtime_path(source_path) || is_non_runtime_path(copied_path);
    let mut warning = None;
    let mut hits = ["api_key=", "apikey=", "secret=", "password=", "bearer "]
        .iter()
        .filter_map(|marker| lower.find(marker).map(|index| (index, *marker)))
        .collect::<Vec<_>>();
    hits.sort_by_key(|(index, _)| *index);
    for (index, marker) in hits {
        let reason = format!("secret-like content marker `{marker}`");
        if non_runtime_context
            || marker_value_is_placeholder(marker, &lower[index + marker.len()..])
        {
            warning.get_or_insert_with(|| {
                SecretFinding::warning(copied_path.display().to_string(), reason)
            });
            continue;
        }
        return Some(SecretFinding::blocking(
            copied_path.display().to_string(),
            reason,
        ));
    }
    warning
}

fn is_non_runtime_path(path: &Path) -> bool {
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

fn marker_value_is_placeholder(marker: &str, rest: &str) -> bool {
    if marker == "bearer " {
        return rest.trim_start().starts_with('<')
            || rest.trim_start().starts_with('{')
            || rest.trim_start().starts_with("$")
            || rest.trim_start().starts_with("test")
            || rest.trim_start().starts_with("example");
    }
    let value = rest
        .trim_start()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`'))
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '&' | ';' | ',' | ')' | '}'))
        .next()
        .unwrap_or("")
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`'));
    if value.is_empty() {
        return true;
    }
    value.starts_with('<')
        || value.starts_with('{')
        || value.starts_with('$')
        || matches!(
            value,
            "test"
                | "example"
                | "changeme"
                | "change-me"
                | "placeholder"
                | "dummy"
                | "local"
                | "development"
        )
}

fn default_finding_severity() -> String {
    "error".to_string()
}

fn default_finding_blocking() -> bool {
    true
}

impl SecretFinding {
    fn warning(path: String, reason: impl Into<String>) -> Self {
        Self {
            path,
            reason: reason.into(),
            severity: "warning".to_string(),
            blocking: false,
        }
    }

    fn blocking(path: String, reason: impl Into<String>) -> Self {
        Self {
            path,
            reason: reason.into(),
            severity: "error".to_string(),
            blocking: true,
        }
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
            if Path::new(&project.project.path).join("Dockerfile").exists() {
                "Dockerfile"
            } else if Path::new(&project.project.path)
                .join("Dockerfile.dev")
                .exists()
            {
                "Dockerfile.dev"
            } else {
                "../Dockerfile"
            }
        } else {
            "../Dockerfile"
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
) -> anyhow::Result<()> {
    let mut content =
        String::from("# Generated placeholder values. Do not paste real secrets here.\n");
    for service in services {
        match service.name.as_str() {
            "postgres" => {
                content.push_str("DATABASE_URL=postgres://user:password@postgres:5432/app\n")
            }
            "redis" => content.push_str("REDIS_URL=redis://redis:6379\n"),
            "smtp" => content.push_str("SMTP_URL=smtp://mailhog:1025\n"),
            _ => {}
        }
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn write_scripts(dir: &Path, strategy: &str, projects: &[PackagedProject]) -> anyhow::Result<()> {
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
    }
    std::fs::write(
        dir.join("preflight.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
echo "[dw] preflight project=$project"
test -f docker-compose.yml
test -f .env
test -f manifest.json
if [ "$strategy" = "desktop_dev" ]; then
  test -s .dw-desktop-projects
  command -v sh >/dev/null
else
  command -v docker >/dev/null
  docker --version
  docker compose version
  docker compose --project-name "$project" config >/dev/null
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
if [ "$strategy" = "desktop_dev" ]; then
  chmod +x scripts/prepare-dev-vm.sh scripts/build-dev.sh
  ./scripts/prepare-dev-vm.sh
  ./scripts/build-dev.sh
else
  docker compose --project-name "$project" up -d --build
  docker compose --project-name "$project" ps
fi
"#,
    )?;
    std::fs::write(
        dir.join("stop.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
if [ "$strategy" = "desktop_dev" ]; then
  echo "[dw] desktop_dev package has no managed compose service to stop"
else
  docker compose --project-name "$project" down
fi
"#,
    )?;
    std::fs::write(
        dir.join("healthcheck.sh"),
        r#"#!/usr/bin/env sh
set -eu
project="${DW_COMPOSE_PROJECT_NAME:-$(basename "$PWD" | tr '-' '_')}"
strategy="$(cat .dw-deploy-strategy 2>/dev/null || echo web_service)"
if [ "$strategy" = "desktop_dev" ]; then
  chmod +x scripts/verify-dev.sh
  ./scripts/verify-dev.sh
else
  docker compose --project-name "$project" ps
  docker compose --project-name "$project" ps --format json >/tmp/dw-compose-ps.json 2>/dev/null || true
  if docker compose --project-name "$project" ps --status exited | grep -q .; then
    echo "[dw] one or more services exited" >&2
    docker compose --project-name "$project" ps >&2
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
if [ "$strategy" = "desktop_dev" ]; then
  find .dw-runbook/logs -type f -maxdepth 1 -print -exec sh -c 'echo "===== $1"; tail -160 "$1"' sh {} \; 2>/dev/null || true
else
  docker compose --project-name "$project" ps
  docker compose --project-name "$project" logs --tail=160
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
) -> anyhow::Result<()> {
    let scripts_root = root.join("scripts");
    for (relative_path, body) in deploy_plan::script_artifacts_from_plan(plan)? {
        if protected_runbook_script(&relative_path, package_strategy) {
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

fn protected_runbook_script(relative_path: &str, package_strategy: &str) -> bool {
    matches!(
        relative_path,
        "scripts/install-base-linux.sh" | "scripts/install-deploy.ps1"
    ) || (package_strategy == "desktop_dev"
        && deploy_runbook_scripts()
            .into_iter()
            .any(|script| script == relative_path))
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
            finding.path.contains("config.txt") && finding.severity == "error" && finding.blocking
        }));
        std::fs::remove_dir_all(root).expect("cleanup");
        std::fs::remove_dir_all(dest).expect("cleanup");
    }

    #[test]
    fn placeholder_secret_markers_are_warnings() {
        let root = temp_root("placeholder-source");
        std::fs::create_dir_all(root.join("templates")).expect("templates");
        std::fs::write(root.join("package.json"), "{}").expect("package");
        std::fs::write(
            root.join("templates/install.rs"),
            "PASSWORD={password}\nAPI_KEY=<api-key>\n",
        )
        .expect("template");
        let dest = temp_root("placeholder-dest");
        let mut findings = Vec::new();
        copy_source_snapshot(&root, &dest, &mut findings).expect("copy");
        assert!(dest.join("templates/install.rs").exists());
        assert!(findings.iter().all(|finding| !finding.blocking));
        assert!(findings.iter().any(|finding| {
            finding.path.contains("install.rs") && finding.reason.contains("password=")
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
    fn runbook_scripts_are_generated_for_every_package() {
        let root = temp_root("runbook");
        let scripts_dir = root.join("scripts");
        write_scripts(&scripts_dir, "web_service", &[]).expect("write scripts");

        for script in deploy_runbook_scripts() {
            assert!(root.join(script).is_file(), "missing {script}");
        }
        let preflight =
            std::fs::read_to_string(scripts_dir.join("preflight.sh")).expect("read preflight");
        assert!(preflight.contains("docker compose"));
        let rollback =
            std::fs::read_to_string(scripts_dir.join("rollback.sh")).expect("read rollback");
        assert!(rollback.contains("reactivating a previous approved version"));
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
        write_scripts(&root.join("scripts"), "desktop_dev", &[project]).expect("scripts");

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
        write_scripts(&root.join("scripts"), "desktop_dev", &[]).expect("scripts");
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
        write_agent_plan_scripts(&root, &plan, "desktop_dev").expect("agent scripts");

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
        std::fs::write(
            project_root.join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0"},"dependencies":{"vite":"latest"}}"#,
        )
        .expect("package");
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
