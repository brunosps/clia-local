use anyhow::{anyhow, Context};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::store;

const MANIFEST_VERSION: &str = "1.1";
const MAX_WKSDW_FILE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSkillSummary {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    pub path: Option<String>,
    pub scope: String,
    pub scope_label: String,
    pub bundled: bool,
    pub owner: Option<String>,
    pub kind: Option<String>,
    pub tier: Option<String>,
    pub group: Option<String>,
    pub framework_id: Option<String>,
    pub framework_label: Option<String>,
    pub exportable: bool,
    pub priority: i32,
    pub installed_targets: Vec<String>,
    pub file_count: usize,
    pub byte_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSkillSearchResult {
    pub name: String,
    pub package: String,
    pub description: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // skills install path kept for future use; not wired in clia.local yet
pub struct WorkspaceSkillInstallInput {
    pub workspace_path: String,
    pub package_slug: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceSkillSyncInput {
    pub workspace_path: String,
    pub name: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionManifest {
    pub schema_version: String,
    pub exported_at: String,
    pub workspace: WorkspaceSolutionWorkspace,
    #[serde(default)]
    pub projects: Vec<WorkspaceSolutionProject>,
    #[serde(default)]
    pub machines: Vec<WorkspaceSolutionMachine>,
    pub skills: Vec<WorkspaceSolutionSkill>,
    pub flows: WorkspaceSolutionFlows,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionWorkspace {
    pub name: String,
    #[serde(default)]
    pub metadata_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionProject {
    pub name: String,
    pub path_hint: String,
    #[serde(default)]
    pub remote_url: Option<String>,
    #[serde(default)]
    pub remotes: Vec<WorkspaceSolutionRemote>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionRemote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionMachine {
    pub display_name: String,
    pub provider: String,
    pub provider_runtime: String,
    pub provider_profile: String,
    pub preset_id: String,
    pub image_family: String,
    pub status: String,
    #[serde(default)]
    pub web_port: Option<i64>,
    #[serde(default)]
    pub rdp_port: Option<i64>,
    #[serde(default)]
    pub ssh_port: Option<i64>,
    #[serde(default)]
    pub last_health_status: Option<String>,
    #[serde(default)]
    pub last_health_summary: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionSkill {
    pub name: String,
    pub path: String,
    pub file_count: usize,
    pub byte_count: u64,
    #[serde(default)]
    pub framework_id: Option<String>,
    #[serde(default)]
    pub framework_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSolutionFlows {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceFrameworkSummary {
    pub id: String,
    pub label: String,
    pub description: String,
    pub source: String,
    pub installed: bool,
    pub installable: bool,
    pub required: bool,
    pub flow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceCapabilities {
    pub skills: Vec<WorkspaceSkillSummary>,
    pub frameworks: Vec<WorkspaceFrameworkSummary>,
}

#[derive(Debug, Clone, Default)]
struct SkillRegistryMetadata {
    description: Option<String>,
    bundled: bool,
    owner: Option<String>,
    kind: Option<String>,
    tier: Option<String>,
    group: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct InstallStateMetadata {
    package: Option<String>,
    framework: Option<String>,
    group: Option<String>,
    managed_files: BTreeSet<String>,
}

impl InstallStateMetadata {
    fn group_label(&self) -> Option<String> {
        self.group
            .clone()
            .or_else(|| self.framework.clone())
            .or_else(|| self.package.as_deref().and_then(package_group_label))
    }
}

#[derive(Debug, Clone)]
struct SkillScanRoot {
    base: PathBuf,
    source: String,
    scope: String,
    scope_label: String,
    exportable: bool,
    priority: i32,
    framework_id: Option<String>,
    framework_label: Option<String>,
    require_managed: bool,
}

pub fn list_workspace_skills(workspace_path: &Path) -> anyhow::Result<Vec<WorkspaceSkillSummary>> {
    let root = canonical_dir(workspace_path)?;
    let registry = skill_registry_metadata(&root);
    let install_state = install_state_metadata(&root);
    let install_state_group = install_state.group_label();
    collect_skills_from_roots(
        &root,
        &registry,
        &install_state,
        workspace_skill_scan_roots(&root, &install_state),
        install_state_group.as_deref(),
    )
}

fn collect_skills_from_roots(
    root: &Path,
    registry: &BTreeMap<String, SkillRegistryMetadata>,
    install_state: &InstallStateMetadata,
    scan_roots: Vec<SkillScanRoot>,
    install_state_group: Option<&str>,
) -> anyhow::Result<Vec<WorkspaceSkillSummary>> {
    let mut by_key = BTreeMap::<String, WorkspaceSkillSummary>::new();

    for scan_root in scan_roots {
        let base = scan_root.base;
        if !base.exists() {
            continue;
        }
        for skill_dir in skill_dirs(&base)? {
            let skill_file = skill_dir.join("SKILL.md");
            let content = fs::read_to_string(&skill_file).unwrap_or_default();
            let metadata = parse_skill_frontmatter(&content);
            let Some(name) = metadata.get("name").cloned().or_else(|| {
                skill_dir
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
            }) else {
                continue;
            };
            let (file_count, byte_count) = dir_stats(&skill_dir)?;
            let relative_path = relative_to(root, &skill_file);
            if scan_root.require_managed
                && !relative_path
                    .as_deref()
                    .is_some_and(|relative| install_state.managed_files.contains(relative))
            {
                continue;
            }
            let source_for_skill = relative_path
                .as_deref()
                .filter(|relative| install_state.managed_files.contains(*relative))
                .map(|_| "bundled")
                .unwrap_or(scan_root.source.as_str());
            let registry_metadata = registry.get(&name).cloned().unwrap_or_default();
            let description = metadata
                .get("description")
                .cloned()
                .or_else(|| registry_metadata.description.clone());
            let framework_label = registry_metadata
                .group
                .clone()
                .or_else(|| scan_root.framework_label.clone())
                .or_else(|| {
                    if source_for_skill == "bundled" {
                        install_state_group.map(ToString::to_string)
                    } else {
                        None
                    }
                });
            let group = skill_group(
                source_for_skill,
                &registry_metadata,
                framework_label.as_deref(),
            );
            let framework_label = framework_label.or_else(|| {
                if group != "Avulsas" {
                    Some(group.clone())
                } else {
                    None
                }
            });
            let framework_id = scan_root
                .framework_id
                .clone()
                .or_else(|| framework_label.as_deref().map(canonical_framework_id));
            let targets = installed_targets(root, &name);
            let key = format!("{}:{}:{}", scan_root.priority, scan_root.scope, name);
            by_key
                .entry(key)
                .and_modify(|existing| {
                    existing.description =
                        existing.description.clone().or_else(|| description.clone());
                    merge_skill_registry_metadata(
                        existing,
                        source_for_skill,
                        &registry_metadata,
                        framework_label.as_deref(),
                    );
                    existing.installed_targets = targets.clone();
                    existing.file_count = existing.file_count.max(file_count);
                    existing.byte_count = existing.byte_count.max(byte_count);
                    if existing.path.is_none() {
                        existing.path = relative_path.clone();
                    }
                    if existing.source != "workspace"
                        && matches!(source_for_skill, "workspace" | "bundled")
                    {
                        existing.source = source_for_skill.to_string();
                    }
                })
                .or_insert(WorkspaceSkillSummary {
                    name,
                    description,
                    source: source_for_skill.to_string(),
                    path: relative_path,
                    scope: scan_root.scope.clone(),
                    scope_label: scan_root.scope_label.clone(),
                    bundled: registry_metadata.bundled || source_for_skill == "bundled",
                    owner: registry_metadata.owner.clone(),
                    kind: registry_metadata.kind.clone(),
                    tier: registry_metadata.tier.clone(),
                    group: Some(group),
                    framework_id,
                    framework_label,
                    exportable: scan_root.exportable,
                    priority: scan_root.priority,
                    installed_targets: targets,
                    file_count,
                    byte_count,
                });
        }
    }

    let mut values = by_key.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.scope_label.cmp(&right.scope_label))
            .then_with(|| {
                left.framework_label
                    .as_deref()
                    .unwrap_or("Avulsas")
                    .cmp(right.framework_label.as_deref().unwrap_or("Avulsas"))
            })
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(values)
}

pub fn find_workspace_skills(query: &str) -> anyhow::Result<Vec<WorkspaceSkillSearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let output = Command::new("npx")
        .args(["skills", "find", query])
        .output()
        .with_context(|| "failed to execute npx skills find")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if !output.status.success() {
        return Err(anyhow!(
            "npx skills find failed with {}: {}",
            output.status,
            combined.trim()
        ));
    }
    Ok(parse_skill_search_output(&combined))
}

#[allow(dead_code)] // skills install path kept for future use; not wired in clia.local yet
pub fn install_workspace_skill(
    input: WorkspaceSkillInstallInput,
) -> anyhow::Result<Vec<WorkspaceSkillSummary>> {
    let root = canonical_dir(Path::new(&input.workspace_path))?;
    let package_slug = input.package_slug.trim();
    if package_slug.is_empty() || package_slug.starts_with('-') {
        return Err(anyhow!("invalid skill package slug"));
    }

    let before = skill_names(&root)?;
    let output = Command::new("npx")
        .current_dir(&root)
        .args(["skills", "add", package_slug, "-y"])
        .output()
        .with_context(|| "failed to execute npx skills add")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if !output.status.success() {
        return Err(anyhow!(
            "npx skills add failed with {}: {}",
            output.status,
            combined.trim()
        ));
    }

    let after = skill_names(&root)?;
    let mut installed: Vec<String> = after.difference(&before).cloned().collect();
    if installed.is_empty() {
        installed.push(skill_name_from_package(package_slug));
    }
    let targets = normalized_targets(&input.targets);
    for name in installed {
        if source_skill_dir(&root, &name).is_some() {
            sync_workspace_skill(WorkspaceSkillSyncInput {
                workspace_path: root.display().to_string(),
                name,
                targets: targets.clone(),
            })?;
        }
    }

    list_workspace_skills(&root)
}

pub fn sync_workspace_skill(
    input: WorkspaceSkillSyncInput,
) -> anyhow::Result<Vec<WorkspaceSkillSummary>> {
    let root = canonical_dir(Path::new(&input.workspace_path))?;
    let name = safe_name(&input.name)?;
    let targets = normalized_targets(&input.targets);
    let source =
        source_skill_dir(&root, &name).ok_or_else(|| anyhow!("skill not found: {name}"))?;

    for target in targets {
        match target.as_str() {
            "workspace" => copy_skill_dir(&source, &workspace_skill_dir(&root, &name))?,
            "codex" => copy_skill_dir(&source, &root.join(".agents").join("skills").join(&name))?,
            "claude" => copy_skill_dir(&source, &root.join(".claude").join("skills").join(&name))?,
            "copilot" => write_copilot_prompt(&root, &name, &source)?,
            _ => {}
        }
    }
    list_workspace_skills(&root)
}

pub fn read_workspace_flow_artifact(
    workspace_path: &Path,
    relative_path: &str,
) -> anyhow::Result<String> {
    let root = canonical_dir(workspace_path)?;
    let path = workspace_flow_artifact_path(&root, relative_path)?;
    let canonical = fs::canonicalize(&path)?;
    if !canonical.starts_with(workspace_flow_root(&root)) {
        return Err(anyhow!("workspace flow path escapes root"));
    }
    Ok(fs::read_to_string(canonical)?)
}

pub fn write_workspace_flow_artifact(
    workspace_path: &Path,
    relative_path: &str,
    content: &str,
) -> anyhow::Result<String> {
    let root = canonical_dir(workspace_path)?;
    let path = workspace_flow_artifact_path(&root, relative_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    Ok(relative_to(&root, &path).unwrap_or_else(|| path.display().to_string()))
}

pub fn sync_workspace_flows(workspace_path: &Path, project_path: &Path) -> anyhow::Result<()> {
    let workspace = canonical_dir(workspace_path)?;
    let project = canonical_dir(project_path)?;
    let source = workspace_flow_root(&workspace);
    if !source.exists() {
        return Ok(());
    }
    let target = project.join(".dw").join("flows");
    copy_dir_contents(&source, &target)?;
    Ok(())
}

pub fn export_workspace_solution(
    workspace: &store::Workspace,
    projects: &[store::Project],
    machines: &[store::WorkspaceMachine],
    destination_path: &Path,
) -> anyhow::Result<WorkspaceSolutionManifest> {
    let root = canonical_dir(Path::new(&workspace.root_path))?;
    let flows = collect_relative_files(&workspace_flow_root(&root))?;
    let exportable_skills = list_workspace_skills(&root)?
        .into_iter()
        .filter(|skill| skill.exportable)
        .collect::<Vec<_>>();
    let skills = exportable_skills
        .iter()
        .map(|skill| WorkspaceSolutionSkill {
            path: skill_archive_path(skill),
            name: skill.name.clone(),
            file_count: skill.file_count,
            byte_count: skill.byte_count,
            framework_id: skill.framework_id.clone(),
            framework_label: skill.framework_label.clone(),
        })
        .collect::<Vec<_>>();
    let manifest = WorkspaceSolutionManifest {
        schema_version: MANIFEST_VERSION.to_string(),
        exported_at: Utc::now().to_rfc3339(),
        workspace: WorkspaceSolutionWorkspace {
            name: workspace.name.clone(),
            metadata_path: Some(".dw/gui/workspace.json".to_string()),
        },
        projects: collect_project_manifest(&root, projects),
        machines: collect_machine_manifest(machines),
        skills,
        flows: WorkspaceSolutionFlows {
            files: flows
                .iter()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .collect(),
        },
    };

    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_workspace_metadata(&root, &manifest)?;
    let file = File::create(destination_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    add_files_to_zip(
        &mut zip,
        &workspace_flow_root(&root),
        "flows",
        &flows,
        options,
    )?;
    let capability_index = capability_index_for_export(&exportable_skills);
    if let Some(index_json) = capability_index {
        zip.start_file("capabilities/index.json", options)?;
        zip.write_all(index_json.as_bytes())?;
    }
    for skill in &exportable_skills {
        let Some(skill_file) = skill_absolute_path(&root, skill) else {
            continue;
        };
        let Some(skill_dir) = skill_file.parent() else {
            continue;
        };
        let files = collect_relative_files(skill_dir)?;
        add_files_to_zip(
            &mut zip,
            skill_dir,
            &skill_archive_path(skill),
            &files,
            options,
        )?;
    }
    zip.finish()?;

    Ok(manifest)
}

pub fn preview_workspace_solution(source_path: &Path) -> anyhow::Result<WorkspaceSolutionManifest> {
    read_solution_manifest(source_path)
}

pub fn import_workspace_solution(
    workspace_path: &Path,
    source_path: &Path,
) -> anyhow::Result<WorkspaceSolutionManifest> {
    let root = canonical_dir(workspace_path)?;
    let metadata = fs::metadata(source_path)?;
    if metadata.len() > MAX_WKSDW_FILE_BYTES {
        return Err(anyhow!("workspace solution is too large"));
    }

    let file = File::open(source_path)?;
    let mut zip = ZipArchive::new(file)?;
    let mut manifest: Option<WorkspaceSolutionManifest> = None;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let name = safe_zip_name(entry.name())?;
        if name == Path::new("manifest.json") {
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            manifest = Some(serde_json::from_str(&text)?);
            continue;
        }

        let target = if let Ok(relative) = name.strip_prefix("flows") {
            workspace_flow_root(&root).join(relative)
        } else if let Ok(relative) = name.strip_prefix("capabilities") {
            workspace_capability_root(&root).join(relative)
        } else if let Ok(relative) = name.strip_prefix("skills") {
            workspace_skill_root(&root).join(relative)
        } else {
            continue;
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(target)?;
        std::io::copy(&mut entry, &mut output)?;
    }

    let manifest =
        manifest.ok_or_else(|| anyhow!("manifest.json not found in workspace solution"))?;
    write_workspace_metadata(&root, &manifest)?;
    Ok(manifest)
}

pub fn list_workspace_capabilities(
    workspace: &store::Workspace,
    active_project: Option<&store::Project>,
) -> anyhow::Result<WorkspaceCapabilities> {
    let root = canonical_dir(Path::new(&workspace.root_path))?;
    let mut skills = Vec::<WorkspaceSkillSummary>::new();
    let workspace_framework_ids = installed_capability_framework_ids(&root);

    if let Some(project) = active_project {
        let project_root = PathBuf::from(&project.path);
        if project_root.is_dir() {
            let project_root = canonical_dir(&project_root)?;
            for mut skill in list_project_skills(&project_root, &project.name)? {
                let project_framework_id = skill
                    .framework_id
                    .clone()
                    .or_else(|| skill.framework_label.as_deref().map(canonical_framework_id));
                if project_framework_id
                    .as_ref()
                    .is_some_and(|id| workspace_framework_ids.contains(id))
                {
                    continue;
                }
                absolutize_skill_path(&mut skill, &project_root, &root);
                skills.push(skill);
            }
        }
    }

    skills.extend(list_workspace_skills(&root)?);
    skills.extend(list_home_skills()?);
    skills.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.scope_label.cmp(&right.scope_label))
            .then_with(|| {
                left.framework_label
                    .as_deref()
                    .unwrap_or("Avulsas")
                    .cmp(right.framework_label.as_deref().unwrap_or("Avulsas"))
            })
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(WorkspaceCapabilities {
        skills,
        frameworks: workspace_frameworks(&root),
    })
}

fn list_project_skills(
    project_root: &Path,
    project_name: &str,
) -> anyhow::Result<Vec<WorkspaceSkillSummary>> {
    let registry = skill_registry_metadata(project_root);
    let install_state = install_state_metadata(project_root);
    let install_state_group = install_state.group_label();
    collect_skills_from_roots(
        project_root,
        &registry,
        &install_state,
        project_skill_scan_roots(project_root, project_name),
        install_state_group.as_deref(),
    )
}

fn list_home_skills() -> anyhow::Result<Vec<WorkspaceSkillSummary>> {
    let Some(home) = dirs::home_dir() else {
        return Ok(Vec::new());
    };
    let registry = BTreeMap::new();
    let install_state = InstallStateMetadata::default();
    let mut skills = collect_skills_from_roots(
        &home,
        &registry,
        &install_state,
        home_skill_scan_roots(),
        None,
    )?;
    for skill in &mut skills {
        if let Some(path) = skill
            .path
            .as_ref()
            .filter(|path| !Path::new(path).is_absolute())
        {
            skill.path = Some(home.join(path).display().to_string());
        }
    }
    Ok(skills)
}

fn absolutize_skill_path(
    skill: &mut WorkspaceSkillSummary,
    skill_root: &Path,
    workspace_root: &Path,
) {
    let Some(path) = skill.path.as_ref() else {
        return;
    };
    let absolute = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        skill_root.join(path)
    };
    skill.path =
        relative_to(workspace_root, &absolute).or_else(|| Some(absolute.display().to_string()));
}

fn collect_project_manifest(
    workspace_root: &Path,
    projects: &[store::Project],
) -> Vec<WorkspaceSolutionProject> {
    projects
        .iter()
        .map(|project| {
            let path = PathBuf::from(&project.path);
            let path_hint = relative_to(workspace_root, &path)
                .unwrap_or_else(|| format!("projects/{}", safe_project_dir_name(&project.name)));
            let mut remotes = project_remotes(&path);
            if remotes.is_empty() {
                if let Some(remote_url) = project
                    .remote_url
                    .as_ref()
                    .filter(|value| !value.is_empty())
                {
                    remotes.push(WorkspaceSolutionRemote {
                        name: "origin".to_string(),
                        url: remote_url.clone(),
                    });
                }
            }
            let remote_url = project
                .remote_url
                .clone()
                .or_else(|| {
                    remotes
                        .iter()
                        .find(|remote| remote.name == "origin")
                        .map(|remote| remote.url.clone())
                })
                .or_else(|| remotes.first().map(|remote| remote.url.clone()));
            WorkspaceSolutionProject {
                name: project.name.clone(),
                path_hint,
                remote_url,
                remotes,
                branch: git_capture(&path, &["symbolic-ref", "--short", "-q", "HEAD"]).ok(),
                upstream: git_capture(&path, &["rev-parse", "--abbrev-ref", "@{u}"]).ok(),
            }
        })
        .collect()
}

fn collect_machine_manifest(machines: &[store::WorkspaceMachine]) -> Vec<WorkspaceSolutionMachine> {
    machines
        .iter()
        .map(|machine| WorkspaceSolutionMachine {
            display_name: machine.display_name.clone(),
            provider: machine.provider.clone(),
            provider_runtime: machine.provider_runtime.clone(),
            provider_profile: machine.provider_profile.clone(),
            preset_id: machine.preset_id.clone(),
            image_family: machine.image_family.clone(),
            status: machine.status.clone(),
            web_port: machine.web_port,
            rdp_port: machine.rdp_port,
            ssh_port: machine.ssh_port,
            last_health_status: machine.last_health_status.clone(),
            last_health_summary: machine.last_health_summary.clone(),
            updated_at: machine.updated_at.clone(),
        })
        .collect()
}

fn project_remotes(path: &Path) -> Vec<WorkspaceSolutionRemote> {
    let names = git_capture(path, &["remote"]).unwrap_or_default();
    names
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .filter_map(|name| {
            let url = git_capture(path, &["remote", "get-url", name]).ok()?;
            Some(WorkspaceSolutionRemote {
                name: name.to_string(),
                url,
            })
        })
        .collect()
}

fn git_capture(path: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git in {}", path.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}").trim().to_string();
    if output.status.success() {
        Ok(combined)
    } else {
        Err(anyhow!("git {} failed: {combined}", args.join(" ")))
    }
}

fn read_solution_manifest(source_path: &Path) -> anyhow::Result<WorkspaceSolutionManifest> {
    let metadata = fs::metadata(source_path)?;
    if metadata.len() > MAX_WKSDW_FILE_BYTES {
        return Err(anyhow!("workspace solution is too large"));
    }
    let file = File::open(source_path)?;
    let mut zip = ZipArchive::new(file)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = safe_zip_name(entry.name())?;
        if name == Path::new("manifest.json") {
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            return Ok(serde_json::from_str(&text)?);
        }
    }
    Err(anyhow!("manifest.json not found in workspace solution"))
}

fn write_workspace_metadata(
    root: &Path,
    manifest: &WorkspaceSolutionManifest,
) -> anyhow::Result<()> {
    let path = root.join(".dw").join("gui").join("workspace.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(manifest)?)?;
    Ok(())
}

fn workspace_frameworks(root: &Path) -> Vec<WorkspaceFrameworkSummary> {
    let installed_flows = installed_flow_presets(root);
    let installed_capabilities = installed_capability_framework_ids(root);
    builtin_frameworks()
        .into_iter()
        .map(|(id, label, description)| {
            let flow_id = installed_flows.get(id).cloned();
            let required = id == "dev-workflow";
            let installed = required || installed_capabilities.contains(id);
            WorkspaceFrameworkSummary {
                id: id.to_string(),
                label: label.to_string(),
                description: description.to_string(),
                source: "builtin".to_string(),
                installed,
                installable: !installed,
                required,
                flow_id,
            }
        })
        .collect()
}

fn installed_flow_presets(root: &Path) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let text = fs::read_to_string(workspace_flow_root(root).join("index.json")).unwrap_or_default();
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return values;
    };
    let Some(flows) = json.get("flows").and_then(|value| value.as_array()) else {
        return values;
    };
    for flow in flows {
        let id = flow.get("id").and_then(|value| value.as_str());
        let preset = flow.get("preset").and_then(|value| value.as_str()).or(id);
        if let (Some(id), Some(preset)) = (id, preset) {
            values
                .entry(preset.to_string())
                .or_insert_with(|| id.to_string());
        }
    }
    values
}

fn builtin_frameworks() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "dev-workflow",
            "dev-workflow",
            "Pipeline PRD -> TechSpec -> Tasks -> Run -> Review -> QA -> Security -> Commit -> PR.",
        ),
        (
            "spec-kit",
            "GitHub spec-kit",
            "Pipeline /speckit.*: constitution -> specify -> clarify -> plan -> tasks -> analyze -> implement.",
        ),
        (
            "openspec",
            "OpenSpec",
            "Change proposals /opsx:*: propose -> apply -> verify -> sync -> archive.",
        ),
        (
            "bmad",
            "BMAD-METHOD",
            "Personas: brief -> PRD -> UX -> architecture -> stories -> readiness -> dev -> review.",
        ),
    ]
}

fn parse_skill_search_output(output: &str) -> Vec<WorkspaceSkillSearchResult> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("npm ") && !line.starts_with("Need to install"))
        .take(20)
        .map(|line| {
            let package = line
                .split_whitespace()
                .find(|token| token.contains('/') || token.contains('@'))
                .unwrap_or(line)
                .trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '"' | '\''))
                .to_string();
            let name = skill_name_from_package(&package);
            WorkspaceSkillSearchResult {
                name,
                package,
                description: Some(line.to_string()),
                raw: line.to_string(),
            }
        })
        .collect()
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
        metadata.insert(
            key.trim().to_string(),
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string(),
        );
    }
    metadata
}

fn skill_registry_metadata(root: &Path) -> BTreeMap<String, SkillRegistryMetadata> {
    let path = root.join(".dw").join("skill-registry.json");
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return BTreeMap::new();
    };
    let Some(items) = json.get("skills").and_then(|value| value.as_array()) else {
        return BTreeMap::new();
    };

    let mut values = BTreeMap::new();
    for item in items {
        let Some(name) = json_string(item, "name") else {
            continue;
        };
        values.insert(
            name,
            SkillRegistryMetadata {
                description: json_string(item, "description"),
                bundled: item
                    .get("bundled")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
                owner: json_string(item, "owner"),
                kind: json_string(item, "kind"),
                tier: json_string(item, "tier"),
                group: json_string(item, "group").or_else(|| json_string(item, "framework")),
            },
        );
    }
    values
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
}

fn install_state_metadata(root: &Path) -> InstallStateMetadata {
    let path = root.join(".dw").join("install-state.json");
    let Ok(text) = fs::read_to_string(path) else {
        return InstallStateMetadata::default();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return InstallStateMetadata::default();
    };
    let managed_files = json
        .get("managed_files")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .map(ToString::to_string)
        .collect();
    InstallStateMetadata {
        package: json_string(&json, "package"),
        framework: json_string(&json, "framework"),
        group: json_string(&json, "group"),
        managed_files,
    }
}

fn package_group_label(package: &str) -> Option<String> {
    let package = package.trim().trim_end_matches(".git");
    if package.is_empty() {
        return None;
    }
    let without_version = package
        .rfind('@')
        .filter(|index| *index > 0)
        .map(|index| &package[..index])
        .unwrap_or(package);
    without_version
        .rsplit('/')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn framework_id_from_label(label: &str) -> String {
    label
        .trim()
        .to_lowercase()
        .replace('&', " and ")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn canonical_framework_id(label: &str) -> String {
    let normalized = framework_id_from_label(label);
    builtin_frameworks()
        .into_iter()
        .find(|(id, builtin_label, _)| {
            *id == label
                || id.eq_ignore_ascii_case(label)
                || builtin_label.eq_ignore_ascii_case(label)
                || framework_id_from_label(builtin_label) == normalized
        })
        .map(|(id, _, _)| id.to_string())
        .unwrap_or(normalized)
}

fn skill_group(
    source: &str,
    metadata: &SkillRegistryMetadata,
    fallback_group: Option<&str>,
) -> String {
    if let Some(group) = metadata.group.as_ref().filter(|value| !value.is_empty()) {
        return group.clone();
    }
    if source == "bundled" {
        return fallback_group.unwrap_or("Bundled").to_string();
    }
    if metadata.bundled {
        return "Bundled".to_string();
    }
    "Avulsas".to_string()
}

fn merge_skill_registry_metadata(
    skill: &mut WorkspaceSkillSummary,
    source: &str,
    metadata: &SkillRegistryMetadata,
    install_state_group: Option<&str>,
) {
    skill.bundled |= metadata.bundled || source == "bundled";
    skill.owner = skill.owner.clone().or_else(|| metadata.owner.clone());
    skill.kind = skill.kind.clone().or_else(|| metadata.kind.clone());
    skill.tier = skill.tier.clone().or_else(|| metadata.tier.clone());
    let group = skill_group(source, metadata, install_state_group);
    if skill
        .group
        .as_deref()
        .is_none_or(|value| value == "Avulsas")
    {
        skill.group = Some(group);
    }
}

fn workspace_skill_scan_roots(
    root: &Path,
    install_state: &InstallStateMetadata,
) -> Vec<SkillScanRoot> {
    let mut roots = Vec::new();
    roots.push(SkillScanRoot {
        base: workspace_skill_root(root),
        source: "workspace".to_string(),
        scope: "workspace".to_string(),
        scope_label: "Workspace".to_string(),
        exportable: true,
        priority: 1,
        framework_id: None,
        framework_label: None,
        require_managed: false,
    });

    let framework_labels = capability_framework_labels(root);
    for framework_dir in capability_framework_dirs(root) {
        let Some(framework_id) = framework_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            continue;
        };
        let framework_label = framework_labels
            .get(&framework_id)
            .cloned()
            .unwrap_or_else(|| framework_id.clone());
        roots.push(SkillScanRoot {
            base: framework_dir.join("skills"),
            source: "bundled".to_string(),
            scope: "workspace".to_string(),
            scope_label: "Workspace".to_string(),
            exportable: true,
            priority: 1,
            framework_id: Some(framework_id),
            framework_label: Some(framework_label),
            require_managed: false,
        });
    }

    if !install_state.managed_files.is_empty() {
        let framework_label = install_state.group_label();
        let framework_id = framework_label.as_deref().map(canonical_framework_id);
        for (base, source) in [
            (root.join(".agents").join("skills"), "codex"),
            (root.join(".claude").join("skills"), "claude"),
        ] {
            roots.push(SkillScanRoot {
                base,
                source: source.to_string(),
                scope: "workspace".to_string(),
                scope_label: "Workspace".to_string(),
                exportable: true,
                priority: 1,
                framework_id: framework_id.clone(),
                framework_label: framework_label.clone(),
                require_managed: true,
            });
        }
    }

    roots
}

fn project_skill_scan_roots(project_root: &Path, project_name: &str) -> Vec<SkillScanRoot> {
    [
        (project_root.join(".agents").join("skills"), "codex"),
        (project_root.join(".claude").join("skills"), "claude"),
    ]
    .into_iter()
    .map(|(base, source)| SkillScanRoot {
        base,
        source: source.to_string(),
        scope: "project".to_string(),
        scope_label: format!("Projeto: {project_name}"),
        exportable: false,
        priority: 0,
        framework_id: None,
        framework_label: None,
        require_managed: false,
    })
    .collect()
}

fn home_skill_scan_roots() -> Vec<SkillScanRoot> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    [
        (
            home.join(".codex").join("skills"),
            "home:codex",
            "Home: Codex",
        ),
        (
            home.join(".claude").join("skills"),
            "home:claude",
            "Home: Claude",
        ),
    ]
    .into_iter()
    .map(|(base, source, label)| SkillScanRoot {
        base,
        source: source.to_string(),
        scope: "home".to_string(),
        scope_label: label.to_string(),
        exportable: false,
        priority: 2,
        framework_id: None,
        framework_label: None,
        require_managed: false,
    })
    .collect()
}

fn skill_dirs(base: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    collect_skill_dirs(base, &mut dirs)?;
    dirs.sort();
    Ok(dirs)
}

fn collect_skill_dirs(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join("SKILL.md").is_file() {
            out.push(path);
        } else {
            collect_skill_dirs(&path, out)?;
        }
    }
    Ok(())
}

fn installed_targets(root: &Path, name: &str) -> Vec<String> {
    let mut targets = Vec::new();
    if workspace_skill_dir(root, name).join("SKILL.md").is_file() {
        targets.push("workspace".to_string());
    }
    if root
        .join(".agents")
        .join("skills")
        .join(name)
        .join("SKILL.md")
        .is_file()
    {
        targets.push("codex".to_string());
    }
    if root
        .join(".claude")
        .join("skills")
        .join(name)
        .join("SKILL.md")
        .is_file()
    {
        targets.push("claude".to_string());
    }
    if root
        .join(".github")
        .join("prompts")
        .join(format!("{name}.prompt.md"))
        .is_file()
    {
        targets.push("copilot".to_string());
    }
    targets
}

fn normalized_targets(targets: &[String]) -> Vec<String> {
    let mut values = BTreeSet::new();
    values.insert("workspace".to_string());
    if targets.is_empty() {
        values.insert("codex".to_string());
        values.insert("claude".to_string());
        values.insert("copilot".to_string());
    } else {
        for target in targets {
            match target.as_str() {
                "workspace" | "codex" | "claude" | "copilot" => {
                    values.insert(target.to_string());
                }
                _ => {}
            }
        }
    }
    values.into_iter().collect()
}

fn source_skill_dir(root: &Path, name: &str) -> Option<PathBuf> {
    let mut candidates = vec![
        workspace_skill_dir(root, name),
        root.join(".agents").join("skills").join(name),
        root.join(".claude").join("skills").join(name),
    ];
    for framework_dir in capability_framework_dirs(root) {
        candidates.push(framework_dir.join("skills").join(name));
    }
    for parent in [root.join("projects"), root.join("repos")] {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let project = entry.path();
                candidates.push(project.join(".agents").join("skills").join(name));
                candidates.push(project.join(".claude").join("skills").join(name));
            }
        }
    }
    candidates
        .into_iter()
        .find(|path| path.join("SKILL.md").is_file())
}

fn copy_skill_dir(source: &Path, target: &Path) -> anyhow::Result<()> {
    if source == target {
        return Ok(());
    }
    if target.exists() {
        fs::remove_dir_all(target)?;
    }
    copy_dir_contents(source, target)
}

fn copy_dir_contents(source: &Path, target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_contents(&source_path, &target_path)?;
        } else if source_path.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn write_copilot_prompt(root: &Path, name: &str, source: &Path) -> anyhow::Result<()> {
    let skill = fs::read_to_string(source.join("SKILL.md"))?;
    let dir = root.join(".github").join("prompts");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join(format!("{name}.prompt.md")),
        format!(
            "# Skill: {name}\n\nUse the following skill instructions when relevant.\n\n{skill}"
        ),
    )?;
    Ok(())
}

#[allow(dead_code)] // helper for the (currently unwired) skills install path
fn skill_names(root: &Path) -> anyhow::Result<BTreeSet<String>> {
    Ok(list_workspace_skills(root)?
        .into_iter()
        .map(|skill| skill.name)
        .collect())
}

fn skill_name_from_package(package: &str) -> String {
    let without_version = package
        .rsplit_once('@')
        .filter(|(left, _)| !left.is_empty())
        .map(|(_, right)| right)
        .unwrap_or(package);
    without_version
        .rsplit('/')
        .next()
        .unwrap_or(without_version)
        .trim()
        .trim_end_matches(".git")
        .to_string()
}

fn safe_project_dir_name(name: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
            output.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    let value = output.trim_matches(['-', '.']).to_string();
    if value.is_empty() {
        "project".to_string()
    } else {
        value
    }
}

fn safe_name(value: &str) -> anyhow::Result<String> {
    let name = value.trim();
    if name.is_empty()
        || name.starts_with('.')
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
    {
        return Err(anyhow!("invalid skill name"));
    }
    Ok(name.to_string())
}

fn normalize_relative_json(relative_path: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(relative_path.trim().trim_start_matches('/'));
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(anyhow!("relative path escapes workspace"));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return Err(anyhow!("workspace flow artifact must be json"));
    }
    Ok(path.to_path_buf())
}

fn safe_zip_name(name: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(anyhow!("zip entry escapes workspace: {name}"));
    }
    Ok(path.to_path_buf())
}

fn collect_relative_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if root.exists() {
        collect_relative_files_inner(root, root, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_relative_files_inner(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files_inner(root, &path, out)?;
        } else if path.is_file() {
            out.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn add_files_to_zip(
    zip: &mut ZipWriter<File>,
    root: &Path,
    prefix: &str,
    files: &[PathBuf],
    options: SimpleFileOptions,
) -> anyhow::Result<()> {
    for relative in files {
        let source = root.join(relative);
        let archive_name = format!("{prefix}/{}", relative.to_string_lossy().replace('\\', "/"));
        zip.start_file(archive_name, options)?;
        let bytes = fs::read(source)?;
        zip.write_all(&bytes)?;
    }
    Ok(())
}

fn skill_archive_path(skill: &WorkspaceSkillSummary) -> String {
    let framework_label = skill.framework_label.as_deref().unwrap_or("Avulsas");
    if framework_label == "Avulsas" {
        return format!("skills/{}", skill.name);
    }
    let framework_id = skill
        .framework_id
        .clone()
        .unwrap_or_else(|| canonical_framework_id(framework_label));
    format!("capabilities/{framework_id}/skills/{}", skill.name)
}

fn skill_absolute_path(root: &Path, skill: &WorkspaceSkillSummary) -> Option<PathBuf> {
    let path = skill.path.as_ref()?;
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(root.join(path))
    }
}

fn capability_index_for_export(skills: &[WorkspaceSkillSummary]) -> Option<String> {
    let mut frameworks = BTreeMap::<String, String>::new();
    for skill in skills {
        let Some(label) = skill.framework_label.as_ref() else {
            continue;
        };
        if label == "Avulsas" {
            continue;
        }
        let id = skill
            .framework_id
            .clone()
            .unwrap_or_else(|| canonical_framework_id(label));
        frameworks.entry(id).or_insert_with(|| label.clone());
    }
    if frameworks.is_empty() {
        return None;
    }
    let value = serde_json::json!({
        "version": 1,
        "frameworks": frameworks
            .into_iter()
            .map(|(id, label)| serde_json::json!({ "id": id, "label": label }))
            .collect::<Vec<_>>()
    });
    serde_json::to_string_pretty(&value).ok()
}

fn dir_stats(dir: &Path) -> anyhow::Result<(usize, u64)> {
    let mut files = 0;
    let mut bytes = 0;
    for relative in collect_relative_files(dir)? {
        files += 1;
        bytes += fs::metadata(dir.join(relative))?.len();
    }
    Ok((files, bytes))
}

fn canonical_dir(path: &Path) -> anyhow::Result<PathBuf> {
    let root = fs::canonicalize(path)?;
    if !root.is_dir() {
        return Err(anyhow!("workspace path is not a directory"));
    }
    Ok(root)
}

fn relative_to(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
}

fn workspace_flow_root(root: &Path) -> PathBuf {
    root.join(".dw").join("gui").join("flows")
}

fn workspace_flow_artifact_path(root: &Path, relative_path: &str) -> anyhow::Result<PathBuf> {
    let relative = normalize_relative_json(relative_path)?;
    if relative.starts_with("flows") {
        Ok(root.join(".dw").join("gui").join(relative))
    } else {
        Ok(workspace_flow_root(root).join(relative))
    }
}

fn workspace_skill_root(root: &Path) -> PathBuf {
    root.join(".dw").join("gui").join("skills")
}

fn workspace_skill_dir(root: &Path, name: &str) -> PathBuf {
    workspace_skill_root(root).join(name)
}

fn workspace_capability_root(root: &Path) -> PathBuf {
    root.join(".dw").join("gui").join("capabilities")
}

fn capability_framework_dirs(root: &Path) -> Vec<PathBuf> {
    let base = workspace_capability_root(root);
    let mut dirs = Vec::new();
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    dirs
}

fn installed_capability_framework_ids(root: &Path) -> BTreeSet<String> {
    capability_framework_dirs(root)
        .into_iter()
        .filter(|path| skill_dirs(&path.join("skills")).is_ok_and(|skills| !skills.is_empty()))
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .collect::<BTreeSet<_>>()
}

fn capability_framework_index_entries(root: &Path) -> BTreeMap<String, String> {
    let path = workspace_capability_root(root).join("index.json");
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return BTreeMap::new();
    };
    let Some(frameworks) = json.get("frameworks").and_then(|value| value.as_array()) else {
        return BTreeMap::new();
    };
    frameworks
        .iter()
        .filter_map(|framework| {
            let id = json_string(framework, "id")?;
            let label = json_string(framework, "label").unwrap_or_else(|| id.clone());
            Some((id, label))
        })
        .collect()
}

fn capability_framework_labels(root: &Path) -> BTreeMap<String, String> {
    let mut labels = builtin_frameworks()
        .into_iter()
        .map(|(id, label, _)| (id.to_string(), label.to_string()))
        .collect::<BTreeMap<_, _>>();
    labels.extend(capability_framework_index_entries(root));
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn write_file(root: &Path, relative_path: &str, content: &str) {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, content).expect("write file");
    }

    fn test_workspace(root: &Path) -> store::Workspace {
        store::Workspace {
            id: 1,
            name: "Workspace".to_string(),
            root_path: root.display().to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn workspace_solution_round_trips_flows_and_skills() {
        let source = temp_root("dw-gui-wksdw-source");
        write_file(
            &source,
            ".dw/gui/flows/index.json",
            r#"{"flows":[{"id":"dev","label":"Dev"}],"default":"dev"}"#,
        );
        write_file(
            &source,
            ".dw/gui/flows/dev.json",
            r#"{"version":1,"phases":[]}"#,
        );
        write_file(
            &source,
            ".dw/gui/skills/demo/SKILL.md",
            "---\nname: demo\ndescription: Demo skill\n---\n",
        );
        let package = source.join("workspace.wksdw");

        let manifest = export_workspace_solution(&test_workspace(&source), &[], &[], &package)
            .expect("export");
        assert_eq!(manifest.skills.len(), 1);
        assert_eq!(manifest.flows.files.len(), 2);

        let target = temp_root("dw-gui-wksdw-target");
        let imported = import_workspace_solution(&target, &package).expect("import");
        assert_eq!(imported.skills[0].name, "demo");
        assert!(target.join(".dw/gui/flows/index.json").is_file());
        assert!(target.join(".dw/gui/skills/demo/SKILL.md").is_file());
        assert!(target.join(".dw/gui/workspace.json").is_file());

        let _ = fs::remove_dir_all(source);
        let _ = fs::remove_dir_all(target);
    }

    #[test]
    fn workspace_solution_exports_project_remotes() {
        let source = temp_root("dw-gui-wksdw-projects");
        let project = source.join("projects").join("demo");
        fs::create_dir_all(&project).expect("project");
        std::process::Command::new("git")
            .arg("init")
            .arg(&project)
            .output()
            .expect("git init");
        std::process::Command::new("git")
            .arg("-C")
            .arg(&project)
            .args([
                "remote",
                "add",
                "origin",
                "https://example.com/acme/demo.git",
            ])
            .output()
            .expect("git remote");
        let package = source.join("workspace.wksdw");
        let projects = vec![store::Project {
            id: 1,
            workspace_id: 1,
            name: "demo".to_string(),
            path: project.display().to_string(),
            remote_url: None,
            parent_project_id: None,
            is_submodule: false,
            submodule_path: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }];

        let manifest =
            export_workspace_solution(&test_workspace(&source), &projects, &[], &package)
                .expect("export");
        assert_eq!(manifest.projects.len(), 1);
        assert_eq!(
            manifest.projects[0].remote_url.as_deref(),
            Some("https://example.com/acme/demo.git")
        );
        assert_eq!(manifest.projects[0].path_hint, "projects/demo");

        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn workspace_solution_exports_only_sync_safe_machine_metadata() {
        let source = temp_root("dw-gui-wksdw-machines");
        let package = source.join("workspace.wksdw");
        let machines = vec![store::WorkspaceMachine {
            id: "machine-1".to_string(),
            workspace_id: 1,
            project_id: None,
            provider: "winbox".to_string(),
            provider_runtime: "native".to_string(),
            provider_profile: "dw-1-dev".to_string(),
            display_name: "Dev VM".to_string(),
            preset_id: "ubuntu_server_lts".to_string(),
            image_family: "linux_distro".to_string(),
            access_user: Some("bruno".to_string()),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: None,
            ssh_port: Some(2222),
            last_health_status: Some("healthy".to_string()),
            last_health_summary: Some("Docker ready".to_string()),
            last_error_code: Some("operation_failed".to_string()),
            last_error_message: Some("secret=should-not-export".to_string()),
            created_at: "2026-05-29T00:00:00Z".to_string(),
            updated_at: "2026-05-29T01:00:00Z".to_string(),
        }];

        let manifest =
            export_workspace_solution(&test_workspace(&source), &[], &machines, &package)
                .expect("export");
        let manifest_json = serde_json::to_string(&manifest).expect("manifest json");

        assert_eq!(manifest.machines.len(), 1);
        assert_eq!(manifest.machines[0].provider_profile, "dw-1-dev");
        assert_eq!(manifest.machines[0].web_port, Some(8006));
        assert!(!manifest_json.contains("should-not-export"));
        assert!(!manifest_json.contains("last_error_message"));

        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn workspace_skills_include_registry_group_metadata() {
        let root = temp_root("dw-gui-skills-metadata");
        write_file(
            &root,
            ".dw/skill-registry.json",
            r#"{"skills":[{"name":"dw-brainstorm","kind":"protocol","tier":"core","owner":"dw-brainstorm","bundled":true}]}"#,
        );
        write_file(
            &root,
            ".agents/skills/dw-brainstorm/SKILL.md",
            "---\nname: dw-brainstorm\ndescription: Brainstorm\n---\n",
        );
        write_file(
            &root,
            ".dw/gui/skills/local-helper/SKILL.md",
            "---\nname: local-helper\ndescription: Local helper\n---\n",
        );
        write_file(
            &root,
            ".dw/install-state.json",
            r#"{"package":"@brunosps00/dev-workflow","managed_files":[".agents/skills/dw-brainstorm/SKILL.md",".agents/skills/api-testing-recipes/SKILL.md"]}"#,
        );
        write_file(
            &root,
            ".agents/skills/api-testing-recipes/SKILL.md",
            "---\nname: api-testing-recipes\ndescription: API recipes\n---\n",
        );

        let skills = list_workspace_skills(&root).expect("skills");
        let bundled = skills
            .iter()
            .find(|skill| skill.name == "dw-brainstorm")
            .expect("bundled skill");
        let managed = skills
            .iter()
            .find(|skill| skill.name == "api-testing-recipes")
            .expect("managed skill");
        let custom = skills
            .iter()
            .find(|skill| skill.name == "local-helper")
            .expect("custom skill");

        assert!(bundled.bundled);
        assert_eq!(bundled.group.as_deref(), Some("dev-workflow"));
        assert_eq!(bundled.owner.as_deref(), Some("dw-brainstorm"));
        assert_eq!(bundled.kind.as_deref(), Some("protocol"));
        assert_eq!(bundled.tier.as_deref(), Some("core"));
        assert!(managed.bundled);
        assert_eq!(managed.source, "bundled");
        assert_eq!(managed.group.as_deref(), Some("dev-workflow"));
        assert!(!custom.bundled);
        assert_eq!(custom.group.as_deref(), Some("Avulsas"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_skills_group_managed_files_by_framework_metadata() {
        let root = temp_root("dw-gui-skills-framework-group");
        write_file(
            &root,
            ".dw/install-state.json",
            r#"{"framework":"GitHub spec-kit","managed_files":[".agents/skills/speckit.specify/SKILL.md"]}"#,
        );
        write_file(
            &root,
            ".agents/skills/speckit.specify/SKILL.md",
            "---\nname: speckit.specify\ndescription: Specify feature\n---\n",
        );

        let skills = list_workspace_skills(&root).expect("skills");
        let spec_kit = skills
            .iter()
            .find(|skill| skill.name == "speckit.specify")
            .expect("spec-kit skill");

        assert!(spec_kit.bundled);
        assert_eq!(spec_kit.source, "bundled");
        assert_eq!(spec_kit.group.as_deref(), Some("GitHub spec-kit"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_capabilities_keep_project_workspace_and_home_scopes_separate() {
        let root = temp_root("dw-gui-capability-scopes");
        let project_root = root.join("projects").join("app");
        write_file(
            &root,
            ".dw/gui/skills/shared-skill/SKILL.md",
            "---\nname: shared-skill\ndescription: Workspace shared\n---\n",
        );
        write_file(
            &project_root,
            ".agents/skills/shared-skill/SKILL.md",
            "---\nname: shared-skill\ndescription: Project shared\n---\n",
        );
        let project = store::Project {
            id: 10,
            workspace_id: 1,
            name: "app".to_string(),
            path: project_root.display().to_string(),
            remote_url: None,
            parent_project_id: None,
            is_submodule: false,
            submodule_path: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let capabilities =
            list_workspace_capabilities(&test_workspace(&root), Some(&project)).expect("caps");
        let shared = capabilities
            .skills
            .iter()
            .filter(|skill| skill.name == "shared-skill")
            .collect::<Vec<_>>();

        assert_eq!(shared.len(), 2);
        assert_eq!(shared[0].scope, "project");
        assert!(!shared[0].exportable);
        assert_eq!(shared[1].scope, "workspace");
        assert!(shared[1].exportable);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dev_workflow_framework_is_always_installed() {
        let root = temp_root("dw-gui-framework-installed");

        let frameworks = workspace_frameworks(&root);
        let dev_workflow = frameworks
            .iter()
            .find(|framework| framework.id == "dev-workflow")
            .expect("dev-workflow framework");

        assert!(dev_workflow.installed);
        assert!(dev_workflow.required);
        assert!(!dev_workflow.installable);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn framework_flow_without_skills_is_still_installable() {
        let root = temp_root("dw-gui-framework-flow-only");
        write_file(
            &root,
            ".dw/gui/flows/index.json",
            r#"{"flows":[{"id":"spec-kit","label":"GitHub spec-kit","preset":"spec-kit"}],"default":"spec-kit"}"#,
        );
        write_file(&root, ".dw/gui/flows/spec-kit.json", r#"{"version":1}"#);

        let frameworks = workspace_frameworks(&root);
        let spec_kit = frameworks
            .iter()
            .find(|framework| framework.id == "spec-kit")
            .expect("spec-kit framework");

        assert!(!spec_kit.installed);
        assert!(spec_kit.installable);
        assert_eq!(spec_kit.flow_id.as_deref(), Some("spec-kit"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_solution_exports_workspace_capabilities_not_project_skills() {
        let source = temp_root("dw-gui-wksdw-capabilities");
        let project_root = source.join("projects").join("app");
        write_file(
            &source,
            ".dw/gui/capabilities/spec-kit/skills/speckit.specify/SKILL.md",
            "---\nname: speckit.specify\ndescription: Specify feature\n---\n",
        );
        write_file(
            &source,
            ".dw/gui/capabilities/index.json",
            r#"{"frameworks":[{"id":"spec-kit","label":"GitHub spec-kit"}]}"#,
        );
        write_file(
            &project_root,
            ".agents/skills/project-only/SKILL.md",
            "---\nname: project-only\ndescription: Project only\n---\n",
        );
        let project = store::Project {
            id: 10,
            workspace_id: 1,
            name: "app".to_string(),
            path: project_root.display().to_string(),
            remote_url: None,
            parent_project_id: None,
            is_submodule: false,
            submodule_path: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let package = source.join("workspace.wksdw");

        let manifest =
            export_workspace_solution(&test_workspace(&source), &[project], &[], &package)
                .expect("export");

        assert_eq!(manifest.skills.len(), 1);
        assert_eq!(manifest.skills[0].name, "speckit.specify");
        assert_eq!(
            manifest.skills[0].path,
            "capabilities/spec-kit/skills/speckit.specify"
        );
        assert_eq!(
            manifest.skills[0].framework_label.as_deref(),
            Some("GitHub spec-kit")
        );

        let target = temp_root("dw-gui-wksdw-capabilities-target");
        import_workspace_solution(&target, &package).expect("import");
        assert!(target
            .join(".dw/gui/capabilities/spec-kit/skills/speckit.specify/SKILL.md")
            .is_file());
        assert!(!target.join(".dw/gui/skills/project-only/SKILL.md").exists());

        let _ = fs::remove_dir_all(source);
        let _ = fs::remove_dir_all(target);
    }

    #[test]
    fn workspace_solution_rejects_zip_slip_entries() {
        let source = temp_root("dw-gui-wksdw-slip");
        let package = source.join("bad.wksdw");
        let file = File::create(&package).expect("package");
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file("../evil.txt", options).expect("entry");
        zip.write_all(b"evil").expect("write");
        zip.finish().expect("finish");

        let target = temp_root("dw-gui-wksdw-slip-target");
        let result = import_workspace_solution(&target, &package);
        assert!(result.is_err());
        assert!(!target.join("evil.txt").exists());

        let _ = fs::remove_dir_all(source);
        let _ = fs::remove_dir_all(target);
    }
}
