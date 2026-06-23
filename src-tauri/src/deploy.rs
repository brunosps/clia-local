use crate::deploy_detect::{self, DeployDetectionReport};
use crate::deploy_env;
use crate::deploy_executor;
use crate::deploy_package::{self, CreateDeployPackageInput};
use crate::deploy_plan::{self, DeployPlanReport, PlanDeployPackageInput};
use crate::store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct DetectDeployStackInput {
    pub workspace_id: i64,
    pub project_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadDeployArtifactInput {
    pub version_id: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApproveDeployVersionInput {
    pub version_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrepareDeployTargetInput {
    pub version_id: String,
    pub machine_id: String,
    pub agent_profile_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeployVersionInput {
    pub version_id: String,
    pub machine_id: String,
    pub agent_profile_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopDeployStackInput {
    pub stack_id: String,
    pub machine_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReactivateDeployVersionInput {
    pub version_id: String,
    pub machine_id: String,
    pub agent_profile_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeployRunLogsInput {
    pub run_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDeployRepairVersionInput {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployStackDetail {
    pub stack: store::DeployStack,
    pub versions: Vec<store::DeployVersion>,
}

pub fn list_stacks(
    db: &store::Database,
    workspace_id: i64,
) -> anyhow::Result<Vec<store::DeployStack>> {
    db.list_deploy_stacks(workspace_id)
}

pub fn get_stack(db: &store::Database, stack_id: &str) -> anyhow::Result<DeployStackDetail> {
    let stack = db.get_deploy_stack(stack_id)?;
    let versions = db.list_deploy_versions(stack_id)?;
    Ok(DeployStackDetail { stack, versions })
}

pub fn detect_stack(
    db: &store::Database,
    input: DetectDeployStackInput,
) -> anyhow::Result<DeployDetectionReport> {
    deploy_detect::detect_projects(db, input.workspace_id, &input.project_ids)
}

pub fn plan_package(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: PlanDeployPackageInput,
) -> anyhow::Result<DeployPlanReport> {
    deploy_plan::plan_package(app, db, input)
}

pub fn create_package(
    db: &store::Database,
    input: CreateDeployPackageInput,
) -> anyhow::Result<store::DeployVersion> {
    deploy_package::create_package(db, input)
}

pub fn read_artifact(
    db: &store::Database,
    input: ReadDeployArtifactInput,
) -> anyhow::Result<String> {
    let version = db.get_deploy_version(&input.version_id)?;
    deploy_package::read_artifact(&version, &input.relative_path)
}

pub fn approve_version(
    db: &store::Database,
    input: ApproveDeployVersionInput,
) -> anyhow::Result<store::DeployVersion> {
    let version = db.get_deploy_version(&input.version_id)?;
    if deploy_package::has_blocking_findings(&version) {
        anyhow::bail!("deploy version has blocking secret findings");
    }
    db.approve_deploy_version(&input.version_id)
}

pub fn get_environment(
    db: &store::Database,
    input: deploy_env::DeployEnvironmentInput,
) -> anyhow::Result<deploy_env::DeployEnvironment> {
    deploy_env::load_environment(db, input)
}

pub fn save_environment(
    db: &store::Database,
    input: deploy_env::SaveDeployEnvironmentInput,
) -> anyhow::Result<deploy_env::DeployEnvironment> {
    deploy_env::save_environment(db, input)
}

pub fn prepare_target(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: PrepareDeployTargetInput,
) -> anyhow::Result<store::DeployRun> {
    deploy_executor::prepare_target(
        app,
        db,
        &input.version_id,
        &input.machine_id,
        input.agent_profile_id,
    )
}

pub fn deploy_version(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: DeployVersionInput,
) -> anyhow::Result<store::DeployRun> {
    deploy_executor::deploy_version(
        app,
        db,
        &input.version_id,
        &input.machine_id,
        input.agent_profile_id,
    )
}

pub fn stop_stack(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: StopDeployStackInput,
) -> anyhow::Result<store::DeployRun> {
    deploy_executor::stop_stack(app, db, &input.stack_id, &input.machine_id)
}

pub fn reactivate_version(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: ReactivateDeployVersionInput,
) -> anyhow::Result<store::DeployRun> {
    deploy_executor::reactivate_version(
        app,
        db,
        &input.version_id,
        &input.machine_id,
        input.agent_profile_id,
    )
}

pub fn list_runs(db: &store::Database, version_id: &str) -> anyhow::Result<Vec<store::DeployRun>> {
    db.list_deploy_runs(version_id)
}

pub fn run_logs(db: &store::Database, input: DeployRunLogsInput) -> anyhow::Result<String> {
    let run = db.get_deploy_run(&input.run_id)?;
    let steps = db.list_deploy_run_steps(&input.run_id)?;
    let mut out = String::new();
    if let Some(agent) = run.agent_name.as_deref() {
        out.push_str(&format!(
            "[{}] agent: {} ({})\n",
            run.orchestration_status,
            agent,
            run.agent_provider.as_deref().unwrap_or("unknown")
        ));
    }
    for step in steps {
        out.push_str(&format!(
            "[{}] {}: {}\n",
            step.status, step.step_key, step.message
        ));
    }
    Ok(out)
}

pub fn create_repair_version(
    db: &store::Database,
    input: CreateDeployRepairVersionInput,
) -> anyhow::Result<store::DeployVersion> {
    deploy_package::create_repair_version_from_run(db, &input.run_id)
}

pub use deploy_package::CreateDeployPackageInput as CreatePackageInput;
