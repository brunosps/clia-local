use anyhow::Context;
use chrono::Utc;
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub path: String,
    pub remote_url: Option<String>,
    /// When this project is a git submodule, the id of its containing project.
    pub parent_project_id: Option<i64>,
    pub is_submodule: bool,
    /// Submodule path relative to the parent project root (e.g. "vendor/lib").
    pub submodule_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequirementCard {
    pub id: i64,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub project_ids: Vec<i64>,
    pub public_id: String,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub priority: String,
    pub checklist_json: String,
    pub agent_prompt: String,
    pub status: String,
    pub prd_slug: Option<String>,
    /// Which workbench flow this card follows (`.dw/flows/<id>.json`). `None` =
    /// unrouted, lives in the shared intake backlog.
    pub flow_id: Option<String>,
    pub archived_from_status: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequirementStageForm {
    pub card_id: i64,
    pub stage_id: String,
    pub payload_json: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequirementAttachment {
    pub id: i64,
    pub card_id: i64,
    pub name: String,
    pub file_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSource {
    pub id: i64,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub blueprint_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub file_path: String,
    pub original_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectBlueprint {
    pub id: String,
    pub workspace_id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub idea: String,
    pub agent_profile_id: Option<i64>,
    pub agent_session_id: Option<i64>,
    pub knowledge_source_ids_json: String,
    pub answers_json: String,
    pub running_summary: String,
    pub detected_subprojects_json: String,
    pub prd: String,
    pub techspec: String,
    pub tasks_json: String,
    pub definition_of_done: String,
    pub project_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectBlueprintMaterialization {
    pub blueprint: ProjectBlueprint,
    pub project: Project,
    pub cards: Vec<RequirementCard>,
    pub spec_dir: String,
}

pub struct ProjectBlueprintUpdate<'a> {
    pub id: &'a str,
    pub status: Option<&'a str>,
    pub agent_session_id: Option<i64>,
    pub knowledge_source_ids_json: Option<&'a str>,
    pub answers_json: Option<&'a str>,
    pub running_summary: Option<&'a str>,
    pub detected_subprojects_json: Option<&'a str>,
    pub prd: Option<&'a str>,
    pub techspec: Option<&'a str>,
    pub tasks_json: Option<&'a str>,
    pub definition_of_done: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceEntry {
    pub id: String,
    pub record_type: String,
    pub run_id: Option<i64>,
    pub item_id: Option<i64>,
    pub workspace_id: Option<i64>,
    pub project_id: Option<i64>,
    pub project_path: String,
    pub prd_slug: Option<String>,
    pub command: Option<String>,
    pub status: String,
    pub summary: String,
    pub kind: String,
    pub title: String,
    pub relative_path: Option<String>,
    pub absolute_path: Option<String>,
    pub terminal_session_id: Option<String>,
    pub terminal_log_path: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfile {
    pub id: i64,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub sandbox: String,
    pub context_mode: String,
    pub rtk_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSession {
    pub id: i64,
    pub profile_id: i64,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub requirement_card_id: Option<i64>,
    pub scope: String,
    pub project_path: String,
    pub provider: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub sandbox: String,
    pub context_mode: String,
    pub provider_session_id: Option<String>,
    pub codex_session_id: Option<String>,
    pub status: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct AgentProfileCreate<'a> {
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub name: &'a str,
    pub provider: &'a str,
    pub model: Option<&'a str>,
    pub reasoning_effort: Option<&'a str>,
    pub sandbox: &'a str,
    pub context_mode: &'a str,
    pub rtk_enabled: bool,
}

pub struct AgentProfileUpdate<'a> {
    pub id: i64,
    pub name: &'a str,
    pub provider: &'a str,
    pub model: Option<&'a str>,
    pub reasoning_effort: Option<&'a str>,
    pub sandbox: &'a str,
    pub context_mode: &'a str,
    pub rtk_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub raw_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRunEvent {
    pub id: i64,
    pub session_id: i64,
    pub run_id: String,
    pub provider: String,
    pub phase: String,
    pub elapsed_ms: i64,
    pub details_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMachine {
    pub id: String,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub provider: String,
    pub provider_runtime: String,
    pub provider_profile: String,
    pub display_name: String,
    pub preset_id: String,
    pub image_family: String,
    pub access_user: Option<String>,
    pub status: String,
    pub web_port: Option<i64>,
    pub rdp_port: Option<i64>,
    pub ssh_port: Option<i64>,
    pub last_health_status: Option<String>,
    pub last_health_summary: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct WorkspaceMachineCreate<'a> {
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub provider: &'a str,
    pub provider_runtime: &'a str,
    pub provider_profile: &'a str,
    pub display_name: &'a str,
    pub preset_id: &'a str,
    pub image_family: &'a str,
    pub access_user: Option<&'a str>,
    pub status: &'a str,
}

pub struct WorkspaceMachineUpdate<'a> {
    pub id: &'a str,
    pub status: &'a str,
    pub web_port: Option<i64>,
    pub rdp_port: Option<i64>,
    pub ssh_port: Option<i64>,
    pub last_health_status: Option<&'a str>,
    pub last_health_summary: Option<&'a str>,
    pub last_error_code: Option<&'a str>,
    pub last_error_message: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployStack {
    pub id: String,
    pub workspace_id: i64,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub active_version_id: Option<String>,
    pub active_machine_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployVersion {
    pub id: String,
    pub stack_id: String,
    pub workspace_id: i64,
    pub label: String,
    pub status: String,
    pub target_machine_id: Option<String>,
    pub artifact_path: String,
    pub manifest_path: String,
    pub manifest_json: String,
    pub review_status: String,
    pub reviewed_at: Option<String>,
    pub blocking_findings_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployVersionProject {
    pub id: i64,
    pub version_id: String,
    pub project_id: i64,
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub dirty: bool,
    pub package_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployRun {
    pub id: String,
    pub stack_id: String,
    pub version_id: Option<String>,
    pub machine_id: Option<String>,
    pub operation: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub summary: String,
    pub agent_profile_id: Option<i64>,
    pub agent_name: Option<String>,
    pub agent_provider: Option<String>,
    pub agent_model: Option<String>,
    pub orchestration_status: String,
    pub orchestration_report_json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployRunStep {
    pub id: i64,
    pub run_id: String,
    pub step_key: String,
    pub status: String,
    pub message: String,
    pub log_path: Option<String>,
    pub error_code: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceDeployReset {
    pub workspace_id: i64,
    pub workspace_machines: i64,
    pub deploy_stacks: i64,
    pub deploy_versions: i64,
    pub deploy_runs: i64,
    pub deploy_run_steps: i64,
    pub deploy_version_projects: i64,
    pub deploy_target_bootstrap: i64,
    pub removed_artifact_dirs: Vec<String>,
}

pub struct DeployStackCreate<'a> {
    pub workspace_id: i64,
    pub name: &'a str,
    pub slug: &'a str,
}

pub struct DeployVersionCreate<'a> {
    pub stack_id: &'a str,
    pub workspace_id: i64,
    pub label: &'a str,
    pub target_machine_id: Option<&'a str>,
    pub artifact_path: &'a str,
    pub manifest_path: &'a str,
    pub manifest_json: &'a str,
    pub blocking_findings_json: &'a str,
}

pub struct DeployVersionProjectCreate<'a> {
    pub version_id: &'a str,
    pub project_id: i64,
    pub name: &'a str,
    pub path: &'a str,
    pub branch: Option<&'a str>,
    pub commit_sha: Option<&'a str>,
    pub dirty: bool,
    pub package_path: &'a str,
}

pub fn agent_session_matches_profile_context(
    session: &AgentSession,
    profile: &AgentProfile,
    workspace_id: i64,
    project_id: Option<i64>,
    project_path: &str,
) -> bool {
    agent_session_matches_profile_context_for_scope(
        session,
        profile,
        workspace_id,
        project_id,
        project_path,
        "chat",
        None,
    )
}

pub fn agent_session_matches_profile_context_for_scope(
    session: &AgentSession,
    profile: &AgentProfile,
    workspace_id: i64,
    project_id: Option<i64>,
    project_path: &str,
    scope: &str,
    requirement_card_id: Option<i64>,
) -> bool {
    session.profile_id == profile.id
        && session.workspace_id == workspace_id
        && session.workspace_id == profile.workspace_id
        && session.project_id == project_id
        && session.scope == scope.trim()
        && session.requirement_card_id == requirement_card_id
        && session.project_path == project_path.trim()
}

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn open(app_dir: &Path) -> anyhow::Result<Self> {
        let path = app_dir.join("clia-local.sqlite3");
        let db = Self { path };
        db.migrate()?;
        Ok(db)
    }

    pub fn list_workspaces(&self) -> anyhow::Result<Vec<Workspace>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select id, name, root_path, created_at from workspaces order by created_at desc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn create_workspace(&self, name: &str, root_path: &str) -> anyhow::Result<Workspace> {
        let created_at = Utc::now().to_rfc3339();
        let root_path = root_path.trim();
        std::fs::create_dir_all(root_path)
            .with_context(|| format!("failed to create workspace root {root_path}"))?;
        let conn = self.connect()?;
        conn.execute(
            "insert into workspaces (name, root_path, created_at) values (?1, ?2, ?3)",
            params![name.trim(), root_path, created_at],
        )?;
        let id = conn.last_insert_rowid();
        let workspace = Workspace {
            id,
            name: name.trim().to_string(),
            root_path: root_path.to_string(),
            created_at,
        };
        let _ = self.workspace_connect(&workspace)?;
        Ok(workspace)
    }

    pub fn get_workspace(&self, id: i64) -> anyhow::Result<Workspace> {
        let conn = self.connect()?;
        conn.query_row(
            "select id, name, root_path, created_at from workspaces where id = ?1",
            [id],
            |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    root_path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("workspace not found: {id}"))
    }

    pub fn get_app_state(&self, key: &str) -> anyhow::Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row("select value from app_state where key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .map_err(Into::into)
    }

    pub fn set_app_state(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let updated_at = Utc::now().to_rfc3339();
        let conn = self.connect()?;
        conn.execute(
            "insert into app_state (key, value, updated_at) values (?1, ?2, ?3)
             on conflict(key) do update set value = ?2, updated_at = ?3",
            params![key, value, updated_at],
        )?;
        Ok(())
    }

    pub fn list_projects(&self, workspace_id: i64) -> anyhow::Result<Vec<Project>> {
        self.reconcile_workspace_projects(workspace_id)?;
        let conn = self.workspace_connect_by_id(workspace_id)?;
        dedupe_workspace_projects(&conn, workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, name, path, remote_url, parent_project_id,
                    is_submodule, submodule_path, created_at
             from projects
             where workspace_id = ?1
             order by name asc",
        )?;
        let rows = stmt.query_map([workspace_id], project_from_row)?;
        collect_rows(rows)
    }

    pub fn list_workspace_machines(
        &self,
        workspace_id: i64,
    ) -> anyhow::Result<Vec<WorkspaceMachine>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, project_id, provider, provider_runtime, provider_profile,
                    display_name, preset_id, image_family, access_user, status,
                    web_port, rdp_port, ssh_port,
                    last_health_status, last_health_summary, last_error_code, last_error_message,
                    created_at, updated_at
             from workspace_machines
             where workspace_id = ?1
             order by updated_at desc, display_name asc",
        )?;
        let rows = stmt.query_map([workspace_id], workspace_machine_from_row)?;
        collect_rows(rows)
    }

    pub fn create_workspace_machine(
        &self,
        input: WorkspaceMachineCreate<'_>,
    ) -> anyhow::Result<WorkspaceMachine> {
        let conn = self.workspace_connect_by_id(input.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        let id = format!(
            "machine-{}",
            Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| Utc::now().timestamp_micros())
        );
        conn.execute(
            "insert into workspace_machines (
               id, workspace_id, project_id, provider, provider_runtime, provider_profile,
               display_name, preset_id, image_family, access_user, status,
               web_port, rdp_port, ssh_port,
               last_health_status, last_health_summary, last_error_code, last_error_message,
               created_at, updated_at
             ) values (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
               null, null, null, null, null, null, null, ?12, ?12
             )",
            params![
                id,
                input.workspace_id,
                input.project_id,
                input.provider.trim(),
                input.provider_runtime.trim(),
                input.provider_profile.trim(),
                input.display_name.trim(),
                input.preset_id.trim(),
                input.image_family.trim(),
                input.access_user.map(str::trim),
                input.status.trim(),
                now
            ],
        )?;
        self.get_workspace_machine(&id)
    }

    pub fn get_workspace_machine(&self, id: &str) -> anyhow::Result<WorkspaceMachine> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(machine) = find_workspace_machine_with_conn(&conn, id)? {
                return Ok(machine);
            }
        }
        anyhow::bail!("workspace machine not found: {id}")
    }

    pub fn find_workspace_machine_by_profile(
        &self,
        workspace_id: i64,
        provider: &str,
        provider_profile: &str,
    ) -> anyhow::Result<Option<WorkspaceMachine>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        conn.query_row(
            "select id, workspace_id, project_id, provider, provider_runtime, provider_profile,
                    display_name, preset_id, image_family, access_user, status,
                    web_port, rdp_port, ssh_port,
                    last_health_status, last_health_summary, last_error_code, last_error_message,
                    created_at, updated_at
             from workspace_machines
             where workspace_id = ?1 and provider = ?2 and provider_profile = ?3
             limit 1",
            params![workspace_id, provider.trim(), provider_profile.trim()],
            workspace_machine_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn update_workspace_machine(
        &self,
        input: WorkspaceMachineUpdate<'_>,
    ) -> anyhow::Result<WorkspaceMachine> {
        let machine = self.get_workspace_machine(input.id)?;
        let conn = self.workspace_connect_by_id(machine.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        let changed = conn.execute(
            "update workspace_machines
             set status = ?1,
                 web_port = ?2,
                 rdp_port = ?3,
                 ssh_port = ?4,
                 last_health_status = ?5,
                 last_health_summary = ?6,
                 last_error_code = ?7,
                 last_error_message = ?8,
                 updated_at = ?9
             where id = ?10",
            params![
                input.status.trim(),
                input.web_port,
                input.rdp_port,
                input.ssh_port,
                input.last_health_status,
                input.last_health_summary,
                input.last_error_code,
                input.last_error_message,
                now,
                input.id
            ],
        )?;
        if changed == 0 {
            anyhow::bail!("workspace machine not found: {}", input.id);
        }
        self.get_workspace_machine(input.id)
    }

    pub fn delete_workspace_machine(&self, id: &str) -> anyhow::Result<()> {
        let machine = self.get_workspace_machine(id)?;
        let conn = self.workspace_connect_by_id(machine.workspace_id)?;
        let changed = conn.execute("delete from workspace_machines where id = ?1", [id])?;
        if changed == 0 {
            anyhow::bail!("workspace machine not found: {id}");
        }
        Ok(())
    }

    pub fn list_deploy_stacks(&self, workspace_id: i64) -> anyhow::Result<Vec<DeployStack>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, name, slug, status, active_version_id, active_machine_id,
                    created_at, updated_at
             from deploy_stacks
             where workspace_id = ?1
             order by updated_at desc, name asc",
        )?;
        let rows = stmt.query_map([workspace_id], deploy_stack_from_row)?;
        collect_rows(rows)
    }

    pub fn reset_workspace_deploy_state(
        &self,
        workspace_id: i64,
    ) -> anyhow::Result<WorkspaceDeployReset> {
        let workspace = self.get_workspace(workspace_id)?;
        let mut conn = self.workspace_connect(&workspace)?;
        let tx = conn.transaction()?;
        let deploy_run_steps = tx.execute(
            "delete from deploy_run_steps
             where run_id in (
               select r.id
               from deploy_runs r
               join deploy_stacks s on s.id = r.stack_id
               where s.workspace_id = ?1
             )",
            [workspace_id],
        )? as i64;
        let deploy_runs = tx.execute(
            "delete from deploy_runs
             where stack_id in (select id from deploy_stacks where workspace_id = ?1)",
            [workspace_id],
        )? as i64;
        let deploy_version_projects = tx.execute(
            "delete from deploy_version_projects
             where version_id in (select id from deploy_versions where workspace_id = ?1)",
            [workspace_id],
        )? as i64;
        let deploy_target_bootstrap = tx.execute(
            "delete from deploy_target_bootstrap where workspace_id = ?1",
            [workspace_id],
        )? as i64;
        let deploy_versions = tx.execute(
            "delete from deploy_versions where workspace_id = ?1",
            [workspace_id],
        )? as i64;
        let deploy_stacks = tx.execute(
            "delete from deploy_stacks where workspace_id = ?1",
            [workspace_id],
        )? as i64;
        let workspace_machines = tx.execute(
            "delete from workspace_machines where workspace_id = ?1",
            [workspace_id],
        )? as i64;
        tx.commit()?;
        conn.execute_batch("vacuum")?;

        let mut removed_artifact_dirs = Vec::new();
        for dirname in ["deploy-packages", "deploy-plans"] {
            let path = Path::new(&workspace.root_path).join(".dw").join(dirname);
            if path.exists() {
                std::fs::remove_dir_all(&path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
                removed_artifact_dirs.push(path.display().to_string());
            }
        }

        Ok(WorkspaceDeployReset {
            workspace_id,
            workspace_machines,
            deploy_stacks,
            deploy_versions,
            deploy_runs,
            deploy_run_steps,
            deploy_version_projects,
            deploy_target_bootstrap,
            removed_artifact_dirs,
        })
    }

    pub fn get_deploy_stack(&self, id: &str) -> anyhow::Result<DeployStack> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(stack) = find_deploy_stack_with_conn(&conn, id)? {
                return Ok(stack);
            }
        }
        anyhow::bail!("deploy stack not found: {id}")
    }

    pub fn find_deploy_stack_by_slug(
        &self,
        workspace_id: i64,
        slug: &str,
    ) -> anyhow::Result<Option<DeployStack>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        conn.query_row(
            "select id, workspace_id, name, slug, status, active_version_id, active_machine_id,
                    created_at, updated_at
             from deploy_stacks
             where workspace_id = ?1 and slug = ?2
             limit 1",
            params![workspace_id, slug.trim()],
            deploy_stack_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn create_deploy_stack(&self, input: DeployStackCreate<'_>) -> anyhow::Result<DeployStack> {
        if let Some(stack) = self.find_deploy_stack_by_slug(input.workspace_id, input.slug)? {
            return Ok(stack);
        }
        let conn = self.workspace_connect_by_id(input.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        let id = new_store_id("deploy-stack");
        conn.execute(
            "insert into deploy_stacks (
               id, workspace_id, name, slug, status, active_version_id, active_machine_id,
               created_at, updated_at
             ) values (?1, ?2, ?3, ?4, 'idle', null, null, ?5, ?5)",
            params![
                id,
                input.workspace_id,
                input.name.trim(),
                input.slug.trim(),
                now
            ],
        )?;
        self.get_deploy_stack(&id)
    }

    pub fn next_deploy_version_label(&self, stack_id: &str) -> anyhow::Result<String> {
        let stack = self.get_deploy_stack(stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let count: i64 = conn.query_row(
            "select count(*) from deploy_versions where stack_id = ?1",
            [stack_id],
            |row| row.get(0),
        )?;
        Ok(format!("deploy-{:03}", count + 1))
    }

    pub fn create_deploy_version(
        &self,
        input: DeployVersionCreate<'_>,
    ) -> anyhow::Result<DeployVersion> {
        let conn = self.workspace_connect_by_id(input.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        let id = new_store_id("deploy-version");
        conn.execute(
            "insert into deploy_versions (
               id, stack_id, workspace_id, label, status, target_machine_id, artifact_path,
               manifest_path, manifest_json, review_status, reviewed_at,
               blocking_findings_json, created_at, updated_at
             ) values (?1, ?2, ?3, ?4, 'review_required', ?5, ?6, ?7, ?8,
                       'pending', null, ?9, ?10, ?10)",
            params![
                id,
                input.stack_id.trim(),
                input.workspace_id,
                input.label.trim(),
                input.target_machine_id,
                input.artifact_path.trim(),
                input.manifest_path.trim(),
                input.manifest_json,
                input.blocking_findings_json,
                now
            ],
        )?;
        self.get_deploy_version(&id)
    }

    pub fn get_deploy_version(&self, id: &str) -> anyhow::Result<DeployVersion> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(version) = find_deploy_version_with_conn(&conn, id)? {
                return Ok(version);
            }
        }
        anyhow::bail!("deploy version not found: {id}")
    }

    pub fn list_deploy_versions(&self, stack_id: &str) -> anyhow::Result<Vec<DeployVersion>> {
        let stack = self.get_deploy_stack(stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, stack_id, workspace_id, label, status, target_machine_id, artifact_path,
                    manifest_path, manifest_json, review_status, reviewed_at,
                    blocking_findings_json, created_at, updated_at
             from deploy_versions
             where stack_id = ?1
             order by created_at desc",
        )?;
        let rows = stmt.query_map([stack_id], deploy_version_from_row)?;
        collect_rows(rows)
    }

    pub fn add_deploy_version_project(
        &self,
        input: DeployVersionProjectCreate<'_>,
    ) -> anyhow::Result<DeployVersionProject> {
        let version = self.get_deploy_version(input.version_id)?;
        let conn = self.workspace_connect_by_id(version.workspace_id)?;
        conn.execute(
            "insert into deploy_version_projects (
               version_id, project_id, name, path, branch, commit_sha, dirty, package_path
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.version_id.trim(),
                input.project_id,
                input.name.trim(),
                input.path.trim(),
                input.branch,
                input.commit_sha,
                if input.dirty { 1 } else { 0 },
                input.package_path.trim()
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(DeployVersionProject {
            id,
            version_id: input.version_id.to_string(),
            project_id: input.project_id,
            name: input.name.trim().to_string(),
            path: input.path.trim().to_string(),
            branch: input.branch.map(ToOwned::to_owned),
            commit_sha: input.commit_sha.map(ToOwned::to_owned),
            dirty: input.dirty,
            package_path: input.package_path.trim().to_string(),
        })
    }

    pub fn list_deploy_version_projects(
        &self,
        version_id: &str,
    ) -> anyhow::Result<Vec<DeployVersionProject>> {
        let version = self.get_deploy_version(version_id)?;
        let conn = self.workspace_connect_by_id(version.workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, version_id, project_id, name, path, branch, commit_sha, dirty, package_path
             from deploy_version_projects
             where version_id = ?1
             order by id asc",
        )?;
        let rows = stmt.query_map([version_id], deploy_version_project_from_row)?;
        collect_rows(rows)
    }

    pub fn approve_deploy_version(&self, version_id: &str) -> anyhow::Result<DeployVersion> {
        let version = self.get_deploy_version(version_id)?;
        let conn = self.workspace_connect_by_id(version.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "update deploy_versions
             set status = 'approved',
                 review_status = 'approved',
                 reviewed_at = ?1,
                 updated_at = ?1
             where id = ?2",
            params![now, version_id],
        )?;
        self.get_deploy_version(version_id)
    }

    pub fn update_deploy_version_status(
        &self,
        version_id: &str,
        status: &str,
    ) -> anyhow::Result<DeployVersion> {
        let version = self.get_deploy_version(version_id)?;
        let conn = self.workspace_connect_by_id(version.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "update deploy_versions set status = ?1, updated_at = ?2 where id = ?3",
            params![status.trim(), now, version_id],
        )?;
        self.get_deploy_version(version_id)
    }

    pub fn update_deploy_version_manifest(
        &self,
        version_id: &str,
        manifest_path: &str,
        manifest_json: &str,
        blocking_findings_json: &str,
    ) -> anyhow::Result<DeployVersion> {
        let version = self.get_deploy_version(version_id)?;
        let conn = self.workspace_connect_by_id(version.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "update deploy_versions
             set manifest_path = ?1,
                 manifest_json = ?2,
                 blocking_findings_json = ?3,
                 updated_at = ?4
             where id = ?5",
            params![
                manifest_path.trim(),
                manifest_json,
                blocking_findings_json,
                now,
                version_id
            ],
        )?;
        self.get_deploy_version(version_id)
    }

    pub fn set_active_deploy_version(
        &self,
        stack_id: &str,
        version_id: &str,
        machine_id: &str,
    ) -> anyhow::Result<DeployStack> {
        let stack = self.get_deploy_stack(stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "update deploy_versions
             set status = case
                 when id = ?1 then 'healthy'
                 when stack_id = ?2 and status = 'healthy' then 'superseded'
                 else status
               end,
               updated_at = ?3
             where stack_id = ?2",
            params![version_id, stack_id, now],
        )?;
        conn.execute(
            "update deploy_stacks
             set status = 'healthy',
                 active_version_id = ?1,
                 active_machine_id = ?2,
                 updated_at = ?3
             where id = ?4",
            params![version_id, machine_id, now, stack_id],
        )?;
        self.get_deploy_stack(stack_id)
    }

    pub fn active_deploy_for_machine(
        &self,
        workspace_id: i64,
        machine_id: &str,
    ) -> anyhow::Result<Option<DeployStack>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        conn.query_row(
            "select id, workspace_id, name, slug, status, active_version_id, active_machine_id,
                    created_at, updated_at
             from deploy_stacks
             where workspace_id = ?1 and active_machine_id = ?2 and active_version_id is not null
             limit 1",
            params![workspace_id, machine_id.trim()],
            deploy_stack_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn create_deploy_run(
        &self,
        stack_id: &str,
        version_id: Option<&str>,
        machine_id: Option<&str>,
        operation: &str,
        agent: Option<&AgentProfile>,
    ) -> anyhow::Result<DeployRun> {
        let stack = self.get_deploy_stack(stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        let id = new_store_id("deploy-run");
        let orchestration_status = if agent.is_some() { "pending" } else { "manual" };
        conn.execute(
            "insert into deploy_runs (
               id, stack_id, version_id, machine_id, operation, status, started_at,
               completed_at, summary, agent_profile_id, agent_name, agent_provider, agent_model,
               orchestration_status, orchestration_report_json
             ) values (?1, ?2, ?3, ?4, ?5, 'running', ?6, null, '', ?7, ?8, ?9, ?10, ?11, '{}')",
            params![
                id,
                stack_id,
                version_id,
                machine_id,
                operation.trim(),
                now,
                agent.map(|profile| profile.id),
                agent.map(|profile| profile.name.as_str()),
                agent.map(|profile| profile.provider.as_str()),
                agent.and_then(|profile| profile.model.as_deref()),
                orchestration_status
            ],
        )?;
        self.get_deploy_run(&id)
    }

    pub fn get_deploy_run(&self, id: &str) -> anyhow::Result<DeployRun> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(run) = find_deploy_run_with_conn(&conn, id)? {
                return Ok(run);
            }
        }
        anyhow::bail!("deploy run not found: {id}")
    }

    pub fn list_deploy_runs(&self, version_id: &str) -> anyhow::Result<Vec<DeployRun>> {
        let version = self.get_deploy_version(version_id)?;
        let conn = self.workspace_connect_by_id(version.workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, stack_id, version_id, machine_id, operation, status,
                    started_at, completed_at, summary, agent_profile_id, agent_name,
                    agent_provider, agent_model, orchestration_status, orchestration_report_json
             from deploy_runs
             where version_id = ?1
             order by started_at desc",
        )?;
        let rows = stmt.query_map([version_id], deploy_run_from_row)?;
        collect_rows(rows)
    }

    pub fn complete_deploy_run(
        &self,
        run_id: &str,
        status: &str,
        summary: &str,
    ) -> anyhow::Result<DeployRun> {
        let run = self.get_deploy_run(run_id)?;
        let stack = self.get_deploy_stack(&run.stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "update deploy_runs
             set status = ?1, summary = ?2, completed_at = ?3
             where id = ?4",
            params![status.trim(), summary.trim(), now, run_id],
        )?;
        self.get_deploy_run(run_id)
    }

    pub fn update_deploy_run_orchestration(
        &self,
        run_id: &str,
        status: &str,
        report_json: &str,
    ) -> anyhow::Result<DeployRun> {
        let run = self.get_deploy_run(run_id)?;
        let stack = self.get_deploy_stack(&run.stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        conn.execute(
            "update deploy_runs
             set orchestration_status = ?1, orchestration_report_json = ?2
             where id = ?3",
            params![status.trim(), report_json.trim(), run_id],
        )?;
        self.get_deploy_run(run_id)
    }

    pub fn add_deploy_run_step(
        &self,
        run_id: &str,
        step_key: &str,
        status: &str,
        message: &str,
        log_path: Option<&str>,
        error_code: Option<&str>,
    ) -> anyhow::Result<DeployRunStep> {
        let run = self.get_deploy_run(run_id)?;
        let stack = self.get_deploy_stack(&run.stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "insert into deploy_run_steps (
               run_id, step_key, status, message, log_path, error_code, started_at, completed_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                run_id.trim(),
                step_key.trim(),
                status.trim(),
                message.trim(),
                log_path,
                error_code,
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(DeployRunStep {
            id,
            run_id: run_id.to_string(),
            step_key: step_key.trim().to_string(),
            status: status.trim().to_string(),
            message: message.trim().to_string(),
            log_path: log_path.map(ToOwned::to_owned),
            error_code: error_code.map(ToOwned::to_owned),
            started_at: now.clone(),
            completed_at: Some(now),
        })
    }

    pub fn list_deploy_run_steps(&self, run_id: &str) -> anyhow::Result<Vec<DeployRunStep>> {
        let run = self.get_deploy_run(run_id)?;
        let stack = self.get_deploy_stack(&run.stack_id)?;
        let conn = self.workspace_connect_by_id(stack.workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, run_id, step_key, status, message, log_path, error_code,
                    started_at, completed_at
             from deploy_run_steps
             where run_id = ?1
             order by id asc",
        )?;
        let rows = stmt.query_map([run_id], deploy_run_step_from_row)?;
        collect_rows(rows)
    }

    pub fn add_project(
        &self,
        workspace_id: i64,
        name: &str,
        path: &str,
        remote_url: Option<&str>,
    ) -> anyhow::Result<Project> {
        let created_at = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let project_path = normalize_project_path(path);
        if let Some(project) =
            self.find_project_by_path_with_conn(&conn, workspace_id, &project_path)?
        {
            return Ok(project);
        }
        if let Some(project) =
            find_project_by_equivalent_path_with_conn(&conn, workspace_id, &project_path)?
        {
            return Ok(project);
        }
        conn.execute(
            "insert into projects (workspace_id, name, path, remote_url, created_at)
             values (?1, ?2, ?3, ?4, ?5)",
            params![
                workspace_id,
                name.trim(),
                project_path,
                remote_url,
                created_at
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Project {
            id,
            workspace_id,
            name: name.trim().to_string(),
            path: project_path,
            remote_url: remote_url.map(ToOwned::to_owned),
            parent_project_id: None,
            is_submodule: false,
            submodule_path: None,
            created_at,
        })
    }

    /// Register a git submodule of `parent_project_id` as its own child project.
    /// Idempotent by path (returns the existing row if already registered).
    pub fn add_submodule_project(
        &self,
        workspace_id: i64,
        parent_project_id: i64,
        name: &str,
        path: &str,
        remote_url: Option<&str>,
        submodule_path: &str,
    ) -> anyhow::Result<Project> {
        let created_at = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let project_path = normalize_project_path(path);
        if let Some(existing) =
            self.find_project_by_path_with_conn(&conn, workspace_id, &project_path)?
        {
            return Ok(existing);
        }
        conn.execute(
            "insert into projects
               (workspace_id, name, path, remote_url, parent_project_id, is_submodule,
                submodule_path, created_at)
             values (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
            params![
                workspace_id,
                name.trim(),
                project_path,
                remote_url,
                parent_project_id,
                submodule_path.trim(),
                created_at
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Project {
            id,
            workspace_id,
            name: name.trim().to_string(),
            path: project_path,
            remote_url: remote_url.map(ToOwned::to_owned),
            parent_project_id: Some(parent_project_id),
            is_submodule: true,
            submodule_path: Some(submodule_path.trim().to_string()),
            created_at,
        })
    }

    pub fn get_project(&self, id: i64) -> anyhow::Result<Project> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(project) = conn
                .query_row(
                    "select id, workspace_id, name, path, remote_url, parent_project_id,
                            is_submodule, submodule_path, created_at
                     from projects
                     where id = ?1",
                    [id],
                    project_from_row,
                )
                .optional()?
            {
                return Ok(project);
            }
        }
        anyhow::bail!("project not found: {id}")
    }

    pub fn list_requirement_cards(
        &self,
        workspace_id: i64,
    ) -> anyhow::Result<Vec<RequirementCard>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        backfill_requirement_public_ids(&conn)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, project_id, public_id, title, slug, body, status, prd_slug,
                    archived_from_status, archived_at, created_at, updated_at, flow_id,
                    priority, checklist_json, agent_prompt
             from requirement_cards
             where workspace_id = ?1
             order by updated_at desc, id desc",
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            let id = row.get(0)?;
            Ok(RequirementCard {
                id,
                project_ids: Vec::new(),
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                public_id: row.get(3)?,
                title: row.get(4)?,
                slug: row.get(5)?,
                body: row.get(6)?,
                status: row.get(7)?,
                prd_slug: row.get(8)?,
                archived_from_status: row.get(9)?,
                archived_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                flow_id: row.get(13)?,
                priority: row.get(14)?,
                checklist_json: row.get(15)?,
                agent_prompt: row.get(16)?,
            })
        })?;
        let mut cards = collect_rows(rows)?;
        drop(stmt);
        for card in &mut cards {
            card.project_ids = self.list_requirement_project_ids_with_conn(&conn, card.id)?;
        }
        Ok(cards)
    }

    #[cfg(test)]
    pub fn count_active_requirement_cards_for_flow(
        &self,
        workspace_id: i64,
        flow_id: &str,
    ) -> anyhow::Result<i64> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let count = conn.query_row(
            "select count(*)
             from requirement_cards
             where workspace_id = ?1
               and flow_id = ?2
               and status <> 'archived'",
            params![workspace_id, flow_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn create_requirement_card(
        &self,
        workspace_id: i64,
        project_id: Option<i64>,
        project_ids: &[i64],
        title: &str,
        slug: &str,
        body: &str,
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let mut conn = self.workspace_connect_by_id(workspace_id)?;
        let tx = conn.transaction()?;
        let mut linked_project_ids = unique_project_ids(project_ids);
        if linked_project_ids.is_empty() {
            if let Some(project_id) = project_id {
                linked_project_ids.push(project_id);
            }
        }
        let primary_project_id = linked_project_ids.first().copied().or(project_id);
        let public_id = if let Some(project_id) = primary_project_id {
            reserve_next_public_id(&tx, workspace_id, project_id)?
        } else {
            String::new()
        };
        tx.execute(
            "insert into requirement_cards (
               workspace_id, project_id, public_id, title, slug, body, status, prd_slug, created_at,
               updated_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, 'draft', null, ?7, ?7)",
            params![
                workspace_id,
                project_id,
                public_id,
                title.trim(),
                slug,
                body.trim(),
                now
            ],
        )?;
        let id = tx.last_insert_rowid();
        if primary_project_id.is_none() {
            tx.execute(
                "update requirement_cards set public_id = ?1 where id = ?2",
                params![format_public_id("CARD", id), id],
            )?;
        }
        for linked_project_id in linked_project_ids {
            tx.execute(
                "insert or ignore into requirement_card_projects (card_id, project_id)
                 values (?1, ?2)",
                params![id, linked_project_id],
            )?;
        }
        let card = self.get_requirement_card_with_conn(&tx, id)?;
        tx.commit()?;
        Ok(card)
    }

    pub fn update_requirement_card_status(
        &self,
        id: i64,
        status: &str,
        prd_slug: Option<&str>,
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(id)?;
        let changed = conn.execute(
            "update requirement_cards
             set status = ?1,
                 prd_slug = coalesce(?2, prd_slug),
                 archived_from_status = null,
                 archived_at = null,
                 updated_at = ?3
             where id = ?4",
            params![status, prd_slug, now, id],
        )?;
        if changed == 0 {
            anyhow::bail!("requirement card not found: {id}");
        }
        self.get_requirement_card_with_conn(&conn, id)
    }

    /// Route a card to a workbench flow (or `None` to send it back to the shared
    /// intake backlog). The caller is responsible for setting the card's status
    /// to the target flow's first phase.
    pub fn set_requirement_card_flow(
        &self,
        id: i64,
        flow_id: Option<&str>,
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(id)?;
        let changed = conn.execute(
            "update requirement_cards set flow_id = ?1, updated_at = ?2 where id = ?3",
            params![flow_id, now, id],
        )?;
        if changed == 0 {
            anyhow::bail!("requirement card not found: {id}");
        }
        self.get_requirement_card_with_conn(&conn, id)
    }

    /// Update the editable fields of a task card. Every argument is optional;
    /// `None` leaves the existing value untouched (SQL `coalesce`).
    pub fn update_requirement_card(
        &self,
        id: i64,
        title: Option<&str>,
        body: Option<&str>,
        priority: Option<&str>,
        checklist_json: Option<&str>,
        agent_prompt: Option<&str>,
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(id)?;
        let title = title.map(str::trim).filter(|value| !value.is_empty());
        let slug = title.map(slugify_store_text);
        let changed = conn.execute(
            "update requirement_cards
             set title = coalesce(?1, title),
                 slug = coalesce(?2, slug),
                 body = coalesce(?3, body),
                 priority = coalesce(?4, priority),
                 checklist_json = coalesce(?5, checklist_json),
                 agent_prompt = coalesce(?6, agent_prompt),
                 updated_at = ?7
             where id = ?8",
            params![
                title,
                slug,
                body.map(str::trim),
                priority,
                checklist_json,
                agent_prompt,
                now,
                id
            ],
        )?;
        if changed == 0 {
            anyhow::bail!("requirement card not found: {id}");
        }
        self.get_requirement_card_with_conn(&conn, id)
    }

    /// Replace the set of projects a task is linked to (must keep at least the
    /// caller-provided ids). The first id becomes the primary `project_id`.
    pub fn set_requirement_card_projects(
        &self,
        id: i64,
        project_ids: &[i64],
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let mut conn = self.workspace_connect_for_card(id)?;
        let ids = unique_project_ids(project_ids);
        let tx = conn.transaction()?;
        tx.execute(
            "delete from requirement_card_projects where card_id = ?1",
            [id],
        )?;
        for project_id in &ids {
            tx.execute(
                "insert or ignore into requirement_card_projects (card_id, project_id)
                 values (?1, ?2)",
                params![id, project_id],
            )?;
        }
        let primary_project_id = ids.first().copied();
        tx.execute(
            "update requirement_cards set project_id = ?1, updated_at = ?2 where id = ?3",
            params![primary_project_id, now, id],
        )?;
        let card = self.get_requirement_card_with_conn(&tx, id)?;
        tx.commit()?;
        Ok(card)
    }

    pub fn archive_requirement_card(&self, id: i64) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(id)?;
        let current_status: String = conn
            .query_row(
                "select status from requirement_cards where id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("requirement card not found: {id}"))?;
        if current_status == "archived" {
            anyhow::bail!("requirement card is already archived");
        }
        let changed = conn.execute(
            "update requirement_cards
             set status = 'archived',
                 archived_from_status = ?1,
                 archived_at = ?2,
                 updated_at = ?2
             where id = ?3",
            params![current_status, now, id],
        )?;
        if changed == 0 {
            anyhow::bail!("requirement card not found: {id}");
        }
        self.get_requirement_card_with_conn(&conn, id)
    }

    pub fn restore_requirement_card(
        &self,
        id: i64,
        status: &str,
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(id)?;
        let current_status: String = conn
            .query_row(
                "select status from requirement_cards where id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("requirement card not found: {id}"))?;
        if current_status != "archived" {
            anyhow::bail!("only archived cards can be restored");
        }
        let changed = conn.execute(
            "update requirement_cards
             set status = ?1,
                 archived_from_status = null,
                 archived_at = null,
                 updated_at = ?2
             where id = ?3",
            params![status, now, id],
        )?;
        if changed == 0 {
            anyhow::bail!("requirement card not found: {id}");
        }
        self.get_requirement_card_with_conn(&conn, id)
    }

    pub fn update_requirement_card_body(
        &self,
        id: i64,
        body: &str,
    ) -> anyhow::Result<RequirementCard> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(id)?;
        let changed = conn.execute(
            "update requirement_cards
             set body = ?1, updated_at = ?2
             where id = ?3",
            params![body.trim(), now, id],
        )?;
        if changed == 0 {
            anyhow::bail!("requirement card not found: {id}");
        }
        self.get_requirement_card_with_conn(&conn, id)
    }

    pub fn list_requirement_stage_forms(
        &self,
        card_id: i64,
    ) -> anyhow::Result<Vec<RequirementStageForm>> {
        let conn = self.workspace_connect_for_card(card_id)?;
        let mut stmt = conn.prepare(
            "select card_id, stage_id, payload_json, updated_at
             from requirement_stage_forms
             where card_id = ?1
             order by stage_id asc",
        )?;
        let rows = stmt.query_map([card_id], |row| {
            Ok(RequirementStageForm {
                card_id: row.get(0)?,
                stage_id: row.get(1)?,
                payload_json: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn upsert_requirement_stage_form(
        &self,
        card_id: i64,
        stage_id: &str,
        payload_json: &str,
    ) -> anyhow::Result<RequirementStageForm> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(card_id)?;
        conn.execute(
            "insert into requirement_stage_forms (card_id, stage_id, payload_json, updated_at)
             values (?1, ?2, ?3, ?4)
             on conflict(card_id, stage_id) do update set
               payload_json = excluded.payload_json,
               updated_at = excluded.updated_at",
            params![card_id, stage_id.trim(), payload_json, now],
        )?;
        Ok(RequirementStageForm {
            card_id,
            stage_id: stage_id.trim().to_string(),
            payload_json: payload_json.to_string(),
            updated_at: now,
        })
    }

    pub fn list_requirement_attachments(
        &self,
        card_id: i64,
    ) -> anyhow::Result<Vec<RequirementAttachment>> {
        let conn = self.workspace_connect_for_card(card_id)?;
        let mut stmt = conn.prepare(
            "select id, card_id, name, file_path, created_at
             from requirement_attachments
             where card_id = ?1
             order by created_at desc, id desc",
        )?;
        let rows = stmt.query_map([card_id], |row| {
            Ok(RequirementAttachment {
                id: row.get(0)?,
                card_id: row.get(1)?,
                name: row.get(2)?,
                file_path: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn add_requirement_attachment(
        &self,
        card_id: i64,
        name: &str,
        file_path: &str,
    ) -> anyhow::Result<RequirementAttachment> {
        let created_at = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_for_card(card_id)?;
        let card = self.get_requirement_card_with_conn(&conn, card_id)?;
        let workspace = self.get_workspace(card.workspace_id)?;
        let source_path = std::fs::canonicalize(file_path.trim())
            .with_context(|| format!("failed to read attachment source {}", file_path.trim()))?;
        if !source_path.is_file() {
            anyhow::bail!("attachment path is not a file");
        }
        let original_name = name.trim();
        let attachment_name = if original_name.is_empty() {
            source_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "attachment".to_string())
        } else {
            original_name.to_string()
        };
        let managed_dir = workspace_attachment_dir(&workspace.root_path, card_id);
        std::fs::create_dir_all(&managed_dir)
            .with_context(|| format!("failed to create {}", managed_dir.display()))?;
        let managed_path = unique_attachment_path(&managed_dir, &attachment_name);
        if source_path != managed_path {
            std::fs::copy(&source_path, &managed_path).with_context(|| {
                format!(
                    "failed to copy attachment {} to {}",
                    source_path.display(),
                    managed_path.display()
                )
            })?;
        }
        let managed_path = managed_path.display().to_string();
        conn.execute(
            "insert into requirement_attachments (card_id, name, file_path, created_at)
             values (?1, ?2, ?3, ?4)",
            params![card_id, attachment_name, managed_path, created_at],
        )?;
        let id = conn.last_insert_rowid();
        Ok(RequirementAttachment {
            id,
            card_id,
            name: attachment_name,
            file_path: managed_path,
            created_at,
        })
    }

    pub fn remove_requirement_attachment(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.workspace_connect_for_attachment(id)?;
        let attachment = self.get_requirement_attachment_with_conn(&conn, id)?;
        let card = self.get_requirement_card_with_conn(&conn, attachment.card_id)?;
        let workspace = self.get_workspace(card.workspace_id)?;
        let changed = conn.execute("delete from requirement_attachments where id = ?1", [id])?;
        if changed == 0 {
            anyhow::bail!("requirement attachment not found: {id}");
        }
        let attachment_path = PathBuf::from(&attachment.file_path);
        if is_managed_attachment_path(&workspace.root_path, &attachment_path) {
            let _ = std::fs::remove_file(attachment_path);
        }
        Ok(())
    }

    pub fn get_requirement_attachment(&self, id: i64) -> anyhow::Result<RequirementAttachment> {
        let conn = self.workspace_connect_for_attachment(id)?;
        self.get_requirement_attachment_with_conn(&conn, id)
    }

    pub fn list_knowledge_sources(
        &self,
        workspace_id: i64,
        project_id: Option<i64>,
    ) -> anyhow::Result<Vec<KnowledgeSource>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, project_id, blueprint_id, scope, name, file_path,
                    original_path, created_at
             from knowledge_sources
             where workspace_id = ?1
               and (?2 is null or project_id is null or project_id = ?2)
             order by created_at desc, id desc",
        )?;
        let rows = stmt.query_map(params![workspace_id, project_id], knowledge_source_from_row)?;
        collect_rows(rows)
    }

    pub fn add_knowledge_source(
        &self,
        workspace_id: i64,
        project_id: Option<i64>,
        blueprint_id: Option<&str>,
        name: &str,
        file_path: &str,
    ) -> anyhow::Result<KnowledgeSource> {
        let workspace = self.get_workspace(workspace_id)?;
        let conn = self.workspace_connect(&workspace)?;
        if let Some(project_id) = project_id {
            let belongs_to_workspace = conn
                .query_row(
                    "select 1 from projects where id = ?1 and workspace_id = ?2 limit 1",
                    params![project_id, workspace_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if !belongs_to_workspace {
                anyhow::bail!("project does not belong to workspace");
            }
        }
        if let Some(blueprint_id) = blueprint_id {
            let belongs_to_workspace = conn
                .query_row(
                    "select 1 from project_blueprints where id = ?1 and workspace_id = ?2 limit 1",
                    params![blueprint_id.trim(), workspace_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if !belongs_to_workspace {
                anyhow::bail!("project blueprint does not belong to workspace");
            }
        }

        let source_path = std::fs::canonicalize(file_path.trim())
            .with_context(|| format!("failed to read knowledge source {}", file_path.trim()))?;
        if !source_path.is_file() {
            anyhow::bail!("knowledge source path is not a file");
        }
        let source_name = if name.trim().is_empty() {
            source_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "knowledge-source".to_string())
        } else {
            name.trim().to_string()
        };
        let managed_dir = workspace_knowledge_dir(&workspace.root_path, project_id, blueprint_id);
        std::fs::create_dir_all(&managed_dir)
            .with_context(|| format!("failed to create {}", managed_dir.display()))?;
        let managed_path = unique_attachment_path(&managed_dir, &source_name);
        if source_path != managed_path {
            std::fs::copy(&source_path, &managed_path).with_context(|| {
                format!(
                    "failed to copy knowledge source {} to {}",
                    source_path.display(),
                    managed_path.display()
                )
            })?;
        }

        let created_at = Utc::now().to_rfc3339();
        let scope = if project_id.is_some() {
            "project"
        } else if blueprint_id.is_some() {
            "blueprint"
        } else {
            "workspace"
        };
        conn.execute(
            "insert into knowledge_sources (
               workspace_id, project_id, blueprint_id, scope, name, file_path, original_path,
               created_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                workspace_id,
                project_id,
                blueprint_id
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                scope,
                source_name,
                managed_path.display().to_string(),
                source_path.display().to_string(),
                created_at
            ],
        )?;
        let id = conn.last_insert_rowid();
        self.get_knowledge_source_with_conn(&conn, id)
    }

    pub fn remove_knowledge_source(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.workspace_connect_for_knowledge_source(id)?;
        let source = self.get_knowledge_source_with_conn(&conn, id)?;
        let workspace = self.get_workspace(source.workspace_id)?;
        let changed = conn.execute("delete from knowledge_sources where id = ?1", [id])?;
        if changed == 0 {
            anyhow::bail!("knowledge source not found: {id}");
        }
        let source_path = PathBuf::from(&source.file_path);
        if is_managed_knowledge_path(&workspace.root_path, &source_path) {
            let _ = std::fs::remove_file(source_path);
        }
        Ok(())
    }

    pub fn create_project_blueprint(
        &self,
        workspace_id: i64,
        title: &str,
        idea: &str,
        agent_profile_id: Option<i64>,
        knowledge_source_ids_json: &str,
    ) -> anyhow::Result<ProjectBlueprint> {
        let title = title.trim();
        if title.is_empty() {
            anyhow::bail!("project blueprint title is required");
        }
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let id = new_store_id("blueprint");
        let slug = unique_project_blueprint_slug(&conn, workspace_id, title)?;
        conn.execute(
            "insert into project_blueprints (
               id, workspace_id, title, slug, status, idea, agent_profile_id, agent_session_id,
               knowledge_source_ids_json, answers_json, running_summary,
               detected_subprojects_json, prd, techspec, tasks_json, definition_of_done,
               project_id, created_at, updated_at
             ) values (
               ?1, ?2, ?3, ?4, 'draft', ?5, ?6, null, ?7, '[]', '', '[]', '', '', '[]', '',
               null, ?8, ?8
             )",
            params![
                id,
                workspace_id,
                title,
                slug,
                idea.trim(),
                agent_profile_id,
                valid_json_or_default(knowledge_source_ids_json, "[]"),
                now
            ],
        )?;
        self.get_project_blueprint(&id)
    }

    pub fn list_project_blueprints(
        &self,
        workspace_id: i64,
    ) -> anyhow::Result<Vec<ProjectBlueprint>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, title, slug, status, idea, agent_profile_id, agent_session_id,
                    knowledge_source_ids_json, answers_json, running_summary,
                    detected_subprojects_json, prd, techspec, tasks_json, definition_of_done,
                    project_id, created_at, updated_at
             from project_blueprints
             where workspace_id = ?1
             order by updated_at desc, created_at desc",
        )?;
        let rows = stmt.query_map([workspace_id], project_blueprint_from_row)?;
        collect_rows(rows)
    }

    pub fn get_project_blueprint(&self, id: &str) -> anyhow::Result<ProjectBlueprint> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(blueprint) = self.find_project_blueprint_with_conn(&conn, id)? {
                return Ok(blueprint);
            }
        }
        anyhow::bail!("project blueprint not found: {id}")
    }

    pub fn update_project_blueprint(
        &self,
        input: ProjectBlueprintUpdate<'_>,
    ) -> anyhow::Result<ProjectBlueprint> {
        let current = self.get_project_blueprint(input.id)?;
        let conn = self.workspace_connect_by_id(current.workspace_id)?;
        let now = Utc::now().to_rfc3339();
        let status = input
            .status
            .map(normalize_project_blueprint_status)
            .transpose()?
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| current.status.clone());
        conn.execute(
            "update project_blueprints
             set status = ?1,
                 agent_session_id = coalesce(?2, agent_session_id),
                 knowledge_source_ids_json = coalesce(?3, knowledge_source_ids_json),
                 answers_json = coalesce(?4, answers_json),
                 running_summary = coalesce(?5, running_summary),
                 detected_subprojects_json = coalesce(?6, detected_subprojects_json),
                 prd = coalesce(?7, prd),
                 techspec = coalesce(?8, techspec),
                 tasks_json = coalesce(?9, tasks_json),
                 definition_of_done = coalesce(?10, definition_of_done),
                 updated_at = ?11
             where id = ?12",
            params![
                status,
                input.agent_session_id,
                input
                    .knowledge_source_ids_json
                    .map(|value| valid_json_or_default(value, "[]")),
                input
                    .answers_json
                    .map(|value| valid_json_or_default(value, "[]")),
                input.running_summary.map(str::trim),
                input
                    .detected_subprojects_json
                    .map(|value| valid_json_or_default(value, "[]")),
                input.prd.map(str::trim),
                input.techspec.map(str::trim),
                input
                    .tasks_json
                    .map(|value| valid_json_or_default(value, "[]")),
                input.definition_of_done.map(str::trim),
                now,
                input.id.trim()
            ],
        )?;
        self.get_project_blueprint(input.id)
    }

    pub fn materialize_project_blueprint(
        &self,
        id: &str,
    ) -> anyhow::Result<ProjectBlueprintMaterialization> {
        let blueprint = self.get_project_blueprint(id)?;
        let workspace = self.get_workspace(blueprint.workspace_id)?;
        if let Some(project_id) = blueprint.project_id {
            let project = self.get_project(project_id)?;
            let cards = self
                .list_requirement_cards(blueprint.workspace_id)?
                .into_iter()
                .filter(|card| card.project_ids.contains(&project.id))
                .collect();
            let spec_dir = project_blueprint_spec_dir(&workspace.root_path, &blueprint.slug)
                .display()
                .to_string();
            return Ok(ProjectBlueprintMaterialization {
                blueprint,
                project,
                cards,
                spec_dir,
            });
        }
        if blueprint.status != "planned" {
            anyhow::bail!("project blueprint must be planned before materialization");
        }

        let project_dir = Path::new(&workspace.root_path)
            .join("projects")
            .join(&blueprint.slug);
        std::fs::create_dir_all(&project_dir)
            .with_context(|| format!("failed to create {}", project_dir.display()))?;
        let project = self.add_project(
            blueprint.workspace_id,
            &blueprint.title,
            &project_dir.display().to_string(),
            None,
        )?;
        let spec_dir = project_blueprint_spec_dir(&workspace.root_path, &blueprint.slug);
        std::fs::create_dir_all(&spec_dir)
            .with_context(|| format!("failed to create {}", spec_dir.display()))?;

        let knowledge_manifest = self.project_blueprint_knowledge_manifest(&blueprint)?;
        write_project_blueprint_docs(&spec_dir, &blueprint, &knowledge_manifest)?;

        let task_entries = project_blueprint_task_entries(&blueprint.tasks_json);
        let mut cards = Vec::new();
        for (index, task) in task_entries.iter().enumerate() {
            let title = if task.title.trim().is_empty() {
                format!("{} task {}", blueprint.title, index + 1)
            } else {
                task.title.clone()
            };
            let slug = slugify_store_text(&title);
            let body = project_blueprint_card_body(&blueprint, &spec_dir, task, index + 1);
            let card = self.create_requirement_card(
                blueprint.workspace_id,
                Some(project.id),
                &[project.id],
                &title,
                &slug,
                &body,
            )?;
            cards.push(card);
        }

        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_by_id(blueprint.workspace_id)?;
        conn.execute(
            "update project_blueprints
             set status = 'materialized', project_id = ?1, updated_at = ?2
             where id = ?3",
            params![project.id, now, blueprint.id],
        )?;
        Ok(ProjectBlueprintMaterialization {
            blueprint: self.get_project_blueprint(&blueprint.id)?,
            project,
            cards,
            spec_dir: spec_dir.display().to_string(),
        })
    }

    pub fn reconcile_workspace_projects(&self, workspace_id: i64) -> anyhow::Result<()> {
        let workspace = self.get_workspace(workspace_id)?;
        let conn = self.workspace_connect(&workspace)?;
        for project_path in discover_workspace_git_projects(&workspace.root_path)? {
            let normalized_path = normalize_project_path(&project_path.display().to_string());
            if self
                .find_project_by_path_with_conn(&conn, workspace_id, &normalized_path)?
                .is_some()
            {
                continue;
            }
            let name = project_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "project".to_string());
            let remote_url = git_remote_origin(&project_path);
            self.add_project(workspace_id, &name, &normalized_path, remote_url.as_deref())?;
        }
        // Register git submodules of each top-level project as child projects.
        let parents: Vec<Project> = {
            let mut stmt = conn.prepare(
                "select id, workspace_id, name, path, remote_url, parent_project_id,
                        is_submodule, submodule_path, created_at
                 from projects
                 where workspace_id = ?1 and is_submodule = 0",
            )?;
            let rows = stmt.query_map([workspace_id], project_from_row)?;
            collect_rows(rows)?
        };
        for parent in parents {
            self.reconcile_submodules_recursive(workspace_id, &conn, &parent, 0)?;
        }
        Ok(())
    }

    /// Register the (initialized) git submodules of `parent` as child projects,
    /// recursing into nested submodules; deregister children no longer present in
    /// the parent's `.gitmodules`. Depth-bounded to avoid pathological chains.
    fn reconcile_submodules_recursive(
        &self,
        workspace_id: i64,
        conn: &Connection,
        parent: &Project,
        depth: u32,
    ) -> anyhow::Result<()> {
        if depth > 5 {
            return Ok(());
        }
        let gitmodules = Path::new(&parent.path).join(".gitmodules");
        let entries = if gitmodules.is_file() {
            parse_gitmodules(&gitmodules)
        } else {
            Vec::new()
        };
        let mut present: Vec<String> = Vec::new();
        for entry in &entries {
            let sub_abs = normalize_project_path(
                &Path::new(&parent.path)
                    .join(&entry.path)
                    .display()
                    .to_string(),
            );
            // Only register initialized submodules (a `.git` file/dir is present).
            if !Path::new(&sub_abs).join(".git").exists() {
                continue;
            }
            present.push(entry.path.clone());
            let name = Path::new(&entry.path)
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| entry.path.clone());
            let child = self.add_submodule_project(
                workspace_id,
                parent.id,
                &name,
                &sub_abs,
                entry.url.as_deref(),
                &entry.path,
            )?;
            self.reconcile_submodules_recursive(workspace_id, conn, &child, depth + 1)?;
        }
        // Deregister child submodule-projects no longer present in `.gitmodules`.
        let stale: Vec<i64> = {
            let mut stmt = conn.prepare(
                "select id, submodule_path from projects
                 where workspace_id = ?1 and parent_project_id = ?2 and is_submodule = 1",
            )?;
            let rows = stmt.query_map(params![workspace_id, parent.id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
            })?;
            let mut ids = Vec::new();
            for row in rows {
                let (id, sub_path) = row?;
                let keep = sub_path
                    .as_deref()
                    .map(|value| present.iter().any(|item| item == value))
                    .unwrap_or(false);
                if !keep {
                    ids.push(id);
                }
            }
            ids
        };
        for id in stale {
            conn.execute("delete from projects where id = ?1", [id])?;
        }
        Ok(())
    }

    pub fn list_evidence(&self, project_path: &str) -> anyhow::Result<Vec<EvidenceEntry>> {
        let mut entries = Vec::new();
        let conn = self.connect()?;
        entries.extend(self.list_evidence_with_conn(&conn, project_path)?);

        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            entries.extend(self.list_evidence_with_conn(&conn, project_path)?);
        }

        entries.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(entries)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_evidence_run(
        &self,
        workspace_id: Option<i64>,
        project_id: Option<i64>,
        project_path: &str,
        prd_slug: Option<&str>,
        command: &str,
        status: &str,
        summary: &str,
        terminal_session_id: Option<&str>,
        terminal_log_path: Option<&str>,
    ) -> anyhow::Result<EvidenceEntry> {
        let created_at = Utc::now().to_rfc3339();
        let conn = if let Some(workspace_id) = workspace_id {
            self.workspace_connect_by_id(workspace_id)?
        } else {
            self.connect()?
        };
        conn.execute(
            "insert into evidence_runs (
               workspace_id, project_id, project_path, prd_slug, command, status, summary,
               terminal_session_id, terminal_log_path, created_at, completed_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, null)",
            params![
                workspace_id,
                project_id,
                project_path,
                prd_slug,
                command.trim(),
                status,
                summary.trim(),
                terminal_session_id,
                terminal_log_path,
                created_at
            ],
        )?;
        let id = conn.last_insert_rowid();
        self.get_evidence_run_with_conn(&conn, id)
    }

    pub fn complete_evidence_run(
        &self,
        id: i64,
        status: &str,
        summary: &str,
        workspace_id: Option<i64>,
    ) -> anyhow::Result<EvidenceEntry> {
        let completed_at = Utc::now().to_rfc3339();
        if let Some(workspace_id) = workspace_id {
            let conn = self.workspace_connect_by_id(workspace_id)?;
            return self.complete_evidence_run_with_conn(&conn, id, status, summary, &completed_at);
        }

        let conn = self.connect()?;
        if self.find_evidence_run_with_conn(&conn, id)?.is_some() {
            return self.complete_evidence_run_with_conn(&conn, id, status, summary, &completed_at);
        }

        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self.find_evidence_run_with_conn(&conn, id)?.is_some() {
                return self.complete_evidence_run_with_conn(
                    &conn,
                    id,
                    status,
                    summary,
                    &completed_at,
                );
            }
        }

        anyhow::bail!("evidence run not found: {id}")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_evidence_item(
        &self,
        project_path: &str,
        kind: &str,
        title: &str,
        relative_path: Option<&str>,
        absolute_path: Option<&str>,
        status: &str,
        summary: &str,
    ) -> anyhow::Result<EvidenceEntry> {
        let created_at = Utc::now().to_rfc3339();
        let conn = self.connect()?;
        conn.execute(
            "insert into evidence_items (
               run_id, project_path, kind, title, relative_path, absolute_path, status, summary,
               created_at
             ) values (null, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project_path,
                kind,
                title.trim(),
                relative_path,
                absolute_path,
                status,
                summary.trim(),
                created_at
            ],
        )?;
        let id = conn.last_insert_rowid();
        self.get_evidence_item(id)
    }

    pub fn upsert_indexed_evidence_item(
        &self,
        project_path: &str,
        kind: &str,
        title: &str,
        relative_path: &str,
        absolute_path: &str,
        summary: &str,
    ) -> anyhow::Result<()> {
        let created_at = Utc::now().to_rfc3339();
        let conn = self.connect()?;
        let existing_id = conn
            .query_row(
                "select id from evidence_items
                 where project_path = ?1 and relative_path = ?2 and status = 'indexed'
                 limit 1",
                params![project_path, relative_path],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            conn.execute(
                "update evidence_items
                 set kind = ?1, title = ?2, absolute_path = ?3, summary = ?4
                 where id = ?5",
                params![kind, title.trim(), absolute_path, summary.trim(), id],
            )?;
        } else {
            conn.execute(
                "insert into evidence_items (
                   run_id, project_path, kind, title, relative_path, absolute_path, status, summary,
                   created_at
                 ) values (null, ?1, ?2, ?3, ?4, ?5, 'indexed', ?6, ?7)",
                params![
                    project_path,
                    kind,
                    title.trim(),
                    relative_path,
                    absolute_path,
                    summary.trim(),
                    created_at
                ],
            )?;
        }
        Ok(())
    }

    pub fn list_agent_profiles(
        &self,
        workspace_id: i64,
        project_id: Option<i64>,
    ) -> anyhow::Result<Vec<AgentProfile>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, workspace_id, project_id, name, provider, model, reasoning_effort,
                    sandbox, context_mode, rtk_enabled, created_at, updated_at
             from agent_profiles
             where workspace_id = ?1
               and (?2 is null or project_id is null or project_id = ?2)
             order by updated_at desc, id desc",
        )?;
        let rows = stmt.query_map(params![workspace_id, project_id], agent_profile_from_row)?;
        collect_rows(rows)
    }

    pub fn create_agent_profile(
        &self,
        input: AgentProfileCreate<'_>,
    ) -> anyhow::Result<AgentProfile> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_by_id(input.workspace_id)?;
        conn.execute(
            "insert into agent_profiles (
               workspace_id, project_id, name, provider, model, reasoning_effort, sandbox, context_mode,
               rtk_enabled, created_at, updated_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                input.workspace_id,
                input.project_id,
                input.name.trim(),
                input.provider.trim(),
                input.model.map(str::trim).filter(|value| !value.is_empty()),
                input
                    .reasoning_effort
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                input.sandbox.trim(),
                input.context_mode.trim(),
                input.rtk_enabled,
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        self.get_agent_profile_with_conn(&conn, id)
    }

    pub fn update_agent_profile(
        &self,
        input: AgentProfileUpdate<'_>,
    ) -> anyhow::Result<AgentProfile> {
        let now = Utc::now().to_rfc3339();
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self
                .find_agent_profile_with_conn(&conn, input.id)?
                .is_none()
            {
                continue;
            }
            conn.execute(
                "update agent_profiles
                 set name = ?1,
                     provider = ?2,
                     model = ?3,
                     reasoning_effort = ?4,
                     sandbox = ?5,
                     context_mode = ?6,
                     rtk_enabled = ?7,
                     updated_at = ?8
                 where id = ?9",
                params![
                    input.name.trim(),
                    input.provider.trim(),
                    input.model.map(str::trim).filter(|value| !value.is_empty()),
                    input
                        .reasoning_effort
                        .map(str::trim)
                        .filter(|value| !value.is_empty()),
                    input.sandbox.trim(),
                    input.context_mode.trim(),
                    input.rtk_enabled,
                    now,
                    input.id
                ],
            )?;
            conn.execute(
                "update agent_sessions
                 set provider = ?1,
                     model = ?2,
                     reasoning_effort = ?3,
                     sandbox = ?4,
                     context_mode = ?5,
                     updated_at = ?6
                 where profile_id = ?7 and status != 'running'",
                params![
                    input.provider.trim(),
                    input.model.map(str::trim).filter(|value| !value.is_empty()),
                    input
                        .reasoning_effort
                        .map(str::trim)
                        .filter(|value| !value.is_empty()),
                    input.sandbox.trim(),
                    input.context_mode.trim(),
                    now,
                    input.id
                ],
            )?;
            return self.get_agent_profile_with_conn(&conn, input.id);
        }
        anyhow::bail!("agent profile not found: {}", input.id)
    }

    pub fn list_agent_sessions(
        &self,
        workspace_id: i64,
        project_id: Option<i64>,
    ) -> anyhow::Result<Vec<AgentSession>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id, profile_id, workspace_id, project_id, requirement_card_id, scope,
                    project_path, provider, model,
                    reasoning_effort, sandbox, context_mode, provider_session_id, codex_session_id, status,
                    title, created_at, updated_at
             from agent_sessions
             where workspace_id = ?1
               and (?2 is null or project_id is null or project_id = ?2)
               and scope = 'chat'
             order by updated_at desc, id desc",
        )?;
        let rows = stmt.query_map(params![workspace_id, project_id], agent_session_from_row)?;
        collect_rows(rows)
    }

    pub fn create_agent_session(
        &self,
        profile: &AgentProfile,
        project_id: Option<i64>,
        project_path: &str,
        title: &str,
    ) -> anyhow::Result<AgentSession> {
        self.create_agent_session_scoped(profile, project_id, project_path, title, "chat", None)
    }

    pub fn create_agent_session_scoped(
        &self,
        profile: &AgentProfile,
        project_id: Option<i64>,
        project_path: &str,
        title: &str,
        scope: &str,
        requirement_card_id: Option<i64>,
    ) -> anyhow::Result<AgentSession> {
        let now = Utc::now().to_rfc3339();
        let conn = self.workspace_connect_by_id(profile.workspace_id)?;
        conn.execute(
            "insert into agent_sessions (
               profile_id, workspace_id, project_id, requirement_card_id, scope, project_path, provider, model,
               reasoning_effort, sandbox, context_mode, provider_session_id, codex_session_id, status, title,
               created_at, updated_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, null, null, 'idle', ?12, ?13, ?13)",
            params![
                profile.id,
                profile.workspace_id,
                project_id,
                requirement_card_id,
                normalize_agent_session_scope(scope),
                project_path.trim(),
                profile.provider,
                profile.model,
                profile.reasoning_effort,
                profile.sandbox,
                profile.context_mode,
                title.trim(),
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        self.get_agent_session_with_conn(&conn, id)
    }

    pub fn find_card_interview_session(
        &self,
        profile_id: i64,
        workspace_id: i64,
        project_id: Option<i64>,
        requirement_card_id: i64,
        project_path: &str,
    ) -> anyhow::Result<Option<AgentSession>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        conn.query_row(
            "select id, profile_id, workspace_id, project_id, requirement_card_id, scope,
                    project_path, provider, model,
                    reasoning_effort, sandbox, context_mode, provider_session_id, codex_session_id, status,
                    title, created_at, updated_at
             from agent_sessions
             where profile_id = ?1
               and workspace_id = ?2
               and (project_id is ?3)
               and requirement_card_id = ?4
               and project_path = ?5
               and scope = 'card_interview'
             order by updated_at desc, id desc
             limit 1",
            params![
                profile_id,
                workspace_id,
                project_id,
                requirement_card_id,
                project_path.trim()
            ],
            agent_session_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn get_agent_profile(&self, id: i64) -> anyhow::Result<AgentProfile> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(profile) = self.find_agent_profile_with_conn(&conn, id)? {
                return Ok(profile);
            }
        }
        anyhow::bail!("agent profile not found: {id}")
    }

    pub fn get_agent_session(&self, id: i64) -> anyhow::Result<AgentSession> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if let Some(session) = self.find_agent_session_with_conn(&conn, id)? {
                return Ok(session);
            }
        }
        anyhow::bail!("agent session not found: {id}")
    }

    pub fn reset_agent_chat(
        &self,
        profile_id: i64,
        workspace_id: i64,
        project_id: Option<i64>,
        project_path: &str,
    ) -> anyhow::Result<AgentSession> {
        let profile = self.get_agent_profile(profile_id)?;
        if profile.workspace_id != workspace_id {
            anyhow::bail!("agent profile does not belong to the active workspace");
        }
        let conn = self.workspace_connect_by_id(workspace_id)?;
        conn.execute(
            "delete from agent_messages
             where session_id in (
               select id from agent_sessions
               where profile_id = ?1
                 and workspace_id = ?2
                 and (project_id is ?3)
                 and scope = 'chat'
             )",
            params![profile_id, workspace_id, project_id],
        )?;
        conn.execute(
            "delete from agent_sessions
             where profile_id = ?1
               and workspace_id = ?2
               and (project_id is ?3)
               and scope = 'chat'",
            params![profile_id, workspace_id, project_id],
        )?;
        self.create_agent_session(&profile, project_id, project_path, "Nova conversa")
    }

    pub fn running_agent_session_ids_for_profile(
        &self,
        profile_id: i64,
        workspace_id: i64,
        project_id: Option<i64>,
    ) -> anyhow::Result<Vec<i64>> {
        let conn = self.workspace_connect_by_id(workspace_id)?;
        let mut stmt = conn.prepare(
            "select id
             from agent_sessions
             where profile_id = ?1
               and workspace_id = ?2
               and (project_id is ?3)
               and scope = 'chat'
               and status = 'running'",
        )?;
        let rows = stmt.query_map(params![profile_id, workspace_id, project_id], |row| {
            row.get(0)
        })?;
        collect_rows(rows)
    }

    pub fn update_agent_session_status(
        &self,
        id: i64,
        status: &str,
        provider_session_id: Option<&str>,
    ) -> anyhow::Result<AgentSession> {
        let now = Utc::now().to_rfc3339();
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self.find_agent_session_with_conn(&conn, id)?.is_none() {
                continue;
            }
            conn.execute(
                "update agent_sessions
                 set status = ?1,
                     provider_session_id = coalesce(?2, provider_session_id),
                     codex_session_id = case when provider = 'codex' then coalesce(?2, codex_session_id) else codex_session_id end,
                     updated_at = ?3
                 where id = ?4",
                params![status.trim(), provider_session_id, now, id],
            )?;
            return self.get_agent_session_with_conn(&conn, id);
        }
        anyhow::bail!("agent session not found: {id}")
    }

    pub fn add_agent_message(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
        raw_json: Option<&str>,
    ) -> anyhow::Result<AgentMessage> {
        let now = Utc::now().to_rfc3339();
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self
                .find_agent_session_with_conn(&conn, session_id)?
                .is_none()
            {
                continue;
            }
            conn.execute(
                "insert into agent_messages (session_id, role, content, raw_json, created_at)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![session_id, role.trim(), content, raw_json, now],
            )?;
            let id = conn.last_insert_rowid();
            return self.get_agent_message_with_conn(&conn, id);
        }
        anyhow::bail!("agent session not found: {session_id}")
    }

    pub fn list_agent_messages(&self, session_id: i64) -> anyhow::Result<Vec<AgentMessage>> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self
                .find_agent_session_with_conn(&conn, session_id)?
                .is_none()
            {
                continue;
            }
            let mut stmt = conn.prepare(
                "select id, session_id, role, content, raw_json, created_at
                 from agent_messages
                 where session_id = ?1
                 order by id asc",
            )?;
            let rows = stmt.query_map([session_id], agent_message_from_row)?;
            return collect_rows(rows);
        }
        Ok(Vec::new())
    }

    pub fn add_agent_run_event(
        &self,
        session_id: i64,
        run_id: &str,
        provider: &str,
        phase: &str,
        elapsed_ms: i64,
        details_json: &str,
    ) -> anyhow::Result<AgentRunEvent> {
        let created_at = Utc::now().to_rfc3339();
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self
                .find_agent_session_with_conn(&conn, session_id)?
                .is_none()
            {
                continue;
            }
            conn.execute(
                "insert into agent_run_events
                   (session_id, run_id, provider, phase, elapsed_ms, details_json, created_at)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    run_id.trim(),
                    provider.trim(),
                    phase.trim(),
                    elapsed_ms,
                    details_json,
                    created_at
                ],
            )?;
            let id = conn.last_insert_rowid();
            return Ok(AgentRunEvent {
                id,
                session_id,
                run_id: run_id.trim().to_string(),
                provider: provider.trim().to_string(),
                phase: phase.trim().to_string(),
                elapsed_ms,
                details_json: details_json.to_string(),
                created_at,
            });
        }
        anyhow::bail!("agent session not found: {session_id}")
    }

    pub fn list_agent_run_events(&self, session_id: i64) -> anyhow::Result<Vec<AgentRunEvent>> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            if self
                .find_agent_session_with_conn(&conn, session_id)?
                .is_none()
            {
                continue;
            }
            let mut stmt = conn.prepare(
                "select id, session_id, run_id, provider, phase, elapsed_ms, details_json, created_at
                 from agent_run_events
                 where session_id = ?1
                 order by id asc",
            )?;
            let rows = stmt.query_map([session_id], agent_run_event_from_row)?;
            return collect_rows(rows);
        }
        Ok(Vec::new())
    }

    fn migrate(&self) -> anyhow::Result<()> {
        let conn = self.connect()?;
        migrate_connection(&conn)
    }

    fn workspace_connect_by_id(&self, workspace_id: i64) -> anyhow::Result<Connection> {
        let workspace = self.get_workspace(workspace_id)?;
        self.workspace_connect(&workspace)
    }

    fn workspace_connect(&self, workspace: &Workspace) -> anyhow::Result<Connection> {
        let db_path = workspace_db_path(&workspace.root_path);
        let parent = db_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid workspace database path"))?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open {}", db_path.display()))?;
        migrate_connection(&conn)?;
        reconcile_workspace_identity(&conn, workspace)?;
        conn.execute(
            "insert or ignore into workspaces (id, name, root_path, created_at)
             values (?1, ?2, ?3, ?4)",
            params![
                workspace.id,
                workspace.name,
                workspace.root_path,
                workspace.created_at
            ],
        )?;
        Ok(conn)
    }

    fn workspace_connect_for_card(&self, card_id: i64) -> anyhow::Result<Connection> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            let exists = conn
                .query_row(
                    "select 1 from requirement_cards where id = ?1 limit 1",
                    [card_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if exists.is_some() {
                return Ok(conn);
            }
        }
        anyhow::bail!("requirement card not found: {card_id}")
    }

    #[cfg(test)]
    fn workspace_connect_for_project(&self, project_id: i64) -> anyhow::Result<Connection> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            let exists = conn
                .query_row(
                    "select 1 from projects where id = ?1 limit 1",
                    [project_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if exists.is_some() {
                return Ok(conn);
            }
        }
        anyhow::bail!("project not found: {project_id}")
    }

    fn workspace_connect_for_attachment(&self, attachment_id: i64) -> anyhow::Result<Connection> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            let exists = conn
                .query_row(
                    "select 1 from requirement_attachments where id = ?1 limit 1",
                    [attachment_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if exists.is_some() {
                return Ok(conn);
            }
        }
        anyhow::bail!("requirement attachment not found: {attachment_id}")
    }

    fn workspace_connect_for_knowledge_source(&self, source_id: i64) -> anyhow::Result<Connection> {
        for workspace in self.list_workspaces()? {
            let conn = self.workspace_connect(&workspace)?;
            let exists = conn
                .query_row(
                    "select 1 from knowledge_sources where id = ?1 limit 1",
                    [source_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if exists.is_some() {
                return Ok(conn);
            }
        }
        anyhow::bail!("knowledge source not found: {source_id}")
    }

    fn find_project_by_path_with_conn(
        &self,
        conn: &Connection,
        workspace_id: i64,
        path: &str,
    ) -> anyhow::Result<Option<Project>> {
        conn.query_row(
            "select id, workspace_id, name, path, remote_url, parent_project_id,
                    is_submodule, submodule_path, created_at
             from projects
             where workspace_id = ?1 and path = ?2
             limit 1",
            params![workspace_id, path],
            project_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_evidence_with_conn(
        &self,
        conn: &Connection,
        project_path: &str,
    ) -> anyhow::Result<Vec<EvidenceEntry>> {
        let mut entries = Vec::new();

        let mut runs = conn.prepare(
            "select id, workspace_id, project_id, project_path, prd_slug, command, status,
                    summary, terminal_session_id, terminal_log_path, created_at, completed_at
             from evidence_runs
             where project_path = ?1
             order by created_at desc",
        )?;
        let run_rows = runs.query_map([project_path], evidence_run_from_row)?;
        entries.extend(collect_rows(run_rows)?);

        let mut items = conn.prepare(
            "select id, run_id, project_path, kind, title, relative_path, absolute_path,
                    status, summary, created_at
             from evidence_items
             where project_path = ?1
             order by created_at desc",
        )?;
        let item_rows = items.query_map([project_path], evidence_item_from_row)?;
        entries.extend(collect_rows(item_rows)?);

        Ok(entries)
    }

    fn complete_evidence_run_with_conn(
        &self,
        conn: &Connection,
        id: i64,
        status: &str,
        summary: &str,
        completed_at: &str,
    ) -> anyhow::Result<EvidenceEntry> {
        let changed = conn.execute(
            "update evidence_runs
             set status = ?1, summary = ?2, completed_at = ?3
             where id = ?4",
            params![status, summary.trim(), completed_at, id],
        )?;
        if changed == 0 {
            anyhow::bail!("evidence run not found: {id}");
        }
        self.get_evidence_run_with_conn(conn, id)
    }

    fn find_evidence_run_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<Option<EvidenceEntry>> {
        conn.query_row(
            "select id, workspace_id, project_id, project_path, prd_slug, command, status,
                    summary, terminal_session_id, terminal_log_path, created_at, completed_at
             from evidence_runs
             where id = ?1",
            [id],
            evidence_run_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    fn get_evidence_run_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<EvidenceEntry> {
        self.find_evidence_run_with_conn(conn, id)?
            .ok_or_else(|| anyhow::anyhow!("evidence run not found: {id}"))
    }

    fn get_requirement_card_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<RequirementCard> {
        let project_ids = self.list_requirement_project_ids_with_conn(conn, id)?;
        conn.query_row(
            "select id, workspace_id, project_id, public_id, title, slug, body, status, prd_slug,
                    archived_from_status, archived_at, created_at, updated_at, flow_id,
                    priority, checklist_json, agent_prompt
             from requirement_cards
             where id = ?1",
            [id],
            |row| {
                Ok(RequirementCard {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    project_id: row.get(2)?,
                    project_ids: project_ids.clone(),
                    public_id: row.get(3)?,
                    title: row.get(4)?,
                    slug: row.get(5)?,
                    body: row.get(6)?,
                    status: row.get(7)?,
                    prd_slug: row.get(8)?,
                    archived_from_status: row.get(9)?,
                    archived_at: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    flow_id: row.get(13)?,
                    priority: row.get(14)?,
                    checklist_json: row.get(15)?,
                    agent_prompt: row.get(16)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("requirement card not found: {id}"))
    }

    fn get_requirement_attachment_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<RequirementAttachment> {
        conn.query_row(
            "select id, card_id, name, file_path, created_at
             from requirement_attachments
             where id = ?1",
            [id],
            |row| {
                Ok(RequirementAttachment {
                    id: row.get(0)?,
                    card_id: row.get(1)?,
                    name: row.get(2)?,
                    file_path: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("requirement attachment not found: {id}"))
    }

    fn get_knowledge_source_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<KnowledgeSource> {
        conn.query_row(
            "select id, workspace_id, project_id, blueprint_id, scope, name, file_path,
                    original_path, created_at
             from knowledge_sources
             where id = ?1",
            [id],
            knowledge_source_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("knowledge source not found: {id}"))
    }

    fn find_project_blueprint_with_conn(
        &self,
        conn: &Connection,
        id: &str,
    ) -> anyhow::Result<Option<ProjectBlueprint>> {
        conn.query_row(
            "select id, workspace_id, title, slug, status, idea, agent_profile_id, agent_session_id,
                    knowledge_source_ids_json, answers_json, running_summary,
                    detected_subprojects_json, prd, techspec, tasks_json, definition_of_done,
                    project_id, created_at, updated_at
             from project_blueprints
             where id = ?1",
            [id.trim()],
            project_blueprint_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    fn project_blueprint_knowledge_manifest(
        &self,
        blueprint: &ProjectBlueprint,
    ) -> anyhow::Result<serde_json::Value> {
        let ids = parse_i64_array(&blueprint.knowledge_source_ids_json);
        let conn = self.workspace_connect_by_id(blueprint.workspace_id)?;
        let mut sources = Vec::new();
        for id in ids {
            if let Some(source) = conn
                .query_row(
                    "select id, workspace_id, project_id, blueprint_id, scope, name, file_path,
                            original_path, created_at
                     from knowledge_sources
                     where id = ?1 and workspace_id = ?2",
                    params![id, blueprint.workspace_id],
                    knowledge_source_from_row,
                )
                .optional()?
            {
                sources.push(serde_json::json!({
                    "id": source.id,
                    "scope": source.scope,
                    "name": source.name,
                    "file_path": source.file_path,
                    "original_path": source.original_path,
                    "project_id": source.project_id,
                    "created_at": source.created_at
                }));
            }
        }
        Ok(serde_json::json!({
            "blueprint_id": blueprint.id,
            "blueprint_slug": blueprint.slug,
            "sources": sources
        }))
    }

    fn list_requirement_project_ids_with_conn(
        &self,
        conn: &Connection,
        card_id: i64,
    ) -> anyhow::Result<Vec<i64>> {
        let mut stmt = conn.prepare(
            "select project_id
             from requirement_card_projects
             where card_id = ?1
             order by project_id asc",
        )?;
        let rows = stmt.query_map([card_id], |row| row.get(0))?;
        collect_rows(rows)
    }

    fn get_evidence_item(&self, id: i64) -> anyhow::Result<EvidenceEntry> {
        let conn = self.connect()?;
        conn.query_row(
            "select id, run_id, project_path, kind, title, relative_path, absolute_path,
                    status, summary, created_at
             from evidence_items
             where id = ?1",
            [id],
            evidence_item_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("evidence item not found: {id}"))
    }

    fn find_agent_profile_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<Option<AgentProfile>> {
        conn.query_row(
            "select id, workspace_id, project_id, name, provider, model, reasoning_effort,
                    sandbox, context_mode, rtk_enabled, created_at, updated_at
             from agent_profiles
             where id = ?1",
            [id],
            agent_profile_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    fn get_agent_profile_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<AgentProfile> {
        self.find_agent_profile_with_conn(conn, id)?
            .ok_or_else(|| anyhow::anyhow!("agent profile not found: {id}"))
    }

    fn find_agent_session_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<Option<AgentSession>> {
        conn.query_row(
            "select id, profile_id, workspace_id, project_id, requirement_card_id, scope,
                    project_path, provider, model,
                    reasoning_effort, sandbox, context_mode, provider_session_id, codex_session_id, status,
                    title, created_at, updated_at
             from agent_sessions
             where id = ?1",
            [id],
            agent_session_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    fn get_agent_session_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<AgentSession> {
        self.find_agent_session_with_conn(conn, id)?
            .ok_or_else(|| anyhow::anyhow!("agent session not found: {id}"))
    }

    fn get_agent_message_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> anyhow::Result<AgentMessage> {
        conn.query_row(
            "select id, session_id, role, content, raw_json, created_at
             from agent_messages
             where id = ?1",
            [id],
            agent_message_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("agent message not found: {id}"))
    }

    fn connect(&self) -> anyhow::Result<Connection> {
        Connection::open(&self.path)
            .with_context(|| format!("failed to open {}", self.path.display()))
    }
}

fn migrate_connection(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
            create table if not exists workspaces (
              id integer primary key autoincrement,
              name text not null,
              root_path text not null,
              created_at text not null
            );

            create table if not exists projects (
              id integer primary key autoincrement,
              workspace_id integer not null references workspaces(id) on delete cascade,
              name text not null,
              path text not null,
              remote_url text,
              parent_project_id integer,
              is_submodule integer not null default 0,
              submodule_path text,
              created_at text not null
            );

            create index if not exists idx_projects_workspace on projects(workspace_id);

            create table if not exists project_card_sequences (
              project_id integer primary key references projects(id) on delete cascade,
              prefix text not null,
              next_number integer not null,
              updated_at text not null
            );

            create table if not exists workspace_machines (
              id text primary key,
              workspace_id integer not null references workspaces(id) on delete cascade,
              project_id integer references projects(id) on delete set null,
              provider text not null,
              provider_runtime text not null,
              provider_profile text not null,
              display_name text not null,
              preset_id text not null,
              image_family text not null,
              status text not null,
              access_user text,
              web_port integer,
              rdp_port integer,
              ssh_port integer,
              last_health_status text,
              last_health_summary text,
              last_error_code text,
              last_error_message text,
              created_at text not null,
              updated_at text not null,
              unique(workspace_id, provider, provider_profile)
            );

            create index if not exists idx_workspace_machines_workspace
              on workspace_machines(workspace_id, updated_at desc);

            create table if not exists deploy_stacks (
              id text primary key,
              workspace_id integer not null references workspaces(id) on delete cascade,
              name text not null,
              slug text not null,
              status text not null,
              active_version_id text,
              active_machine_id text,
              created_at text not null,
              updated_at text not null,
              unique(workspace_id, slug)
            );

            create index if not exists idx_deploy_stacks_workspace
              on deploy_stacks(workspace_id, updated_at desc);

            create table if not exists deploy_versions (
              id text primary key,
              stack_id text not null references deploy_stacks(id) on delete cascade,
              workspace_id integer not null references workspaces(id) on delete cascade,
              label text not null,
              status text not null,
              target_machine_id text,
              artifact_path text not null,
              manifest_path text not null,
              manifest_json text not null default '{}',
              review_status text not null,
              reviewed_at text,
              blocking_findings_json text not null default '[]',
              created_at text not null,
              updated_at text not null,
              unique(stack_id, label)
            );

            create index if not exists idx_deploy_versions_stack
              on deploy_versions(stack_id, created_at desc);

            create table if not exists deploy_version_projects (
              id integer primary key autoincrement,
              version_id text not null references deploy_versions(id) on delete cascade,
              project_id integer not null references projects(id) on delete cascade,
              name text not null,
              path text not null,
              branch text,
              commit_sha text,
              dirty integer not null default 0,
              package_path text not null
            );

            create index if not exists idx_deploy_version_projects_version
              on deploy_version_projects(version_id);

            create table if not exists deploy_runs (
              id text primary key,
              stack_id text not null references deploy_stacks(id) on delete cascade,
              version_id text references deploy_versions(id) on delete set null,
              machine_id text,
              operation text not null,
              status text not null,
              started_at text not null,
              completed_at text,
              summary text not null default '',
              agent_profile_id integer references agent_profiles(id) on delete set null,
              agent_name text,
              agent_provider text,
              agent_model text,
              orchestration_status text not null default 'manual',
              orchestration_report_json text not null default '{}'
            );

            create index if not exists idx_deploy_runs_version
              on deploy_runs(version_id, started_at desc);

            create table if not exists deploy_run_steps (
              id integer primary key autoincrement,
              run_id text not null references deploy_runs(id) on delete cascade,
              step_key text not null,
              status text not null,
              message text not null,
              log_path text,
              error_code text,
              started_at text not null,
              completed_at text
            );

            create index if not exists idx_deploy_run_steps_run
              on deploy_run_steps(run_id, id asc);

            create table if not exists deploy_target_bootstrap (
              machine_id text primary key,
              workspace_id integer not null references workspaces(id) on delete cascade,
              target_os text not null,
              status text not null,
              ssh_public_key_path text,
              last_preflight_json text not null default '{}',
              updated_at text not null
            );

            create table if not exists requirement_cards (
              id integer primary key autoincrement,
              workspace_id integer not null references workspaces(id) on delete cascade,
              project_id integer references projects(id) on delete set null,
              public_id text not null default '',
              title text not null,
              slug text not null,
              body text not null default '',
              priority text not null default 'medium',
              checklist_json text not null default '[]',
              agent_prompt text not null default '',
              status text not null,
              prd_slug text,
              archived_from_status text,
              archived_at text,
              created_at text not null,
              updated_at text not null
            );

            create index if not exists idx_requirement_cards_workspace_updated
              on requirement_cards(workspace_id, updated_at desc);

            create table if not exists requirement_card_projects (
              card_id integer not null references requirement_cards(id) on delete cascade,
              project_id integer not null references projects(id) on delete cascade,
              primary key (card_id, project_id)
            );

            create index if not exists idx_requirement_card_projects_project
              on requirement_card_projects(project_id);

            insert or ignore into requirement_card_projects (card_id, project_id)
              select id, project_id
              from requirement_cards
              where project_id is not null;

            create table if not exists requirement_stage_forms (
              card_id integer not null references requirement_cards(id) on delete cascade,
              stage_id text not null,
              payload_json text not null default '{}',
              updated_at text not null,
              primary key (card_id, stage_id)
            );

            create table if not exists requirement_attachments (
              id integer primary key autoincrement,
              card_id integer not null references requirement_cards(id) on delete cascade,
              name text not null,
              file_path text not null,
              created_at text not null
            );

            create index if not exists idx_requirement_attachments_card_created
              on requirement_attachments(card_id, created_at desc);

            create table if not exists project_blueprints (
              id text primary key,
              workspace_id integer not null references workspaces(id) on delete cascade,
              title text not null,
              slug text not null,
              status text not null,
              idea text not null default '',
              agent_profile_id integer references agent_profiles(id) on delete set null,
              agent_session_id integer references agent_sessions(id) on delete set null,
              knowledge_source_ids_json text not null default '[]',
              answers_json text not null default '[]',
              running_summary text not null default '',
              detected_subprojects_json text not null default '[]',
              prd text not null default '',
              techspec text not null default '',
              tasks_json text not null default '[]',
              definition_of_done text not null default '',
              project_id integer references projects(id) on delete set null,
              created_at text not null,
              updated_at text not null,
              unique(workspace_id, slug)
            );

            create index if not exists idx_project_blueprints_workspace_updated
              on project_blueprints(workspace_id, updated_at desc);

            create table if not exists knowledge_sources (
              id integer primary key autoincrement,
              workspace_id integer not null references workspaces(id) on delete cascade,
              project_id integer references projects(id) on delete set null,
              blueprint_id text references project_blueprints(id) on delete set null,
              scope text not null,
              name text not null,
              file_path text not null,
              original_path text,
              created_at text not null
            );

            create index if not exists idx_knowledge_sources_workspace_created
              on knowledge_sources(workspace_id, created_at desc);
            create index if not exists idx_knowledge_sources_project_created
              on knowledge_sources(project_id, created_at desc);

            create table if not exists evidence_runs (
              id integer primary key autoincrement,
              workspace_id integer,
              project_id integer,
              project_path text not null,
              prd_slug text,
              command text not null,
              status text not null,
              summary text not null default '',
              terminal_session_id text,
              terminal_log_path text,
              created_at text not null,
              completed_at text
            );

            create table if not exists evidence_items (
              id integer primary key autoincrement,
              run_id integer references evidence_runs(id) on delete set null,
              project_path text not null,
              kind text not null,
              title text not null,
              relative_path text,
              absolute_path text,
              status text not null,
              summary text not null default '',
              created_at text not null
            );

            create table if not exists agent_profiles (
              id integer primary key autoincrement,
              workspace_id integer not null references workspaces(id) on delete cascade,
              project_id integer references projects(id) on delete set null,
              name text not null,
              provider text not null,
              model text,
              reasoning_effort text,
              sandbox text not null,
              context_mode text not null default 'auto_lean',
              rtk_enabled integer not null default 0,
              created_at text not null,
              updated_at text not null
            );

            create table if not exists agent_sessions (
              id integer primary key autoincrement,
              profile_id integer not null references agent_profiles(id) on delete cascade,
              workspace_id integer not null references workspaces(id) on delete cascade,
              project_id integer references projects(id) on delete set null,
              requirement_card_id integer references requirement_cards(id) on delete set null,
              scope text not null default 'chat',
              project_path text not null,
              provider text not null,
              model text,
              reasoning_effort text,
              sandbox text not null,
              context_mode text not null default 'auto_lean',
              provider_session_id text,
              codex_session_id text,
              status text not null,
              title text not null,
              created_at text not null,
              updated_at text not null
            );

            create table if not exists agent_messages (
              id integer primary key autoincrement,
              session_id integer not null references agent_sessions(id) on delete cascade,
              role text not null,
              content text not null default '',
              raw_json text,
              created_at text not null
            );

            create table if not exists agent_run_events (
              id integer primary key autoincrement,
              session_id integer not null references agent_sessions(id) on delete cascade,
              run_id text not null,
              provider text not null,
              phase text not null,
              elapsed_ms integer not null default 0,
              details_json text not null default '{}',
              created_at text not null
            );

            create index if not exists idx_evidence_runs_project_created
              on evidence_runs(project_path, created_at desc);
            create index if not exists idx_evidence_items_project_created
              on evidence_items(project_path, created_at desc);
            drop index if exists idx_evidence_items_project_relative;
            create index if not exists idx_evidence_items_project_relative
              on evidence_items(project_path, relative_path);
            create index if not exists idx_agent_profiles_workspace_project
              on agent_profiles(workspace_id, project_id, updated_at desc);
            create index if not exists idx_agent_sessions_workspace_project
              on agent_sessions(workspace_id, project_id, updated_at desc);
            create index if not exists idx_agent_messages_session
              on agent_messages(session_id, id asc);
            create index if not exists idx_agent_run_events_session_run
              on agent_run_events(session_id, run_id, id asc);
            ",
    )?;
    ensure_column(
        conn,
        "requirement_cards",
        "public_id",
        "text not null default ''",
    )?;
    ensure_column(conn, "requirement_cards", "archived_from_status", "text")?;
    ensure_column(conn, "requirement_cards", "archived_at", "text")?;
    ensure_column(conn, "requirement_cards", "flow_id", "text")?;
    ensure_column(conn, "projects", "parent_project_id", "integer")?;
    ensure_column(conn, "projects", "is_submodule", "integer not null default 0")?;
    ensure_column(conn, "projects", "submodule_path", "text")?;
    ensure_column(
        conn,
        "requirement_cards",
        "priority",
        "text not null default 'medium'",
    )?;
    ensure_column(
        conn,
        "requirement_cards",
        "checklist_json",
        "text not null default '[]'",
    )?;
    ensure_column(
        conn,
        "requirement_cards",
        "agent_prompt",
        "text not null default ''",
    )?;
    ensure_column(conn, "workspace_machines", "access_user", "text")?;
    ensure_column(conn, "agent_profiles", "reasoning_effort", "text")?;
    ensure_column(
        conn,
        "agent_profiles",
        "context_mode",
        "text not null default 'auto_lean'",
    )?;
    ensure_column(
        conn,
        "agent_profiles",
        "rtk_enabled",
        "integer not null default 0",
    )?;
    ensure_column(conn, "agent_sessions", "requirement_card_id", "integer")?;
    ensure_column(
        conn,
        "agent_sessions",
        "scope",
        "text not null default 'chat'",
    )?;
    ensure_column(conn, "agent_sessions", "reasoning_effort", "text")?;
    ensure_column(
        conn,
        "agent_sessions",
        "context_mode",
        "text not null default 'auto_lean'",
    )?;
    ensure_column(conn, "agent_sessions", "provider_session_id", "text")?;
    ensure_column(conn, "deploy_runs", "agent_profile_id", "integer")?;
    ensure_column(conn, "deploy_runs", "agent_name", "text")?;
    ensure_column(conn, "deploy_runs", "agent_provider", "text")?;
    ensure_column(conn, "deploy_runs", "agent_model", "text")?;
    ensure_column(
        conn,
        "deploy_runs",
        "orchestration_status",
        "text not null default 'manual'",
    )?;
    ensure_column(
        conn,
        "deploy_runs",
        "orchestration_report_json",
        "text not null default '{}'",
    )?;
    conn.execute(
        "create index if not exists idx_agent_sessions_card_interview
         on agent_sessions(workspace_id, requirement_card_id, scope, updated_at desc)",
        [],
    )?;
    conn.execute(
        "update agent_sessions
         set provider_session_id = codex_session_id
         where provider_session_id is null and codex_session_id is not null",
        [],
    )?;
    backfill_requirement_public_ids(conn)?;
    conn.execute(
        "create unique index if not exists idx_requirement_cards_workspace_public_id
         on requirement_cards(workspace_id, public_id)
         where public_id <> ''",
        [],
    )?;
    conn.execute(
        "create table if not exists app_state (
           key text primary key,
           value text not null,
           updated_at text not null
         )",
        [],
    )?;
    Ok(())
}

fn reconcile_workspace_identity(conn: &Connection, workspace: &Workspace) -> anyhow::Result<()> {
    let matches = matching_workspace_identities(conn, workspace)?;
    if matches.is_empty() {
        return Ok(());
    };

    conn.execute(
        "insert or ignore into workspaces (id, name, root_path, created_at)
         values (?1, ?2, ?3, ?4)",
        params![
            workspace.id,
            workspace.name,
            workspace.root_path,
            workspace.created_at
        ],
    )?;
    for existing in &matches {
        if existing.root_path != workspace.root_path
            && paths_are_workspace_equivalent(&existing.root_path, &workspace.root_path)
        {
            rewrite_workspace_path_references(conn, &existing.root_path, &workspace.root_path)?;
        }
    }
    for existing in matches {
        if existing.id == workspace.id {
            continue;
        }
        conn.execute(
            "update projects set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute(
            "update requirement_cards set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute(
            "update project_blueprints set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute(
            "update knowledge_sources set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute(
            "update evidence_runs set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute(
            "update agent_profiles set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute(
            "update agent_sessions set workspace_id = ?1 where workspace_id = ?2",
            params![workspace.id, existing.id],
        )?;
        conn.execute("delete from workspaces where id = ?1", params![existing.id])?;
    }
    conn.execute(
        "update workspaces set name = ?1, root_path = ?2, created_at = ?3 where id = ?4",
        params![
            workspace.name,
            workspace.root_path,
            workspace.created_at,
            workspace.id
        ],
    )?;
    Ok(())
}

#[derive(Debug)]
struct WorkspaceIdentity {
    id: i64,
    root_path: String,
}

fn matching_workspace_identities(
    conn: &Connection,
    workspace: &Workspace,
) -> anyhow::Result<Vec<WorkspaceIdentity>> {
    let mut stmt = conn.prepare("select id, root_path from workspaces")?;
    let rows = stmt.query_map([], |row| {
        Ok(WorkspaceIdentity {
            id: row.get(0)?,
            root_path: row.get(1)?,
        })
    })?;

    let mut matches = Vec::new();
    for row in rows {
        let candidate = row?;
        if candidate.root_path == workspace.root_path
            || paths_are_workspace_equivalent(&candidate.root_path, &workspace.root_path)
        {
            matches.push(candidate);
        }
    }

    Ok(matches)
}

fn rewrite_workspace_path_references(
    conn: &Connection,
    old_root: &str,
    new_root: &str,
) -> anyhow::Result<()> {
    rewrite_table_path_column(conn, "projects", "path", old_root, new_root)?;
    rewrite_table_path_column(conn, "evidence_runs", "project_path", old_root, new_root)?;
    rewrite_table_path_column(conn, "evidence_items", "project_path", old_root, new_root)?;
    rewrite_table_path_column(conn, "evidence_items", "absolute_path", old_root, new_root)?;
    rewrite_table_path_column(
        conn,
        "requirement_attachments",
        "file_path",
        old_root,
        new_root,
    )?;
    rewrite_table_path_column(conn, "knowledge_sources", "file_path", old_root, new_root)?;
    rewrite_table_path_column(
        conn,
        "knowledge_sources",
        "original_path",
        old_root,
        new_root,
    )?;
    rewrite_table_path_column(conn, "agent_sessions", "project_path", old_root, new_root)?;
    Ok(())
}

fn rewrite_table_path_column(
    conn: &Connection,
    table: &str,
    column: &str,
    old_root: &str,
    new_root: &str,
) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(&format!(
        "select id, {column} from {table} where {column} is not null"
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut updates = Vec::new();
    for row in rows {
        let (id, value) = row?;
        if let Some(next_value) = rewrite_path_root(&value, old_root, new_root) {
            if next_value != value {
                updates.push((id, next_value));
            }
        }
    }
    drop(stmt);

    for (id, value) in updates {
        conn.execute(
            &format!("update {table} set {column} = ?1 where id = ?2"),
            params![value, id],
        )?;
    }
    Ok(())
}

fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        path: row.get(3)?,
        remote_url: row.get(4)?,
        parent_project_id: row.get(5)?,
        is_submodule: row.get::<_, i64>(6)? != 0,
        submodule_path: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn find_project_by_equivalent_path_with_conn(
    conn: &Connection,
    workspace_id: i64,
    path: &str,
) -> anyhow::Result<Option<Project>> {
    let target_key = project_path_identity(path);
    let mut stmt = conn.prepare(
        "select id, workspace_id, name, path, remote_url, parent_project_id,
                is_submodule, submodule_path, created_at
         from projects
         where workspace_id = ?1",
    )?;
    let rows = stmt.query_map([workspace_id], project_from_row)?;
    for row in rows {
        let project = row?;
        if project_path_identity(&project.path) == target_key {
            return Ok(Some(project));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone)]
struct ProjectIdentity {
    id: i64,
    name: String,
    path: String,
}

fn dedupe_workspace_projects(conn: &Connection, workspace_id: i64) -> anyhow::Result<()> {
    let projects = {
        let mut stmt = conn.prepare(
            "select id, name, path
             from projects
             where workspace_id = ?1
             order by id asc",
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            Ok(ProjectIdentity {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
            })
        })?;
        collect_rows(rows)?
    };

    let mut by_path = BTreeMap::<String, Vec<ProjectIdentity>>::new();
    for project in projects {
        by_path
            .entry(project_path_identity(&project.path))
            .or_default()
            .push(project);
    }

    for mut group in by_path.into_values().filter(|items| items.len() > 1) {
        group.sort_by(|left, right| {
            project_reference_count(conn, right.id)
                .unwrap_or_default()
                .cmp(&project_reference_count(conn, left.id).unwrap_or_default())
                .then_with(|| left.id.cmp(&right.id))
        });
        let primary = group.remove(0);
        let normalized_path = normalize_filesystem_path(&primary.path);
        conn.execute(
            "update projects set path = ?1 where id = ?2",
            params![normalized_path, primary.id],
        )?;
        for duplicate in group {
            merge_duplicate_project(conn, workspace_id, &primary, &duplicate)?;
        }
    }
    Ok(())
}

fn project_reference_count(conn: &Connection, project_id: i64) -> anyhow::Result<i64> {
    let tables = [
        ("requirement_cards", "project_id"),
        ("requirement_card_projects", "project_id"),
        ("agent_profiles", "project_id"),
        ("agent_sessions", "project_id"),
        ("evidence_runs", "project_id"),
        ("project_card_sequences", "project_id"),
        ("knowledge_sources", "project_id"),
        ("project_blueprints", "project_id"),
    ];
    let mut count = 0;
    for (table, column) in tables {
        count += conn.query_row(
            &format!("select count(*) from {table} where {column} = ?1"),
            [project_id],
            |row| row.get::<_, i64>(0),
        )?;
    }
    Ok(count)
}

fn merge_duplicate_project(
    conn: &Connection,
    workspace_id: i64,
    primary: &ProjectIdentity,
    duplicate: &ProjectIdentity,
) -> anyhow::Result<()> {
    merge_project_card_sequence(conn, workspace_id, primary, duplicate)?;
    conn.execute(
        "update requirement_cards set project_id = ?1 where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "insert or ignore into requirement_card_projects (card_id, project_id)
         select card_id, ?1 from requirement_card_projects where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "delete from requirement_card_projects where project_id = ?1",
        params![duplicate.id],
    )?;
    conn.execute(
        "update agent_profiles set project_id = ?1 where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "update agent_sessions set project_id = ?1 where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "update evidence_runs set project_id = ?1 where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "update knowledge_sources set project_id = ?1 where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "update project_blueprints set project_id = ?1 where project_id = ?2",
        params![primary.id, duplicate.id],
    )?;
    conn.execute(
        "delete from project_card_sequences where project_id = ?1",
        params![duplicate.id],
    )?;
    conn.execute("delete from projects where id = ?1", params![duplicate.id])?;
    Ok(())
}

fn merge_project_card_sequence(
    conn: &Connection,
    workspace_id: i64,
    primary: &ProjectIdentity,
    duplicate: &ProjectIdentity,
) -> anyhow::Result<()> {
    let mut sequences = Vec::new();
    let mut stmt = conn.prepare(
        "select project_id, prefix, next_number, updated_at
         from project_card_sequences
         where project_id in (?1, ?2)",
    )?;
    let rows = stmt.query_map(params![primary.id, duplicate.id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        sequences.push(row?);
    }
    drop(stmt);

    if sequences.is_empty() {
        return Ok(());
    }

    let primary_sequence = sequences
        .iter()
        .find(|(project_id, _, _, _)| *project_id == primary.id);
    let fallback_sequence = sequences.first();
    let prefix = primary_sequence
        .or(fallback_sequence)
        .map(|(_, prefix, _, _)| normalize_card_prefix(prefix))
        .unwrap_or_else(|| default_project_prefix(&primary.name));
    let next_from_sequences = sequences
        .iter()
        .map(|(_, _, next_number, _)| *next_number)
        .max()
        .unwrap_or(1);
    let updated_at = sequences
        .iter()
        .map(|(_, _, _, updated_at)| updated_at.as_str())
        .max()
        .unwrap_or("")
        .to_string();
    let next_number = next_from_sequences
        .max(max_public_id_number(conn, workspace_id, &prefix)? + 1)
        .max(1);
    conn.execute(
        "insert into project_card_sequences (project_id, prefix, next_number, updated_at)
         values (?1, ?2, ?3, ?4)
         on conflict(project_id) do update set
           prefix = excluded.prefix,
           next_number = excluded.next_number,
           updated_at = excluded.updated_at",
        params![
            primary.id,
            prefix,
            next_number,
            if updated_at.is_empty() {
                Utc::now().to_rfc3339()
            } else {
                updated_at
            }
        ],
    )?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(&format!("pragma table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing_column in columns {
        if existing_column? == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("alter table {table} add column {column} {definition}"),
        [],
    )?;
    Ok(())
}

fn ensure_project_card_sequence_configs(
    conn: &Connection,
    workspace_id: i64,
) -> anyhow::Result<()> {
    let projects = {
        let mut stmt = conn.prepare("select id, name from projects where workspace_id = ?1")?;
        let rows = stmt.query_map([workspace_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        collect_rows(rows)?
    };
    for (project_id, project_name) in projects {
        let exists = conn
            .query_row(
                "select 1 from project_card_sequences where project_id = ?1",
                [project_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if exists.is_some() {
            continue;
        }
        let prefix = default_project_prefix(&project_name);
        let next_number = max_public_id_number(conn, workspace_id, &prefix)? + 1;
        conn.execute(
            "insert into project_card_sequences (project_id, prefix, next_number, updated_at)
             values (?1, ?2, ?3, ?4)",
            params![
                project_id,
                prefix,
                next_number.max(1),
                Utc::now().to_rfc3339()
            ],
        )?;
    }
    Ok(())
}

fn backfill_requirement_public_ids(conn: &Connection) -> anyhow::Result<()> {
    let cards = {
        let mut stmt = conn.prepare(
            "select id, workspace_id, project_id
             from requirement_cards
             where public_id = ''
             order by id asc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })?;
        collect_rows(rows)?
    };
    for (card_id, workspace_id, project_id) in cards {
        ensure_project_card_sequence_configs(conn, workspace_id)?;
        let linked_project_id = first_requirement_project_id(conn, card_id)?.or(project_id);
        let public_id = if let Some(project_id) = linked_project_id {
            reserve_next_public_id(conn, workspace_id, project_id)?
        } else {
            format_public_id("CARD", card_id)
        };
        conn.execute(
            "update requirement_cards set public_id = ?1 where id = ?2",
            params![public_id, card_id],
        )?;
    }
    Ok(())
}

fn first_requirement_project_id(conn: &Connection, card_id: i64) -> anyhow::Result<Option<i64>> {
    conn.query_row(
        "select project_id
         from requirement_card_projects
         where card_id = ?1
         order by project_id asc
         limit 1",
        [card_id],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn reserve_next_public_id(
    conn: &Connection,
    workspace_id: i64,
    project_id: i64,
) -> anyhow::Result<String> {
    let (project_name, configured): (String, Option<(String, i64)>) = conn
        .query_row(
            "select p.name, s.prefix, s.next_number
             from projects p
             left join project_card_sequences s on s.project_id = p.id
             where p.id = ?1 and p.workspace_id = ?2",
            params![project_id, workspace_id],
            |row| {
                let prefix: Option<String> = row.get(1)?;
                let next_number: Option<i64> = row.get(2)?;
                Ok((row.get(0)?, prefix.zip(next_number)))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("project not found in workspace: {project_id}"))?;
    let (prefix, next_number) =
        configured.unwrap_or_else(|| (default_project_prefix(&project_name), 1));
    let prefix = normalize_card_prefix(&prefix);
    let number = next_number
        .max(max_public_id_number(conn, workspace_id, &prefix)? + 1)
        .max(1);
    conn.execute(
        "insert into project_card_sequences (project_id, prefix, next_number, updated_at)
         values (?1, ?2, ?3, ?4)
         on conflict(project_id) do update set
           prefix = excluded.prefix,
           next_number = excluded.next_number,
           updated_at = excluded.updated_at",
        params![project_id, prefix, number + 1, Utc::now().to_rfc3339()],
    )?;
    Ok(format_public_id(&prefix, number))
}

fn max_public_id_number(conn: &Connection, workspace_id: i64, prefix: &str) -> anyhow::Result<i64> {
    let prefix = normalize_card_prefix(prefix);
    let marker = format!("{prefix}-");
    let mut stmt =
        conn.prepare("select public_id from requirement_cards where workspace_id = ?1")?;
    let rows = stmt.query_map([workspace_id], |row| row.get::<_, String>(0))?;
    let mut max_number = 0;
    for row in rows {
        let public_id = row?;
        let Some(suffix) = public_id.strip_prefix(&marker) else {
            continue;
        };
        if suffix.chars().all(|ch| ch.is_ascii_digit()) {
            if let Ok(number) = suffix.parse::<i64>() {
                max_number = max_number.max(number);
            }
        }
    }
    Ok(max_number)
}

fn format_public_id(prefix: &str, number: i64) -> String {
    format!("{}-{number:03}", normalize_card_prefix(prefix))
}

fn default_project_prefix(project_name: &str) -> String {
    let words: Vec<String> = project_name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    let prefix = if words.len() > 1 {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .collect::<String>()
    } else {
        words
            .first()
            .map(|word| word.chars().take(3).collect::<String>())
            .unwrap_or_default()
    };
    normalize_card_prefix(&prefix)
}

fn normalize_card_prefix(prefix: &str) -> String {
    let normalized: String = prefix
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(8)
        .flat_map(|ch| ch.to_uppercase())
        .collect();
    if normalized.is_empty() {
        "CARD".to_string()
    } else {
        normalized
    }
}

fn workspace_db_path(root_path: &str) -> PathBuf {
    Path::new(root_path).join(".dw").join("clia-local.sqlite3")
}

fn workspace_attachment_dir(root_path: &str, card_id: i64) -> PathBuf {
    Path::new(root_path)
        .join(".dw")
        .join("gui")
        .join("attachments")
        .join(card_id.to_string())
}

fn is_managed_attachment_path(root_path: &str, file_path: &Path) -> bool {
    let managed_root = Path::new(root_path)
        .join(".dw")
        .join("gui")
        .join("attachments");
    file_path.starts_with(managed_root)
}

fn workspace_knowledge_dir(
    root_path: &str,
    project_id: Option<i64>,
    blueprint_id: Option<&str>,
) -> PathBuf {
    let mut path = Path::new(root_path).join(".dw").join("knowledge");
    if let Some(project_id) = project_id {
        path = path.join("projects").join(project_id.to_string());
    } else if let Some(blueprint_id) = blueprint_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        path = path
            .join("blueprints")
            .join(sanitize_filename(blueprint_id));
    } else {
        path = path.join("workspace");
    }
    path
}

fn is_managed_knowledge_path(root_path: &str, file_path: &Path) -> bool {
    file_path.starts_with(Path::new(root_path).join(".dw").join("knowledge"))
}

fn project_blueprint_spec_dir(root_path: &str, slug: &str) -> PathBuf {
    Path::new(root_path)
        .join(".dw")
        .join("spec")
        .join("projects")
        .join(slug)
}

fn unique_attachment_path(dir: &Path, name: &str) -> PathBuf {
    let safe_name = sanitize_filename(name);
    let timestamp = Utc::now().timestamp_millis();
    let mut candidate = dir.join(format!("{timestamp}-{safe_name}"));
    let mut index = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{timestamp}-{index}-{safe_name}"));
        index += 1;
    }
    candidate
}

fn sanitize_filename(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|value| match value {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            value if value.is_control() => '-',
            value => value,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();
    if sanitized.is_empty() {
        "attachment".to_string()
    } else {
        sanitized
    }
}

fn unique_project_blueprint_slug(
    conn: &Connection,
    workspace_id: i64,
    title: &str,
) -> anyhow::Result<String> {
    let base = slugify_store_text(title);
    let mut candidate = base.clone();
    let mut index = 2;
    loop {
        let exists = conn
            .query_row(
                "select 1 from project_blueprints where workspace_id = ?1 and slug = ?2 limit 1",
                params![workspace_id, candidate],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        if !exists {
            return Ok(candidate);
        }
        candidate = format!("{base}-{index}");
        index += 1;
    }
}

fn slugify_store_text(value: &str) -> String {
    let slug = value
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
        "project".to_string()
    } else {
        slug
    }
}

fn normalize_project_blueprint_status(status: &str) -> anyhow::Result<&'static str> {
    match status.trim().to_ascii_lowercase().as_str() {
        "draft" => Ok("draft"),
        "interviewing" => Ok("interviewing"),
        "planned" => Ok("planned"),
        "materialized" => Ok("materialized"),
        "archived" => Ok("archived"),
        other => anyhow::bail!("unsupported project blueprint status: {other}"),
    }
}

fn valid_json_or_default(value: &str, default: &str) -> String {
    serde_json::from_str::<serde_json::Value>(value)
        .map(|_| value.trim().to_string())
        .unwrap_or_else(|_| default.to_string())
}

fn parse_i64_array(value: &str) -> Vec<i64> {
    serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .and_then(|json| json.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_i64())
        .collect()
}

#[derive(Debug, Clone)]
struct ProjectBlueprintTask {
    title: String,
    body: String,
    dependencies: Vec<String>,
}

fn project_blueprint_task_entries(tasks_json: &str) -> Vec<ProjectBlueprintTask> {
    let parsed = serde_json::from_str::<serde_json::Value>(tasks_json).ok();
    let tasks = parsed
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            if let Some(text) = item
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return Some(ProjectBlueprintTask {
                    title: text.to_string(),
                    body: String::new(),
                    dependencies: Vec::new(),
                });
            }
            let object = item.as_object()?;
            let title = object
                .get("title")
                .or_else(|| object.get("name"))
                .or_else(|| object.get("task"))
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_string();
            let body = object
                .get("body")
                .or_else(|| object.get("description"))
                .or_else(|| object.get("details"))
                .and_then(|value| value.as_str())
                .map(str::trim)
                .unwrap_or("")
                .to_string();
            let dependencies = object
                .get("dependencies")
                .or_else(|| object.get("depends_on"))
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::trim))
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            Some(ProjectBlueprintTask {
                title,
                body,
                dependencies,
            })
        })
        .collect::<Vec<_>>();

    if tasks.is_empty() {
        vec![
            ProjectBlueprintTask {
                title: "Revisar PRD e TechSpec".to_string(),
                body: "Validar a especificação materializada antes de iniciar código.".to_string(),
                dependencies: Vec::new(),
            },
            ProjectBlueprintTask {
                title: "Implementar primeira versão".to_string(),
                body: "Executar as tarefas principais do plano aprovado.".to_string(),
                dependencies: vec!["Revisar PRD e TechSpec".to_string()],
            },
            ProjectBlueprintTask {
                title: "Validar definição de pronto".to_string(),
                body: "Rodar os gates de teste, revisão e aceite declarados no blueprint."
                    .to_string(),
                dependencies: vec!["Implementar primeira versão".to_string()],
            },
        ]
    } else {
        tasks
    }
}

fn project_blueprint_card_body(
    blueprint: &ProjectBlueprint,
    spec_dir: &Path,
    task: &ProjectBlueprintTask,
    order: usize,
) -> String {
    let dependencies = if task.dependencies.is_empty() {
        "Nenhuma.".to_string()
    } else {
        task.dependencies
            .iter()
            .map(|dependency| format!("- {dependency}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    [
        format!("Blueprint: {}", blueprint.title),
        format!("Blueprint ID: {}", blueprint.id),
        format!("Ordem: {order}"),
        format!("Spec: {}", spec_dir.display()),
        String::new(),
        "## Descrição".to_string(),
        if task.body.trim().is_empty() {
            "Sem descrição detalhada no plano do agente.".to_string()
        } else {
            task.body.clone()
        },
        String::new(),
        "## Dependências".to_string(),
        dependencies,
    ]
    .join("\n")
}

fn write_project_blueprint_docs(
    spec_dir: &Path,
    blueprint: &ProjectBlueprint,
    knowledge_manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    let tasks = project_blueprint_task_entries(&blueprint.tasks_json);
    let tasks_markdown = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| {
            let deps = if task.dependencies.is_empty() {
                "nenhuma".to_string()
            } else {
                task.dependencies.join(", ")
            };
            format!(
                "{}. {}\n   - Descrição: {}\n   - Dependências: {}",
                index + 1,
                task.title,
                if task.body.trim().is_empty() {
                    "Sem descrição detalhada."
                } else {
                    task.body.as_str()
                },
                deps
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let answers = pretty_json_or_raw(&blueprint.answers_json);
    let subprojects = pretty_json_or_raw(&blueprint.detected_subprojects_json);
    std::fs::write(
        spec_dir.join("prd.md"),
        markdown_or_fallback(
            &blueprint.prd,
            &format!(
                "# PRD - {}\n\n## Ideia\n{}\n\n## Resumo\n{}",
                blueprint.title,
                blueprint.idea,
                empty_as_placeholder(&blueprint.running_summary)
            ),
        ),
    )?;
    std::fs::write(
        spec_dir.join("techspec.md"),
        markdown_or_fallback(
            &blueprint.techspec,
            &format!(
                "# TechSpec - {}\n\nA especificação técnica ainda não foi preenchida pelo agente.",
                blueprint.title
            ),
        ),
    )?;
    std::fs::write(
        spec_dir.join("tasks.md"),
        format!("# Tasks - {}\n\n{tasks_markdown}\n", blueprint.title),
    )?;
    std::fs::write(
        spec_dir.join("definition-of-done.md"),
        markdown_or_fallback(
            &blueprint.definition_of_done,
            "# Definition of Done\n\n- PRD, TechSpec e tasks revisados.\n- Implementação validada por testes relevantes.\n- Evidências registradas no workspace.",
        ),
    )?;
    std::fs::write(
        spec_dir.join("interview.md"),
        format!(
            "# Entrevista - {}\n\n## Resumo em andamento\n{}\n\n## Respostas\n```json\n{}\n```\n\n## Subprojetos detectados\n```json\n{}\n```\n",
            blueprint.title,
            empty_as_placeholder(&blueprint.running_summary),
            answers,
            subprojects
        ),
    )?;
    std::fs::write(
        spec_dir.join("knowledge-manifest.json"),
        serde_json::to_string_pretty(knowledge_manifest)?,
    )?;
    Ok(())
}

fn markdown_or_fallback(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn empty_as_placeholder(value: &str) -> &str {
    if value.trim().is_empty() {
        "Sem resumo registrado."
    } else {
        value.trim()
    }
}

fn pretty_json_or_raw(value: &str) -> String {
    serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .and_then(|json| serde_json::to_string_pretty(&json).ok())
        .unwrap_or_else(|| value.trim().to_string())
}

fn knowledge_source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeSource> {
    Ok(KnowledgeSource {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        project_id: row.get(2)?,
        blueprint_id: row.get(3)?,
        scope: row.get(4)?,
        name: row.get(5)?,
        file_path: row.get(6)?,
        original_path: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn project_blueprint_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectBlueprint> {
    Ok(ProjectBlueprint {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        slug: row.get(3)?,
        status: row.get(4)?,
        idea: row.get(5)?,
        agent_profile_id: row.get(6)?,
        agent_session_id: row.get(7)?,
        knowledge_source_ids_json: row_text_lossy(row, 8)?,
        answers_json: row_text_lossy(row, 9)?,
        running_summary: row.get(10)?,
        detected_subprojects_json: row_text_lossy(row, 11)?,
        prd: row.get(12)?,
        techspec: row.get(13)?,
        tasks_json: row_text_lossy(row, 14)?,
        definition_of_done: row.get(15)?,
        project_id: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

struct GitmoduleEntry {
    path: String,
    url: Option<String>,
}

/// Minimal `.gitmodules` parser: returns the `path`/`url` of each `[submodule]`
/// block (branch and other keys are ignored).
fn parse_gitmodules(gitmodules_path: &Path) -> Vec<GitmoduleEntry> {
    let Ok(content) = std::fs::read_to_string(gitmodules_path) else {
        return Vec::new();
    };
    let mut entries: Vec<GitmoduleEntry> = Vec::new();
    let mut path: Option<String> = None;
    let mut url: Option<String> = None;
    let mut in_submodule = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            if in_submodule {
                if let Some(p) = path.take() {
                    entries.push(GitmoduleEntry { path: p, url: url.clone() });
                }
            }
            path = None;
            url = None;
            in_submodule = line.starts_with("[submodule");
            continue;
        }
        if !in_submodule {
            continue;
        }
        if let Some(rest) = line.strip_prefix("path") {
            if let Some((_, value)) = rest.split_once('=') {
                path = Some(value.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("url") {
            if let Some((_, value)) = rest.split_once('=') {
                url = Some(value.trim().to_string());
            }
        }
    }
    if in_submodule {
        if let Some(p) = path.take() {
            entries.push(GitmoduleEntry { path: p, url });
        }
    }
    entries.into_iter().filter(|entry| !entry.path.is_empty()).collect()
}

fn discover_workspace_git_projects(root_path: &str) -> anyhow::Result<Vec<PathBuf>> {
    let root = PathBuf::from(root_path);
    let mut projects = BTreeMap::<String, PathBuf>::new();
    if is_git_project_dir(&root) {
        projects.insert(
            normalize_project_path(&root.display().to_string()),
            root.clone(),
        );
    }
    for child_dir in [root.join("projects"), root.join("repos")] {
        if !child_dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&child_dir)
            .with_context(|| format!("failed to read {}", child_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if is_git_project_dir(&path) {
                projects.insert(normalize_project_path(&path.display().to_string()), path);
            }
        }
    }
    Ok(projects.into_values().collect())
}

fn is_git_project_dir(path: &Path) -> bool {
    path.is_dir() && path.join(".git").exists()
}

fn git_remote_origin(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!remote.is_empty()).then_some(remote)
}

pub fn normalize_filesystem_path(path: &str) -> String {
    let mut value = path.trim().to_string();
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        value = format!(r"\\{rest}");
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        value = rest.to_string();
    } else if let Some(rest) = value.strip_prefix(r"\??\") {
        value = rest.to_string();
    }
    value
}

fn normalize_project_path(path: &str) -> String {
    let normalized = std::fs::canonicalize(path.trim())
        .unwrap_or_else(|_| PathBuf::from(path.trim()))
        .display()
        .to_string();
    normalize_filesystem_path(&normalized)
}

fn project_path_identity(path: &str) -> String {
    let normalized = normalize_path_separators(path);
    if let Some(parts) = portable_drive_path_parts(&normalized) {
        return format!(
            "{}:{}",
            parts.drive,
            parts
                .components
                .iter()
                .map(|part| part.to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join("/")
        );
    }
    normalized
}

fn paths_are_workspace_equivalent(left: &str, right: &str) -> bool {
    if left.trim() == right.trim() {
        return true;
    }
    let Some(left_parts) = portable_drive_path_parts(left) else {
        return normalize_path_separators(left) == normalize_path_separators(right);
    };
    let Some(right_parts) = portable_drive_path_parts(right) else {
        return false;
    };
    left_parts.drive == right_parts.drive
        && components_equal_ignore_ascii_case(&left_parts.components, &right_parts.components)
}

fn rewrite_path_root(path: &str, old_root: &str, new_root: &str) -> Option<String> {
    if paths_are_workspace_equivalent(path, old_root) {
        return Some(new_root.trim().to_string());
    }

    if let (Some(path_parts), Some(old_parts)) = (
        portable_drive_path_parts(path),
        portable_drive_path_parts(old_root),
    ) {
        if path_parts.drive == old_parts.drive
            && components_start_with_ignore_ascii_case(
                &path_parts.components,
                &old_parts.components,
            )
        {
            return Some(join_under_root(
                new_root,
                &path_parts.components[old_parts.components.len()..],
            ));
        }
    }

    let normalized_path = normalize_path_separators(path);
    let normalized_old = normalize_path_separators(old_root);
    if let Some(suffix) = normalized_path
        .strip_prefix(&normalized_old)
        .and_then(|suffix| suffix.strip_prefix('/'))
    {
        let suffix_components = suffix
            .split('/')
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        return Some(join_under_root(new_root, &suffix_components));
    }

    None
}

#[derive(Debug, PartialEq, Eq)]
struct PortableDrivePath {
    drive: char,
    components: Vec<String>,
}

fn portable_drive_path_parts(path: &str) -> Option<PortableDrivePath> {
    let normalized = normalize_path_separators(path);
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Some(PortableDrivePath {
            drive: (bytes[0] as char).to_ascii_lowercase(),
            components: split_path_components(&normalized[2..]),
        });
    }

    if normalized.len() >= 7 && normalized[..5].eq_ignore_ascii_case("/mnt/") {
        let drive = normalized.as_bytes()[5] as char;
        if drive.is_ascii_alphabetic() && normalized.as_bytes().get(6) == Some(&b'/') {
            return Some(PortableDrivePath {
                drive: drive.to_ascii_lowercase(),
                components: split_path_components(&normalized[7..]),
            });
        }
    }

    None
}

fn normalize_path_separators(path: &str) -> String {
    let mut normalized = normalize_filesystem_path(path).replace('\\', "/");
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn split_path_components(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn components_equal_ignore_ascii_case(left: &[String], right: &[String]) -> bool {
    left.len() == right.len()
        && components_start_with_ignore_ascii_case(left, right)
        && components_start_with_ignore_ascii_case(right, left)
}

fn components_start_with_ignore_ascii_case(path: &[String], root: &[String]) -> bool {
    path.len() >= root.len()
        && path
            .iter()
            .zip(root.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn join_under_root(root: &str, suffix_components: &[String]) -> String {
    let trimmed_root = root.trim();
    if suffix_components.is_empty() {
        return trimmed_root.to_string();
    }
    let separator = if trimmed_root.contains('\\') {
        "\\"
    } else {
        "/"
    };
    let mut output = trimmed_root.trim_end_matches(['/', '\\']).to_string();
    for component in suffix_components {
        output.push_str(separator);
        output.push_str(component);
    }
    output
}

fn agent_profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentProfile> {
    Ok(AgentProfile {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        project_id: row.get(2)?,
        name: row.get(3)?,
        provider: row.get(4)?,
        model: row.get(5)?,
        reasoning_effort: row.get(6)?,
        sandbox: row.get(7)?,
        context_mode: row.get(8)?,
        rtk_enabled: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn agent_session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentSession> {
    Ok(AgentSession {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        workspace_id: row.get(2)?,
        project_id: row.get(3)?,
        requirement_card_id: row.get(4)?,
        scope: row.get(5)?,
        project_path: row.get(6)?,
        provider: row.get(7)?,
        model: row.get(8)?,
        reasoning_effort: row.get(9)?,
        sandbox: row.get(10)?,
        context_mode: row.get(11)?,
        provider_session_id: row.get(12)?,
        codex_session_id: row.get(13)?,
        status: row.get(14)?,
        title: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn agent_message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentMessage> {
    Ok(AgentMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        raw_json: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn agent_run_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentRunEvent> {
    Ok(AgentRunEvent {
        id: row.get(0)?,
        session_id: row.get(1)?,
        run_id: row.get(2)?,
        provider: row.get(3)?,
        phase: row.get(4)?,
        elapsed_ms: row.get(5)?,
        details_json: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn deploy_stack_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeployStack> {
    Ok(DeployStack {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        slug: row.get(3)?,
        status: row.get(4)?,
        active_version_id: row.get(5)?,
        active_machine_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn deploy_version_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeployVersion> {
    Ok(DeployVersion {
        id: row.get(0)?,
        stack_id: row.get(1)?,
        workspace_id: row.get(2)?,
        label: row.get(3)?,
        status: row.get(4)?,
        target_machine_id: row.get(5)?,
        artifact_path: row.get(6)?,
        manifest_path: row.get(7)?,
        manifest_json: row_text_lossy(row, 8)?,
        review_status: row.get(9)?,
        reviewed_at: row.get(10)?,
        blocking_findings_json: row_text_lossy(row, 11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn row_text_lossy(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    match row.get_ref(index)? {
        ValueRef::Text(value) | ValueRef::Blob(value) => {
            Ok(String::from_utf8_lossy(value).to_string())
        }
        ValueRef::Integer(value) => Ok(value.to_string()),
        ValueRef::Real(value) => Ok(value.to_string()),
        ValueRef::Null => Ok(String::new()),
    }
}

fn deploy_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeployRun> {
    Ok(DeployRun {
        id: row.get(0)?,
        stack_id: row.get(1)?,
        version_id: row.get(2)?,
        machine_id: row.get(3)?,
        operation: row.get(4)?,
        status: row.get(5)?,
        started_at: row.get(6)?,
        completed_at: row.get(7)?,
        summary: row.get(8)?,
        agent_profile_id: row.get(9)?,
        agent_name: row.get(10)?,
        agent_provider: row.get(11)?,
        agent_model: row.get(12)?,
        orchestration_status: row.get(13)?,
        orchestration_report_json: row_text_lossy(row, 14)?,
    })
}

fn deploy_run_step_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeployRunStep> {
    Ok(DeployRunStep {
        id: row.get(0)?,
        run_id: row.get(1)?,
        step_key: row.get(2)?,
        status: row.get(3)?,
        message: row.get(4)?,
        log_path: row.get(5)?,
        error_code: row.get(6)?,
        started_at: row.get(7)?,
        completed_at: row.get(8)?,
    })
}

fn deploy_version_project_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DeployVersionProject> {
    Ok(DeployVersionProject {
        id: row.get(0)?,
        version_id: row.get(1)?,
        project_id: row.get(2)?,
        name: row.get(3)?,
        path: row.get(4)?,
        branch: row.get(5)?,
        commit_sha: row.get(6)?,
        dirty: row.get::<_, i64>(7)? != 0,
        package_path: row.get(8)?,
    })
}

fn find_deploy_stack_with_conn(conn: &Connection, id: &str) -> anyhow::Result<Option<DeployStack>> {
    conn.query_row(
        "select id, workspace_id, name, slug, status, active_version_id, active_machine_id,
                created_at, updated_at
         from deploy_stacks
         where id = ?1
         limit 1",
        [id],
        deploy_stack_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn find_deploy_version_with_conn(
    conn: &Connection,
    id: &str,
) -> anyhow::Result<Option<DeployVersion>> {
    conn.query_row(
        "select id, stack_id, workspace_id, label, status, target_machine_id, artifact_path,
                manifest_path, manifest_json, review_status, reviewed_at,
                blocking_findings_json, created_at, updated_at
         from deploy_versions
         where id = ?1
         limit 1",
        [id],
        deploy_version_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn find_deploy_run_with_conn(conn: &Connection, id: &str) -> anyhow::Result<Option<DeployRun>> {
    conn.query_row(
        "select id, stack_id, version_id, machine_id, operation, status,
                started_at, completed_at, summary, agent_profile_id, agent_name, agent_provider,
                agent_model, orchestration_status, orchestration_report_json
         from deploy_runs
         where id = ?1
         limit 1",
        [id],
        deploy_run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn workspace_machine_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceMachine> {
    Ok(WorkspaceMachine {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        project_id: row.get(2)?,
        provider: row.get(3)?,
        provider_runtime: row.get(4)?,
        provider_profile: row.get(5)?,
        display_name: row.get(6)?,
        preset_id: row.get(7)?,
        image_family: row.get(8)?,
        access_user: row.get(9)?,
        status: row.get(10)?,
        web_port: row.get(11)?,
        rdp_port: row.get(12)?,
        ssh_port: row.get(13)?,
        last_health_status: row.get(14)?,
        last_health_summary: row.get(15)?,
        last_error_code: row.get(16)?,
        last_error_message: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn find_workspace_machine_with_conn(
    conn: &Connection,
    id: &str,
) -> anyhow::Result<Option<WorkspaceMachine>> {
    conn.query_row(
        "select id, workspace_id, project_id, provider, provider_runtime, provider_profile,
                display_name, preset_id, image_family, access_user, status,
                web_port, rdp_port, ssh_port,
                last_health_status, last_health_summary, last_error_code, last_error_message,
                created_at, updated_at
         from workspace_machines
         where id = ?1
         limit 1",
        [id],
        workspace_machine_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn evidence_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceEntry> {
    let id: i64 = row.get(0)?;
    let command: String = row.get(5)?;
    Ok(EvidenceEntry {
        id: format!("run:{id}"),
        record_type: "run".to_string(),
        run_id: Some(id),
        item_id: None,
        workspace_id: row.get(1)?,
        project_id: row.get(2)?,
        project_path: row.get(3)?,
        prd_slug: row.get(4)?,
        command: Some(command.clone()),
        status: row.get(6)?,
        summary: row.get(7)?,
        kind: "run".to_string(),
        title: command,
        relative_path: None,
        absolute_path: None,
        terminal_session_id: row.get(8)?,
        terminal_log_path: row.get(9)?,
        created_at: row.get(10)?,
        completed_at: row.get(11)?,
        stale: false,
    })
}

fn evidence_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceEntry> {
    let id: i64 = row.get(0)?;
    Ok(EvidenceEntry {
        id: format!("item:{id}"),
        record_type: "item".to_string(),
        run_id: row.get(1)?,
        item_id: Some(id),
        workspace_id: None,
        project_id: None,
        project_path: row.get(2)?,
        prd_slug: None,
        command: None,
        status: row.get(7)?,
        summary: row.get(8)?,
        kind: row.get(3)?,
        title: row.get(4)?,
        relative_path: row.get(5)?,
        absolute_path: row.get(6)?,
        terminal_session_id: None,
        terminal_log_path: None,
        created_at: row.get(9)?,
        completed_at: None,
        stale: false,
    })
}

fn normalize_agent_session_scope(scope: &str) -> &str {
    match scope.trim() {
        "card_interview" => "card_interview",
        "project_blueprint" => "project_blueprint",
        _ => "chat",
    }
}

fn new_store_id(prefix: &str) -> String {
    format!(
        "{}-{}",
        prefix,
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_micros())
    )
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> anyhow::Result<Vec<T>> {
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn unique_project_ids(project_ids: &[i64]) -> Vec<i64> {
    let mut unique_ids = Vec::new();
    for project_id in project_ids
        .iter()
        .copied()
        .filter(|project_id| *project_id > 0)
    {
        if !unique_ids.contains(&project_id) {
            unique_ids.push(project_id);
        }
    }
    unique_ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db() -> (Database, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-gui-store-test-{unique}"));
        std::fs::create_dir_all(&root).expect("create db dir");
        (Database::open(&root).expect("open db"), root)
    }

    #[test]
    fn app_state_round_trips_and_overwrites() {
        let (db, _root) = temp_db();

        assert_eq!(db.get_app_state("last_workspace_id").expect("get"), None);

        db.set_app_state("last_workspace_id", "7").expect("set");
        assert_eq!(
            db.get_app_state("last_workspace_id").expect("get"),
            Some("7".to_string())
        );

        db.set_app_state("last_workspace_id", "42")
            .expect("overwrite");
        assert_eq!(
            db.get_app_state("last_workspace_id").expect("get"),
            Some("42".to_string())
        );
    }

    #[test]
    fn workspace_machines_are_workspace_scoped_and_secret_free() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let machine = db
            .create_workspace_machine(WorkspaceMachineCreate {
                workspace_id: workspace.id,
                project_id: None,
                provider: "winbox",
                provider_runtime: "native",
                provider_profile: "dw-1-dev",
                display_name: "Dev VM",
                preset_id: "ubuntu_server_lts",
                image_family: "linux_distro",
                access_user: Some("bruno"),
                status: "creating",
            })
            .expect("create machine");

        let updated = db
            .update_workspace_machine(WorkspaceMachineUpdate {
                id: &machine.id,
                status: "running",
                web_port: Some(8006),
                rdp_port: None,
                ssh_port: Some(2222),
                last_health_status: Some("healthy"),
                last_health_summary: Some("Docker ready"),
                last_error_code: None,
                last_error_message: None,
            })
            .expect("update machine");
        let rows = db
            .list_workspace_machines(workspace.id)
            .expect("list machines");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, machine.id);
        assert_eq!(updated.status, "running");
        assert_eq!(updated.web_port, Some(8006));
        assert_eq!(updated.access_user.as_deref(), Some("bruno"));
        assert_eq!(updated.last_health_summary.as_deref(), Some("Docker ready"));

        let conn = Connection::open(workspace_db_path(&workspace.root_path)).expect("workspace db");
        let password_columns: i64 = conn
            .query_row(
                "select count(*)
                 from pragma_table_info('workspace_machines')
                 where lower(name) like '%pass%' or lower(name) like '%secret%' or lower(name) like '%token%'",
                [],
                |row| row.get(0),
            )
            .expect("secret columns");
        assert_eq!(password_columns, 0);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn reset_workspace_deploy_state_clears_workspace_db_and_artifacts_only() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let project_root = workspace_root.join("project");
        std::fs::create_dir_all(&project_root).expect("create project dir");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "Project",
                &project_root.display().to_string(),
                None,
            )
            .expect("create project");
        let machine = db
            .create_workspace_machine(WorkspaceMachineCreate {
                workspace_id: workspace.id,
                project_id: None,
                provider: "winbox",
                provider_runtime: "native",
                provider_profile: "dw-1-dev",
                display_name: "Dev VM",
                preset_id: "ubuntu_desktop_deploy_vm",
                image_family: "linux_cloud",
                access_user: Some("bruno"),
                status: "running",
            })
            .expect("create machine");
        let stack = db
            .create_deploy_stack(DeployStackCreate {
                workspace_id: workspace.id,
                name: "Project deploy",
                slug: "project-deploy",
            })
            .expect("create stack");
        let artifact_root = workspace_root
            .join(".dw")
            .join("deploy-packages")
            .join(workspace.id.to_string())
            .join("project-deploy")
            .join("deploy-001");
        let plan_root = workspace_root
            .join(".dw")
            .join("deploy-plans")
            .join("plan-test")
            .join("analysis");
        std::fs::create_dir_all(&artifact_root).expect("create artifact dir");
        std::fs::create_dir_all(&plan_root).expect("create plan dir");
        let version = db
            .create_deploy_version(DeployVersionCreate {
                stack_id: &stack.id,
                workspace_id: workspace.id,
                label: "deploy-001",
                target_machine_id: Some(&machine.id),
                artifact_path: &artifact_root.display().to_string(),
                manifest_path: &artifact_root.join("manifest.json").display().to_string(),
                manifest_json: "{}",
                blocking_findings_json: "[]",
            })
            .expect("create version");
        db.add_deploy_version_project(DeployVersionProjectCreate {
            version_id: &version.id,
            project_id: project.id,
            name: &project.name,
            path: &project.path,
            branch: None,
            commit_sha: None,
            dirty: false,
            package_path: "projects/project",
        })
        .expect("link project");
        let run = db
            .create_deploy_run(
                &stack.id,
                Some(&version.id),
                Some(&machine.id),
                "prepare",
                None,
            )
            .expect("create run");
        db.add_deploy_run_step(&run.id, "preflight", "passed", "ok", None, None)
            .expect("create step");
        let conn = Connection::open(workspace_db_path(&workspace.root_path)).expect("workspace db");
        conn.execute(
            "insert into deploy_target_bootstrap (
               machine_id, workspace_id, target_os, status, ssh_public_key_path,
               last_preflight_json, updated_at
             ) values (?1, ?2, 'linux', 'ready', null, '{}', ?3)",
            params![machine.id, workspace.id, Utc::now().to_rfc3339()],
        )
        .expect("insert bootstrap");
        drop(conn);

        let reset = db
            .reset_workspace_deploy_state(workspace.id)
            .expect("reset deploy state");

        assert_eq!(reset.workspace_id, workspace.id);
        assert_eq!(reset.workspace_machines, 1);
        assert_eq!(reset.deploy_stacks, 1);
        assert_eq!(reset.deploy_versions, 1);
        assert_eq!(reset.deploy_runs, 1);
        assert_eq!(reset.deploy_run_steps, 1);
        assert_eq!(reset.deploy_version_projects, 1);
        assert_eq!(reset.deploy_target_bootstrap, 1);
        assert_eq!(
            db.list_workspace_machines(workspace.id)
                .expect("list machines after reset")
                .len(),
            0
        );
        assert_eq!(
            db.list_deploy_stacks(workspace.id)
                .expect("list stacks after reset")
                .len(),
            0
        );
        assert_eq!(
            db.list_projects(workspace.id)
                .expect("projects survive reset")
                .len(),
            1
        );
        assert!(!workspace_root.join(".dw").join("deploy-packages").exists());
        assert!(!workspace_root.join(".dw").join("deploy-plans").exists());
        assert!(workspace_db_path(&workspace.root_path).exists());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn workspace_machine_provider_profile_is_unique_per_workspace() {
        let (db, root) = temp_db();
        let first_root = root.join("workspace-a");
        let second_root = root.join("workspace-b");
        let first = db
            .create_workspace("First", &first_root.display().to_string())
            .expect("create first workspace");
        let second = db
            .create_workspace("Second", &second_root.display().to_string())
            .expect("create second workspace");

        let create = |workspace_id| WorkspaceMachineCreate {
            workspace_id,
            project_id: None,
            provider: "winbox",
            provider_runtime: "native",
            provider_profile: "shared-profile",
            display_name: "Dev VM",
            preset_id: "windows_11",
            image_family: "windows",
            access_user: Some("bruno"),
            status: "stopped",
        };
        db.create_workspace_machine(create(first.id))
            .expect("create first machine");
        assert!(db.create_workspace_machine(create(first.id)).is_err());
        db.create_workspace_machine(create(second.id))
            .expect("same profile in different workspace is registry-safe");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn deploy_version_reader_accepts_legacy_blob_json() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let stack = db
            .create_deploy_stack(DeployStackCreate {
                workspace_id: workspace.id,
                name: "Smoke",
                slug: "smoke",
            })
            .expect("create stack");
        let version = db
            .create_deploy_version(DeployVersionCreate {
                stack_id: &stack.id,
                workspace_id: workspace.id,
                label: "deploy-001",
                target_machine_id: None,
                artifact_path: "/tmp/package",
                manifest_path: "/tmp/package/manifest.json",
                manifest_json: "{}",
                blocking_findings_json: "[]",
            })
            .expect("create version");
        let conn = Connection::open(workspace_db_path(&workspace.root_path)).expect("workspace db");
        conn.execute(
            "update deploy_versions
             set manifest_json = ?1,
                 blocking_findings_json = ?2
             where id = ?3",
            params![b"{\"ok\":true}".as_slice(), b"[]".as_slice(), version.id],
        )
        .expect("store blob json");

        let reloaded = db.get_deploy_version(&version.id).expect("reload version");

        assert_eq!(reloaded.manifest_json, "{\"ok\":true}");
        assert_eq!(reloaded.blocking_findings_json, "[]");
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn deploy_run_records_agent_orchestration_metadata() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let profile = db
            .create_agent_profile(AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: None,
                name: "Codex Deploy",
                provider: "codex",
                model: Some("gpt-5.5"),
                reasoning_effort: None,
                sandbox: "danger-full-access",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create profile");
        let stack = db
            .create_deploy_stack(DeployStackCreate {
                workspace_id: workspace.id,
                name: "Stack",
                slug: "stack",
            })
            .expect("create stack");

        let run = db
            .create_deploy_run(&stack.id, None, None, "prepare", Some(&profile))
            .expect("create run");
        let updated = db
            .update_deploy_run_orchestration(&run.id, "passed", r#"{"ok":true}"#)
            .expect("update orchestration");

        assert_eq!(updated.agent_profile_id, Some(profile.id));
        assert_eq!(updated.agent_name.as_deref(), Some("Codex Deploy"));
        assert_eq!(updated.agent_provider.as_deref(), Some("codex"));
        assert_eq!(updated.agent_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(updated.orchestration_status, "passed");
        assert_eq!(updated.orchestration_report_json, r#"{"ok":true}"#);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn evidence_runs_can_be_created_completed_and_listed() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(workspace.id, "App", "/repo/app", None)
            .expect("create project");
        let run = db
            .create_evidence_run(
                Some(workspace.id),
                Some(project.id),
                "/repo/app",
                Some("prd-demo"),
                "/dw-plan prd-demo",
                "submitted",
                "sent to terminal",
                Some("terminal-1"),
                Some("/tmp/terminal.log"),
            )
            .expect("create run");

        let run_id = run.run_id.expect("run id");
        db.complete_evidence_run(run_id, "passed", "verified", Some(workspace.id))
            .expect("complete run");
        let entries = db.list_evidence("/repo/app").expect("list evidence");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, "passed");
        assert_eq!(entries[0].summary, "verified");
        assert_eq!(entries[0].prd_slug.as_deref(), Some("prd-demo"));
        assert_eq!(entries[0].workspace_id, Some(workspace.id));
        assert_eq!(entries[0].project_id, Some(project.id));

        let registry_conn = db.connect().expect("connect registry");
        let registry_runs: i64 = registry_conn
            .query_row("select count(*) from evidence_runs", [], |row| row.get(0))
            .expect("count registry runs");
        assert_eq!(registry_runs, 0);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn requirement_cards_are_workspace_scoped_and_status_trackable() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(workspace.id, "App", "/repo/app", None)
            .expect("create project");
        let api_project = db
            .add_project(workspace.id, "API", "/repo/api", None)
            .expect("create second project");

        let card = db
            .create_requirement_card(
                workspace.id,
                Some(project.id),
                &[project.id, api_project.id],
                "Add local PR packaging",
                "add-local-pr-packaging",
                "Prepare branch docs and evidence for manual push.",
            )
            .expect("create card");
        assert_eq!(card.status, "draft");
        assert_eq!(card.public_id, "APP-001");

        let updated = db
            .update_requirement_card_status(card.id, "planned", Some("prd-local-pr-packaging"))
            .expect("update card");
        assert_eq!(updated.status, "planned");
        assert_eq!(updated.prd_slug.as_deref(), Some("prd-local-pr-packaging"));
        let updated_body = db
            .update_requirement_card_body(card.id, "Updated card description.")
            .expect("update card body");
        assert_eq!(updated_body.body, "Updated card description.");
        db.set_requirement_card_flow(card.id, Some("spec-kit"))
            .expect("set card flow");
        assert_eq!(
            db.count_active_requirement_cards_for_flow(workspace.id, "spec-kit")
                .expect("count active cards"),
            1
        );
        let archived = db.archive_requirement_card(card.id).expect("archive card");
        assert_eq!(archived.status, "archived");
        assert_eq!(archived.archived_from_status.as_deref(), Some("planned"));
        assert!(archived.archived_at.is_some());
        assert!(db.archive_requirement_card(card.id).is_err());
        assert_eq!(
            db.count_active_requirement_cards_for_flow(workspace.id, "spec-kit")
                .expect("count archived cards"),
            0
        );
        let restored = db
            .restore_requirement_card(card.id, "qa")
            .expect("restore card");
        assert_eq!(restored.status, "qa");
        assert!(restored.archived_from_status.is_none());
        assert!(restored.archived_at.is_none());
        assert_eq!(
            db.count_active_requirement_cards_for_flow(workspace.id, "spec-kit")
                .expect("count restored cards"),
            1
        );

        let cards = db.list_requirement_cards(workspace.id).expect("list cards");
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].body, "Updated card description.");
        assert_eq!(cards[0].project_id, Some(project.id));
        assert_eq!(cards[0].project_ids, vec![project.id, api_project.id]);
        assert_eq!(cards[0].public_id, "APP-001");
        assert!(workspace_db_path(&workspace.root_path).exists());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn existing_workspace_database_is_adopted_when_registry_id_changes() {
        let (first_db, first_root) = temp_db();
        let workspace_root = first_root.join("workspace");
        let workspace = first_db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create original workspace");
        let project = first_db
            .add_project(workspace.id, "App", "/repo/app", None)
            .expect("create original project");
        first_db
            .create_requirement_card(
                workspace.id,
                Some(project.id),
                &[project.id],
                "Keep existing cards",
                "keep-existing-cards",
                "Card should survive registry recreation.",
            )
            .expect("create original card");

        let (second_db, second_root) = temp_db();
        let other_root = second_root.join("other-workspace");
        second_db
            .create_workspace("Other", &other_root.display().to_string())
            .expect("create unrelated workspace");
        let imported = second_db
            .create_workspace("Imported", &workspace_root.display().to_string())
            .expect("register existing workspace");

        let cards = second_db
            .list_requirement_cards(imported.id)
            .expect("list adopted cards");
        let projects = second_db
            .list_projects(imported.id)
            .expect("list adopted projects");

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].workspace_id, imported.id);
        assert_eq!(cards[0].public_id, "APP-001");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].workspace_id, imported.id);

        std::fs::remove_dir_all(first_root).expect("cleanup first");
        std::fs::remove_dir_all(second_root).expect("cleanup second");
    }

    #[test]
    fn existing_workspace_database_is_adopted_across_wsl_and_windows_paths() {
        let conn = Connection::open_in_memory().expect("connect memory");
        migrate_connection(&conn).expect("migrate");
        let old_root = "/mnt/e/winbox-wks";
        let new_root = r"E:\winbox-wks";
        conn.execute(
            "insert into workspaces (id, name, root_path, created_at)
             values (10, 'Winbox', ?1, '2026-05-28T00:00:00Z')",
            [old_root],
        )
        .expect("insert old workspace");
        conn.execute(
            "insert into workspaces (id, name, root_path, created_at)
             values (77, 'Winbox', ?1, '2026-05-29T00:00:00Z')",
            [new_root],
        )
        .expect("insert empty current workspace from prior Windows run");
        conn.execute(
            "insert into projects (id, workspace_id, name, path, remote_url, created_at)
             values (20, 10, 'winbox-gui', ?1, null, '2026-05-28T00:00:00Z')",
            [format!("{old_root}/projects/winbox-gui")],
        )
        .expect("insert project");
        conn.execute(
            "insert into requirement_cards
             (id, workspace_id, project_id, public_id, title, slug, body, status, created_at, updated_at)
             values (30, 10, 20, 'WB-001', 'Keep cards', 'keep-cards', '', 'draft',
                     '2026-05-28T00:00:00Z', '2026-05-28T00:00:00Z')",
            [],
        )
        .expect("insert card");
        conn.execute(
            "insert into agent_profiles
             (id, workspace_id, project_id, name, provider, model, sandbox, created_at, updated_at)
             values (40, 10, 20, 'Codex', 'codex', 'gpt-5', 'workspace-write',
                     '2026-05-28T00:00:00Z', '2026-05-28T00:00:00Z')",
            [],
        )
        .expect("insert profile");
        conn.execute(
            "insert into agent_sessions
             (id, profile_id, workspace_id, project_id, scope, project_path, provider, model, sandbox,
              status, title, created_at, updated_at)
             values (50, 40, 10, 20, 'chat', ?1, 'codex', 'gpt-5', 'workspace-write',
                     'done', 'Session', '2026-05-28T00:00:00Z', '2026-05-28T00:00:00Z')",
            [format!("{old_root}/projects/winbox-gui")],
        )
        .expect("insert session");

        let workspace = Workspace {
            id: 77,
            name: "Winbox".to_string(),
            root_path: new_root.to_string(),
            created_at: "2026-05-29T00:00:00Z".to_string(),
        };
        reconcile_workspace_identity(&conn, &workspace).expect("reconcile");

        let card_workspace_id: i64 = conn
            .query_row(
                "select workspace_id from requirement_cards where id = 30",
                [],
                |row| row.get(0),
            )
            .expect("card workspace");
        let profile_workspace_id: i64 = conn
            .query_row(
                "select workspace_id from agent_profiles where id = 40",
                [],
                |row| row.get(0),
            )
            .expect("profile workspace");
        let session_workspace_id: i64 = conn
            .query_row(
                "select workspace_id from agent_sessions where id = 50",
                [],
                |row| row.get(0),
            )
            .expect("session workspace");
        let project_path: String = conn
            .query_row("select path from projects where id = 20", [], |row| {
                row.get(0)
            })
            .expect("project path");
        let session_path: String = conn
            .query_row(
                "select project_path from agent_sessions where id = 50",
                [],
                |row| row.get(0),
            )
            .expect("session path");
        let workspace_root: String = conn
            .query_row(
                "select root_path from workspaces where id = 77",
                [],
                |row| row.get(0),
            )
            .expect("workspace root");

        assert_eq!(card_workspace_id, 77);
        assert_eq!(profile_workspace_id, 77);
        assert_eq!(session_workspace_id, 77);
        assert_eq!(project_path, r"E:\winbox-wks\projects\winbox-gui");
        assert_eq!(session_path, r"E:\winbox-wks\projects\winbox-gui");
        assert_eq!(workspace_root, new_root);
        assert_eq!(
            conn.query_row("select count(*) from workspaces where id = 10", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("old workspace count"),
            0
        );
    }

    #[test]
    fn workspace_path_equivalence_maps_wsl_mounts_to_windows_drives() {
        assert!(paths_are_workspace_equivalent(
            "/mnt/e/winbox-wks",
            r"E:\winbox-wks"
        ));
        assert!(paths_are_workspace_equivalent(
            r"\\?\E:\winbox-wks",
            r"E:\winbox-wks"
        ));
        assert_eq!(
            normalize_filesystem_path(r"\\?\E:\winbox-wks\projects\winbox-gui"),
            r"E:\winbox-wks\projects\winbox-gui"
        );
        assert_eq!(
            rewrite_path_root(
                "/mnt/e/winbox-wks/projects/winbox-gui",
                "/mnt/e/winbox-wks",
                r"E:\winbox-wks",
            ),
            Some(r"E:\winbox-wks\projects\winbox-gui".to_string())
        );
    }

    #[test]
    fn duplicate_windows_extended_project_paths_are_merged() {
        let conn = Connection::open_in_memory().expect("connect memory");
        migrate_connection(&conn).expect("migrate");
        conn.execute(
            "insert into workspaces (id, name, root_path, created_at)
             values (1, 'Winbox', 'E:\\winbox-wks', '2026-05-29T00:00:00Z')",
            [],
        )
        .expect("insert workspace");
        conn.execute(
            "insert into projects (id, workspace_id, name, path, remote_url, created_at)
             values (1, 1, 'winbox-gui', 'E:\\winbox-wks\\projects\\winbox-gui',
                     'git@example.com:winbox-gui.git', '2026-05-28T00:00:00Z')",
            [],
        )
        .expect("insert project");
        conn.execute(
            "insert into projects (id, workspace_id, name, path, remote_url, created_at)
             values (2, 1, 'winbox-gui', '\\\\?\\E:\\winbox-wks\\projects\\winbox-gui',
                     'git@example.com:winbox-gui.git', '2026-05-29T00:00:00Z')",
            [],
        )
        .expect("insert duplicate project");
        conn.execute(
            "insert into project_card_sequences (project_id, prefix, next_number, updated_at)
             values (1, 'WG', 7, '2026-05-28T00:00:00Z')",
            [],
        )
        .expect("insert sequence");
        conn.execute(
            "insert into project_card_sequences (project_id, prefix, next_number, updated_at)
             values (2, 'WG', 1, '2026-05-29T00:00:00Z')",
            [],
        )
        .expect("insert duplicate sequence");
        conn.execute(
            "insert into requirement_cards
             (id, workspace_id, project_id, public_id, title, slug, body, status, created_at, updated_at)
             values (1, 1, 1, 'WG-006', 'Existing', 'existing', '', 'draft',
                     '2026-05-28T00:00:00Z', '2026-05-28T00:00:00Z')",
            [],
        )
        .expect("insert card");

        dedupe_workspace_projects(&conn, 1).expect("dedupe");

        let project_count: i64 = conn
            .query_row("select count(*) from projects", [], |row| row.get(0))
            .expect("project count");
        let project_path: String = conn
            .query_row("select path from projects where id = 1", [], |row| {
                row.get(0)
            })
            .expect("project path");
        let next_number: i64 = conn
            .query_row(
                "select next_number from project_card_sequences where project_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("next number");

        assert_eq!(project_count, 1);
        assert_eq!(project_path, r"E:\winbox-wks\projects\winbox-gui");
        assert_eq!(next_number, 7);
    }

    #[test]
    fn requirement_public_ids_are_sequenced_by_project_prefix() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let app = db
            .add_project(workspace.id, "Dev Workflow", "/repo/app", None)
            .expect("create app project");
        let api = db
            .add_project(workspace.id, "API", "/repo/api", None)
            .expect("create api project");

        let conn = db
            .workspace_connect_for_project(app.id)
            .expect("open project db");
        conn.execute(
            "insert into project_card_sequences (project_id, prefix, next_number, updated_at)
             values (?1, ?2, ?3, ?4)",
            params![app.id, "DW", 10, Utc::now().to_rfc3339()],
        )
        .expect("configure app sequence");
        let first = db
            .create_requirement_card(workspace.id, Some(app.id), &[app.id], "First", "first", "")
            .expect("create first card");
        let second = db
            .create_requirement_card(
                workspace.id,
                Some(app.id),
                &[app.id],
                "Second",
                "second",
                "",
            )
            .expect("create second card");
        let third = db
            .create_requirement_card(workspace.id, Some(api.id), &[api.id], "Third", "third", "")
            .expect("create third card");

        assert_eq!(first.public_id, "DW-010");
        assert_eq!(second.public_id, "DW-011");
        assert_eq!(third.public_id, "API-001");
        let next_number: i64 = conn
            .query_row(
                "select next_number from project_card_sequences where project_id = ?1",
                [app.id],
                |row| row.get(0),
            )
            .expect("next number");
        assert_eq!(next_number, 12);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn workspace_projects_are_imported_from_existing_git_folders() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project_root = workspace_root.join("repos").join("dev-workflow");
        std::fs::create_dir_all(project_root.join(".git")).expect("create git folder");

        let projects = db.list_projects(workspace.id).expect("list projects");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "dev-workflow");
        assert_eq!(
            projects[0].path,
            normalize_project_path(&project_root.display().to_string())
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn adding_the_same_project_path_returns_existing_project() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let project_root = root.join("project");
        std::fs::create_dir_all(&project_root).expect("create project root");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");

        let first = db
            .add_project(
                workspace.id,
                "Project",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let second = db
            .add_project(
                workspace.id,
                "Project again",
                &project_root.display().to_string(),
                Some("git@example.com:project.git"),
            )
            .expect("add duplicate project");

        assert_eq!(first.id, second.id);
        assert_eq!(first.path, second.path);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn requirement_stage_forms_are_upserted_by_card_and_stage() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let card = db
            .create_requirement_card(workspace.id, None, &[], "Run QA", "run-qa", "")
            .expect("create card");

        db.upsert_requirement_stage_form(card.id, "qa", r#"{"mode":"uat"}"#)
            .expect("insert form");
        db.upsert_requirement_stage_form(card.id, "qa", r#"{"mode":"fix"}"#)
            .expect("update form");

        let forms = db
            .list_requirement_stage_forms(card.id)
            .expect("list forms");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].stage_id, "qa");
        assert_eq!(forms[0].payload_json, r#"{"mode":"fix"}"#);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn requirement_attachments_can_be_added_listed_and_removed() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let card = db
            .create_requirement_card(workspace.id, None, &[], "Attach files", "attach-files", "")
            .expect("create card");
        let source = root.join("brief.pdf");
        std::fs::write(&source, b"brief").expect("write source attachment");

        let attachment = db
            .add_requirement_attachment(card.id, "brief.pdf", &source.display().to_string())
            .expect("add attachment");
        let attachments = db
            .list_requirement_attachments(card.id)
            .expect("list attachments");
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].name, "brief.pdf");
        assert_ne!(attachments[0].file_path, source.display().to_string());
        assert!(Path::new(&attachments[0].file_path).exists());

        db.remove_requirement_attachment(attachment.id)
            .expect("remove attachment");
        assert!(!Path::new(&attachment.file_path).exists());
        assert!(db
            .list_requirement_attachments(card.id)
            .expect("list after remove")
            .is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn knowledge_sources_are_copied_listed_and_removed() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let source_path = root.join("brief.md");
        std::fs::write(&source_path, "context").expect("write source");

        let source = db
            .add_knowledge_source(
                workspace.id,
                None,
                None,
                "",
                &source_path.display().to_string(),
            )
            .expect("add knowledge source");
        assert_eq!(source.scope, "workspace");
        assert_eq!(source.name, "brief.md");
        assert_ne!(source.file_path, source_path.display().to_string());
        assert!(Path::new(&source.file_path).exists());

        let sources = db
            .list_knowledge_sources(workspace.id, None)
            .expect("list knowledge sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, source.id);

        db.remove_knowledge_source(source.id)
            .expect("remove knowledge source");
        assert!(!Path::new(&source.file_path).exists());
        assert!(db
            .list_knowledge_sources(workspace.id, None)
            .expect("list after remove")
            .is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_blueprints_materialize_specs_project_and_cards() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let source_path = root.join("brief.md");
        std::fs::write(&source_path, "user research").expect("write source");
        let source = db
            .add_knowledge_source(
                workspace.id,
                None,
                None,
                "Research brief",
                &source_path.display().to_string(),
            )
            .expect("add knowledge source");
        let source_ids = format!("[{}]", source.id);
        let blueprint = db
            .create_project_blueprint(
                workspace.id,
                "Client Portal",
                "Build a client portal",
                None,
                &source_ids,
            )
            .expect("create blueprint");
        let planned = db
            .update_project_blueprint(ProjectBlueprintUpdate {
                id: &blueprint.id,
                status: Some("planned"),
                agent_session_id: None,
                knowledge_source_ids_json: Some(&source_ids),
                answers_json: Some(
                    r#"[{"id":"business-01","area":"Negócio","question":"Problema?","answer":"Portal manual hoje."}]"#,
                ),
                running_summary: Some("Client portal plan"),
                detected_subprojects_json: Some(r#"["frontend","backend"]"#),
                prd: Some("# PRD\nClient portal"),
                techspec: Some("# TechSpec\nReact and API"),
                tasks_json: Some(
                    r#"[
                      {"title":"Create shell","body":"Create project shell"},
                      {"title":"Implement login","body":"Implement auth","dependencies":["Create shell"]}
                    ]"#,
                ),
                definition_of_done: Some("# Definition of Done\nTests pass"),
            })
            .expect("plan blueprint");

        let materialized = db
            .materialize_project_blueprint(&planned.id)
            .expect("materialize blueprint");
        assert_eq!(materialized.blueprint.status, "materialized");
        assert_eq!(materialized.project.name, "Client Portal");
        assert_eq!(materialized.cards.len(), 2);
        assert!(Path::new(&materialized.project.path).exists());

        let spec_dir = Path::new(&materialized.spec_dir);
        assert!(spec_dir.join("prd.md").exists());
        assert!(spec_dir.join("techspec.md").exists());
        assert!(spec_dir.join("tasks.md").exists());
        assert!(spec_dir.join("definition-of-done.md").exists());
        assert!(spec_dir.join("interview.md").exists());
        let manifest =
            std::fs::read_to_string(spec_dir.join("knowledge-manifest.json")).expect("manifest");
        assert!(manifest.contains("Research brief"));

        let cards = db.list_requirement_cards(workspace.id).expect("list cards");
        assert_eq!(cards.len(), 2);
        assert!(cards
            .iter()
            .all(|card| card.project_ids.contains(&materialized.project.id)));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_evidence_items_are_upserted_by_relative_path() {
        let (db, root) = temp_db();
        db.upsert_indexed_evidence_item(
            "/repo/app",
            "qa-report",
            "QA report",
            "spec/prd-demo/QA/qa-report.md",
            "/repo/app/.dw/spec/prd-demo/QA/qa-report.md",
            "indexed QA report",
        )
        .expect("index item");
        db.upsert_indexed_evidence_item(
            "/repo/app",
            "qa-report",
            "QA report updated",
            "spec/prd-demo/QA/qa-report.md",
            "/repo/app/.dw/spec/prd-demo/QA/qa-report.md",
            "updated",
        )
        .expect("upsert item");

        let entries = db.list_evidence("/repo/app").expect("list evidence");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "QA report updated");
        assert_eq!(entries[0].summary, "updated");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn manual_evidence_can_link_an_indexed_artifact() {
        let (db, root) = temp_db();
        db.upsert_indexed_evidence_item(
            "/repo/app",
            "qa-report",
            "QA report",
            "spec/prd-demo/QA/qa-report.md",
            "/repo/app/.dw/spec/prd-demo/QA/qa-report.md",
            "indexed",
        )
        .expect("index item");
        db.create_evidence_item(
            "/repo/app",
            "qa-report",
            "Manual QA note",
            Some("spec/prd-demo/QA/qa-report.md"),
            Some("/repo/app/.dw/spec/prd-demo/QA/qa-report.md"),
            "passed",
            "reviewed manually",
        )
        .expect("manual item");

        let entries = db.list_evidence("/repo/app").expect("list evidence");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| entry.status == "indexed"));
        assert!(entries.iter().any(|entry| entry.status == "passed"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn agent_profiles_sessions_and_messages_are_workspace_scoped() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let project_root = root.join("project");
        std::fs::create_dir_all(&project_root).expect("create project root");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "Project",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");

        let profile = db
            .create_agent_profile(AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: Some(project.id),
                name: "Codex",
                provider: "codex",
                model: Some("gpt-5.4"),
                reasoning_effort: Some("medium"),
                sandbox: "read-only",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create profile");
        let session = db
            .create_agent_session(&profile, Some(project.id), &project.path, "Brainstorm")
            .expect("create session");
        db.add_agent_message(session.id, "user", "/dw-brainstorm Demo", None)
            .expect("add user message");
        db.add_agent_message(
            session.id,
            "assistant",
            "Done",
            Some(r#"{"type":"message","content":"Done"}"#),
        )
        .expect("add assistant message");
        let updated = db
            .update_agent_session_status(session.id, "running", Some("codex-session-1"))
            .expect("update session");
        let edited_profile = db
            .update_agent_profile(AgentProfileUpdate {
                id: profile.id,
                name: "Codex updated",
                provider: "codex",
                model: Some("gpt-5.5"),
                reasoning_effort: Some("high"),
                sandbox: "workspace-write",
                context_mode: "full",
                rtk_enabled: true,
            })
            .expect("update profile");

        assert_eq!(
            updated.provider_session_id.as_deref(),
            Some("codex-session-1")
        );
        assert_eq!(updated.codex_session_id.as_deref(), Some("codex-session-1"));
        assert_eq!(edited_profile.name, "Codex updated");
        assert_eq!(edited_profile.model.as_deref(), Some("gpt-5.5"));
        assert_eq!(edited_profile.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(edited_profile.sandbox, "workspace-write");
        assert_eq!(edited_profile.context_mode, "full");
        assert_eq!(
            db.list_agent_profiles(workspace.id, Some(project.id))
                .expect("list profiles")
                .len(),
            1
        );
        assert_eq!(
            db.list_agent_sessions(workspace.id, Some(project.id))
                .expect("list sessions")
                .len(),
            1
        );
        let messages = db
            .list_agent_messages(session.id)
            .expect("list agent messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].content, "Done");
        let reset_after_running = db
            .reset_agent_chat(profile.id, workspace.id, Some(project.id), &project.path)
            .expect("reset running chat");
        assert_eq!(reset_after_running.title, "Nova conversa");
        assert_eq!(
            db.list_agent_sessions(workspace.id, Some(project.id))
                .expect("list sessions after running reset")
                .len(),
            1
        );
        db.update_agent_session_status(reset_after_running.id, "done", None)
            .expect("complete reset session");
        let fresh_session = db
            .reset_agent_chat(profile.id, workspace.id, Some(project.id), &project.path)
            .expect("reset chat");
        assert_eq!(fresh_session.title, "Nova conversa");
        assert_eq!(
            db.list_agent_sessions(workspace.id, Some(project.id))
                .expect("list reset sessions")
                .len(),
            1
        );
        assert!(db
            .list_agent_messages(fresh_session.id)
            .expect("list reset messages")
            .is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_blueprint_agent_sessions_are_not_regular_chat_sessions() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let project_root = root.join("project");
        std::fs::create_dir_all(&project_root).expect("create project root");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let project = db
            .add_project(
                workspace.id,
                "Project",
                &project_root.display().to_string(),
                None,
            )
            .expect("add project");
        let profile = db
            .create_agent_profile(AgentProfileCreate {
                workspace_id: workspace.id,
                project_id: Some(project.id),
                name: "Codex",
                provider: "codex",
                model: Some("gpt-5.4"),
                reasoning_effort: Some("medium"),
                sandbox: "read-only",
                context_mode: "auto_lean",
                rtk_enabled: false,
            })
            .expect("create profile");

        let session = db
            .create_agent_session_scoped(
                &profile,
                Some(project.id),
                &project.path,
                "Novo projeto",
                "project_blueprint",
                None,
            )
            .expect("create scoped session");
        assert_eq!(session.scope, "project_blueprint");
        assert!(db
            .list_agent_sessions(workspace.id, Some(project.id))
            .expect("list chat sessions")
            .is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn agent_session_context_must_match_profile_workspace_project_and_path() {
        let profile = AgentProfile {
            id: 10,
            workspace_id: 1,
            project_id: Some(100),
            name: "Codex".to_string(),
            provider: "codex".to_string(),
            model: Some("gpt-5.4-mini".to_string()),
            reasoning_effort: Some("medium".to_string()),
            sandbox: "read-only".to_string(),
            context_mode: "auto_lean".to_string(),
            rtk_enabled: false,
            created_at: "2026-05-23T00:00:00Z".to_string(),
            updated_at: "2026-05-23T00:00:00Z".to_string(),
        };
        let session = AgentSession {
            id: 20,
            profile_id: 10,
            workspace_id: 1,
            project_id: Some(100),
            requirement_card_id: None,
            scope: "chat".to_string(),
            project_path: "/repo/app".to_string(),
            provider: "codex".to_string(),
            model: Some("gpt-5.4-mini".to_string()),
            reasoning_effort: Some("medium".to_string()),
            sandbox: "read-only".to_string(),
            context_mode: "auto_lean".to_string(),
            provider_session_id: None,
            codex_session_id: None,
            status: "idle".to_string(),
            title: "Nova conversa".to_string(),
            created_at: "2026-05-23T00:00:00Z".to_string(),
            updated_at: "2026-05-23T00:00:00Z".to_string(),
        };

        assert!(agent_session_matches_profile_context(
            &session,
            &profile,
            1,
            Some(100),
            "/repo/app"
        ));
        assert!(!agent_session_matches_profile_context(
            &AgentSession {
                profile_id: 11,
                ..session.clone()
            },
            &profile,
            1,
            Some(100),
            "/repo/app"
        ));
        assert!(!agent_session_matches_profile_context(
            &session,
            &profile,
            1,
            Some(200),
            "/repo/app"
        ));
        assert!(!agent_session_matches_profile_context(
            &session,
            &profile,
            1,
            Some(100),
            "/repo/other"
        ));
    }
}
