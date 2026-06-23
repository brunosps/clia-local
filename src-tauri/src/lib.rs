mod agent;
mod deploy;
mod deploy_detect;
mod deploy_env;
mod deploy_executor;
mod deploy_package;
mod deploy_plan;
mod deploy_repair;
mod git;
mod lsp;
mod machine;
mod rtk;
mod shell;
mod solution;
mod store;
mod terminal;
mod winbox_provider;

use anyhow::Context as _;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::Manager;

#[derive(Debug, Serialize)]
struct AppError {
    message: String,
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

type AppResult<T> = Result<T, AppError>;

const MAX_SOURCE_FILE_BYTES: u64 = 1024 * 1024;
const BINARY_SAMPLE_BYTES: usize = 8192;
const MAX_ATTACHMENT_PREVIEW_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct PreflightReport {
    project_path: String,
    has_git: bool,
    has_dw: bool,
    has_dw_commands: bool,
    git_status: String,
    docker_version: Option<String>,
    docker_compose_version: Option<String>,
    node_version: Option<String>,
    pnpm_version: Option<String>,
    rust_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct DwArtifact {
    relative_path: String,
    category: String,
    name: String,
    bytes: u64,
}

#[derive(Debug, Deserialize)]
struct DwArtifactWriteInput {
    relative_path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceFlowArtifactWriteInput {
    relative_path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceFlowSyncInput {
    workspace_path: String,
    project_path: String,
}

#[derive(Debug, Serialize)]
struct DwCommand {
    name: String,
    command: String,
    relative_path: String,
    title: String,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct DwSkill {
    name: String,
    description: Option<String>,
    kind: Option<String>,
    tier: Option<String>,
    owner: Option<String>,
    trigger: Option<String>,
    path: Option<String>,
    source: String,
}

#[derive(Debug, Serialize)]
struct WorkflowStateSummary {
    stages: Vec<WorkflowStageState>,
    gates: Vec<WorkflowGate>,
    resume_entries: Vec<WorkflowResumeEntry>,
}

#[derive(Debug, Serialize)]
struct WorkflowStageState {
    id: String,
    label: String,
    command: String,
    state: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct WorkflowGate {
    label: String,
    state: String,
    path: Option<String>,
    detail: String,
}

#[derive(Debug, Serialize)]
struct WorkflowResumeEntry {
    kind: String,
    label: String,
    command: String,
    path: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct SourceEntry {
    relative_path: String,
    name: String,
    kind: String,
    extension: Option<String>,
    bytes: Option<u64>,
    children: Vec<SourceEntry>,
}

#[derive(Debug, Serialize)]
struct SourceFile {
    relative_path: String,
    name: String,
    extension: Option<String>,
    bytes: u64,
    content: String,
}

#[derive(Debug, Serialize)]
struct SearchMatch {
    line: u32,
    col: u32,
    length: u32,
    text: String,
}

#[derive(Debug, Serialize)]
struct SearchFileResult {
    relative_path: String,
    matches: Vec<SearchMatch>,
}

#[derive(Debug, Serialize)]
struct AttachmentPreview {
    id: i64,
    name: String,
    mime_type: String,
    data_base64: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceInput {
    name: String,
    root_path: String,
}

#[derive(Debug, Deserialize)]
struct ProjectInput {
    workspace_id: i64,
    name: String,
    path: String,
    remote_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloneProjectInput {
    workspace_id: i64,
    name: Option<String>,
    remote_url: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSolutionImportInput {
    source_path: String,
    destination_root: String,
    workspace_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct WorkspaceSolutionImportReport {
    workspace: store::Workspace,
    manifest: solution::WorkspaceSolutionManifest,
    projects: Vec<WorkspaceSolutionProjectImportResult>,
}

#[derive(Debug, Serialize)]
struct WorkspaceSolutionProjectImportResult {
    name: String,
    remote_url: Option<String>,
    path: Option<String>,
    status: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RequirementCardInput {
    workspace_id: i64,
    project_id: Option<i64>,
    project_ids: Option<Vec<i64>>,
    title: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct RequirementCardStatusInput {
    id: i64,
    status: String,
    prd_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RequirementCardRestoreInput {
    id: i64,
    status: String,
}

#[derive(Debug, Deserialize)]
struct RequirementCardFlowInput {
    id: i64,
    /// Target flow id, or `None`/empty to return the card to the intake backlog.
    flow_id: Option<String>,
    /// Status to set after routing (the target flow's first phase). Optional;
    /// when omitted the card keeps its current status.
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RequirementCardBodyInput {
    id: i64,
    body: String,
}

#[derive(Debug, Deserialize)]
struct RequirementStageFormInput {
    card_id: i64,
    stage_id: String,
    payload_json: String,
}

#[derive(Debug, Deserialize)]
struct RequirementAttachmentInput {
    card_id: i64,
    file_path: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RequirementAttachmentDownloadInput {
    id: i64,
    destination_path: String,
}

#[derive(Debug, Deserialize)]
struct KnowledgeSourceInput {
    workspace_id: i64,
    project_id: Option<i64>,
    blueprint_id: Option<String>,
    file_path: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectBlueprintInput {
    workspace_id: i64,
    title: String,
    idea: String,
    agent_profile_id: Option<i64>,
    knowledge_source_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct ProjectBlueprintUpdateInput {
    id: String,
    status: Option<String>,
    agent_session_id: Option<i64>,
    knowledge_source_ids: Option<Vec<i64>>,
    answers_json: Option<String>,
    running_summary: Option<String>,
    detected_subprojects_json: Option<String>,
    prd: Option<String>,
    techspec: Option<String>,
    tasks_json: Option<String>,
    definition_of_done: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EvidenceRunInput {
    workspace_id: Option<i64>,
    project_id: Option<i64>,
    prd_slug: Option<String>,
    command: String,
    summary: Option<String>,
    terminal_session_id: Option<String>,
    terminal_log_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EvidenceCompletionInput {
    id: i64,
    workspace_id: Option<i64>,
    status: String,
    summary: String,
}

#[derive(Debug, Deserialize)]
struct ManualEvidenceInput {
    title: String,
    status: String,
    summary: String,
    relative_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AgentProfileInput {
    workspace_id: i64,
    project_id: Option<i64>,
    name: String,
    provider: String,
    model: Option<String>,
    reasoning_effort: Option<String>,
    sandbox: String,
    context_mode: Option<String>,
    rtk_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AgentProfileUpdateInput {
    id: i64,
    name: String,
    provider: String,
    model: Option<String>,
    reasoning_effort: Option<String>,
    sandbox: String,
    context_mode: Option<String>,
    rtk_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AgentChatResetInput {
    profile_id: i64,
    workspace_id: i64,
    project_id: Option<i64>,
    project_path: String,
}

#[derive(Debug, Deserialize)]
struct AgentMessageInput {
    profile_id: i64,
    session_id: Option<i64>,
    workspace_id: i64,
    project_id: Option<i64>,
    requirement_card_id: Option<i64>,
    scope: Option<String>,
    title: Option<String>,
    project_path: String,
    message: String,
    skill: Option<AgentSkillInvocationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentSkillInvocationInput {
    name: String,
    scope: Option<String>,
    scope_label: Option<String>,
    framework_id: Option<String>,
    framework_label: Option<String>,
    source: Option<String>,
    path: Option<String>,
    byte_count: Option<i64>,
}

#[tauri::command]
fn preflight(project_path: String) -> AppResult<PreflightReport> {
    let path = PathBuf::from(project_path);
    let has_git = path.join(".git").exists();
    let has_dw = path.join(".dw").exists();
    let has_dw_commands = path.join(".dw").join("commands").exists();

    Ok(PreflightReport {
        project_path: path.display().to_string(),
        has_git,
        has_dw,
        has_dw_commands,
        git_status: git::status(&path).unwrap_or_else(|error| error.to_string()),
        docker_version: shell::run_capture(None, "docker", &["--version"]).ok(),
        docker_compose_version: shell::run_capture(None, "docker", &["compose", "version"]).ok(),
        node_version: shell::run_capture(None, "node", &["--version"]).ok(),
        pnpm_version: shell::run_capture(None, "corepack", &["pnpm", "--version"]).ok(),
        rust_version: shell::run_capture(None, "rustc", &["--version"]).ok(),
    })
}

#[tauri::command]
fn list_workspaces(app: tauri::AppHandle) -> AppResult<Vec<store::Workspace>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_workspaces()?)
}

#[tauri::command]
fn create_workspace(app: tauri::AppHandle, input: WorkspaceInput) -> AppResult<store::Workspace> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.create_workspace(&input.name, &input.root_path)?)
}

#[tauri::command]
fn list_projects(app: tauri::AppHandle, workspace_id: i64) -> AppResult<Vec<store::Project>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_projects(workspace_id)?)
}

#[tauri::command]
fn add_local_project(app: tauri::AppHandle, input: ProjectInput) -> AppResult<store::Project> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.add_project(
        input.workspace_id,
        &input.name,
        &input.path,
        input.remote_url.as_deref(),
    )?)
}

#[tauri::command]
fn clone_git_project(app: tauri::AppHandle, input: CloneProjectInput) -> AppResult<store::Project> {
    let remote_url = input.remote_url.trim();
    if remote_url.is_empty() {
        return Err(anyhow::anyhow!("remote URL is required").into());
    }

    let db = store::Database::open(&app_data_dir(&app)?)?;
    let workspace = db.get_workspace(input.workspace_id)?;
    let name = input
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| project_name_from_remote(remote_url));
    Ok(clone_project_into_workspace(
        &db, &workspace, &name, remote_url,
    )?)
}

fn clone_project_into_workspace(
    db: &store::Database,
    workspace: &store::Workspace,
    name: &str,
    remote_url: &str,
) -> anyhow::Result<store::Project> {
    let safe_dir = safe_project_dir_name(name);
    let workspace_root = PathBuf::from(&workspace.root_path);
    let projects_dir = workspace_root.join("projects");
    std::fs::create_dir_all(&projects_dir).map_err(anyhow::Error::from)?;
    let legacy_destination = workspace_root.join("repos").join(&safe_dir);
    let destination = if legacy_destination.exists() {
        legacy_destination
    } else {
        projects_dir.join(&safe_dir)
    };
    let destination_string = destination.display().to_string();
    if destination.exists() {
        if !is_git_project_dir(&destination) {
            return Err(anyhow::anyhow!(
                "project folder already exists and is not a Git project: {}",
                destination.display()
            ));
        }
        return db.add_project(workspace.id, name, &destination_string, Some(remote_url));
    }

    shell::run_capture(None, "git", &["clone", remote_url, &destination_string])?;
    db.add_project(workspace.id, name, &destination_string, Some(remote_url))
}

fn create_or_open_workspace(
    db: &store::Database,
    name: &str,
    root_path: &str,
) -> anyhow::Result<store::Workspace> {
    std::fs::create_dir_all(root_path)
        .with_context(|| format!("failed to create workspace root {root_path}"))?;
    let normalized = std::fs::canonicalize(root_path)
        .unwrap_or_else(|_| PathBuf::from(root_path))
        .display()
        .to_string();
    for workspace in db.list_workspaces()? {
        let existing = std::fs::canonicalize(&workspace.root_path)
            .unwrap_or_else(|_| PathBuf::from(&workspace.root_path))
            .display()
            .to_string();
        if existing == normalized {
            return Ok(workspace);
        }
    }
    db.create_workspace(name, &normalized)
}

fn import_solution_project(
    db: &store::Database,
    workspace: &store::Workspace,
    manifest_project: &solution::WorkspaceSolutionProject,
) -> WorkspaceSolutionProjectImportResult {
    let remote_url = primary_solution_remote(manifest_project);
    let Some(remote_url) = remote_url else {
        return WorkspaceSolutionProjectImportResult {
            name: manifest_project.name.clone(),
            remote_url: None,
            path: None,
            status: "skipped".to_string(),
            message: "Projeto sem Git remote no .wksdw; adicione manualmente.".to_string(),
        };
    };

    match clone_project_into_workspace(db, workspace, &manifest_project.name, &remote_url) {
        Ok(project) => {
            let warnings = configure_imported_project(&project.path, manifest_project);
            let status = if warnings.is_empty() {
                "cloned"
            } else {
                "warning"
            };
            let message = if warnings.is_empty() {
                "Projeto clonado.".to_string()
            } else {
                warnings.join(" ")
            };
            WorkspaceSolutionProjectImportResult {
                name: manifest_project.name.clone(),
                remote_url: Some(remote_url),
                path: Some(project.path),
                status: status.to_string(),
                message,
            }
        }
        Err(error) => WorkspaceSolutionProjectImportResult {
            name: manifest_project.name.clone(),
            remote_url: Some(remote_url),
            path: None,
            status: "failed".to_string(),
            message: error.to_string(),
        },
    }
}

fn primary_solution_remote(project: &solution::WorkspaceSolutionProject) -> Option<String> {
    project
        .remote_url
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            project
                .remotes
                .iter()
                .find(|remote| remote.name == "origin")
                .map(|remote| remote.url.clone())
        })
        .or_else(|| project.remotes.first().map(|remote| remote.url.clone()))
}

fn configure_imported_project(
    project_path: &str,
    manifest_project: &solution::WorkspaceSolutionProject,
) -> Vec<String> {
    let path = PathBuf::from(project_path);
    let mut warnings = Vec::new();
    for remote in &manifest_project.remotes {
        if remote.name.trim().is_empty() || remote.url.trim().is_empty() {
            continue;
        }
        let existing = shell::run_capture(Some(&path), "git", &["remote", "get-url", &remote.name]);
        match existing {
            Ok(url) if url.trim() == remote.url.trim() => {}
            Ok(_) => {
                if let Err(error) = shell::run_capture(
                    Some(&path),
                    "git",
                    &["remote", "set-url", &remote.name, &remote.url],
                ) {
                    warnings.push(format!("Remote {} não atualizado: {error}.", remote.name));
                }
            }
            Err(_) => {
                if let Err(error) = shell::run_capture(
                    Some(&path),
                    "git",
                    &["remote", "add", &remote.name, &remote.url],
                ) {
                    warnings.push(format!("Remote {} não adicionado: {error}.", remote.name));
                }
            }
        }
    }
    if let Some(branch) = manifest_project
        .branch
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if let Err(error) = shell::run_capture(Some(&path), "git", &["checkout", branch]) {
            warnings.push(format!("Branch {branch} não selecionada: {error}."));
        }
    }
    warnings
}

#[tauri::command]
fn list_requirement_cards(
    app: tauri::AppHandle,
    workspace_id: i64,
) -> AppResult<Vec<store::RequirementCard>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_requirement_cards(workspace_id)?)
}

#[tauri::command]
fn create_requirement_card(
    app: tauri::AppHandle,
    input: RequirementCardInput,
) -> AppResult<store::RequirementCard> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(anyhow::anyhow!("requirement title is required").into());
    }
    let slug = slugify_requirement(title);
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let project_ids = input.project_ids.unwrap_or_default();
    Ok(db.create_requirement_card(
        input.workspace_id,
        input.project_id,
        &project_ids,
        title,
        &slug,
        &input.body,
    )?)
}

#[tauri::command]
fn update_requirement_card_status(
    app: tauri::AppHandle,
    input: RequirementCardStatusInput,
) -> AppResult<store::RequirementCard> {
    let status = normalize_requirement_status(&input.status)?;
    let prd_slug = input
        .prd_slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.update_requirement_card_status(input.id, &status, prd_slug)?)
}

#[tauri::command]
fn set_requirement_card_flow(
    app: tauri::AppHandle,
    input: RequirementCardFlowInput,
) -> AppResult<store::RequirementCard> {
    let flow_id = input
        .flow_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let card = db.set_requirement_card_flow(input.id, flow_id)?;
    // Route + advance to the target flow's first phase in one round-trip.
    if let Some(status) = input
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let status = normalize_requirement_status(status)?;
        return Ok(db.update_requirement_card_status(input.id, &status, None)?);
    }
    Ok(card)
}

#[tauri::command]
fn archive_requirement_card(app: tauri::AppHandle, id: i64) -> AppResult<store::RequirementCard> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.archive_requirement_card(id)?)
}

#[tauri::command]
fn restore_requirement_card(
    app: tauri::AppHandle,
    input: RequirementCardRestoreInput,
) -> AppResult<store::RequirementCard> {
    let status = normalize_restorable_requirement_status(&input.status)?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.restore_requirement_card(input.id, &status)?)
}

#[tauri::command]
fn update_requirement_card_body(
    app: tauri::AppHandle,
    input: RequirementCardBodyInput,
) -> AppResult<store::RequirementCard> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.update_requirement_card_body(input.id, &input.body)?)
}

#[tauri::command]
fn list_requirement_stage_forms(
    app: tauri::AppHandle,
    card_id: i64,
) -> AppResult<Vec<store::RequirementStageForm>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_requirement_stage_forms(card_id)?)
}

#[tauri::command]
fn upsert_requirement_stage_form(
    app: tauri::AppHandle,
    input: RequirementStageFormInput,
) -> AppResult<store::RequirementStageForm> {
    let stage_id = input.stage_id.trim();
    if stage_id.is_empty() {
        return Err(anyhow::anyhow!("stage id is required").into());
    }
    serde_json::from_str::<serde_json::Value>(&input.payload_json)
        .map_err(|error| anyhow::anyhow!("invalid stage form JSON: {error}"))?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.upsert_requirement_stage_form(input.card_id, stage_id, &input.payload_json)?)
}

#[tauri::command]
fn list_requirement_attachments(
    app: tauri::AppHandle,
    card_id: i64,
) -> AppResult<Vec<store::RequirementAttachment>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_requirement_attachments(card_id)?)
}

#[tauri::command]
fn add_requirement_attachment(
    app: tauri::AppHandle,
    input: RequirementAttachmentInput,
) -> AppResult<store::RequirementAttachment> {
    let path = std::fs::canonicalize(input.file_path.trim()).map_err(anyhow::Error::from)?;
    if !path.is_file() {
        return Err(anyhow::anyhow!("attachment path is not a file").into());
    }
    let name = input
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            path.file_name()
                .map(|value| value.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "attachment".to_string());
    let file_path = path.display().to_string();
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.add_requirement_attachment(input.card_id, &name, &file_path)?)
}

#[tauri::command]
fn remove_requirement_attachment(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.remove_requirement_attachment(id)?)
}

#[tauri::command]
fn preview_requirement_attachment(app: tauri::AppHandle, id: i64) -> AppResult<AttachmentPreview> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let attachment = db.get_requirement_attachment(id)?;
    let path = PathBuf::from(&attachment.file_path);
    let mime_type = preview_mime_type(&path).ok_or_else(|| {
        anyhow::anyhow!("attachment preview is available only for images and PDFs")
    })?;
    let metadata = std::fs::metadata(&path)
        .map_err(|error| anyhow::anyhow!("failed to read attachment metadata: {error}"))?;
    if !metadata.is_file() {
        return Err(anyhow::anyhow!("attachment path is not a file").into());
    }
    if metadata.len() > MAX_ATTACHMENT_PREVIEW_BYTES {
        return Err(anyhow::anyhow!("attachment is too large to preview").into());
    }
    let bytes = std::fs::read(&path)
        .map_err(|error| anyhow::anyhow!("failed to read attachment: {error}"))?;
    Ok(AttachmentPreview {
        id: attachment.id,
        name: attachment.name,
        mime_type: mime_type.to_string(),
        data_base64: general_purpose::STANDARD.encode(bytes),
    })
}

#[tauri::command]
fn download_requirement_attachment(
    app: tauri::AppHandle,
    input: RequirementAttachmentDownloadInput,
) -> AppResult<()> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let attachment = db.get_requirement_attachment(input.id)?;
    let source = PathBuf::from(&attachment.file_path);
    if !source.is_file() {
        return Err(anyhow::anyhow!("attachment source is not a file").into());
    }
    let destination = PathBuf::from(input.destination_path.trim());
    if destination.as_os_str().is_empty() {
        return Err(anyhow::anyhow!("destination path is required").into());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| anyhow::anyhow!("failed to create destination directory: {error}"))?;
    }
    std::fs::copy(&source, &destination)
        .map_err(|error| anyhow::anyhow!("failed to copy attachment: {error}"))?;
    Ok(())
}

#[tauri::command]
fn list_knowledge_sources(
    app: tauri::AppHandle,
    workspace_id: i64,
    project_id: Option<i64>,
) -> AppResult<Vec<store::KnowledgeSource>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_knowledge_sources(workspace_id, project_id)?)
}

#[tauri::command]
fn add_knowledge_source(
    app: tauri::AppHandle,
    input: KnowledgeSourceInput,
) -> AppResult<store::KnowledgeSource> {
    let path = std::fs::canonicalize(input.file_path.trim()).map_err(anyhow::Error::from)?;
    if !path.is_file() {
        return Err(anyhow::anyhow!("knowledge source path is not a file").into());
    }
    let name = input
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            path.file_name()
                .map(|value| value.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "knowledge-source".to_string());
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.add_knowledge_source(
        input.workspace_id,
        input.project_id,
        input.blueprint_id.as_deref(),
        &name,
        &path.display().to_string(),
    )?)
}

#[tauri::command]
fn remove_knowledge_source(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.remove_knowledge_source(id)?)
}

#[tauri::command]
fn create_project_blueprint(
    app: tauri::AppHandle,
    input: ProjectBlueprintInput,
) -> AppResult<store::ProjectBlueprint> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(anyhow::anyhow!("project blueprint title is required").into());
    }
    let knowledge_source_ids_json =
        serde_json::to_string(&input.knowledge_source_ids.unwrap_or_default())
            .map_err(anyhow::Error::from)?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.create_project_blueprint(
        input.workspace_id,
        title,
        &input.idea,
        input.agent_profile_id,
        &knowledge_source_ids_json,
    )?)
}

#[tauri::command]
fn list_project_blueprints(
    app: tauri::AppHandle,
    workspace_id: i64,
) -> AppResult<Vec<store::ProjectBlueprint>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_project_blueprints(workspace_id)?)
}

#[tauri::command]
fn update_project_blueprint(
    app: tauri::AppHandle,
    input: ProjectBlueprintUpdateInput,
) -> AppResult<store::ProjectBlueprint> {
    let knowledge_source_ids_json = input
        .knowledge_source_ids
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(anyhow::Error::from)?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.update_project_blueprint(store::ProjectBlueprintUpdate {
        id: &input.id,
        status: input.status.as_deref(),
        agent_session_id: input.agent_session_id,
        knowledge_source_ids_json: knowledge_source_ids_json.as_deref(),
        answers_json: input.answers_json.as_deref(),
        running_summary: input.running_summary.as_deref(),
        detected_subprojects_json: input.detected_subprojects_json.as_deref(),
        prd: input.prd.as_deref(),
        techspec: input.techspec.as_deref(),
        tasks_json: input.tasks_json.as_deref(),
        definition_of_done: input.definition_of_done.as_deref(),
    })?)
}

#[tauri::command]
fn materialize_project_blueprint(
    app: tauri::AppHandle,
    id: String,
) -> AppResult<store::ProjectBlueprintMaterialization> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.materialize_project_blueprint(&id)?)
}

#[tauri::command]
fn list_evidence(app: tauri::AppHandle, path: String) -> AppResult<Vec<store::EvidenceEntry>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let mut entries = db.list_evidence(&project_path_key(&root))?;
    mark_evidence_staleness(&root, &mut entries);
    Ok(entries)
}

#[tauri::command]
fn create_evidence_run(
    app: tauri::AppHandle,
    path: String,
    input: EvidenceRunInput,
) -> AppResult<store::EvidenceEntry> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let command = input.command.trim();
    if command.is_empty() {
        return Err(anyhow::anyhow!("evidence command is required").into());
    }

    let inferred_prd_slug = infer_prd_slug(command);
    let prd_slug = input
        .prd_slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or(inferred_prd_slug.as_deref());
    let summary = input
        .summary
        .unwrap_or_else(|| "Sent to terminal.".to_string());
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.create_evidence_run(
        input.workspace_id,
        input.project_id,
        &project_path_key(&root),
        prd_slug,
        command,
        "submitted",
        &redact_summary(&summary),
        input.terminal_session_id.as_deref(),
        input.terminal_log_path.as_deref(),
    )?)
}

#[tauri::command]
fn complete_evidence_run(
    app: tauri::AppHandle,
    input: EvidenceCompletionInput,
) -> AppResult<store::EvidenceEntry> {
    let status = normalize_evidence_status(&input.status)?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.complete_evidence_run(
        input.id,
        status,
        &redact_summary(&input.summary),
        input.workspace_id,
    )?)
}

#[tauri::command]
fn create_manual_evidence(
    app: tauri::AppHandle,
    path: String,
    input: ManualEvidenceInput,
) -> AppResult<Vec<store::EvidenceEntry>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let title = input.title.trim();
    if title.is_empty() {
        return Err(anyhow::anyhow!("evidence title is required").into());
    }
    let status = normalize_evidence_status(&input.status)?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let project_path = project_path_key(&root);
    let summary = redact_summary(&input.summary);
    let mut entries = Vec::new();
    let relative_paths = input
        .relative_paths
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if relative_paths.is_empty() {
        entries.push(db.create_evidence_item(
            &project_path,
            "note",
            title,
            None,
            None,
            status,
            &summary,
        )?);
        return Ok(entries);
    }

    for relative_path in relative_paths {
        let normalized = normalize_existing_dw_relative_path(&root, relative_path)?;
        let absolute_path = root.join(".dw").join(&normalized);
        entries.push(db.create_evidence_item(
            &project_path,
            evidence_kind_for_path(&normalized),
            title,
            Some(&normalized),
            Some(&absolute_path.display().to_string()),
            status,
            &summary,
        )?);
    }

    Ok(entries)
}

#[tauri::command]
fn index_project_evidence(
    app: tauri::AppHandle,
    path: String,
) -> AppResult<Vec<store::EvidenceEntry>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let project_path = project_path_key(&root);
    for indexed in collect_project_evidence_files(&root)? {
        db.upsert_indexed_evidence_item(
            &project_path,
            evidence_kind_for_path(&indexed.relative_path),
            &indexed.title,
            &indexed.relative_path,
            &indexed.absolute_path,
            &indexed.summary,
        )?;
    }
    let mut entries = db.list_evidence(&project_path)?;
    mark_evidence_staleness(&root, &mut entries);
    Ok(entries)
}

#[tauri::command]
fn list_agent_profiles(
    app: tauri::AppHandle,
    workspace_id: i64,
    project_id: Option<i64>,
) -> AppResult<Vec<store::AgentProfile>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_agent_profiles(workspace_id, project_id)?)
}

#[tauri::command]
fn create_agent_profile(
    app: tauri::AppHandle,
    input: AgentProfileInput,
) -> AppResult<store::AgentProfile> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow::anyhow!("agent name is required").into());
    }
    let provider = normalize_agent_provider(&input.provider)?;
    let sandbox = normalize_agent_sandbox(&input.sandbox)?;
    let context_mode = normalize_agent_context_mode(input.context_mode.as_deref())?;
    let model = input
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let reasoning_effort = normalize_agent_reasoning_effort(input.reasoning_effort.as_deref())?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.create_agent_profile(store::AgentProfileCreate {
        workspace_id: input.workspace_id,
        project_id: input.project_id,
        name,
        provider,
        model,
        reasoning_effort,
        sandbox,
        context_mode,
        rtk_enabled: input.rtk_enabled.unwrap_or(false),
    })?)
}

#[tauri::command]
fn update_agent_profile(
    app: tauri::AppHandle,
    input: AgentProfileUpdateInput,
) -> AppResult<store::AgentProfile> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow::anyhow!("agent name is required").into());
    }
    let provider = normalize_agent_provider(&input.provider)?;
    let sandbox = normalize_agent_sandbox(&input.sandbox)?;
    let context_mode = normalize_agent_context_mode(input.context_mode.as_deref())?;
    let model = input
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let reasoning_effort = normalize_agent_reasoning_effort(input.reasoning_effort.as_deref())?;
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let existing = db.get_agent_profile(input.id)?;
    Ok(db.update_agent_profile(store::AgentProfileUpdate {
        id: input.id,
        name,
        provider,
        model,
        reasoning_effort,
        sandbox,
        context_mode,
        rtk_enabled: input.rtk_enabled.unwrap_or(existing.rtk_enabled),
    })?)
}

#[tauri::command]
fn list_agent_sessions(
    app: tauri::AppHandle,
    workspace_id: i64,
    project_id: Option<i64>,
) -> AppResult<Vec<store::AgentSession>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_agent_sessions(workspace_id, project_id)?)
}

#[tauri::command]
fn list_agent_messages(
    app: tauri::AppHandle,
    session_id: i64,
) -> AppResult<Vec<store::AgentMessage>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_agent_messages(session_id)?)
}

#[tauri::command]
fn reset_agent_chat(
    app: tauri::AppHandle,
    input: AgentChatResetInput,
) -> AppResult<store::AgentSession> {
    let project_root = canonical_project_root(&PathBuf::from(&input.project_path))?;
    let project_path = project_path_key(&project_root);
    let db = store::Database::open(&app_data_dir(&app)?)?;
    for session_id in db.running_agent_session_ids_for_profile(
        input.profile_id,
        input.workspace_id,
        input.project_id,
    )? {
        agent::stop_agent_process(session_id);
        let _ = db.update_agent_session_status(session_id, "stopped", None);
    }
    Ok(db.reset_agent_chat(
        input.profile_id,
        input.workspace_id,
        input.project_id,
        &project_path,
    )?)
}

#[tauri::command]
fn send_agent_message(
    app: tauri::AppHandle,
    input: AgentMessageInput,
) -> AppResult<store::AgentSession> {
    let message = input.message.trim();
    if message.is_empty() {
        return Err(anyhow::anyhow!("agent message is required").into());
    }
    let project_root = canonical_project_root(&PathBuf::from(&input.project_path))?;
    let project_path = project_path_key(&project_root);
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let profile = db.get_agent_profile(input.profile_id)?;
    let scope = normalize_agent_session_scope(input.scope.as_deref())?;
    let requirement_card_id = if scope == "card_interview" {
        Some(
            input
                .requirement_card_id
                .ok_or_else(|| anyhow::anyhow!("requirement card is required for interview"))?,
        )
    } else {
        None
    };
    if profile.workspace_id != input.workspace_id {
        return Err(
            anyhow::anyhow!("agent profile does not belong to the active workspace").into(),
        );
    }
    let session = if let Some(session_id) = input.session_id {
        let session = db.get_agent_session(session_id)?;
        let session_matches = if scope == "chat" {
            store::agent_session_matches_profile_context(
                &session,
                &profile,
                input.workspace_id,
                input.project_id,
                &project_path,
            )
        } else {
            store::agent_session_matches_profile_context_for_scope(
                &session,
                &profile,
                input.workspace_id,
                input.project_id,
                &project_path,
                scope,
                requirement_card_id,
            )
        };
        if session_matches {
            session
        } else {
            create_scoped_agent_session(
                &db,
                &profile,
                input.project_id,
                &project_path,
                input.title.as_deref().unwrap_or(message),
                scope,
                requirement_card_id,
            )?
        }
    } else if scope == "card_interview" {
        if let Some(session) = db.find_card_interview_session(
            profile.id,
            input.workspace_id,
            input.project_id,
            requirement_card_id.expect("checked above"),
            &project_path,
        )? {
            session
        } else {
            create_scoped_agent_session(
                &db,
                &profile,
                input.project_id,
                &project_path,
                input.title.as_deref().unwrap_or(message),
                scope,
                requirement_card_id,
            )?
        }
    } else if scope != "chat" {
        create_scoped_agent_session(
            &db,
            &profile,
            input.project_id,
            &project_path,
            input.title.as_deref().unwrap_or(message),
            scope,
            requirement_card_id,
        )?
    } else {
        db.create_agent_session(
            &profile,
            input.project_id,
            &project_path,
            input.title.as_deref().unwrap_or(message),
        )?
    };
    db.add_agent_message(session.id, "user", message, None)?;
    let metadata = input.skill.as_ref().and_then(|skill| {
        serde_json::to_value(skill)
            .ok()
            .map(|skill| serde_json::json!({ "skill": skill }))
    });
    agent::spawn_agent_message(app, db, session.clone(), message.to_string(), metadata);
    Ok(session)
}

#[tauri::command]
fn stop_agent_session(app: tauri::AppHandle, session_id: i64) -> AppResult<store::AgentSession> {
    agent::stop_agent_process(session_id);
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.update_agent_session_status(session_id, "stopped", None)?)
}

#[tauri::command]
fn agent_usage(provider: String, project_path: String) -> AppResult<agent::AgentUsage> {
    Ok(agent::agent_usage(&provider, &project_path)?)
}

#[tauri::command]
fn check_agent_provider_health(
    provider: String,
    project_path: String,
) -> AppResult<agent::AgentProviderHealth> {
    Ok(agent::check_provider_health(&provider, &project_path)?)
}

#[tauri::command]
fn list_agent_run_metrics(
    app: tauri::AppHandle,
    session_id: i64,
) -> AppResult<Vec<store::AgentRunEvent>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.list_agent_run_events(session_id)?)
}

#[derive(Debug, Deserialize)]
struct WarmAgentRuntimeInput {
    profile_id: i64,
    workspace_id: i64,
    project_id: Option<i64>,
    project_path: String,
    scope: Option<String>,
    provider_session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RtkStatusInput {
    profile_id: Option<i64>,
    project_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RtkConfigureInput {
    profile_id: i64,
    project_path: String,
    apply: bool,
}

#[derive(Debug, Deserialize)]
struct RtkInstallInput {
    profile_id: Option<i64>,
    project_path: Option<String>,
}

#[tauri::command]
fn get_rtk_status(app: tauri::AppHandle, input: RtkStatusInput) -> AppResult<rtk::RtkStatus> {
    let enabled = if let Some(profile_id) = input.profile_id {
        let db = store::Database::open(&app_data_dir(&app)?)?;
        db.get_agent_profile(profile_id)
            .map(|profile| profile.rtk_enabled)
            .unwrap_or(false)
    } else {
        false
    };
    Ok(rtk::status(&app, enabled, input.project_path.as_deref()))
}

#[tauri::command]
fn install_rtk(app: tauri::AppHandle, input: RtkInstallInput) -> AppResult<rtk::RtkInstallResult> {
    let enabled = if let Some(profile_id) = input.profile_id {
        let db = store::Database::open(&app_data_dir(&app)?)?;
        db.get_agent_profile(profile_id)
            .map(|profile| profile.rtk_enabled)
            .unwrap_or(false)
    } else {
        false
    };
    Ok(rtk::ensure_installed(
        &app,
        enabled,
        input.project_path.as_deref(),
    )?)
}

#[tauri::command]
fn configure_rtk(
    app: tauri::AppHandle,
    input: RtkConfigureInput,
) -> AppResult<rtk::RtkSetupResult> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let profile = db.get_agent_profile(input.profile_id)?;
    Ok(rtk::configure_profile(
        &app,
        &profile.provider,
        &input.project_path,
        profile.rtk_enabled,
        input.apply,
    )?)
}

#[tauri::command]
fn warm_agent_runtime(
    app: tauri::AppHandle,
    input: WarmAgentRuntimeInput,
) -> AppResult<agent::AgentProviderHealth> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let profile = db.get_agent_profile(input.profile_id)?;
    if profile.workspace_id != input.workspace_id
        || (profile.project_id.is_some() && profile.project_id != input.project_id)
    {
        return Err(anyhow::anyhow!("agent profile does not match active context").into());
    }
    let scope = normalize_agent_session_scope(input.scope.as_deref())?;
    let health = agent::check_provider_health(&profile.provider, &input.project_path)?;
    if health.ok && profile.provider == "claude" {
        agent::warm_runtime(
            &store::AgentSession {
                id: 0,
                profile_id: profile.id,
                workspace_id: profile.workspace_id,
                project_id: input.project_id,
                requirement_card_id: None,
                scope: scope.to_string(),
                project_path: input.project_path.clone(),
                provider: profile.provider.clone(),
                model: profile.model.clone(),
                reasoning_effort: profile.reasoning_effort.clone(),
                sandbox: profile.sandbox.clone(),
                context_mode: profile.context_mode.clone(),
                provider_session_id: input.provider_session_id,
                codex_session_id: None,
                status: "warming".to_string(),
                title: String::new(),
                created_at: String::new(),
                updated_at: String::new(),
            },
            &app,
            profile.rtk_enabled,
        )?;
    }
    Ok(health)
}

fn create_scoped_agent_session(
    db: &store::Database,
    profile: &store::AgentProfile,
    project_id: Option<i64>,
    project_path: &str,
    title: &str,
    scope: &str,
    requirement_card_id: Option<i64>,
) -> anyhow::Result<store::AgentSession> {
    db.create_agent_session_scoped(
        profile,
        project_id,
        project_path,
        title,
        scope,
        requirement_card_id,
    )
}

#[tauri::command]
fn git_status(path: String) -> AppResult<String> {
    Ok(git::status(PathBuf::from(path).as_path())?)
}

#[tauri::command]
fn git_diff(path: String) -> AppResult<String> {
    Ok(git::diff(PathBuf::from(path).as_path())?)
}

#[tauri::command]
fn git_staged_diff(path: String) -> AppResult<String> {
    Ok(git::staged_diff(PathBuf::from(path).as_path())?)
}

#[tauri::command]
fn git_log_graph(path: String) -> AppResult<String> {
    Ok(git::log_graph(PathBuf::from(path).as_path())?)
}

#[tauri::command]
fn git_blame(path: String, file_path: String) -> AppResult<String> {
    Ok(git::blame(PathBuf::from(path).as_path(), &file_path)?)
}

#[tauri::command]
fn git_blame_porcelain(path: String, file_path: String) -> AppResult<Vec<git::BlameLine>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::blame_porcelain(&root, &file_path)?)
}

#[tauri::command]
fn git_blame_porcelain_for_content(
    path: String,
    file_path: String,
    content: String,
) -> AppResult<Vec<git::BlameLine>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::blame_porcelain_for_contents(
        &root, &file_path, &content,
    )?)
}

#[tauri::command]
fn list_changed_files(path: String) -> AppResult<Vec<git::ChangedFile>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::changed_files(&root)?)
}

#[tauri::command]
fn git_worktree_fingerprint(path: String) -> AppResult<git::GitWorktreeFingerprint> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::worktree_fingerprint(&root)?)
}

#[tauri::command]
fn git_worktree_snapshot(
    path: String,
    untracked_limit: u32,
) -> AppResult<git::GitWorktreeSnapshot> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::worktree_snapshot(&root, untracked_limit)?)
}

#[tauri::command]
fn read_file_patch(path: String, file_path: String, area: String) -> AppResult<git::FilePatch> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::file_patch(&root, &file_path, &area)?)
}

#[tauri::command]
fn git_file_patch_text(path: String, file_path: String, area: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::file_patch_text(&root, &file_path, &area)?)
}

#[tauri::command]
fn stage_file(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stage_file(&root, &file_path)?)
}

#[tauri::command]
fn unstage_file(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::unstage_file(&root, &file_path)?)
}

#[tauri::command]
fn git_stage_all(path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stage_all(&root)?)
}

#[tauri::command]
fn git_unstage_all(path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::unstage_all(&root)?)
}

#[tauri::command]
fn stage_hunk(path: String, hunk_patch: String) -> AppResult<git::PatchCheckResult> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stage_hunk(&root, &hunk_patch)?)
}

#[tauri::command]
fn unstage_hunk(path: String, hunk_patch: String) -> AppResult<git::PatchCheckResult> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::unstage_hunk(&root, &hunk_patch)?)
}

#[tauri::command]
fn check_imported_patch(path: String, patch: String) -> AppResult<git::PatchCheckResult> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::check_patch(&root, &patch)?)
}

#[tauri::command]
fn apply_imported_patch(path: String, patch: String) -> AppResult<git::PatchCheckResult> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::apply_patch(&root, &patch)?)
}

#[tauri::command]
fn git_commit_graph(
    path: String,
    include_remotes: bool,
    include_tags: bool,
    limit: u32,
    skip: u32,
) -> AppResult<Vec<git::Commit>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::commit_graph(
        &root,
        include_remotes,
        include_tags,
        limit,
        skip,
    )?)
}

#[tauri::command]
fn git_repo_snapshot(
    path: String,
    include_remotes: bool,
    include_tags: bool,
    limit: u32,
) -> AppResult<git::GitRepoSnapshot> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::repo_snapshot(
        &root,
        include_remotes,
        include_tags,
        limit,
    )?)
}

#[tauri::command]
fn git_commit_detail(path: String, sha: String) -> AppResult<git::CommitDetail> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::commit_detail(&root, &sha)?)
}

#[tauri::command]
fn git_log_file(path: String, file_path: String, limit: u32) -> AppResult<Vec<git::Commit>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::log_file(&root, &file_path, limit)?)
}

#[tauri::command]
fn git_show_file(path: String, sha: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::show_file(&root, &sha, &file_path)?)
}

#[tauri::command]
fn git_commit_file_diff(path: String, sha: String, file_path: String) -> AppResult<git::FilePatch> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::commit_file_diff(&root, &sha, &file_path)?)
}

#[tauri::command]
fn git_repo_state(path: String) -> AppResult<git::RepoState> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::repo_state(&root)?)
}

#[tauri::command]
fn git_list_branches(path: String) -> AppResult<Vec<git::Branch>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::list_branches(&root)?)
}

#[tauri::command]
fn git_list_remote_branches(path: String) -> AppResult<Vec<git::RemoteBranch>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::list_remote_branches(&root)?)
}

#[tauri::command]
fn git_list_tags(path: String) -> AppResult<Vec<git::TagEntry>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::list_tags(&root)?)
}

#[tauri::command]
fn git_list_stashes(path: String) -> AppResult<Vec<git::StashEntry>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::list_stashes(&root)?)
}

#[tauri::command]
fn git_stash_detail(path: String, index: u32) -> AppResult<git::CommitDetail> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_detail(&root, index)?)
}

#[tauri::command]
fn git_stash_file_diff(path: String, index: u32, file_path: String) -> AppResult<git::FilePatch> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_file_diff(&root, index, &file_path)?)
}

#[tauri::command]
fn git_commit(path: String, message: String, amend: bool) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::commit(&root, &message, amend)?)
}

#[tauri::command]
fn git_fetch(path: String, remote: Option<String>) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::fetch(&root, remote.as_deref())?)
}

#[tauri::command]
fn git_pull(path: String, rebase: bool) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::pull(&root, rebase)?)
}

#[tauri::command]
fn git_push(path: String, set_upstream: bool, force_with_lease: bool) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::push(&root, set_upstream, force_with_lease)?)
}

#[tauri::command]
fn git_checkout_branch(path: String, name: String, mode: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::checkout_branch(&root, &name, &mode)?)
}

#[tauri::command]
fn git_create_branch(
    path: String,
    name: String,
    start_point: Option<String>,
    checkout: bool,
) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::create_branch(
        &root,
        &name,
        start_point.as_deref(),
        checkout,
    )?)
}

#[tauri::command]
fn git_rename_branch(path: String, old_name: String, new_name: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::rename_branch(&root, &old_name, &new_name)?)
}

#[tauri::command]
fn git_delete_branch(path: String, name: String, force: bool) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::delete_branch(&root, &name, force)?)
}

#[tauri::command]
fn git_merge_branch(path: String, name: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::merge_branch(&root, &name)?)
}

#[tauri::command]
fn git_rebase_branch(path: String, onto: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::rebase_branch(&root, &onto)?)
}

#[tauri::command]
fn git_cherry_pick(path: String, sha: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::cherry_pick(&root, &sha)?)
}

#[tauri::command]
fn git_revert(path: String, sha: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::revert(&root, &sha)?)
}

#[tauri::command]
fn git_reset(path: String, sha: String, mode: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::reset(&root, &sha, &mode)?)
}

#[tauri::command]
fn git_create_tag(
    path: String,
    name: String,
    sha: Option<String>,
    message: Option<String>,
) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::create_tag(
        &root,
        &name,
        sha.as_deref(),
        message.as_deref(),
    )?)
}

#[tauri::command]
fn git_abort_operation(path: String, operation: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::abort_operation(&root, &operation)?)
}

#[tauri::command]
fn git_delete_tag(path: String, name: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::delete_tag(&root, &name)?)
}

#[tauri::command]
fn git_stash_save(
    path: String,
    message: Option<String>,
    include_untracked: bool,
) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_save(
        &root,
        message.as_deref(),
        include_untracked,
    )?)
}

#[tauri::command]
fn git_stash_file(path: String, file_path: String, message: Option<String>) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_file(&root, &file_path, message.as_deref())?)
}

#[tauri::command]
fn git_ignore_file(path: String, file_path: String, target: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::ignore_file(&root, &file_path, &target)?)
}

#[tauri::command]
fn git_external_diff(path: String, file_path: String, area: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::external_diff(&root, &file_path, &area)?)
}

#[tauri::command]
fn git_stash_pop(path: String, index: u32) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_pop(&root, index)?)
}

#[tauri::command]
fn git_stash_apply(path: String, index: u32) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_apply(&root, index)?)
}

#[tauri::command]
fn git_stash_drop(path: String, index: u32) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::stash_drop(&root, index)?)
}

#[tauri::command]
fn git_checkout_commit(path: String, sha: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::checkout_commit(&root, &sha)?)
}

#[tauri::command]
fn git_use_ours(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::use_ours(&root, &file_path)?)
}

#[tauri::command]
fn git_use_theirs(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::use_theirs(&root, &file_path)?)
}

#[tauri::command]
fn git_mark_resolved(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::mark_resolved(&root, &file_path)?)
}

#[tauri::command]
fn git_continue_operation(path: String, operation: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::continue_operation(&root, &operation)?)
}

#[tauri::command]
fn git_list_submodules(path: String) -> AppResult<Vec<git::Submodule>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::list_submodules(&root)?)
}

#[tauri::command]
fn git_update_submodule(path: String, sub_path: String, init: bool) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::update_submodule(&root, &sub_path, init)?)
}

#[tauri::command]
fn git_start_interactive_rebase(
    path: String,
    base: String,
    steps: Vec<git::RebaseStep>,
) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let exe = std::env::current_exe()
        .map_err(|err| anyhow::anyhow!("cannot resolve current exe: {err}"))?;
    // git runs the sequence editor via a shell, so quote the path.
    let editor_cmd = format!("\"{}\" --dwgui-rebase-editor", exe.display());
    Ok(git::start_interactive_rebase(
        &root,
        &base,
        &steps,
        &editor_cmd,
    )?)
}

#[tauri::command]
fn git_discard_file(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::discard_file(&root, &file_path)?)
}

#[tauri::command]
fn git_discard_hunk(path: String, hunk_patch: String) -> AppResult<git::PatchCheckResult> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(git::discard_hunk(&root, &hunk_patch)?)
}

#[tauri::command]
fn open_project_file(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let file_path = scoped_child_path_allow_missing(&root, &file_path)?;
    if !file_path.exists() {
        return Err(anyhow::anyhow!("file does not exist: {}", file_path.display()).into());
    }
    open_system_path(&file_path)?;
    Ok(format!("Opened {}", file_path.display()))
}

#[tauri::command]
fn reveal_project_file(path: String, file_path: String) -> AppResult<String> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let file_path = scoped_child_path_allow_missing(&root, &file_path)?;
    reveal_system_path(&file_path)?;
    Ok(format!("Revealed {}", file_path.display()))
}

#[tauri::command]
fn run_shell(path: String, command: String) -> AppResult<String> {
    Ok(shell::run_shell(PathBuf::from(path).as_path(), &command)?)
}

#[tauri::command]
fn create_terminal_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, terminal::TerminalManager>,
    path: String,
    shell: Option<String>,
    initial_input: Option<String>,
) -> AppResult<terminal::TerminalSession> {
    Ok(terminal::create_session(
        app,
        state.inner().clone(),
        PathBuf::from(path),
        shell,
        initial_input,
    )?)
}

#[tauri::command]
fn write_terminal_input(
    state: tauri::State<'_, terminal::TerminalManager>,
    session_id: String,
    data: String,
) -> AppResult<()> {
    Ok(state.write_input(&session_id, &data)?)
}

#[tauri::command]
fn resize_terminal_session(
    state: tauri::State<'_, terminal::TerminalManager>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> AppResult<()> {
    Ok(state.resize(&session_id, cols, rows)?)
}

#[tauri::command]
fn stop_terminal_session(
    state: tauri::State<'_, terminal::TerminalManager>,
    session_id: String,
) -> AppResult<terminal::TerminalSession> {
    Ok(state.stop(&session_id)?)
}

#[tauri::command]
fn close_terminal_session(
    state: tauri::State<'_, terminal::TerminalManager>,
    session_id: String,
) -> AppResult<()> {
    Ok(state.close(&session_id)?)
}

#[tauri::command]
fn list_terminal_sessions(
    state: tauri::State<'_, terminal::TerminalManager>,
) -> AppResult<Vec<terminal::TerminalSession>> {
    Ok(state.list_sessions()?)
}

#[tauri::command]
fn list_source_tree(path: String) -> AppResult<Vec<SourceEntry>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let mut entries = collect_source_entries(&root, &root, true)?;
    sort_source_entries(&mut entries);
    Ok(entries)
}

#[tauri::command]
fn read_source_file(path: String, relative_path: String) -> AppResult<SourceFile> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let file_path = scoped_child_path(&root, &relative_path)?;
    let metadata = std::fs::metadata(&file_path).map_err(anyhow::Error::from)?;

    if !metadata.is_file() {
        return Err(anyhow::anyhow!("source path is not a file").into());
    }

    if metadata.len() > MAX_SOURCE_FILE_BYTES {
        return Err(anyhow::anyhow!("source file exceeds 1 MiB read limit").into());
    }

    if is_binary_file(&file_path)? {
        return Err(anyhow::anyhow!("source file appears to be binary").into());
    }

    let content = std::fs::read_to_string(&file_path).map_err(anyhow::Error::from)?;
    let name = file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.clone());

    Ok(SourceFile {
        relative_path,
        name,
        extension: file_extension(&file_path),
        bytes: metadata.len(),
        content,
    })
}

#[tauri::command]
fn write_source_file(
    path: String,
    relative_path: String,
    content: String,
) -> AppResult<SourceFile> {
    if content.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err(anyhow::anyhow!("source file exceeds 1 MiB write limit").into());
    }

    let root = canonical_project_root(&PathBuf::from(path))?;
    let file_path = scoped_child_path(&root, &relative_path)?;
    let metadata = std::fs::metadata(&file_path).map_err(anyhow::Error::from)?;
    if !metadata.is_file() {
        return Err(anyhow::anyhow!("source path is not a file").into());
    }
    if is_binary_file(&file_path)? {
        return Err(anyhow::anyhow!("source file appears to be binary").into());
    }

    std::fs::write(&file_path, content.as_bytes()).map_err(anyhow::Error::from)?;
    let metadata = std::fs::metadata(&file_path).map_err(anyhow::Error::from)?;
    Ok(SourceFile {
        relative_path,
        name: file_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.display().to_string()),
        extension: file_extension(&file_path),
        bytes: metadata.len(),
        content,
    })
}

/// Caps to keep a project-wide search responsive and the payload bounded.
const SEARCH_MAX_FILES: usize = 800;
const SEARCH_MAX_MATCHES_PER_FILE: usize = 200;
const SEARCH_MAX_TOTAL_MATCHES: usize = 5000;
const SEARCH_LINE_CAP: usize = 500;

/// Build a regex honoring the case-sensitive / whole-word / regex toggles.
fn build_search_regex(
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
    use_regex: bool,
) -> anyhow::Result<regex::Regex> {
    let mut pattern = if use_regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    if whole_word {
        pattern = format!(r"\b(?:{pattern})\b");
    }
    regex::RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .map_err(anyhow::Error::from)
}

/// Find every match in `content`, one entry per match (line may repeat).
fn search_lines(content: &str, regex: &regex::Regex) -> Vec<SearchMatch> {
    let mut matches = Vec::new();
    for (index, line) in content.lines().enumerate() {
        for found in regex.find_iter(line) {
            if found.is_empty() {
                continue;
            }
            let col = line[..found.start()].chars().count() as u32;
            let length = found.as_str().chars().count() as u32;
            let text: String = line.chars().take(SEARCH_LINE_CAP).collect();
            matches.push(SearchMatch {
                line: index as u32 + 1,
                col,
                length,
                text,
            });
            if matches.len() >= SEARCH_MAX_MATCHES_PER_FILE {
                return matches;
            }
        }
    }
    matches
}

fn collect_searchable_files(
    root: &Path,
    current: &Path,
    is_root: bool,
    out: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if should_exclude_source_dir(root, &path, &name, is_root) {
                continue;
            }
            collect_searchable_files(root, &path, false, out)?;
        } else if path.is_file() && should_include_source_file(&path, &name, is_root) {
            out.push(path);
        }
        if out.len() >= SEARCH_MAX_FILES {
            return Ok(());
        }
    }
    Ok(())
}

#[tauri::command]
fn search_in_files(
    path: String,
    query: String,
    case_sensitive: bool,
    whole_word: bool,
    use_regex: bool,
) -> AppResult<Vec<SearchFileResult>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let root = canonical_project_root(&PathBuf::from(path))?;
    let regex = build_search_regex(&query, case_sensitive, whole_word, use_regex)?;

    let mut files = Vec::new();
    collect_searchable_files(&root, &root, true, &mut files)?;
    files.sort();

    let mut results = Vec::new();
    let mut total = 0usize;
    for file in files {
        if total >= SEARCH_MAX_TOTAL_MATCHES {
            break;
        }
        let metadata = match std::fs::metadata(&file) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.len() > MAX_SOURCE_FILE_BYTES || is_binary_file(&file).unwrap_or(true) {
            continue;
        }
        let content = match std::fs::read_to_string(&file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let matches = search_lines(&content, &regex);
        if matches.is_empty() {
            continue;
        }
        total += matches.len();
        results.push(SearchFileResult {
            relative_path: normalize_relative_path(&root, &file)?,
            matches,
        });
    }
    Ok(results)
}

/// Validate a relative path for a file/dir that does not exist yet (canonicalize
/// can't be used on a missing path). Rejects absolute paths and `..` segments.
fn scoped_new_path(root: &Path, relative_path: &str) -> anyhow::Result<PathBuf> {
    let rel = Path::new(relative_path);
    if rel.is_absolute() {
        return Err(anyhow::anyhow!("path must be relative"));
    }
    for component in rel.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => return Err(anyhow::anyhow!("path contains an invalid segment")),
        }
    }
    let joined = root.join(rel);
    if !joined.starts_with(root) {
        return Err(anyhow::anyhow!("source path escapes project root"));
    }
    Ok(joined)
}

#[tauri::command]
fn create_source_file(path: String, relative_path: String) -> AppResult<SourceFile> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let file_path = scoped_new_path(&root, &relative_path)?;
    if !is_supported_source_file(&file_path) {
        return Err(anyhow::anyhow!("unsupported file extension").into());
    }
    if file_path.exists() {
        return Err(anyhow::anyhow!("file already exists").into());
    }
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(anyhow::Error::from)?;
    }
    std::fs::write(&file_path, "").map_err(anyhow::Error::from)?;
    let name = file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.clone());
    Ok(SourceFile {
        relative_path,
        name,
        extension: file_extension(&file_path),
        bytes: 0,
        content: String::new(),
    })
}

/// Does a project-relative path (file OR dir, inside or outside `.dw/`) exist?
/// Used as a generic per-flow "already analyzed" marker check.
fn project_path_exists_impl(root: &Path, relative_path: &str) -> bool {
    match std::fs::canonicalize(root.join(relative_path)) {
        Ok(resolved) => resolved.starts_with(root),
        Err(_) => false,
    }
}

#[tauri::command]
fn project_path_exists(path: String, relative_path: String) -> AppResult<bool> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(project_path_exists_impl(&root, &relative_path))
}

#[tauri::command]
fn create_source_dir(path: String, relative_path: String) -> AppResult<()> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let dir_path = scoped_new_path(&root, &relative_path)?;
    if dir_path.exists() {
        return Err(anyhow::anyhow!("directory already exists").into());
    }
    std::fs::create_dir_all(&dir_path).map_err(anyhow::Error::from)?;
    Ok(())
}

/// Temp-file extension for the external formatter of a dw language, if any.
fn external_formatter_ext(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some("rs"),
        "python" => Some("py"),
        "csharp" => Some("cs"),
        _ => None,
    }
}

/// Run an in-place formatter; Ok(None) means the binary is not installed.
fn run_formatter_cmd(
    program: &str,
    args: &[String],
) -> anyhow::Result<Option<std::process::Output>> {
    match std::process::Command::new(program).args(args).output() {
        Ok(output) => Ok(Some(output)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Format `content` via an external in-place formatter (rustfmt / ruff|black /
/// csharpier) using a temp file. Returns a friendly error if no tool is found.
fn format_external_impl(language: &str, content: &str) -> anyhow::Result<String> {
    let ext = external_formatter_ext(language)
        .ok_or_else(|| anyhow::anyhow!("no external formatter for {language}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("dwgui-fmt-{stamp}"));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(format!("buffer.{ext}"));
    std::fs::write(&file, content)?;
    let target = file.to_string_lossy().to_string();

    let candidates: Vec<(&str, Vec<String>)> = match language {
        "rust" => vec![("rustfmt", vec![target.clone()])],
        "python" => vec![
            ("ruff", vec!["format".into(), target.clone()]),
            ("black", vec!["-q".into(), target.clone()]),
        ],
        "csharp" => vec![
            ("csharpier", vec!["format".into(), target.clone()]),
            ("csharpier", vec![target.clone()]),
        ],
        _ => vec![],
    };

    let mut tool_error: Option<String> = None;
    let mut formatted: Option<String> = None;
    for (program, args) in &candidates {
        match run_formatter_cmd(program, args)? {
            None => continue,
            Some(output) if output.status.success() => {
                formatted = Some(std::fs::read_to_string(&file)?);
                break;
            }
            Some(output) => {
                tool_error = Some(format!(
                    "{program}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);

    match formatted {
        Some(result) => Ok(result),
        None => Err(anyhow::anyhow!(
            "no formatter available for {language}{}",
            tool_error
                .map(|error| format!(" ({error})"))
                .unwrap_or_default()
        )),
    }
}

#[tauri::command]
fn format_external(language: String, content: String) -> AppResult<String> {
    if content.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err(anyhow::anyhow!("source exceeds 1 MiB format limit").into());
    }
    Ok(format_external_impl(&language, &content)?)
}

#[tauri::command]
fn lsp_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, lsp::LspManager>,
    language: String,
    cwd: String,
) -> AppResult<u32> {
    Ok(state.start(&app, &language, &cwd)?)
}

#[tauri::command]
fn lsp_send(state: tauri::State<'_, lsp::LspManager>, id: u32, message: String) -> AppResult<()> {
    Ok(state.send(id, &message)?)
}

#[tauri::command]
fn lsp_stop(state: tauri::State<'_, lsp::LspManager>, id: u32) -> AppResult<()> {
    Ok(state.stop(id)?)
}

#[derive(Debug, Serialize)]
struct LspServerStatus {
    language: String,
    program: String,
    installed: bool,
    can_install: bool,
}

#[tauri::command]
fn lsp_server_status(language: String) -> AppResult<Option<LspServerStatus>> {
    let Some((program, _)) = lsp::server_command(&language) else {
        return Ok(None);
    };
    let installed = lsp::binary_on_path(program);
    let can_install = lsp::install_prereq(&language)
        .map(lsp::binary_on_path)
        .unwrap_or(false);
    Ok(Some(LspServerStatus {
        language,
        program: program.to_string(),
        installed,
        can_install,
    }))
}

#[tauri::command]
fn lsp_install(language: String) -> AppResult<String> {
    Ok(lsp::install_server(&language)?)
}

#[tauri::command]
fn list_dw_artifacts(path: String) -> AppResult<Vec<DwArtifact>> {
    let project_path = PathBuf::from(path);
    let dw_path = project_path.join(".dw");
    if !dw_path.exists() {
        return Ok(Vec::new());
    }

    let mut artifacts = Vec::new();
    collect_dw_artifacts(&dw_path, &dw_path, &mut artifacts)?;
    artifacts.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(artifacts)
}

const FETCH_URL_CHAR_BUDGET: usize = 60_000;

/// Find the earliest `<script` / `<style` opening in `haystack` and the matching
/// close tag to look for. Returns (byte offset of `<`, close-tag literal).
fn find_html_block_start(haystack: &str) -> Option<(usize, &'static str)> {
    let script = haystack.find("<script").map(|pos| (pos, "</script>"));
    let style = haystack.find("<style").map(|pos| (pos, "</style>"));
    match (script, style) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Drop `<script>`/`<style>` regions (case-insensitive). ASCII-lowercasing keeps
/// byte length, so indices from the lowercased copy align with the original.
fn remove_html_blocks(body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    let mut out = String::with_capacity(body.len());
    let mut i = 0usize;
    while i < body.len() {
        match find_html_block_start(&lower[i..]) {
            Some((rel, close)) => {
                let start = i + rel;
                out.push_str(&body[i..start]);
                match lower[start..].find(close) {
                    Some(end) => i = start + end + close.len(),
                    None => break, // unterminated block: drop the rest
                }
            }
            None => {
                out.push_str(&body[i..]);
                break;
            }
        }
    }
    out
}

/// Turn an HTML page into compact text for an agent prompt: strip script/style,
/// remove remaining tags, collapse whitespace, and cap the character count so
/// the prompt's token cost stays bounded.
fn strip_and_truncate_html(body: &str, budget: usize) -> String {
    let without_blocks = remove_html_blocks(body);
    let mut text = String::with_capacity(without_blocks.len());
    let mut in_tag = false;
    for ch in without_blocks.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > budget {
        let truncated: String = collapsed.chars().take(budget).collect();
        format!("{truncated}\n…[truncated]")
    } else {
        collapsed
    }
}

/// Fetch an http(s) URL and return its content as compact text (for building a
/// flow from a tool's docs). Pure-Rust TLS via ureq/rustls.
#[tauri::command]
fn fetch_url(url: String) -> AppResult<String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(anyhow::anyhow!("only http(s) URLs are supported").into());
    }
    let response = ureq::get(trimmed)
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|err| anyhow::anyhow!("fetch failed: {err}"))?;
    let body = response
        .into_string()
        .map_err(|err| anyhow::anyhow!("read body failed: {err}"))?;
    Ok(strip_and_truncate_html(&body, FETCH_URL_CHAR_BUDGET))
}

#[tauri::command]
fn read_dw_artifact(path: String, relative_path: String) -> AppResult<String> {
    let project_path = PathBuf::from(path);
    let dw_path = project_path.join(".dw");
    let artifact_path = dw_path.join(&relative_path);
    let canonical_dw = std::fs::canonicalize(&dw_path).map_err(anyhow::Error::from)?;
    let canonical_artifact = std::fs::canonicalize(&artifact_path).map_err(anyhow::Error::from)?;

    if !canonical_artifact.starts_with(canonical_dw) {
        return Err(anyhow::anyhow!("artifact path escapes .dw").into());
    }

    Ok(std::fs::read_to_string(canonical_artifact).map_err(anyhow::Error::from)?)
}

#[tauri::command]
fn write_dw_artifact(path: String, input: DwArtifactWriteInput) -> AppResult<DwArtifact> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let relative_path = normalize_new_dw_relative_path(&input.relative_path)?;
    if !is_supported_artifact(&relative_path) {
        return Err(anyhow::anyhow!("unsupported artifact extension").into());
    }

    let dw_path = root.join(".dw");
    std::fs::create_dir_all(&dw_path).map_err(anyhow::Error::from)?;
    let artifact_path = dw_path.join(&relative_path);
    if let Some(parent) = artifact_path.parent() {
        std::fs::create_dir_all(parent).map_err(anyhow::Error::from)?;
    }
    std::fs::write(&artifact_path, input.content).map_err(anyhow::Error::from)?;

    let normalized = normalize_relative_path(&dw_path, &artifact_path)?;
    let metadata = std::fs::metadata(&artifact_path).map_err(anyhow::Error::from)?;
    Ok(DwArtifact {
        category: artifact_category(&normalized).to_string(),
        name: artifact_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| normalized.clone()),
        relative_path: normalized,
        bytes: metadata.len(),
    })
}

#[tauri::command]
fn list_dw_commands(path: String) -> AppResult<Vec<DwCommand>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let commands_path = root.join(".dw").join("commands");
    if !commands_path.exists() {
        return Ok(Vec::new());
    }

    let mut commands = Vec::new();
    for entry in std::fs::read_dir(commands_path).map_err(anyhow::Error::from)? {
        let entry = entry.map_err(anyhow::Error::from)?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Any `*.md` under .dw/commands/ is a command (dw-*, speckit.*, custom),
        // so the builder palette can surface non-dev-workflow slash commands.
        if !path.is_file() || !name.ends_with(".md") {
            continue;
        }

        let command_name = name.trim_end_matches(".md").to_string();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        commands.push(DwCommand {
            name: command_name.clone(),
            command: format!("/{command_name}"),
            relative_path: normalize_relative_path(&root, &path)?,
            title: first_markdown_heading(&content).unwrap_or_else(|| command_name.clone()),
            description: first_use_bullet(&content),
        });
    }

    commands.sort_by(|left, right| left.command.cmp(&right.command));
    Ok(commands)
}

#[tauri::command]
fn list_dw_skills(path: String) -> AppResult<Vec<DwSkill>> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    let managed_files = managed_files(&root);
    let mut skills = BTreeMap::<String, DwSkill>::new();

    let registry_path = root.join(".dw").join("skill-registry.json");
    if registry_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&registry_path) {
            if let Ok(value) = serde_json::from_str::<Value>(&content) {
                if let Some(items) = value.get("skills").and_then(Value::as_array) {
                    for item in items {
                        let Some(name) = json_string(item, "name") else {
                            continue;
                        };
                        skills.insert(
                            name.clone(),
                            DwSkill {
                                name,
                                description: None,
                                kind: json_string(item, "kind"),
                                tier: json_string(item, "tier"),
                                owner: json_string(item, "owner"),
                                trigger: json_string(item, "trigger"),
                                path: None,
                                source: if item
                                    .get("bundled")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false)
                                {
                                    "bundled".to_string()
                                } else {
                                    "custom".to_string()
                                },
                            },
                        );
                    }
                }
            }
        }
    }

    let skills_root = root.join(".agents").join("skills");
    let mut skill_files = Vec::new();
    collect_skill_files(&skills_root, &mut skill_files)?;
    for skill_path in skill_files {
        let content = std::fs::read_to_string(&skill_path).unwrap_or_default();
        let metadata = parse_skill_frontmatter(&content);
        let Some(name) = metadata.get("name").cloned().or_else(|| {
            skill_path
                .parent()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().to_string())
        }) else {
            continue;
        };
        let relative_path = normalize_relative_path(&root, &skill_path)?;
        let source = if managed_files.contains(&relative_path) {
            "bundled"
        } else {
            "custom"
        }
        .to_string();

        skills
            .entry(name.clone())
            .and_modify(|skill| {
                skill.description = skill
                    .description
                    .clone()
                    .or_else(|| metadata.get("description").cloned());
                skill.path = Some(relative_path.clone());
                if skill.source != "custom" {
                    skill.source = source.clone();
                }
            })
            .or_insert(DwSkill {
                name,
                description: metadata.get("description").cloned(),
                kind: None,
                tier: None,
                owner: None,
                trigger: None,
                path: Some(relative_path),
                source,
            });
    }

    Ok(skills.into_values().collect())
}

#[tauri::command]
fn list_workspace_skills(
    workspace_path: String,
) -> AppResult<Vec<solution::WorkspaceSkillSummary>> {
    Ok(solution::list_workspace_skills(&PathBuf::from(
        workspace_path,
    ))?)
}

#[tauri::command]
fn find_workspace_skills(query: String) -> AppResult<Vec<solution::WorkspaceSkillSearchResult>> {
    Ok(solution::find_workspace_skills(&query)?)
}

#[tauri::command]
fn sync_workspace_skill(
    input: solution::WorkspaceSkillSyncInput,
) -> AppResult<Vec<solution::WorkspaceSkillSummary>> {
    Ok(solution::sync_workspace_skill(input)?)
}

#[tauri::command]
fn read_workspace_flow_artifact(
    workspace_path: String,
    relative_path: String,
) -> AppResult<String> {
    Ok(solution::read_workspace_flow_artifact(
        &PathBuf::from(workspace_path),
        &relative_path,
    )?)
}

#[tauri::command]
fn write_workspace_flow_artifact(
    workspace_path: String,
    input: WorkspaceFlowArtifactWriteInput,
) -> AppResult<String> {
    Ok(solution::write_workspace_flow_artifact(
        &PathBuf::from(workspace_path),
        &input.relative_path,
        &input.content,
    )?)
}

#[tauri::command]
fn sync_workspace_flows(input: WorkspaceFlowSyncInput) -> AppResult<()> {
    Ok(solution::sync_workspace_flows(
        &PathBuf::from(input.workspace_path),
        &PathBuf::from(input.project_path),
    )?)
}

#[tauri::command]
fn list_workspace_capabilities(
    app: tauri::AppHandle,
    workspace_id: i64,
    project_id: Option<i64>,
) -> AppResult<solution::WorkspaceCapabilities> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let workspace = db.get_workspace(workspace_id)?;
    let projects = db.list_projects(workspace_id)?;
    let active_project = project_id.and_then(|id| projects.iter().find(|project| project.id == id));
    Ok(solution::list_workspace_capabilities(
        &workspace,
        active_project,
    )?)
}

#[tauri::command]
fn preview_workspace_solution(
    source_path: String,
) -> AppResult<solution::WorkspaceSolutionManifest> {
    Ok(solution::preview_workspace_solution(&PathBuf::from(
        source_path,
    ))?)
}

#[tauri::command]
fn export_workspace_solution(
    app: tauri::AppHandle,
    workspace_id: i64,
    destination_path: String,
) -> AppResult<solution::WorkspaceSolutionManifest> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let workspace = db.get_workspace(workspace_id)?;
    let projects = db.list_projects(workspace_id)?;
    let machines = db.list_workspace_machines(workspace_id)?;
    Ok(solution::export_workspace_solution(
        &workspace,
        &projects,
        &machines,
        &PathBuf::from(destination_path),
    )?)
}

#[tauri::command]
fn import_workspace_solution(
    workspace_path: String,
    source_path: String,
) -> AppResult<solution::WorkspaceSolutionManifest> {
    Ok(solution::import_workspace_solution(
        &PathBuf::from(workspace_path),
        &PathBuf::from(source_path),
    )?)
}

#[tauri::command]
fn import_workspace_solution_as_workspace(
    app: tauri::AppHandle,
    input: WorkspaceSolutionImportInput,
) -> AppResult<WorkspaceSolutionImportReport> {
    let source_path = PathBuf::from(input.source_path.trim());
    let destination_root = input.destination_root.trim();
    if destination_root.is_empty() {
        return Err(anyhow::anyhow!("destination root is required").into());
    }

    let manifest = solution::preview_workspace_solution(&source_path)?;
    let workspace_name = input
        .workspace_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&manifest.workspace.name);
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let workspace = create_or_open_workspace(&db, workspace_name, destination_root)?;
    let manifest =
        solution::import_workspace_solution(&PathBuf::from(&workspace.root_path), &source_path)?;

    let mut project_results = Vec::new();
    for project in &manifest.projects {
        project_results.push(import_solution_project(&db, &workspace, project));
    }

    Ok(WorkspaceSolutionImportReport {
        workspace,
        manifest,
        projects: project_results,
    })
}

#[tauri::command]
fn read_workflow_state(path: String) -> AppResult<WorkflowStateSummary> {
    let root = canonical_project_root(&PathBuf::from(path))?;
    Ok(build_workflow_state(&root))
}

#[tauri::command]
fn read_text_file(path: String) -> AppResult<String> {
    Ok(std::fs::read_to_string(path).map_err(anyhow::Error::from)?)
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> AppResult<()> {
    Ok(std::fs::write(path, content).map_err(anyhow::Error::from)?)
}

#[tauri::command]
fn get_app_state(app: tauri::AppHandle, key: String) -> AppResult<Option<String>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.get_app_state(&key)?)
}

#[tauri::command]
fn set_app_state(app: tauri::AppHandle, key: String, value: String) -> AppResult<()> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.set_app_state(&key, &value)?)
}

#[tauri::command]
fn open_external_url(url: String) -> AppResult<()> {
    machine::open_external_url(&url)?;
    Ok(())
}

#[tauri::command]
fn check_machine_provider() -> AppResult<winbox_provider::MachineProviderStatus> {
    Ok(machine::provider_status())
}

#[tauri::command]
fn list_machine_presets() -> AppResult<Vec<machine::MachinePreset>> {
    Ok(machine::presets())
}

#[tauri::command]
fn list_workspace_machines(
    app: tauri::AppHandle,
    workspace_id: i64,
) -> AppResult<Vec<store::WorkspaceMachine>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(machine::list_machines(&db, workspace_id)?)
}

#[tauri::command]
async fn create_workspace_machine(
    app: tauri::AppHandle,
    input: machine::CreateWorkspaceMachineInput,
) -> AppResult<store::WorkspaceMachine> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || machine::create_machine(&app_clone, &db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
fn refresh_workspace_machine(
    app: tauri::AppHandle,
    machine_id: String,
) -> AppResult<store::WorkspaceMachine> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(machine::refresh_machine(&db, &machine_id)?)
}

#[tauri::command]
async fn start_workspace_machine(
    app: tauri::AppHandle,
    machine_id: String,
) -> AppResult<store::WorkspaceMachine> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        machine::start_machine(&app_clone, &db, &machine_id)
    })
    .await
    .map_err(|error| anyhow::anyhow!(error.to_string()))?
    .map_err(Into::into)
}

#[tauri::command]
fn stop_workspace_machine(
    app: tauri::AppHandle,
    machine_id: String,
) -> AppResult<store::WorkspaceMachine> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(machine::stop_machine(&db, &machine_id)?)
}

#[tauri::command]
async fn set_workspace_machine_password(
    app: tauri::AppHandle,
    input: machine::SetWorkspaceMachinePasswordInput,
) -> AppResult<store::WorkspaceMachine> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    tauri::async_runtime::spawn_blocking(move || machine::set_machine_password(&db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
async fn open_workspace_machine(
    app: tauri::AppHandle,
    machine_id: String,
) -> AppResult<machine::MachineViewer> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        machine::open_machine(&app_clone, &db, &machine_id)
    })
    .await
    .map_err(|error| anyhow::anyhow!(error.to_string()))?
    .map_err(Into::into)
}

#[tauri::command]
fn get_workspace_machine_logs(
    app: tauri::AppHandle,
    machine_id: String,
    tail: Option<u32>,
) -> AppResult<String> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(machine::machine_logs(&db, &machine_id, tail)?)
}

#[tauri::command]
fn refresh_workspace_machine_health(app: tauri::AppHandle) -> AppResult<String> {
    let _db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(machine::health_summary()?)
}

#[tauri::command]
async fn probe_workspace_machine_ssh(
    app: tauri::AppHandle,
    machine_id: String,
) -> AppResult<machine::MachineSshProbe> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    tauri::async_runtime::spawn_blocking(move || machine::probe_machine_ssh(&db, &machine_id))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
fn remove_workspace_machine(
    app: tauri::AppHandle,
    input: machine::RemoveWorkspaceMachineInput,
) -> AppResult<()> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(machine::remove_machine(&db, input)?)
}

#[tauri::command]
fn list_deploy_stacks(
    app: tauri::AppHandle,
    workspace_id: i64,
) -> AppResult<Vec<store::DeployStack>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::list_stacks(&db, workspace_id)?)
}

#[tauri::command]
fn reset_workspace_deploy_state(
    app: tauri::AppHandle,
    workspace_id: i64,
) -> AppResult<store::WorkspaceDeployReset> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(db.reset_workspace_deploy_state(workspace_id)?)
}

#[tauri::command]
fn get_deploy_stack(
    app: tauri::AppHandle,
    stack_id: String,
) -> AppResult<deploy::DeployStackDetail> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::get_stack(&db, &stack_id)?)
}

#[tauri::command]
fn detect_deploy_stack(
    app: tauri::AppHandle,
    input: deploy::DetectDeployStackInput,
) -> AppResult<deploy_detect::DeployDetectionReport> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::detect_stack(&db, input)?)
}

#[tauri::command]
async fn plan_deploy_package(
    app: tauri::AppHandle,
    input: deploy_plan::PlanDeployPackageInput,
) -> AppResult<deploy_plan::DeployPlanReport> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_for_plan = app.clone();
    tauri::async_runtime::spawn_blocking(move || deploy::plan_package(&app_for_plan, &db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
async fn create_deploy_package(
    app: tauri::AppHandle,
    input: deploy::CreatePackageInput,
) -> AppResult<store::DeployVersion> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    tauri::async_runtime::spawn_blocking(move || deploy::create_package(&db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
fn read_deploy_artifact(
    app: tauri::AppHandle,
    input: deploy::ReadDeployArtifactInput,
) -> AppResult<String> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::read_artifact(&db, input)?)
}

#[tauri::command]
fn approve_deploy_version(
    app: tauri::AppHandle,
    input: deploy::ApproveDeployVersionInput,
) -> AppResult<store::DeployVersion> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::approve_version(&db, input)?)
}

#[tauri::command]
fn create_deploy_repair_version(
    app: tauri::AppHandle,
    input: deploy::CreateDeployRepairVersionInput,
) -> AppResult<store::DeployVersion> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::create_repair_version(&db, input)?)
}

#[tauri::command]
fn get_deploy_environment(
    app: tauri::AppHandle,
    input: deploy_env::DeployEnvironmentInput,
) -> AppResult<deploy_env::DeployEnvironment> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::get_environment(&db, input)?)
}

#[tauri::command]
fn save_deploy_environment(
    app: tauri::AppHandle,
    input: deploy_env::SaveDeployEnvironmentInput,
) -> AppResult<deploy_env::DeployEnvironment> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::save_environment(&db, input)?)
}

#[tauri::command]
async fn prepare_deploy_target(
    app: tauri::AppHandle,
    input: deploy::PrepareDeployTargetInput,
) -> AppResult<store::DeployRun> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || deploy::prepare_target(&app_clone, &db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
async fn deploy_version(
    app: tauri::AppHandle,
    input: deploy::DeployVersionInput,
) -> AppResult<store::DeployRun> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || deploy::deploy_version(&app_clone, &db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
async fn stop_deploy_stack(
    app: tauri::AppHandle,
    input: deploy::StopDeployStackInput,
) -> AppResult<store::DeployRun> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || deploy::stop_stack(&app_clone, &db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
async fn reactivate_deploy_version(
    app: tauri::AppHandle,
    input: deploy::ReactivateDeployVersionInput,
) -> AppResult<store::DeployRun> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || deploy::reactivate_version(&app_clone, &db, input))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
fn list_deploy_runs(app: tauri::AppHandle, version_id: String) -> AppResult<Vec<store::DeployRun>> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::list_runs(&db, &version_id)?)
}

#[tauri::command]
fn get_deploy_run_logs(
    app: tauri::AppHandle,
    input: deploy::DeployRunLogsInput,
) -> AppResult<String> {
    let db = store::Database::open(&app_data_dir(&app)?)?;
    Ok(deploy::run_logs(&db, input)?)
}

struct IndexedEvidenceFile {
    relative_path: String,
    absolute_path: String,
    title: String,
    summary: String,
}

fn canonical_project_root(path: &Path) -> anyhow::Result<PathBuf> {
    let root = std::fs::canonicalize(path)?;
    if !root.is_dir() {
        return Err(anyhow::anyhow!("project path is not a directory"));
    }
    Ok(root)
}

fn scoped_child_path(root: &Path, relative_path: &str) -> anyhow::Result<PathBuf> {
    let child = root.join(relative_path);
    let canonical_child = std::fs::canonicalize(&child)?;
    if !canonical_child.starts_with(root) {
        return Err(anyhow::anyhow!("source path escapes project root"));
    }
    Ok(canonical_child)
}

fn scoped_child_path_allow_missing(root: &Path, relative_path: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(anyhow::anyhow!("source path escapes project root"));
    }
    let child = root.join(relative);
    let parent = child.parent().unwrap_or(root);
    let canonical_parent = if parent.exists() {
        std::fs::canonicalize(parent)?
    } else {
        root.to_path_buf()
    };
    if !canonical_parent.starts_with(root) {
        return Err(anyhow::anyhow!("source path escapes project root"));
    }
    Ok(child)
}

fn open_system_path(path: &Path) -> anyhow::Result<()> {
    let command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]).arg(path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        command
    } else if is_wsl_mount_path(path) {
        let mut command = Command::new("explorer.exe");
        command.arg(wsl_windows_path(path).unwrap_or_else(|| path.display().to_string()));
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    spawn_detached(command, "open", path)
}

fn reveal_system_path(path: &Path) -> anyhow::Result<()> {
    let command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", path.display()));
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        command
    } else if is_wsl_mount_path(path) {
        let reveal_target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let mut command = Command::new("explorer.exe");
        command.arg(
            wsl_windows_path(reveal_target).unwrap_or_else(|| reveal_target.display().to_string()),
        );
        command
    } else {
        let mut command = Command::new("xdg-open");
        let reveal_target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        command.arg(reveal_target);
        command
    };
    spawn_detached(command, "reveal", path)
}

fn spawn_detached(mut command: Command, action: &str, path: &Path) -> anyhow::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| anyhow::anyhow!("failed to {action} {}: {err}", path.display()))
}

fn is_wsl_mount_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with("/mnt/")
}

fn wsl_windows_path(path: &Path) -> Option<String> {
    let output = Command::new("wslpath").arg("-w").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn project_path_key(root: &Path) -> String {
    store::normalize_filesystem_path(&root.display().to_string())
}

fn is_git_project_dir(path: &Path) -> bool {
    path.is_dir() && path.join(".git").exists()
}

fn project_name_from_remote(remote_url: &str) -> String {
    let trimmed = remote_url.trim().trim_end_matches('/');
    let segment = trimmed
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("project")
        .trim_end_matches(".git");
    let safe = safe_project_dir_name(segment);
    if safe.is_empty() {
        "project".to_string()
    } else {
        safe
    }
}

fn safe_project_dir_name(name: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            output.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    output.trim_matches(['-', '.']).to_string()
}

fn slugify_requirement(title: &str) -> String {
    let slug = title
        .trim()
        .to_ascii_lowercase()
        .chars()
        .fold(String::new(), |mut acc, ch| {
            if ch.is_ascii_alphanumeric() {
                acc.push(ch);
            } else if !acc.ends_with('-') {
                acc.push('-');
            }
            acc
        })
        .trim_matches('-')
        .to_string();
    if slug.is_empty() {
        "requirement".to_string()
    } else {
        slug
    }
}

// Statuses are defined by the workbench schema (.dw/workbench.json), so the
// backend no longer hardcodes a whitelist. It validates that the status is a
// safe slug and reserves `archived` for the dedicated archive command.
fn normalize_requirement_status(status: &str) -> anyhow::Result<String> {
    let trimmed = status.trim();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("requirement status is required"));
    }
    if trimmed == "archived" {
        return Err(anyhow::anyhow!(
            "cannot set status to archived directly; use the archive command"
        ));
    }
    let valid = trimmed.len() <= 40
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-');
    if !valid {
        return Err(anyhow::anyhow!("unsupported requirement status: {status}"));
    }
    Ok(trimmed.to_string())
}

fn normalize_restorable_requirement_status(status: &str) -> anyhow::Result<String> {
    normalize_requirement_status(status)
}

fn normalize_agent_provider(provider: &str) -> anyhow::Result<&'static str> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "codex" => Ok("codex"),
        "claude" => Ok("claude"),
        "copilot" => Ok("copilot"),
        _ => Err(anyhow::anyhow!("unsupported agent provider: {provider}")),
    }
}

fn normalize_agent_session_scope(scope: Option<&str>) -> anyhow::Result<&'static str> {
    match scope
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        None | Some("chat") => Ok("chat"),
        Some("card_interview") => Ok("card_interview"),
        Some("project_blueprint") => Ok("project_blueprint"),
        Some(other) => Err(anyhow::anyhow!("unsupported agent session scope: {other}")),
    }
}

fn normalize_agent_sandbox(sandbox: &str) -> anyhow::Result<&'static str> {
    match sandbox.trim().to_ascii_lowercase().as_str() {
        "" | "read-only" => Ok("read-only"),
        "workspace-write" => Ok("workspace-write"),
        "danger-full-access" => Ok("danger-full-access"),
        _ => Err(anyhow::anyhow!("unsupported agent sandbox: {sandbox}")),
    }
}

fn normalize_agent_reasoning_effort(effort: Option<&str>) -> anyhow::Result<Option<&'static str>> {
    match effort
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        None => Ok(None),
        Some("none") => Ok(Some("none")),
        Some("low") => Ok(Some("low")),
        Some("medium") => Ok(Some("medium")),
        Some("high") => Ok(Some("high")),
        Some("xhigh") => Ok(Some("xhigh")),
        Some("max") => Ok(Some("max")),
        Some(other) => Err(anyhow::anyhow!("unsupported reasoning effort: {other}")),
    }
}

fn normalize_agent_context_mode(mode: Option<&str>) -> anyhow::Result<&'static str> {
    match mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        None => Ok("auto_lean"),
        Some("auto_lean") => Ok("auto_lean"),
        Some("full") => Ok("full"),
        Some(other) => Err(anyhow::anyhow!("unsupported agent context mode: {other}")),
    }
}

fn preview_mime_type(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "apng" => Some("image/apng"),
        "avif" => Some("image/avif"),
        "gif" => Some("image/gif"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

fn normalize_evidence_status(status: &str) -> anyhow::Result<&'static str> {
    match status.trim().to_ascii_lowercase().as_str() {
        "submitted" => Ok("submitted"),
        "passed" => Ok("passed"),
        "failed" => Ok("failed"),
        "unknown" => Ok("unknown"),
        "indexed" => Ok("indexed"),
        _ => Err(anyhow::anyhow!("unsupported evidence status: {status}")),
    }
}

fn infer_prd_slug(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .map(|token| token.trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ',' | ';')))
        .find(|token| token.starts_with("prd-"))
        .map(ToOwned::to_owned)
}

fn redact_summary(input: &str) -> String {
    let mut redacted = input
        .chars()
        .take(4000)
        .collect::<String>()
        .lines()
        .map(redact_line)
        .collect::<Vec<_>>()
        .join("\n");
    if input.chars().count() > 4000 {
        redacted.push_str("\n[truncated]");
    }
    redacted
}

fn redact_line(line: &str) -> String {
    line.split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch| matches!(ch, '"' | '\'' | '{' | '}' | ','));
    let lower = trimmed.to_ascii_lowercase();
    if lower == "bearer" || lower.starts_with("bearer:") {
        return "Bearer ***".to_string();
    }

    for separator in ['=', ':'] {
        if let Some((key, _value)) = trimmed.split_once(separator) {
            let clean_key = key.trim_matches(|ch| matches!(ch, '"' | '\'' | '{' | '}'));
            if is_sensitive_key(clean_key) {
                return format!("{clean_key}{separator}***");
            }
        }
    }

    token.to_string()
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
        "auth",
        "credential",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn normalize_existing_dw_relative_path(root: &Path, relative_path: &str) -> anyhow::Result<String> {
    let trimmed = relative_path
        .trim()
        .trim_start_matches("./")
        .strip_prefix(".dw/")
        .unwrap_or_else(|| relative_path.trim().trim_start_matches("./"));
    if trimmed.is_empty() || Path::new(trimmed).is_absolute() {
        return Err(anyhow::anyhow!("evidence path must be relative to .dw"));
    }

    let dw_path = root.join(".dw");
    let canonical_dw = std::fs::canonicalize(&dw_path).map_err(anyhow::Error::from)?;
    let canonical_child =
        std::fs::canonicalize(dw_path.join(trimmed)).map_err(anyhow::Error::from)?;
    if !canonical_child.starts_with(&canonical_dw) {
        return Err(anyhow::anyhow!("evidence path escapes .dw"));
    }
    normalize_relative_path(&canonical_dw, &canonical_child)
}

fn normalize_new_dw_relative_path(relative_path: &str) -> anyhow::Result<PathBuf> {
    let trimmed = relative_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .strip_prefix(".dw/")
        .unwrap_or_else(|| relative_path.trim().trim_start_matches("./"))
        .to_string();
    if trimmed.is_empty() || Path::new(&trimmed).is_absolute() {
        return Err(anyhow::anyhow!("artifact path must be relative to .dw"));
    }

    let mut clean = PathBuf::new();
    for component in Path::new(&trimmed).components() {
        match component {
            Component::Normal(value) => clean.push(value),
            Component::CurDir => {}
            _ => return Err(anyhow::anyhow!("artifact path escapes .dw")),
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(anyhow::anyhow!("artifact path must be relative to .dw"));
    }
    Ok(clean)
}

fn mark_evidence_staleness(root: &Path, entries: &mut [store::EvidenceEntry]) {
    for entry in entries {
        entry.stale = if let Some(relative_path) = entry.relative_path.as_deref() {
            !root.join(".dw").join(relative_path).exists()
        } else if let Some(absolute_path) = entry.absolute_path.as_deref() {
            !Path::new(absolute_path).exists()
        } else if let Some(log_path) = entry.terminal_log_path.as_deref() {
            !Path::new(log_path).exists()
        } else {
            false
        };
        if entry.stale && entry.status == "indexed" {
            entry.status = "stale".to_string();
        }
    }
}

fn collect_project_evidence_files(root: &Path) -> anyhow::Result<Vec<IndexedEvidenceFile>> {
    let dw_path = root.join(".dw");
    let specs_root = dw_path.join("spec");
    if !specs_root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&specs_root).map_err(anyhow::Error::from)? {
        let entry = entry.map_err(anyhow::Error::from)?;
        let spec_path = entry.path();
        if !spec_path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();

        let run_log = spec_path.join("run-log.md");
        if run_log.exists() {
            files.push(indexed_evidence_file(&dw_path, &run_log, &slug)?);
        }

        let qa_path = spec_path.join("QA");
        if qa_path.exists() {
            collect_evidence_files_under(&dw_path, &qa_path, &slug, &mut files)?;
        }
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn collect_evidence_files_under(
    dw_path: &Path,
    current: &Path,
    slug: &str,
    files: &mut Vec<IndexedEvidenceFile>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current).map_err(anyhow::Error::from)? {
        let entry = entry.map_err(anyhow::Error::from)?;
        let path = entry.path();
        if path.is_dir() {
            collect_evidence_files_under(dw_path, &path, slug, files)?;
        } else if is_supported_evidence_file(&path) {
            files.push(indexed_evidence_file(dw_path, &path, slug)?);
        }
    }
    Ok(())
}

fn indexed_evidence_file(
    dw_path: &Path,
    path: &Path,
    slug: &str,
) -> anyhow::Result<IndexedEvidenceFile> {
    let relative_path = normalize_relative_path(dw_path, path)?;
    let kind = evidence_kind_for_path(&relative_path);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.clone());
    Ok(IndexedEvidenceFile {
        relative_path,
        absolute_path: path.display().to_string(),
        title: format!("{slug} - {file_name}"),
        summary: format!("Indexed {kind} artifact from .dw."),
    })
}

fn is_supported_evidence_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "txt" | "log" | "json" | "png" | "jpg" | "jpeg" | "webp"
            )
        })
}

fn evidence_kind_for_path(relative_path: &str) -> &'static str {
    let lower = relative_path.to_ascii_lowercase();
    if lower.ends_with("qa-report.md") {
        "qa-report"
    } else if lower.ends_with("bugs.md") {
        "bug-report"
    } else if lower.contains("/review") || lower.contains("review-") {
        "review"
    } else if lower.contains("/logs/") || lower.ends_with(".log") {
        "log"
    } else if lower.contains("/scripts/") {
        "script"
    } else if lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
    {
        "screenshot"
    } else if lower.ends_with("run-log.md") {
        "run-log"
    } else {
        "artifact"
    }
}

fn collect_source_entries(
    root: &Path,
    current: &Path,
    is_root: bool,
) -> anyhow::Result<Vec<SourceEntry>> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if should_exclude_source_dir(root, &path, &name, is_root) {
                continue;
            }

            let relative_path = normalize_relative_path(root, &path)?;
            let mut children = collect_source_entries(root, &path, false)?;
            sort_source_entries(&mut children);

            entries.push(SourceEntry {
                relative_path,
                name,
                kind: "directory".to_string(),
                extension: None,
                bytes: None,
                children,
            });
            continue;
        }

        if path.is_file() && should_include_source_file(&path, &name, is_root) {
            let metadata = entry.metadata()?;
            entries.push(SourceEntry {
                relative_path: normalize_relative_path(root, &path)?,
                name,
                kind: "file".to_string(),
                extension: file_extension(&path),
                bytes: Some(metadata.len()),
                children: Vec::new(),
            });
        }
    }

    sort_source_entries(&mut entries);
    Ok(entries)
}

fn sort_source_entries(entries: &mut [SourceEntry]) {
    entries.sort_by(|left, right| {
        let left_is_file = left.kind == "file";
        let right_is_file = right.kind == "file";
        left_is_file
            .cmp(&right_is_file)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

fn should_exclude_source_dir(root: &Path, path: &Path, name: &str, is_root: bool) -> bool {
    if matches!(
        name,
        ".git" | ".dw" | "node_modules" | "dist" | "build" | "coverage" | "target" | ".backup"
    ) {
        return true;
    }

    if !is_root && name.starts_with('.') {
        return true;
    }

    normalize_relative_path(root, path).is_ok_and(|relative| relative == "src-tauri/target")
}

fn should_include_source_file(path: &Path, name: &str, is_root: bool) -> bool {
    if name.starts_with('.') {
        return is_root;
    }

    is_supported_source_file(path)
}

fn is_supported_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "ts" | "tsx"
                    | "js"
                    | "jsx"
                    | "mjs"
                    | "cjs"
                    | "json"
                    | "md"
                    | "mdx"
                    | "rs"
                    | "py"
                    | "cs"
                    | "css"
                    | "scss"
                    | "less"
                    | "html"
                    | "htm"
                    | "txt"
                    | "toml"
                    | "yml"
                    | "yaml"
            )
        })
}

fn normalize_relative_path(root: &Path, path: &Path) -> anyhow::Result<String> {
    Ok(path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/"))
}

fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(ToOwned::to_owned)
}

fn is_binary_file(path: &Path) -> anyhow::Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0; BINARY_SAMPLE_BYTES];
    let bytes_read = file.read(&mut buffer)?;
    Ok(buffer[..bytes_read].contains(&0))
}

fn collect_dw_artifacts(
    root: &Path,
    current: &Path,
    artifacts: &mut Vec<DwArtifact>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name.starts_with('.') && file_name != ".gitkeep" {
            continue;
        }

        if path.is_dir() {
            if matches!(
                file_name.as_str(),
                ".backup" | "templates" | "references" | "scripts"
            ) {
                continue;
            }
            collect_dw_artifacts(root, &path, artifacts)?;
            continue;
        }

        if !is_supported_artifact(&path) {
            continue;
        }

        let relative_path = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");

        let metadata = entry.metadata()?;
        artifacts.push(DwArtifact {
            category: artifact_category(&relative_path).to_string(),
            name: file_name,
            relative_path,
            bytes: metadata.len(),
        });
    }

    Ok(())
}

fn first_markdown_heading(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.strip_prefix("# ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn first_use_bullet(content: &str) -> Option<String> {
    let mut in_when_to_use = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_when_to_use = trimmed.contains("Quando Usar") || trimmed.contains("When To Use");
            continue;
        }
        if in_when_to_use {
            if let Some(item) = trimmed.strip_prefix("- ") {
                let item = item.trim();
                if !item.is_empty() {
                    return Some(item.to_string());
                }
            }
        }
    }
    None
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn managed_files(root: &Path) -> BTreeSet<String> {
    let path = root.join(".dw").join("install-state.json");
    let Ok(content) = std::fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return BTreeSet::new();
    };
    value
        .get("managed_files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_skill_files(root: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_skill_files(&path, files)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "SKILL.md")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}

fn parse_skill_frontmatter(content: &str) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return metadata;
    }

    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if !value.is_empty() {
            metadata.insert(key.trim().to_string(), value.to_string());
        }
    }

    metadata
}

fn build_workflow_state(root: &Path) -> WorkflowStateSummary {
    let specs = collect_prd_summaries(root);
    let has_any_prd = !specs.is_empty();
    let has_tasks = specs.iter().any(|spec| spec.has_tasks);
    let has_run_log = specs.iter().any(|spec| spec.has_run_log);
    let has_qa = specs.iter().any(|spec| spec.has_qa);
    let has_review = specs.iter().any(|spec| spec.has_review);

    let stages = vec![
        workflow_stage(
            "brainstorm",
            "Brainstorm",
            "/dw-brainstorm",
            "ready",
            "Optional discovery stage.",
        ),
        workflow_stage(
            "plan",
            "Plan",
            "/dw-plan",
            if has_tasks {
                "complete"
            } else if has_any_prd {
                "active"
            } else {
                "ready"
            },
            if has_tasks {
                "At least one PRD has approved task artifacts."
            } else if has_any_prd {
                "PRD artifacts exist; TechSpec or Tasks may still be pending."
            } else {
                "No PRD artifacts detected yet."
            },
        ),
        workflow_stage(
            "run",
            "Run",
            "/dw-run",
            if has_run_log {
                "complete"
            } else if has_tasks {
                "ready"
            } else {
                "blocked"
            },
            if has_tasks {
                "Tasks are available for execution."
            } else {
                "Requires approved tasks."
            },
        ),
        workflow_stage(
            "qa",
            "QA",
            "/dw-qa",
            if has_qa {
                "complete"
            } else if has_run_log {
                "ready"
            } else {
                "blocked"
            },
            if has_run_log {
                "Run log exists; QA can collect behavior evidence."
            } else {
                "Requires implementation evidence."
            },
        ),
        workflow_stage(
            "review",
            "Review",
            "/dw-review",
            if has_review {
                "complete"
            } else if has_tasks {
                "ready"
            } else {
                "blocked"
            },
            if has_tasks {
                "Review can compare implementation against plan artifacts."
            } else {
                "Requires PRD planning artifacts."
            },
        ),
        workflow_stage(
            "commit",
            "Commit",
            "/dw-commit",
            "blocked",
            "Available after review and verification.",
        ),
        workflow_stage(
            "pr",
            "PR",
            "/dw-generate-pr",
            "blocked",
            "Out of scope for runner v1.",
        ),
    ];

    let mut gates = Vec::new();
    let mut resume_entries = Vec::new();
    if root.join(".dw").join("STATE.md").exists() {
        resume_entries.push(WorkflowResumeEntry {
            kind: "state".to_string(),
            label: "Session state".to_string(),
            command: "/dw-resume".to_string(),
            path: ".dw/STATE.md".to_string(),
            status: "available".to_string(),
        });
    }

    for spec in specs {
        if let Some(status) = spec.autopilot_status.clone() {
            let gate_state = match status.as_str() {
                "planning" => "awaiting approval",
                "plan_complete" => "approved",
                "goal_active" => "active",
                "completed" => "complete",
                _ => "pending",
            };
            gates.push(WorkflowGate {
                label: format!("Autopilot {}", spec.slug),
                state: gate_state.to_string(),
                path: Some(format!("spec/{}/autopilot-state.json", spec.slug)),
                detail: format!("status: {status}"),
            });
            resume_entries.push(WorkflowResumeEntry {
                kind: "autopilot".to_string(),
                label: spec.slug.clone(),
                command: format!("/dw-autopilot {}", spec.slug),
                path: format!(".dw/spec/{}/autopilot-state.json", spec.slug),
                status,
            });
        }

        if spec.has_prd && !spec.has_techspec {
            gates.push(WorkflowGate {
                label: format!("PRD {}", spec.slug),
                state: "awaiting approval".to_string(),
                path: Some(format!("spec/{}/prd.md", spec.slug)),
                detail: "TechSpec not found yet.".to_string(),
            });
        } else if spec.has_techspec && !spec.has_tasks {
            gates.push(WorkflowGate {
                label: format!("TechSpec {}", spec.slug),
                state: "awaiting approval".to_string(),
                path: Some(format!("spec/{}/techspec.md", spec.slug)),
                detail: "Tasks not found yet.".to_string(),
            });
        } else if spec.has_tasks {
            gates.push(WorkflowGate {
                label: format!("Tasks {}", spec.slug),
                state: "complete".to_string(),
                path: Some(format!("spec/{}/tasks.md", spec.slug)),
                detail: "Task artifact exists.".to_string(),
            });
        }

        resume_entries.push(WorkflowResumeEntry {
            kind: "prd".to_string(),
            label: spec.slug.clone(),
            command: if spec.has_tasks {
                format!("/dw-run .dw/spec/{}", spec.slug)
            } else if spec.has_techspec {
                format!("/dw-plan tasks {}", spec.slug)
            } else {
                format!("/dw-plan techspec {}", spec.slug)
            },
            path: format!(".dw/spec/{}/prd.md", spec.slug),
            status: spec.status_label(),
        });
    }

    WorkflowStateSummary {
        stages,
        gates,
        resume_entries,
    }
}

struct PrdSummary {
    slug: String,
    has_prd: bool,
    has_techspec: bool,
    has_tasks: bool,
    has_run_log: bool,
    has_qa: bool,
    has_review: bool,
    autopilot_status: Option<String>,
}

impl PrdSummary {
    fn status_label(&self) -> String {
        if self.has_review {
            "reviewed".to_string()
        } else if self.has_qa {
            "qa".to_string()
        } else if self.has_run_log {
            "run".to_string()
        } else if self.has_tasks {
            "tasks".to_string()
        } else if self.has_techspec {
            "techspec".to_string()
        } else {
            "prd".to_string()
        }
    }
}

fn collect_prd_summaries(root: &Path) -> Vec<PrdSummary> {
    let specs_root = root.join(".dw").join("spec");
    let Ok(entries) = std::fs::read_dir(specs_root) else {
        return Vec::new();
    };

    let mut specs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let has_prd = path.join("prd.md").exists();
        if !has_prd {
            continue;
        }
        let autopilot_status = read_json_string(&path.join("autopilot-state.json"), "status");
        specs.push(PrdSummary {
            slug,
            has_prd,
            has_techspec: path.join("techspec.md").exists(),
            has_tasks: path.join("tasks.md").exists(),
            has_run_log: path.join("run-log.md").exists(),
            has_qa: path.join("QA").join("qa-report.md").exists(),
            has_review: path.join("QA").join("review-consolidated.md").exists(),
            autopilot_status,
        });
    }
    specs.sort_by(|left, right| left.slug.cmp(&right.slug));
    specs
}

fn read_json_string(path: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<Value>(&content).ok()?;
    json_string(&value, key)
}

fn workflow_stage(
    id: &str,
    label: &str,
    command: &str,
    state: &str,
    detail: &str,
) -> WorkflowStageState {
    WorkflowStageState {
        id: id.to_string(),
        label: label.to_string(),
        command: command.to_string(),
        state: state.to_string(),
        detail: detail.to_string(),
    }
}

fn is_supported_artifact(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "md" | "json" | "txt"))
}

fn artifact_category(relative_path: &str) -> &'static str {
    if relative_path == "STATE.md" {
        "state"
    } else if relative_path.starts_with("spec/") {
        "spec"
    } else if relative_path.starts_with("bugfixes/") {
        "bugfix"
    } else if relative_path.starts_with("commands/") {
        "command"
    } else if relative_path.starts_with("rules/") {
        "rule"
    } else {
        "support"
    }
}

fn app_data_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let dir = std::env::var("DW_GUI_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| app.path().app_data_dir().ok())
        .or_else(|| dirs::data_local_dir().map(|base| base.join("clia-local")))
        .unwrap_or_else(|| PathBuf::from(".clia-local"));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn run() {
    // Hidden subcommand: when invoked by git as the interactive-rebase sequence
    // editor (`<exe> --dwgui-rebase-editor <todo-file>`), overwrite the todo file
    // with the plan prepared in DWGUI_REBASE_TODO and exit before Tauri starts.
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|arg| arg == "--dwgui-rebase-editor") {
        if let Some(file) = args.get(pos + 1) {
            let todo = std::env::var("DWGUI_REBASE_TODO").unwrap_or_default();
            let _ = std::fs::write(file, todo);
        }
        std::process::exit(0);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(terminal::TerminalManager::new())
        .manage(lsp::LspManager::new())
        .setup(|app| {
            if let Err(error) = app.state::<terminal::TerminalManager>().cleanup_temp_logs() {
                eprintln!("failed to clean terminal temp logs: {error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            preflight,
            list_workspaces,
            create_workspace,
            list_projects,
            add_local_project,
            clone_git_project,
            list_requirement_cards,
            create_requirement_card,
            update_requirement_card_status,
            set_requirement_card_flow,
            archive_requirement_card,
            restore_requirement_card,
            update_requirement_card_body,
            list_requirement_stage_forms,
            upsert_requirement_stage_form,
            list_requirement_attachments,
            add_requirement_attachment,
            remove_requirement_attachment,
            preview_requirement_attachment,
            download_requirement_attachment,
            list_knowledge_sources,
            add_knowledge_source,
            remove_knowledge_source,
            create_project_blueprint,
            list_project_blueprints,
            update_project_blueprint,
            materialize_project_blueprint,
            list_evidence,
            create_evidence_run,
            complete_evidence_run,
            create_manual_evidence,
            index_project_evidence,
            list_agent_profiles,
            create_agent_profile,
            update_agent_profile,
            list_agent_sessions,
            list_agent_messages,
            reset_agent_chat,
            send_agent_message,
            stop_agent_session,
            agent_usage,
            check_agent_provider_health,
            list_agent_run_metrics,
            get_rtk_status,
            install_rtk,
            configure_rtk,
            warm_agent_runtime,
            git_status,
            git_diff,
            git_staged_diff,
            git_log_graph,
            git_blame,
            git_blame_porcelain,
            git_blame_porcelain_for_content,
            list_changed_files,
            git_worktree_fingerprint,
            git_worktree_snapshot,
            read_file_patch,
            git_file_patch_text,
            stage_file,
            unstage_file,
            git_stage_all,
            git_unstage_all,
            stage_hunk,
            unstage_hunk,
            check_imported_patch,
            apply_imported_patch,
            list_source_tree,
            search_in_files,
            create_source_file,
            create_source_dir,
            project_path_exists,
            format_external,
            lsp_start,
            lsp_send,
            lsp_stop,
            lsp_server_status,
            lsp_install,
            read_source_file,
            write_source_file,
            list_dw_artifacts,
            read_dw_artifact,
            write_dw_artifact,
            fetch_url,
            list_dw_commands,
            list_dw_skills,
            list_workspace_skills,
            list_workspace_capabilities,
            find_workspace_skills,
            sync_workspace_skill,
            read_workspace_flow_artifact,
            write_workspace_flow_artifact,
            sync_workspace_flows,
            preview_workspace_solution,
            export_workspace_solution,
            import_workspace_solution,
            import_workspace_solution_as_workspace,
            read_workflow_state,
            run_shell,
            create_terminal_session,
            write_terminal_input,
            resize_terminal_session,
            stop_terminal_session,
            close_terminal_session,
            list_terminal_sessions,
            read_text_file,
            write_text_file,
            get_app_state,
            set_app_state,
            open_external_url,
            check_machine_provider,
            list_machine_presets,
            list_workspace_machines,
            create_workspace_machine,
            refresh_workspace_machine,
            start_workspace_machine,
            stop_workspace_machine,
            set_workspace_machine_password,
            open_workspace_machine,
            get_workspace_machine_logs,
            refresh_workspace_machine_health,
            probe_workspace_machine_ssh,
            remove_workspace_machine,
            list_deploy_stacks,
            reset_workspace_deploy_state,
            get_deploy_stack,
            detect_deploy_stack,
            plan_deploy_package,
            create_deploy_package,
            read_deploy_artifact,
            approve_deploy_version,
            create_deploy_repair_version,
            get_deploy_environment,
            save_deploy_environment,
            prepare_deploy_target,
            deploy_version,
            stop_deploy_stack,
            reactivate_deploy_version,
            list_deploy_runs,
            get_deploy_run_logs,
            git_commit_graph,
            git_repo_snapshot,
            git_log_file,
            git_show_file,
            git_commit_detail,
            git_commit_file_diff,
            git_repo_state,
            git_list_branches,
            git_list_remote_branches,
            git_list_tags,
            git_list_stashes,
            git_stash_detail,
            git_stash_file_diff,
            git_commit,
            git_fetch,
            git_pull,
            git_push,
            git_checkout_branch,
            git_checkout_commit,
            git_create_branch,
            git_rename_branch,
            git_delete_branch,
            git_delete_tag,
            git_stash_save,
            git_stash_file,
            git_ignore_file,
            git_external_diff,
            git_stash_pop,
            git_stash_apply,
            git_stash_drop,
            git_use_ours,
            git_use_theirs,
            git_mark_resolved,
            git_continue_operation,
            git_start_interactive_rebase,
            git_list_submodules,
            git_update_submodule,
            git_merge_branch,
            git_rebase_branch,
            git_cherry_pick,
            git_revert,
            git_reset,
            git_create_tag,
            git_abort_operation,
            git_discard_file,
            git_discard_hunk,
            open_project_file,
            reveal_project_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running clia.dev");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn requirement_status_accepts_schema_slugs_and_rejects_invalid() {
        assert_eq!(normalize_requirement_status("draft").unwrap(), "draft");
        assert_eq!(
            normalize_requirement_status(" security ").unwrap(),
            "security"
        );
        assert_eq!(
            normalize_requirement_status("local_pr").unwrap(),
            "local_pr"
        );
        assert_eq!(normalize_requirement_status("phase-1").unwrap(), "phase-1");
        assert!(normalize_requirement_status("").is_err());
        assert!(normalize_requirement_status("archived").is_err());
        assert!(normalize_requirement_status("Has Space").is_err());
        assert!(normalize_requirement_status("UPPER").is_err());
    }

    fn fixture_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-gui-source-test-{unique}"));
        std::fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    fn write_file(root: &Path, relative_path: &str, contents: &[u8]) {
        let path = root.join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, contents).expect("write fixture file");
    }

    #[test]
    fn source_tree_includes_root_dotfiles_and_excludes_generated_dirs() {
        let root = fixture_root();
        write_file(&root, ".gitignore", b"node_modules\n");
        write_file(&root, "src/App.tsx", b"export function App() {}\n");
        write_file(&root, ".git/config", b"[core]\n");
        write_file(&root, ".dw/STATE.md", b"# state\n");
        write_file(&root, "node_modules/pkg/index.js", b"module.exports = {}\n");
        write_file(&root, "src-tauri/target/debug/app", b"binary");

        let canonical = canonical_project_root(&root).expect("canonical root");
        let entries = collect_source_entries(&canonical, &canonical, true).expect("source tree");
        let flattened = flatten_paths(&entries);

        assert!(flattened.contains(&".gitignore".to_string()));
        assert!(flattened.contains(&"src".to_string()));
        assert!(flattened.contains(&"src/App.tsx".to_string()));
        assert!(!flattened
            .iter()
            .any(|path| path == ".git" || path.starts_with(".git/")));
        assert!(!flattened.iter().any(|path| path.starts_with(".dw")));
        assert!(!flattened
            .iter()
            .any(|path| path.starts_with("node_modules")));
        assert!(!flattened
            .iter()
            .any(|path| path.starts_with("src-tauri/target")));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn source_file_write_updates_existing_text_file() {
        let root = fixture_root();
        write_file(&root, "src/app.ts", b"export const value = 1;\n");

        let file = write_source_file(
            root.display().to_string(),
            "src/app.ts".to_string(),
            "export const value = 2;\n".to_string(),
        )
        .expect("write source file");

        assert_eq!(file.content, "export const value = 2;\n");
        assert_eq!(
            std::fs::read_to_string(root.join("src/app.ts")).expect("read written file"),
            "export const value = 2;\n"
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_names_are_derived_and_sanitized() {
        assert_eq!(
            project_name_from_remote("git@github.com:org/clia-app.git"),
            "clia-app"
        );
        assert_eq!(safe_project_dir_name("../bad project"), "bad-project");
        assert_eq!(
            slugify_requirement("Local PR: branch + package"),
            "local-pr-branch-package"
        );
    }

    #[test]
    fn scoped_child_path_rejects_project_escape() {
        let root = fixture_root();
        let outside = root.with_extension("outside");
        write_file(&root, "inside.txt", b"inside\n");
        std::fs::write(&outside, b"outside\n").expect("write outside file");

        let canonical = canonical_project_root(&root).expect("canonical root");
        let result = scoped_child_path(&canonical, "../dw-gui-source-test-outside");

        assert!(result.is_err());

        std::fs::remove_dir_all(root).expect("cleanup root");
        let _ = std::fs::remove_file(outside);
    }

    #[test]
    fn scoped_new_path_rejects_parent_and_absolute_segments() {
        let root = fixture_root();
        let canonical = canonical_project_root(&root).expect("canonical root");

        assert!(scoped_new_path(&canonical, "src/new.ts").is_ok());
        assert!(scoped_new_path(&canonical, "../escape.ts").is_err());
        assert!(scoped_new_path(&canonical, "/etc/passwd").is_err());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn create_source_file_creates_nested_supported_file() {
        let root = fixture_root();

        let created = create_source_file(root.display().to_string(), "src/lib/new.ts".to_string())
            .expect("create file");
        assert_eq!(created.relative_path, "src/lib/new.ts");
        assert_eq!(created.bytes, 0);
        assert!(root.join("src/lib/new.ts").is_file());

        // Unsupported extension and duplicate creation are rejected.
        assert!(create_source_file(root.display().to_string(), "notes.bin".to_string()).is_err());
        assert!(
            create_source_file(root.display().to_string(), "src/lib/new.ts".to_string()).is_err()
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_path_exists_checks_files_dirs_and_rejects_escape() {
        let root = fixture_root();
        write_file(&root, ".dw/rules/index.md", b"# rules\n");
        let canonical = canonical_project_root(&root).expect("canonical root");

        assert!(project_path_exists_impl(&canonical, ".dw/rules/index.md"));
        assert!(project_path_exists_impl(&canonical, ".dw/rules")); // directory
        assert!(!project_path_exists_impl(
            &canonical,
            ".dw/rules/missing.md"
        ));
        assert!(!project_path_exists_impl(&canonical, "../escape"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn external_formatter_ext_maps_dw_languages_only() {
        assert_eq!(external_formatter_ext("rust"), Some("rs"));
        assert_eq!(external_formatter_ext("python"), Some("py"));
        assert_eq!(external_formatter_ext("csharp"), Some("cs"));
        assert_eq!(external_formatter_ext("markdown"), None);
        assert!(format_external_impl("markdown", "x").is_err());
    }

    #[test]
    fn binary_detection_flags_nul_bytes() {
        let root = fixture_root();
        write_file(&root, "image.txt", b"text\0binary");

        assert!(is_binary_file(&root.join("image.txt")).expect("binary check"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn search_regex_respects_case_and_whole_word_toggles() {
        let content = "let Foo = 1;\nfunction foobar() {}\nconst foo = Foo;\n";

        let ci = build_search_regex("foo", false, false, false).expect("ci");
        assert_eq!(search_lines(content, &ci).len(), 4); // Foo, foobar, foo, Foo

        let cs = build_search_regex("foo", true, false, false).expect("cs");
        assert_eq!(search_lines(content, &cs).len(), 2); // foobar, foo

        let word = build_search_regex("foo", false, true, false).expect("word");
        // whole word: Foo, foo, Foo (not foobar)
        assert_eq!(search_lines(content, &word).len(), 3);
    }

    #[test]
    fn search_match_reports_line_and_char_offsets() {
        let content = "alpha\n  café = beta\n";
        let regex = build_search_regex("beta", true, false, false).expect("regex");
        let matches = search_lines(content, &regex);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 2);
        assert_eq!(matches[0].col, 9); // char offset after "  café = "
        assert_eq!(matches[0].length, 4);
    }

    #[test]
    fn search_regex_rejects_invalid_pattern() {
        assert!(build_search_regex("(", false, false, true).is_err());
    }

    #[test]
    fn strip_and_truncate_html_drops_scripts_and_caps_length() {
        let html = "<html><head><style>.x{color:red}</style></head>\
            <body><h1>Title</h1><script>evil()</script><p>Hello world</p></body></html>";
        let text = strip_and_truncate_html(html, 60_000);
        assert!(!text.contains("evil"), "script content removed");
        assert!(!text.contains("color:red"), "style content removed");
        assert!(text.contains("Title"));
        assert!(text.contains("Hello world"));
        assert!(!text.contains('<'), "tags stripped");

        let long = "a ".repeat(40_000); // 80k chars before collapse
        let capped = strip_and_truncate_html(&long, 100);
        assert!(capped.chars().count() <= 100 + "\n…[truncated]".chars().count());
        assert!(capped.ends_with("[truncated]"));
    }

    #[test]
    fn dw_commands_are_discovered_from_command_artifacts() {
        let root = fixture_root();
        write_file(
            &root,
            ".dw/commands/dw-plan.md",
            b"# Plan Command\n\n## Quando Usar\n- Use para planejar uma feature.\n",
        );
        // Non dw- commands (spec-kit, custom) are also surfaced for the builder palette.
        write_file(&root, ".dw/commands/speckit.specify.md", b"# Specify\n");

        let commands = list_dw_commands(root.display().to_string()).expect("commands");

        assert_eq!(commands.len(), 2);
        // Sorted by command string: /dw-plan before /speckit.specify.
        assert_eq!(commands[0].command, "/dw-plan");
        assert_eq!(commands[0].title, "Plan Command");
        assert_eq!(
            commands[0].description.as_deref(),
            Some("Use para planejar uma feature.")
        );
        assert_eq!(commands[1].command, "/speckit.specify");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn dw_artifact_write_creates_supported_file_inside_dw() {
        let root = fixture_root();

        let artifact = write_dw_artifact(
            root.display().to_string(),
            DwArtifactWriteInput {
                relative_path: "workbench/cards/DW-001/brainstorm.md".to_string(),
                content: "# Brainstorm\n".to_string(),
            },
        )
        .expect("write artifact");

        assert_eq!(
            artifact.relative_path,
            "workbench/cards/DW-001/brainstorm.md"
        );
        assert_eq!(artifact.category, "support");
        assert_eq!(
            std::fs::read_to_string(root.join(".dw/workbench/cards/DW-001/brainstorm.md"))
                .expect("read artifact"),
            "# Brainstorm\n"
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn dw_artifact_write_rejects_escape_paths() {
        let root = fixture_root();

        let result = write_dw_artifact(
            root.display().to_string(),
            DwArtifactWriteInput {
                relative_path: "../outside.md".to_string(),
                content: "bad".to_string(),
            },
        );

        assert!(result.is_err());
        assert!(!root.join("outside.md").exists());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn dw_skills_merge_registry_and_custom_skill_files() {
        let root = fixture_root();
        write_file(
            &root,
            ".dw/skill-registry.json",
            br#"{"skills":[{"name":"dw-plan","kind":"protocol","tier":"core","owner":"dw-plan","trigger":"planning","bundled":true}]}"#,
        );
        write_file(
            &root,
            ".dw/install-state.json",
            br#"{"managed_files":[".agents/skills/dw-plan/SKILL.md"]}"#,
        );
        write_file(
            &root,
            ".agents/skills/dw-plan/SKILL.md",
            b"---\nname: dw-plan\ndescription: Plan command wrapper\n---\n",
        );
        write_file(
            &root,
            ".agents/skills/my-skill/SKILL.md",
            b"---\nname: my-skill\ndescription: Local project skill\n---\n",
        );

        let skills = list_dw_skills(root.display().to_string()).expect("skills");
        let bundled = skills
            .iter()
            .find(|skill| skill.name == "dw-plan")
            .expect("bundled skill");
        let custom = skills
            .iter()
            .find(|skill| skill.name == "my-skill")
            .expect("custom skill");

        assert_eq!(bundled.source, "bundled");
        assert_eq!(bundled.description.as_deref(), Some("Plan command wrapper"));
        assert_eq!(custom.source, "custom");
        assert_eq!(custom.description.as_deref(), Some("Local project skill"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn redact_summary_masks_sensitive_key_values() {
        let redacted = redact_summary(
            "OPENAI_API_KEY=sk-test password:secret Bearer token credential={\"value\":\"x\"}",
        );

        assert!(redacted.contains("OPENAI_API_KEY=***"));
        assert!(redacted.contains("password:***"));
        assert!(redacted.contains("Bearer ***"));
        assert!(!redacted.contains("sk-test"));
        assert!(!redacted.contains("password:secret"));
    }

    #[test]
    fn preview_mime_type_allows_images_and_pdfs_only() {
        assert_eq!(
            preview_mime_type(Path::new("brief.pdf")),
            Some("application/pdf")
        );
        assert_eq!(preview_mime_type(Path::new("photo.PNG")), Some("image/png"));
        assert_eq!(preview_mime_type(Path::new("notes.txt")), None);
    }

    #[test]
    fn project_evidence_indexer_finds_qa_and_run_artifacts() {
        let root = fixture_root();
        write_file(&root, ".dw/spec/prd-demo/run-log.md", b"# run\n");
        write_file(&root, ".dw/spec/prd-demo/QA/qa-report.md", b"# qa\n");
        write_file(&root, ".dw/spec/prd-demo/QA/logs/verify.log", b"ok\n");
        write_file(
            &root,
            ".dw/spec/prd-demo/QA/screenshots/smoke.png",
            b"not really png\n",
        );

        let canonical = canonical_project_root(&root).expect("canonical root");
        let files = collect_project_evidence_files(&canonical).expect("evidence files");
        let paths = files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert!(paths.contains(&"spec/prd-demo/run-log.md"));
        assert!(paths.contains(&"spec/prd-demo/QA/qa-report.md"));
        assert!(paths.contains(&"spec/prd-demo/QA/logs/verify.log"));
        assert!(paths.contains(&"spec/prd-demo/QA/screenshots/smoke.png"));
        assert_eq!(
            evidence_kind_for_path("spec/prd-demo/QA/screenshots/smoke.png"),
            "screenshot"
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn flatten_paths(entries: &[SourceEntry]) -> Vec<String> {
        let mut paths = Vec::new();
        for entry in entries {
            paths.push(entry.relative_path.clone());
            paths.extend(flatten_paths(&entry.children));
        }
        paths
    }
}
