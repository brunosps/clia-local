import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  AttachmentPreview,
  AgentMessage,
  AgentProviderHealth,
  AgentProfile,
  AgentRunEvent,
  AgentSkillInvocation,
  RtkInstallResult,
  RtkSetupResult,
  RtkStatus,
  AgentUsage,
  BlameLine,
  AgentSession,
  Branch,
  ChangedFile,
  Commit,
  CommitDetail,
  DwCommand,
  DwArtifact,
  DwSkill,
  EvidenceEntry,
  FilePatch,
  GitRepoSnapshot,
  GitWorktreeFingerprint,
  GitWorktreeSnapshot,
  PatchArea,
  PatchCheckResult,
  RebaseStep,
  RemoteBranch,
  RepoState,
  Submodule,
  StashEntry,
  TagEntry,
  PreflightReport,
  Project,
  KnowledgeSource,
  ProjectBlueprint,
  ProjectBlueprintMaterialization,
  RequirementAttachment,
  RequirementCard,
  RequirementStageForm,
  LspServerStatus,
  SearchFileResult,
  SourceEntry,
  SourceFile,
  TerminalSession,
  WorkflowStateSummary,
  Workspace,
  WorkspaceCapabilities,
  AgentProvider,
  WorkspaceSkill,
  WorkspaceSkillSearchResult,
  WorkspaceSolutionImportReport,
  WorkspaceSolutionManifest,
  WorkspaceSkillTarget,
  MachinePreset,
  MachineProviderStatus,
  MachineSshProbe,
  MachineViewer,
  WorkspaceMachine,
  CreateWorkspaceMachineInput,
  CreateDeployPackageInput,
  DeployDetectionReport,
  DeployEnvironment,
  DeployEnvironmentValueInput,
  DeployPlanReport,
  DeployRun,
  DeployStack,
  DeployStackDetail,
  DeployVersion,
  WorkspaceDeployReset,
} from "./types";

function normalizeError(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

export async function invokeSafe<T>(command: string, args?: Record<string, unknown>) {
  try {
    return { ok: true as const, value: await invoke<T>(command, args) };
  } catch (error) {
    return { ok: false as const, error: normalizeError(error) };
  }
}

export const api = {
  async pickFile() {
    try {
      const selected = await open({ directory: false, multiple: false });
      if (Array.isArray(selected)) return { ok: true as const, value: selected[0] ?? null };
      return { ok: true as const, value: selected };
    } catch (error) {
      return { ok: false as const, error: normalizeError(error) };
    }
  },
  async pickDirectory() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (Array.isArray(selected)) return { ok: true as const, value: selected[0] ?? null };
      return { ok: true as const, value: selected };
    } catch (error) {
      return { ok: false as const, error: normalizeError(error) };
    }
  },
  async pickFiles() {
    try {
      const selected = await open({ directory: false, multiple: true });
      if (!selected) return { ok: true as const, value: [] };
      return { ok: true as const, value: Array.isArray(selected) ? selected : [selected] };
    } catch (error) {
      return { ok: false as const, error: normalizeError(error) };
    }
  },
  async pickSavePath(defaultPath?: string) {
    try {
      return { ok: true as const, value: await save({ defaultPath }) };
    } catch (error) {
      return { ok: false as const, error: normalizeError(error) };
    }
  },
  preflight(projectPath: string) {
    return invokeSafe<PreflightReport>("preflight", { projectPath });
  },
  listWorkspaces() {
    return invokeSafe<Workspace[]>("list_workspaces");
  },
  getAppState(key: string) {
    return invokeSafe<string | null>("get_app_state", { key });
  },
  setAppState(key: string, value: string) {
    return invokeSafe<void>("set_app_state", { key, value });
  },
  openExternalUrl(url: string) {
    return invokeSafe<void>("open_external_url", { url });
  },
  checkMachineProvider() {
    return invokeSafe<MachineProviderStatus>("check_machine_provider");
  },
  listMachinePresets() {
    return invokeSafe<MachinePreset[]>("list_machine_presets");
  },
  listWorkspaceMachines(workspaceId: number) {
    return invokeSafe<WorkspaceMachine[]>("list_workspace_machines", { workspaceId });
  },
  createWorkspaceMachine(input: CreateWorkspaceMachineInput) {
    return invokeSafe<WorkspaceMachine>("create_workspace_machine", { input });
  },
  refreshWorkspaceMachine(machineId: string) {
    return invokeSafe<WorkspaceMachine>("refresh_workspace_machine", { machineId });
  },
  startWorkspaceMachine(machineId: string) {
    return invokeSafe<WorkspaceMachine>("start_workspace_machine", { machineId });
  },
  stopWorkspaceMachine(machineId: string) {
    return invokeSafe<WorkspaceMachine>("stop_workspace_machine", { machineId });
  },
  setWorkspaceMachinePassword(machineId: string, password: string) {
    return invokeSafe<WorkspaceMachine>("set_workspace_machine_password", {
      input: { machine_id: machineId, password },
    });
  },
  openWorkspaceMachine(machineId: string) {
    return invokeSafe<MachineViewer>("open_workspace_machine", { machineId });
  },
  getWorkspaceMachineLogs(machineId: string, tail?: number) {
    return invokeSafe<string>("get_workspace_machine_logs", { machineId, tail: tail ?? null });
  },
  refreshWorkspaceMachineHealth() {
    return invokeSafe<string>("refresh_workspace_machine_health");
  },
  probeWorkspaceMachineSsh(machineId: string) {
    return invokeSafe<MachineSshProbe>("probe_workspace_machine_ssh", { machineId });
  },
  removeWorkspaceMachine(machineId: string) {
    return invokeSafe<void>("remove_workspace_machine", { input: { machine_id: machineId } });
  },
  listDeployStacks(workspaceId: number) {
    return invokeSafe<DeployStack[]>("list_deploy_stacks", { workspaceId });
  },
  resetWorkspaceDeployState(workspaceId: number) {
    return invokeSafe<WorkspaceDeployReset>("reset_workspace_deploy_state", { workspaceId });
  },
  getDeployStack(stackId: string) {
    return invokeSafe<DeployStackDetail>("get_deploy_stack", { stackId });
  },
  detectDeployStack(workspaceId: number, projectIds: number[]) {
    return invokeSafe<DeployDetectionReport>("detect_deploy_stack", {
      input: { workspace_id: workspaceId, project_ids: projectIds },
    });
  },
  planDeployPackage(
    workspaceId: number,
    projectIds: number[],
    targetMachineId: string | null,
    agentProfileId: number,
    includeDirty: boolean,
  ) {
    return invokeSafe<DeployPlanReport>("plan_deploy_package", {
      input: {
        workspace_id: workspaceId,
        project_ids: projectIds,
        target_machine_id: targetMachineId,
        agent_profile_id: agentProfileId,
        include_dirty: includeDirty,
      },
    });
  },
  createDeployPackage(input: CreateDeployPackageInput) {
    return invokeSafe<DeployVersion>("create_deploy_package", { input });
  },
  readDeployArtifact(versionId: string, relativePath: string) {
    return invokeSafe<string>("read_deploy_artifact", {
      input: { version_id: versionId, relative_path: relativePath },
    });
  },
  approveDeployVersion(versionId: string) {
    return invokeSafe<DeployVersion>("approve_deploy_version", {
      input: { version_id: versionId },
    });
  },
  createDeployRepairVersion(runId: string) {
    return invokeSafe<DeployVersion>("create_deploy_repair_version", {
      input: { run_id: runId },
    });
  },
  getDeployEnvironment(versionId: string, machineId: string) {
    return invokeSafe<DeployEnvironment>("get_deploy_environment", {
      input: { version_id: versionId, machine_id: machineId },
    });
  },
  saveDeployEnvironment(
    versionId: string,
    machineId: string,
    variables: DeployEnvironmentValueInput[],
  ) {
    return invokeSafe<DeployEnvironment>("save_deploy_environment", {
      input: { version_id: versionId, machine_id: machineId, variables },
    });
  },
  prepareDeployTarget(versionId: string, machineId: string, agentProfileId: number) {
    return invokeSafe<DeployRun>("prepare_deploy_target", {
      input: { version_id: versionId, machine_id: machineId, agent_profile_id: agentProfileId },
    });
  },
  deployVersion(versionId: string, machineId: string, agentProfileId: number) {
    return invokeSafe<DeployRun>("deploy_version", {
      input: { version_id: versionId, machine_id: machineId, agent_profile_id: agentProfileId },
    });
  },
  stopDeployStack(stackId: string, machineId: string) {
    return invokeSafe<DeployRun>("stop_deploy_stack", {
      input: { stack_id: stackId, machine_id: machineId },
    });
  },
  reactivateDeployVersion(versionId: string, machineId: string, agentProfileId: number) {
    return invokeSafe<DeployRun>("reactivate_deploy_version", {
      input: { version_id: versionId, machine_id: machineId, agent_profile_id: agentProfileId },
    });
  },
  listDeployRuns(versionId: string) {
    return invokeSafe<DeployRun[]>("list_deploy_runs", { versionId });
  },
  getDeployRunLogs(runId: string) {
    return invokeSafe<string>("get_deploy_run_logs", { input: { run_id: runId } });
  },
  createWorkspace(name: string, rootPath: string) {
    return invokeSafe<Workspace>("create_workspace", { input: { name, root_path: rootPath } });
  },
  listProjects(workspaceId: number) {
    return invokeSafe<Project[]>("list_projects", { workspaceId });
  },
  addLocalProject(workspaceId: number, name: string, path: string, remoteUrl?: string) {
    return invokeSafe<Project>("add_local_project", {
      input: { workspace_id: workspaceId, name, path, remote_url: remoteUrl || null },
    });
  },
  cloneGitProject(workspaceId: number, remoteUrl: string, name?: string) {
    return invokeSafe<Project>("clone_git_project", {
      input: { workspace_id: workspaceId, remote_url: remoteUrl, name: name || null },
    });
  },
  /** Streaming clone (emits `clone://progress`); cancel via cancelClone(cloneId).
   *  On a private repo without a credential, fails with an "AUTH_REQUIRED:" error. */
  cloneGitProjectStreamed(input: {
    workspace_id: number;
    remote_url: string;
    name?: string | null;
    clone_id: string;
    username?: string | null;
    token?: string | null;
  }) {
    return invokeSafe<Project>("clone_git_project_streamed", {
      input: {
        workspace_id: input.workspace_id,
        remote_url: input.remote_url,
        name: input.name ?? null,
        clone_id: input.clone_id,
        username: input.username ?? null,
        token: input.token ?? null,
      },
    });
  },
  cancelClone(cloneId: string) {
    return invokeSafe<void>("cancel_clone", { cloneId });
  },
  listRequirementCards(workspaceId: number) {
    return invokeSafe<RequirementCard[]>("list_requirement_cards", { workspaceId });
  },
  createRequirementCard(
    workspaceId: number,
    projectId: number | null,
    projectIds: number[],
    title: string,
    body: string,
    priority?: string | null,
  ) {
    return invokeSafe<RequirementCard>("create_requirement_card", {
      input: {
        workspace_id: workspaceId,
        project_id: projectId,
        project_ids: projectIds,
        title,
        body,
        priority: priority ?? null,
      },
    });
  },
  updateRequirementCard(input: {
    id: number;
    title?: string | null;
    body?: string | null;
    priority?: string | null;
    checklist_json?: string | null;
    agent_prompt?: string | null;
  }) {
    return invokeSafe<RequirementCard>("update_requirement_card", { input });
  },
  setRequirementCardProjects(id: number, projectIds: number[]) {
    return invokeSafe<RequirementCard>("set_requirement_card_projects", {
      input: { id, project_ids: projectIds },
    });
  },
  updateRequirementCardStatus(id: number, status: string, prdSlug?: string | null) {
    return invokeSafe<RequirementCard>("update_requirement_card_status", {
      input: { id, status, prd_slug: prdSlug || null },
    });
  },
  setRequirementCardFlow(id: number, flowId: string | null, status?: string | null) {
    return invokeSafe<RequirementCard>("set_requirement_card_flow", {
      input: { id, flow_id: flowId, status: status || null },
    });
  },
  archiveRequirementCard(id: number) {
    return invokeSafe<RequirementCard>("archive_requirement_card", { id });
  },
  restoreRequirementCard(id: number, status: string) {
    return invokeSafe<RequirementCard>("restore_requirement_card", {
      input: { id, status },
    });
  },
  updateRequirementCardBody(id: number, body: string) {
    return invokeSafe<RequirementCard>("update_requirement_card_body", {
      input: { id, body },
    });
  },
  listRequirementStageForms(cardId: number) {
    return invokeSafe<RequirementStageForm[]>("list_requirement_stage_forms", { cardId });
  },
  upsertRequirementStageForm(cardId: number, stageId: string, payload: Record<string, unknown>) {
    return invokeSafe<RequirementStageForm>("upsert_requirement_stage_form", {
      input: { card_id: cardId, stage_id: stageId, payload_json: JSON.stringify(payload) },
    });
  },
  listRequirementAttachments(cardId: number) {
    return invokeSafe<RequirementAttachment[]>("list_requirement_attachments", { cardId });
  },
  addRequirementAttachment(cardId: number, filePath: string, name?: string | null) {
    return invokeSafe<RequirementAttachment>("add_requirement_attachment", {
      input: { card_id: cardId, file_path: filePath, name: name || null },
    });
  },
  removeRequirementAttachment(id: number) {
    return invokeSafe<void>("remove_requirement_attachment", { id });
  },
  previewRequirementAttachment(id: number) {
    return invokeSafe<AttachmentPreview>("preview_requirement_attachment", { id });
  },
  downloadRequirementAttachment(id: number, destinationPath: string) {
    return invokeSafe<void>("download_requirement_attachment", {
      input: { id, destination_path: destinationPath },
    });
  },
  listKnowledgeSources(workspaceId: number, projectId?: number | null) {
    return invokeSafe<KnowledgeSource[]>("list_knowledge_sources", {
      workspaceId,
      projectId: projectId ?? null,
    });
  },
  addKnowledgeSource(input: {
    workspace_id: number;
    project_id?: number | null;
    blueprint_id?: string | null;
    file_path: string;
    name?: string | null;
  }) {
    return invokeSafe<KnowledgeSource>("add_knowledge_source", { input });
  },
  removeKnowledgeSource(id: number) {
    return invokeSafe<void>("remove_knowledge_source", { id });
  },
  createProjectBlueprint(input: {
    workspace_id: number;
    title: string;
    idea: string;
    agent_profile_id?: number | null;
    knowledge_source_ids?: number[] | null;
  }) {
    return invokeSafe<ProjectBlueprint>("create_project_blueprint", { input });
  },
  listProjectBlueprints(workspaceId: number) {
    return invokeSafe<ProjectBlueprint[]>("list_project_blueprints", { workspaceId });
  },
  updateProjectBlueprint(input: {
    id: string;
    status?: string | null;
    agent_session_id?: number | null;
    knowledge_source_ids?: number[] | null;
    answers_json?: string | null;
    running_summary?: string | null;
    detected_subprojects_json?: string | null;
    prd?: string | null;
    techspec?: string | null;
    tasks_json?: string | null;
    definition_of_done?: string | null;
  }) {
    return invokeSafe<ProjectBlueprint>("update_project_blueprint", { input });
  },
  materializeProjectBlueprint(id: string) {
    return invokeSafe<ProjectBlueprintMaterialization>("materialize_project_blueprint", { id });
  },
  listEvidence(path: string) {
    return invokeSafe<EvidenceEntry[]>("list_evidence", { path });
  },
  createEvidenceRun(
    path: string,
    input: {
      workspace_id?: number | null;
      project_id?: number | null;
      prd_slug?: string | null;
      command: string;
      summary?: string | null;
      terminal_session_id?: string | null;
      terminal_log_path?: string | null;
    },
  ) {
    return invokeSafe<EvidenceEntry>("create_evidence_run", { path, input });
  },
  completeEvidenceRun(id: number, status: string, summary: string, workspaceId?: number | null) {
    return invokeSafe<EvidenceEntry>("complete_evidence_run", {
      input: { id, workspace_id: workspaceId ?? null, status, summary },
    });
  },
  createManualEvidence(
    path: string,
    input: { title: string; status: string; summary: string; relative_paths: string[] },
  ) {
    return invokeSafe<EvidenceEntry[]>("create_manual_evidence", { path, input });
  },
  indexProjectEvidence(path: string) {
    return invokeSafe<EvidenceEntry[]>("index_project_evidence", { path });
  },
  listAgentProfiles(workspaceId: number, projectId?: number | null) {
    return invokeSafe<AgentProfile[]>("list_agent_profiles", {
      workspaceId,
      projectId: projectId ?? null,
    });
  },
  createAgentProfile(input: {
    workspace_id: number;
    project_id?: number | null;
    name: string;
    provider: AgentProvider;
    model?: string | null;
    reasoning_effort?: string | null;
    sandbox: string;
    context_mode?: string | null;
    rtk_enabled?: boolean | null;
  }) {
    return invokeSafe<AgentProfile>("create_agent_profile", { input });
  },
  updateAgentProfile(input: {
    id: number;
    name: string;
    provider: AgentProvider;
    model?: string | null;
    reasoning_effort?: string | null;
    sandbox: string;
    context_mode?: string | null;
    rtk_enabled?: boolean | null;
  }) {
    return invokeSafe<AgentProfile>("update_agent_profile", { input });
  },
  getRtkStatus(profileId?: number | null, projectPath?: string | null) {
    return invokeSafe<RtkStatus>("get_rtk_status", {
      input: { profile_id: profileId ?? null, project_path: projectPath ?? null },
    });
  },
  installRtk(profileId?: number | null, projectPath?: string | null) {
    return invokeSafe<RtkInstallResult>("install_rtk", {
      input: { profile_id: profileId ?? null, project_path: projectPath ?? null },
    });
  },
  configureRtk(profileId: number, projectPath: string, apply: boolean) {
    return invokeSafe<RtkSetupResult>("configure_rtk", {
      input: { profile_id: profileId, project_path: projectPath, apply },
    });
  },
  resetAgentChat(input: {
    profile_id: number;
    workspace_id: number;
    project_id?: number | null;
    project_path: string;
  }) {
    return invokeSafe<AgentSession>("reset_agent_chat", { input });
  },
  listAgentSessions(workspaceId: number, projectId?: number | null) {
    return invokeSafe<AgentSession[]>("list_agent_sessions", {
      workspaceId,
      projectId: projectId ?? null,
    });
  },
  listAgentMessages(sessionId: number) {
    return invokeSafe<AgentMessage[]>("list_agent_messages", { sessionId });
  },
  listAgentSessionsForCard(workspaceId: number, requirementCardId: number) {
    return invokeSafe<AgentSession[]>("list_agent_sessions_for_card", {
      workspaceId,
      requirementCardId,
    });
  },
  sendAgentMessage(input: {
    profile_id: number;
    session_id?: number | null;
    workspace_id: number;
    project_id?: number | null;
    requirement_card_id?: number | null;
    scope?: "chat" | "card_interview" | string | null;
    title?: string | null;
    project_path: string;
    message: string;
    skill?: AgentSkillInvocation | null;
  }) {
    return invokeSafe<AgentSession>("send_agent_message", { input });
  },
  stopAgentSession(sessionId: number) {
    return invokeSafe<AgentSession>("stop_agent_session", { sessionId });
  },
  agentUsage(provider: string, projectPath: string) {
    return invokeSafe<AgentUsage>("agent_usage", { provider, projectPath });
  },
  checkAgentProviderHealth(provider: string, projectPath: string) {
    return invokeSafe<AgentProviderHealth>("check_agent_provider_health", {
      provider,
      projectPath,
    });
  },
  listAgentRunMetrics(sessionId: number) {
    return invokeSafe<AgentRunEvent[]>("list_agent_run_metrics", { sessionId });
  },
  warmAgentRuntime(input: {
    profile_id: number;
    workspace_id: number;
    project_id?: number | null;
    project_path: string;
    scope?: string | null;
    provider_session_id?: string | null;
  }) {
    return invokeSafe<AgentProviderHealth>("warm_agent_runtime", { input });
  },
  gitStatus(path: string) {
    return invokeSafe<string>("git_status", { path });
  },
  gitDiff(path: string) {
    return invokeSafe<string>("git_diff", { path });
  },
  gitStagedDiff(path: string) {
    return invokeSafe<string>("git_staged_diff", { path });
  },
  gitLogGraph(path: string) {
    return invokeSafe<string>("git_log_graph", { path });
  },
  gitBlame(path: string, filePath: string) {
    return invokeSafe<string>("git_blame", { path, filePath });
  },
  gitBlamePorcelain(path: string, filePath: string) {
    return invokeSafe<BlameLine[]>("git_blame_porcelain", { path, filePath });
  },
  gitBlamePorcelainForContent(path: string, filePath: string, content: string) {
    return invokeSafe<BlameLine[]>("git_blame_porcelain_for_content", {
      path,
      filePath,
      content,
    });
  },
  listChangedFiles(path: string) {
    return invokeSafe<ChangedFile[]>("list_changed_files", { path });
  },
  gitWorktreeFingerprint(path: string) {
    return invokeSafe<GitWorktreeFingerprint>("git_worktree_fingerprint", { path });
  },
  gitWorktreeSnapshot(path: string, options: { untrackedLimit?: number } = {}) {
    return invokeSafe<GitWorktreeSnapshot>("git_worktree_snapshot", {
      path,
      untrackedLimit: options.untrackedLimit ?? 500,
    });
  },
  readFilePatch(path: string, filePath: string, area: PatchArea) {
    return invokeSafe<FilePatch>("read_file_patch", { path, filePath, area });
  },
  gitFilePatchText(path: string, filePath: string, area: PatchArea) {
    return invokeSafe<string>("git_file_patch_text", { path, filePath, area });
  },
  stageFile(path: string, filePath: string) {
    return invokeSafe<string>("stage_file", { path, filePath });
  },
  unstageFile(path: string, filePath: string) {
    return invokeSafe<string>("unstage_file", { path, filePath });
  },
  gitStageAll(path: string) {
    return invokeSafe<string>("git_stage_all", { path });
  },
  gitUnstageAll(path: string) {
    return invokeSafe<string>("git_unstage_all", { path });
  },
  stageHunk(path: string, hunkPatch: string) {
    return invokeSafe<PatchCheckResult>("stage_hunk", { path, hunkPatch });
  },
  unstageHunk(path: string, hunkPatch: string) {
    return invokeSafe<PatchCheckResult>("unstage_hunk", { path, hunkPatch });
  },
  checkImportedPatch(path: string, patch: string) {
    return invokeSafe<PatchCheckResult>("check_imported_patch", { path, patch });
  },
  applyImportedPatch(path: string, patch: string) {
    return invokeSafe<PatchCheckResult>("apply_imported_patch", { path, patch });
  },
  gitCommitGraph(
    path: string,
    options: {
      includeRemotes?: boolean;
      includeTags?: boolean;
      limit?: number;
      skip?: number;
    } = {},
  ) {
    return invokeSafe<Commit[]>("git_commit_graph", {
      path,
      includeRemotes: options.includeRemotes ?? false,
      includeTags: options.includeTags ?? false,
      limit: options.limit ?? 200,
      skip: options.skip ?? 0,
    });
  },
  gitRepoSnapshot(
    path: string,
    options: { includeRemotes?: boolean; includeTags?: boolean; limit?: number } = {},
  ) {
    return invokeSafe<GitRepoSnapshot>("git_repo_snapshot", {
      path,
      includeRemotes: options.includeRemotes ?? false,
      includeTags: options.includeTags ?? false,
      limit: options.limit ?? 200,
    });
  },
  gitCommitDetail(path: string, sha: string) {
    return invokeSafe<CommitDetail>("git_commit_detail", { path, sha });
  },
  gitLogFile(path: string, filePath: string, limit = 100) {
    return invokeSafe<Commit[]>("git_log_file", { path, filePath, limit });
  },
  gitShowFile(path: string, sha: string, filePath: string) {
    return invokeSafe<string>("git_show_file", { path, sha, filePath });
  },
  gitCommitFileDiff(path: string, sha: string, filePath: string) {
    return invokeSafe<FilePatch>("git_commit_file_diff", { path, sha, filePath });
  },
  gitRepoState(path: string) {
    return invokeSafe<RepoState>("git_repo_state", { path });
  },
  gitListBranches(path: string) {
    return invokeSafe<Branch[]>("git_list_branches", { path });
  },
  gitListRemoteBranches(path: string) {
    return invokeSafe<RemoteBranch[]>("git_list_remote_branches", { path });
  },
  gitListTags(path: string) {
    return invokeSafe<TagEntry[]>("git_list_tags", { path });
  },
  gitListStashes(path: string) {
    return invokeSafe<StashEntry[]>("git_list_stashes", { path });
  },
  gitStashDetail(path: string, index: number) {
    return invokeSafe<CommitDetail>("git_stash_detail", { path, index });
  },
  gitStashFileDiff(path: string, index: number, filePath: string) {
    return invokeSafe<FilePatch>("git_stash_file_diff", { path, index, filePath });
  },
  gitCommit(path: string, message: string, amend = false) {
    return invokeSafe<string>("git_commit", { path, message, amend });
  },
  gitFetch(path: string, remote?: string) {
    return invokeSafe<string>("git_fetch", { path, remote: remote ?? null });
  },
  gitPull(path: string, rebase = false) {
    return invokeSafe<string>("git_pull", { path, rebase });
  },
  gitPush(path: string, options: { setUpstream?: boolean; forceWithLease?: boolean } = {}) {
    return invokeSafe<string>("git_push", {
      path,
      setUpstream: options.setUpstream ?? false,
      forceWithLease: options.forceWithLease ?? false,
    });
  },
  gitCheckoutBranch(
    path: string,
    name: string,
    mode: "plain" | "discard" | "stash" | "stash_apply" = "plain",
  ) {
    return invokeSafe<string>("git_checkout_branch", { path, name, mode });
  },
  gitCreateBranch(path: string, name: string, startPoint?: string, checkout = true) {
    return invokeSafe<string>("git_create_branch", {
      path,
      name,
      startPoint: startPoint ?? null,
      checkout,
    });
  },
  gitRenameBranch(path: string, oldName: string, newName: string) {
    return invokeSafe<string>("git_rename_branch", { path, oldName, newName });
  },
  gitDeleteBranch(path: string, name: string, force = false) {
    return invokeSafe<string>("git_delete_branch", { path, name, force });
  },
  gitMergeBranch(path: string, name: string) {
    return invokeSafe<string>("git_merge_branch", { path, name });
  },
  gitRebaseBranch(path: string, onto: string) {
    return invokeSafe<string>("git_rebase_branch", { path, onto });
  },
  gitCherryPick(path: string, sha: string) {
    return invokeSafe<string>("git_cherry_pick", { path, sha });
  },
  gitRevert(path: string, sha: string) {
    return invokeSafe<string>("git_revert", { path, sha });
  },
  gitReset(path: string, sha: string, mode: "soft" | "mixed" | "hard") {
    return invokeSafe<string>("git_reset", { path, sha, mode });
  },
  gitCreateTag(path: string, name: string, sha?: string, message?: string) {
    return invokeSafe<string>("git_create_tag", {
      path,
      name,
      sha: sha ?? null,
      message: message ?? null,
    });
  },
  gitDeleteTag(path: string, name: string) {
    return invokeSafe<string>("git_delete_tag", { path, name });
  },
  gitStashSave(path: string, message?: string, includeUntracked = false) {
    return invokeSafe<string>("git_stash_save", {
      path,
      message: message ?? null,
      includeUntracked,
    });
  },
  gitStashFile(path: string, filePath: string, message?: string) {
    return invokeSafe<string>("git_stash_file", {
      path,
      filePath,
      message: message ?? null,
    });
  },
  gitIgnoreFile(path: string, filePath: string, target: "info_exclude" | "gitignore") {
    return invokeSafe<string>("git_ignore_file", { path, filePath, target });
  },
  gitExternalDiff(path: string, filePath: string, area: PatchArea) {
    return invokeSafe<string>("git_external_diff", { path, filePath, area });
  },
  gitStashPop(path: string, index: number) {
    return invokeSafe<string>("git_stash_pop", { path, index });
  },
  gitStashApply(path: string, index: number) {
    return invokeSafe<string>("git_stash_apply", { path, index });
  },
  gitStashDrop(path: string, index: number) {
    return invokeSafe<string>("git_stash_drop", { path, index });
  },
  gitCheckoutCommit(path: string, sha: string) {
    return invokeSafe<string>("git_checkout_commit", { path, sha });
  },
  gitAbortOperation(path: string, operation: string) {
    return invokeSafe<string>("git_abort_operation", { path, operation });
  },
  gitUseOurs(path: string, filePath: string) {
    return invokeSafe<string>("git_use_ours", { path, filePath });
  },
  gitUseTheirs(path: string, filePath: string) {
    return invokeSafe<string>("git_use_theirs", { path, filePath });
  },
  gitMarkResolved(path: string, filePath: string) {
    return invokeSafe<string>("git_mark_resolved", { path, filePath });
  },
  gitContinueOperation(path: string, operation: string) {
    return invokeSafe<string>("git_continue_operation", { path, operation });
  },
  gitStartInteractiveRebase(path: string, base: string, steps: RebaseStep[]) {
    return invokeSafe<string>("git_start_interactive_rebase", { path, base, steps });
  },
  gitListSubmodules(path: string) {
    return invokeSafe<Submodule[]>("git_list_submodules", { path });
  },
  gitUpdateSubmodule(path: string, subPath: string, init: boolean) {
    return invokeSafe<string>("git_update_submodule", { path, subPath, init });
  },
  gitUpdateAllSubmodules(path: string, init = true) {
    return invokeSafe<string>("git_update_all_submodules", { path, init });
  },
  gitSyncSubmodules(path: string) {
    return invokeSafe<string>("git_sync_submodules", { path });
  },
  gitUpdateSubmoduleRemote(path: string, subPath: string) {
    return invokeSafe<string>("git_update_submodule_remote", { path, subPath });
  },
  gitCheckoutSubmoduleBranch(path: string, subPath: string) {
    return invokeSafe<string>("git_checkout_submodule_branch", { path, subPath });
  },
  gitDiscardFile(path: string, filePath: string) {
    return invokeSafe<string>("git_discard_file", { path, filePath });
  },
  gitDiscardHunk(path: string, hunkPatch: string) {
    return invokeSafe<PatchCheckResult>("git_discard_hunk", { path, hunkPatch });
  },
  openProjectFile(path: string, filePath: string) {
    return invokeSafe<string>("open_project_file", { path, filePath });
  },
  revealProjectFile(path: string, filePath: string) {
    return invokeSafe<string>("reveal_project_file", { path, filePath });
  },
  listSourceTree(path: string) {
    return invokeSafe<SourceEntry[]>("list_source_tree", { path });
  },
  searchInFiles(
    path: string,
    query: string,
    options: { caseSensitive: boolean; wholeWord: boolean; useRegex: boolean },
  ) {
    return invokeSafe<SearchFileResult[]>("search_in_files", {
      path,
      query,
      caseSensitive: options.caseSensitive,
      wholeWord: options.wholeWord,
      useRegex: options.useRegex,
    });
  },
  readSourceFile(path: string, relativePath: string) {
    return invokeSafe<SourceFile>("read_source_file", { path, relativePath });
  },
  createSourceFile(path: string, relativePath: string) {
    return invokeSafe<SourceFile>("create_source_file", { path, relativePath });
  },
  createSourceDir(path: string, relativePath: string) {
    return invokeSafe<null>("create_source_dir", { path, relativePath });
  },
  projectPathExists(path: string, relativePath: string) {
    return invokeSafe<boolean>("project_path_exists", { path, relativePath });
  },
  formatExternal(language: string, content: string) {
    return invokeSafe<string>("format_external", { language, content });
  },
  lsp_start(language: string, cwd: string) {
    return invokeSafe<number>("lsp_start", { language, cwd });
  },
  lsp_send(id: number, message: string) {
    return invokeSafe<null>("lsp_send", { id, message });
  },
  lsp_stop(id: number) {
    return invokeSafe<null>("lsp_stop", { id });
  },
  lspServerStatus(language: string) {
    return invokeSafe<LspServerStatus | null>("lsp_server_status", { language });
  },
  lspInstall(language: string) {
    return invokeSafe<string>("lsp_install", { language });
  },
  writeSourceFile(path: string, relativePath: string, content: string) {
    return invokeSafe<SourceFile>("write_source_file", { path, relativePath, content });
  },
  readTextFile(path: string) {
    return invokeSafe<string>("read_text_file", { path });
  },
  writeTextFile(path: string, content: string) {
    return invokeSafe<void>("write_text_file", { path, content });
  },
  listDwArtifacts(path: string) {
    return invokeSafe<DwArtifact[]>("list_dw_artifacts", { path });
  },
  readDwArtifact(path: string, relativePath: string) {
    return invokeSafe<string>("read_dw_artifact", { path, relativePath });
  },
  writeDwArtifact(path: string, relativePath: string, content: string) {
    return invokeSafe<DwArtifact>("write_dw_artifact", {
      path,
      input: { relative_path: relativePath, content },
    });
  },
  listDwCommands(path: string) {
    return invokeSafe<DwCommand[]>("list_dw_commands", { path });
  },
  listDwSkills(path: string) {
    return invokeSafe<DwSkill[]>("list_dw_skills", { path });
  },
  listWorkspaceSkills(workspacePath: string) {
    return invokeSafe<WorkspaceSkill[]>("list_workspace_skills", { workspacePath });
  },
  listWorkspaceCapabilities(workspaceId: number, projectId?: number | null) {
    return invokeSafe<WorkspaceCapabilities>("list_workspace_capabilities", {
      workspaceId,
      projectId: projectId ?? null,
    });
  },
  findWorkspaceSkills(query: string) {
    return invokeSafe<WorkspaceSkillSearchResult[]>("find_workspace_skills", { query });
  },
  syncWorkspaceSkill(workspacePath: string, name: string, targets: WorkspaceSkillTarget[]) {
    return invokeSafe<WorkspaceSkill[]>("sync_workspace_skill", {
      input: { workspace_path: workspacePath, name, targets },
    });
  },
  readWorkspaceFlowArtifact(workspacePath: string, relativePath: string) {
    return invokeSafe<string>("read_workspace_flow_artifact", { workspacePath, relativePath });
  },
  writeWorkspaceFlowArtifact(workspacePath: string, relativePath: string, content: string) {
    return invokeSafe<string>("write_workspace_flow_artifact", {
      workspacePath,
      input: { relative_path: relativePath, content },
    });
  },
  syncWorkspaceFlows(workspacePath: string, projectPath: string) {
    return invokeSafe<void>("sync_workspace_flows", {
      input: { workspace_path: workspacePath, project_path: projectPath },
    });
  },
  previewWorkspaceSolution(sourcePath: string) {
    return invokeSafe<WorkspaceSolutionManifest>("preview_workspace_solution", { sourcePath });
  },
  exportWorkspaceSolution(workspaceId: number, destinationPath: string) {
    return invokeSafe<WorkspaceSolutionManifest>("export_workspace_solution", {
      workspaceId,
      destinationPath,
    });
  },
  importWorkspaceSolution(workspacePath: string, sourcePath: string) {
    return invokeSafe<WorkspaceSolutionManifest>("import_workspace_solution", {
      workspacePath,
      sourcePath,
    });
  },
  importWorkspaceSolutionAsWorkspace(
    sourcePath: string,
    destinationRoot: string,
    workspaceName?: string,
  ) {
    return invokeSafe<WorkspaceSolutionImportReport>("import_workspace_solution_as_workspace", {
      input: {
        source_path: sourcePath,
        destination_root: destinationRoot,
        workspace_name: workspaceName || null,
      },
    });
  },
  fetchUrl(url: string) {
    return invokeSafe<string>("fetch_url", { url });
  },
  readWorkflowState(path: string) {
    return invokeSafe<WorkflowStateSummary>("read_workflow_state", { path });
  },
  runShell(path: string, command: string) {
    return invokeSafe<string>("run_shell", { path, command });
  },
  createTerminalSession(path: string, shell?: string, initialInput?: string) {
    return invokeSafe<TerminalSession>("create_terminal_session", {
      path,
      shell: shell || null,
      initialInput: initialInput || null,
    });
  },
  writeTerminalInput(sessionId: string, data: string) {
    return invokeSafe<void>("write_terminal_input", { sessionId, data });
  },
  resizeTerminalSession(sessionId: string, cols: number, rows: number) {
    return invokeSafe<void>("resize_terminal_session", { sessionId, cols, rows });
  },
  stopTerminalSession(sessionId: string) {
    return invokeSafe<TerminalSession>("stop_terminal_session", { sessionId });
  },
  closeTerminalSession(sessionId: string) {
    return invokeSafe<void>("close_terminal_session", { sessionId });
  },
  listTerminalSessions() {
    return invokeSafe<TerminalSession[]>("list_terminal_sessions");
  },
};
