export type PreflightReport = {
  project_path: string;
  has_git: boolean;
  has_dw: boolean;
  has_dw_commands: boolean;
  git_status: string;
  docker_version?: string | null;
  docker_compose_version?: string | null;
  node_version?: string | null;
  pnpm_version?: string | null;
  rust_version?: string | null;
};

export type Workspace = {
  id: number;
  name: string;
  root_path: string;
  created_at: string;
};

export type Project = {
  id: number;
  workspace_id: number;
  name: string;
  path: string;
  remote_url?: string | null;
  /** When set, this project is a git submodule of that parent project. */
  parent_project_id?: number | null;
  is_submodule?: boolean;
  /** Submodule path relative to the parent project root (e.g. "vendor/lib"). */
  submodule_path?: string | null;
  created_at: string;
};

export type MachineProviderStatus = {
  provider: string;
  runtime: "native" | "wsl" | "unavailable" | string;
  executable?: string | null;
  version?: string | null;
  status: "ready" | "unavailable" | "incompatible" | string;
  message: string;
  hint?: string | null;
};

export type MachinePreset = {
  id: "ubuntu_deploy_vm" | "ubuntu_desktop_deploy_vm" | "windows_11" | string;
  label: string;
  image_family: "windows" | "linux_distro" | "linux_cloud" | string;
  boot?: string | null;
  cloud_init_profile?: string | null;
  version?: string | null;
  default_ram: string;
  default_cpu: string;
  default_disk: string;
  deploy_capable: boolean;
  supported: boolean;
  disabled_reason?: string | null;
};

export type WorkspaceMachine = {
  id: string;
  workspace_id: number;
  project_id?: number | null;
  provider: "winbox" | string;
  provider_runtime: "native" | "wsl" | string;
  provider_profile: string;
  display_name: string;
  preset_id: string;
  image_family: "windows" | "linux_distro" | "linux_cloud" | string;
  access_user?: string | null;
  status: "unknown" | "creating" | "running" | "stopped" | "paused" | "error" | "removing" | string;
  web_port?: number | null;
  rdp_port?: number | null;
  ssh_port?: number | null;
  last_health_status?: "unknown" | "healthy" | "warning" | "failed" | string | null;
  last_health_summary?: string | null;
  last_error_code?: string | null;
  last_error_message?: string | null;
  created_at: string;
  updated_at: string;
};

export type MachineProgressEvent = {
  run_id: string;
  machine_id?: string | null;
  provider_profile: string;
  operation: string;
  phase: string;
  status: string;
  message: string;
  percent?: number | null;
  timestamp: string;
};

export type MachineViewer = {
  machine_id: string;
  url: string;
};

export type MachineSshProbe = {
  machine_id: string;
  status: "ready" | "not_ready" | "missing_port" | string;
  port?: number | null;
  user: string;
  command?: string | null;
  message: string;
};

export type CreateWorkspaceMachineInput = {
  workspace_id: number;
  project_id?: number | null;
  preset_id: string;
  display_name: string;
  provider_profile?: string | null;
  ram?: string | null;
  cpu?: string | null;
  disk?: string | null;
  user?: string | null;
  password?: string | null;
};

export type DeployServiceSuggestion = {
  name: string;
  reason: string;
};

export type DeployPortSuggestion = {
  container: number;
  host: number;
  confidence: string;
};

export type DeployProjectDetection = {
  project_id: number;
  name: string;
  path: string;
  language: string;
  framework?: string | null;
  package_manager?: string | null;
  has_dockerfile: boolean;
  has_compose: boolean;
  services: DeployServiceSuggestion[];
  ports: DeployPortSuggestion[];
  healthcheck?: string | null;
  deploy_strategy: string;
  strategy_reason: string;
  runtime_commands: string[];
  requires_desktop_session: boolean;
  warnings: string[];
};

export type DeployDetectionReport = {
  workspace_id: number;
  projects: DeployProjectDetection[];
  services: DeployServiceSuggestion[];
  ports: DeployPortSuggestion[];
  warnings: string[];
};

export type DeployPlanReport = {
  workspace_id: number;
  project_ids: number[];
  target_machine_id?: string | null;
  agent_profile_id: number;
  agent_session_id?: number | null;
  agent_name: string;
  mode: string;
  planning_status: string;
  status: string;
  confidence: string;
  summary: string;
  guided_summary: Record<string, unknown>;
  project_context_path?: string | null;
  deploy_plan_path?: string | null;
  validation_report_path?: string | null;
  project_context_json: string;
  deploy_plan_json: string;
  validation_report_json: string;
  validation_errors: string[];
  warnings: string[];
};

export type DeployStack = {
  id: string;
  workspace_id: number;
  name: string;
  slug: string;
  status: string;
  active_version_id?: string | null;
  active_machine_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type DeployVersion = {
  id: string;
  stack_id: string;
  workspace_id: number;
  label: string;
  status: string;
  target_machine_id?: string | null;
  artifact_path: string;
  manifest_path: string;
  manifest_json: string;
  review_status: string;
  reviewed_at?: string | null;
  blocking_findings_json: string;
  created_at: string;
  updated_at: string;
};

export type DeployRun = {
  id: string;
  stack_id: string;
  version_id?: string | null;
  machine_id?: string | null;
  operation: string;
  status: string;
  started_at: string;
  completed_at?: string | null;
  summary: string;
  agent_profile_id?: number | null;
  agent_name?: string | null;
  agent_provider?: string | null;
  agent_model?: string | null;
  orchestration_status: string;
  orchestration_report_json: string;
};

export type DeployRunStep = {
  id: number;
  run_id: string;
  step_key: string;
  status: string;
  message: string;
  log_path?: string | null;
  error_code?: string | null;
  started_at: string;
  completed_at?: string | null;
};

export type WorkspaceDeployReset = {
  workspace_id: number;
  workspace_machines: number;
  deploy_stacks: number;
  deploy_versions: number;
  deploy_runs: number;
  deploy_run_steps: number;
  deploy_version_projects: number;
  deploy_target_bootstrap: number;
  removed_artifact_dirs: string[];
};

export type DeployEnvironmentVariable = {
  key: string;
  value: string;
  placeholder: string;
  required: boolean;
  secret: boolean;
  saved: boolean;
};

export type DeployEnvironment = {
  version_id: string;
  stack_id: string;
  machine_id: string;
  file_path: string;
  ready: boolean;
  required_count: number;
  saved_count: number;
  missing_keys: string[];
  variables: DeployEnvironmentVariable[];
};

export type DeployEnvironmentValueInput = {
  key: string;
  value: string;
};

export type DeployStackDetail = {
  stack: DeployStack;
  versions: DeployVersion[];
};

export type DeployProgressEvent = {
  run_id: string;
  stack_id: string;
  version_id?: string | null;
  machine_id?: string | null;
  step_key: string;
  status: string;
  message: string;
  percent?: number | null;
  timestamp: string;
};

export type CreateDeployPackageInput = {
  workspace_id: number;
  stack_name: string;
  project_ids: number[];
  target_machine_id?: string | null;
  agent_profile_id: number;
  deploy_plan_path?: string | null;
  include_dirty: boolean;
};

export type RequirementStatus =
  | "draft"
  | "brainstorming"
  | "planned"
  | "running"
  | "reviewing"
  | "qa"
  | "ready_for_pr"
  | "local_pr"
  | "done"
  | "archived";

export type RequirementCard = {
  id: number;
  workspace_id: number;
  project_id?: number | null;
  project_ids: number[];
  public_id: string;
  title: string;
  slug: string;
  body: string;
  /** Task priority. high | medium | low. */
  priority: string;
  /** JSON-encoded ChecklistItem[] of subtasks. */
  checklist_json: string;
  /** Free-form instructions appended to the agent prompt when running this task. */
  agent_prompt: string;
  status: RequirementStatus | string;
  prd_slug?: string | null;
  /** Which workbench flow this card follows; null/undefined = shared intake backlog. */
  flow_id?: string | null;
  archived_from_status?: string | null;
  archived_at?: string | null;
  created_at: string;
  updated_at: string;
};

/** A single subtask inside a task card's checklist (`checklist_json`). */
export type ChecklistItem = {
  id: string;
  text: string;
  done: boolean;
};

export type RequirementStageForm = {
  card_id: number;
  stage_id: string;
  payload_json: string;
  updated_at: string;
};

export type RequirementAttachment = {
  id: number;
  card_id: number;
  name: string;
  file_path: string;
  created_at: string;
};

export type KnowledgeSource = {
  id: number;
  workspace_id: number;
  project_id?: number | null;
  blueprint_id?: string | null;
  scope: "workspace" | "project" | "blueprint" | string;
  name: string;
  file_path: string;
  original_path?: string | null;
  created_at: string;
};

export type ProjectBlueprintStatus =
  | "draft"
  | "interviewing"
  | "planned"
  | "materialized"
  | "archived";

export type ProjectBlueprint = {
  id: string;
  workspace_id: number;
  title: string;
  slug: string;
  status: ProjectBlueprintStatus | string;
  idea: string;
  agent_profile_id?: number | null;
  agent_session_id?: number | null;
  knowledge_source_ids_json: string;
  answers_json: string;
  running_summary: string;
  detected_subprojects_json: string;
  prd: string;
  techspec: string;
  tasks_json: string;
  definition_of_done: string;
  project_id?: number | null;
  created_at: string;
  updated_at: string;
};

export type ProjectBlueprintMaterialization = {
  blueprint: ProjectBlueprint;
  project: Project;
  cards: RequirementCard[];
  spec_dir: string;
};

export type AttachmentPreview = {
  id: number;
  name: string;
  mime_type: string;
  data_base64: string;
};

export type EvidenceStatus = "submitted" | "passed" | "failed" | "unknown" | "indexed" | "stale";

export type EvidenceEntry = {
  id: string;
  record_type: "run" | "item" | string;
  run_id?: number | null;
  item_id?: number | null;
  workspace_id?: number | null;
  project_id?: number | null;
  project_path: string;
  prd_slug?: string | null;
  command?: string | null;
  status: EvidenceStatus | string;
  summary: string;
  kind: string;
  title: string;
  relative_path?: string | null;
  absolute_path?: string | null;
  terminal_session_id?: string | null;
  terminal_log_path?: string | null;
  created_at: string;
  completed_at?: string | null;
  stale: boolean;
};

export type DwArtifact = {
  relative_path: string;
  category: "state" | "spec" | "bugfix" | "command" | "rule" | "support";
  name: string;
  bytes: number;
};

export type SourceEntry = {
  relative_path: string;
  name: string;
  kind: "file" | "directory";
  extension?: string | null;
  bytes?: number | null;
  children: SourceEntry[];
};

export type SourceFile = {
  relative_path: string;
  name: string;
  extension?: string | null;
  bytes: number;
  content: string;
};

export type SearchMatch = {
  line: number;
  col: number;
  length: number;
  text: string;
};

export type SearchFileResult = {
  relative_path: string;
  matches: SearchMatch[];
};

export type LspServerStatus = {
  language: string;
  program: string;
  installed: boolean;
  can_install: boolean;
};

export type PatchArea = "staged" | "unstaged";

export type ChangedFile = {
  path: string;
  old_path?: string | null;
  status: string;
  area: PatchArea;
  additions: number;
  deletions: number;
  can_stage_hunks: boolean;
};

export type Submodule = {
  path: string;
  sha: string;
  status: string;
  describe?: string | null;
};

export type BlameLine = {
  line: number;
  sha: string;
  short_sha: string;
  author: string;
  author_email: string;
  date: string;
  summary: string;
};

export type RebaseAction = "pick" | "reword" | "edit" | "squash" | "fixup" | "drop";

export type RebaseStep = {
  action: RebaseAction;
  sha: string;
};

export type PatchHunk = {
  id: string;
  header: string;
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  patch: string;
};

export type FilePatch = {
  path: string;
  area: PatchArea;
  patch: string;
  hunks: PatchHunk[];
};

export type PatchCheckResult = {
  ok: boolean;
  output: string;
};

export type CommitRefKind = "head" | "branch" | "remote" | "tag";

export type CommitRef = {
  name: string;
  kind: CommitRefKind | string;
};

export type Commit = {
  sha: string;
  short_sha: string;
  parents: string[];
  refs: CommitRef[];
  author_name: string;
  author_email: string;
  date: string;
  subject: string;
};

export type CommitFile = {
  path: string;
  old_path?: string | null;
  status: string;
  additions: number;
  deletions: number;
};

export type CommitDetail = {
  sha: string;
  short_sha: string;
  parents: string[];
  refs: CommitRef[];
  author_name: string;
  author_email: string;
  date: string;
  subject: string;
  body: string;
  files: CommitFile[];
};

export type Branch = {
  name: string;
  is_head: boolean;
  upstream?: string | null;
  ahead: number;
  behind: number;
};

export type RemoteBranch = {
  remote: string;
  name: string;
  full: string;
};

export type TagEntry = {
  name: string;
  sha: string;
};

export type StashEntry = {
  index: number;
  label: string;
  message: string;
  sha: string;
};

export type RepoState = {
  branch?: string | null;
  detached: boolean;
  upstream?: string | null;
  ahead: number;
  behind: number;
  operation?: string | null;
  conflicts: string[];
  dirty: boolean;
};

export type GitRepoSnapshotOptions = {
  include_remotes: boolean;
  include_tags: boolean;
  limit: number;
};

export type GitRepoSnapshot = {
  commits: Commit[];
  branches: Branch[];
  remote_branches: RemoteBranch[];
  tags: TagEntry[];
  stashes: StashEntry[];
  submodules: Submodule[];
  repo_state: RepoState;
  generated_at: string;
  options: GitRepoSnapshotOptions;
  warnings: string[];
};

export type WorktreeCounts = {
  staged: number;
  unstaged: number;
  untracked: number;
  conflicts: number;
  total: number;
};

export type GitWorktreeFingerprint = {
  counts: WorktreeCounts;
  fingerprint: string;
  generated_at: string;
};

export type GitWorktreeSnapshot = {
  files: ChangedFile[];
  counts: WorktreeCounts;
  untracked_truncated: boolean;
  fingerprint: string;
  generated_at: string;
};

export type TerminalStatus = "idle" | "running" | "exited" | "failed" | "stopped";

export type TerminalSession = {
  id: string;
  title: string;
  cwd: string;
  shell: string;
  status: TerminalStatus;
  log_path: string;
  created_at: string;
  updated_at: string;
  exit_code?: number | null;
};

export type TerminalOutputEvent = {
  session_id: string;
  data: string;
};

export type TerminalStatusEvent = {
  session_id: string;
  status: TerminalStatus;
  exit_code?: number | null;
  message?: string | null;
};

export type TerminalErrorEvent = {
  session_id: string;
  message: string;
};

export type AgentProvider = "codex" | "claude" | "copilot";

export type AgentStatus = "idle" | "running" | "done" | "failed" | "stopped" | string;

export type AgentProfile = {
  id: number;
  workspace_id: number;
  project_id?: number | null;
  name: string;
  provider: AgentProvider | string;
  model?: string | null;
  reasoning_effort?: string | null;
  sandbox: string;
  context_mode: "auto_lean" | "full" | string;
  rtk_enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type AgentSession = {
  id: number;
  profile_id: number;
  workspace_id: number;
  project_id?: number | null;
  requirement_card_id?: number | null;
  scope: "chat" | "card_interview" | string;
  project_path: string;
  provider: AgentProvider | string;
  model?: string | null;
  reasoning_effort?: string | null;
  sandbox: string;
  context_mode: "auto_lean" | "full" | string;
  provider_session_id?: string | null;
  codex_session_id?: string | null;
  status: AgentStatus;
  title: string;
  created_at: string;
  updated_at: string;
};

export type AgentMessage = {
  id: number;
  session_id: number;
  role: "user" | "assistant" | "system" | "event" | string;
  content: string;
  raw_json?: string | null;
  created_at: string;
};

export type AgentRunEvent = {
  id: number;
  session_id: number;
  run_id: string;
  provider: string;
  phase: string;
  elapsed_ms: number;
  details_json: string;
  created_at: string;
};

export type AgentMetricEvent = {
  session_id: number;
  run_id: string;
  provider: string;
  phase: string;
  elapsed_ms: number;
  details: unknown;
};

export type AgentProviderHealth = {
  provider: string;
  ok: boolean;
  supported: boolean;
  program: string;
  version?: string | null;
  message: string;
  details: unknown;
};

export type RtkStatus = {
  enabled: boolean;
  available: boolean;
  supported: boolean;
  telemetry_blocked: boolean;
  version?: string | null;
  binary_path?: string | null;
  binary_source?: string | null;
  setup_state: string;
  message: string;
  gain_summary?: string | null;
};

export type RtkSetupCommand = {
  provider: string;
  cwd: string;
  command: string;
  description: string;
};

export type RtkSetupResult = {
  applied: boolean;
  commands: RtkSetupCommand[];
  stdout: string;
  stderr: string;
  status: RtkStatus;
};

export type RtkInstallResult = {
  installed: boolean;
  version: string;
  binary_path?: string | null;
  stdout: string;
  stderr: string;
  status: RtkStatus;
};

export type AgentSkillInvocation = {
  name: string;
  scope?: string | null;
  scope_label?: string | null;
  framework_id?: string | null;
  framework_label?: string | null;
  source?: string | null;
  path?: string | null;
  byte_count?: number | null;
};

export type UsageWindow = {
  label: string;
  pct?: number | null;
};

export type AgentUsage = {
  provider: string;
  supported: boolean;
  raw: string;
  windows: UsageWindow[];
};

export type AgentStreamEvent = {
  session_id: number;
  kind: string;
  content: string;
  raw_json?: string | null;
  message?: AgentMessage | null;
};

export type AgentStatusEvent = {
  session: AgentSession;
};

export type WorkflowStage = {
  id: string;
  label: string;
  command: string;
  state: WorkflowStageState;
  description: string;
};

export type WorkflowStageState = "ready" | "active" | "complete" | "pending" | "blocked";

export type DwCommand = {
  name: string;
  command: string;
  relative_path: string;
  title: string;
  description?: string | null;
};

export type DwSkill = {
  name: string;
  description?: string | null;
  kind?: string | null;
  tier?: string | null;
  owner?: string | null;
  trigger?: string | null;
  path?: string | null;
  source: "bundled" | "custom" | string;
};

export type WorkspaceSkillTarget = "workspace" | "codex" | "claude" | "copilot";

export type WorkspaceSkill = {
  name: string;
  description?: string | null;
  source: string;
  path?: string | null;
  scope?: "project" | "workspace" | "home" | string;
  scope_label?: string;
  bundled?: boolean;
  owner?: string | null;
  kind?: string | null;
  tier?: string | null;
  group?: string | null;
  framework_id?: string | null;
  framework_label?: string | null;
  exportable?: boolean;
  priority?: number;
  installed_targets: WorkspaceSkillTarget[] | string[];
  file_count: number;
  byte_count: number;
};

export type WorkspaceSkillSearchResult = {
  name: string;
  package: string;
  description?: string | null;
  raw: string;
};

export type WorkspaceFramework = {
  id: string;
  label: string;
  description: string;
  source: string;
  installed: boolean;
  installable: boolean;
  required: boolean;
  flow_id?: string | null;
};

export type WorkspaceCapabilities = {
  skills: WorkspaceSkill[];
  frameworks: WorkspaceFramework[];
};

export type WorkspaceSolutionManifest = {
  schema_version: string;
  exported_at: string;
  workspace: { name: string; metadata_path?: string | null };
  projects?: WorkspaceSolutionProject[];
  machines?: WorkspaceSolutionMachine[];
  skills: Array<{
    name: string;
    path: string;
    file_count: number;
    byte_count: number;
    framework_id?: string | null;
    framework_label?: string | null;
  }>;
  flows: { files: string[] };
};

export type WorkspaceSolutionMachine = {
  display_name: string;
  provider: string;
  provider_runtime: string;
  provider_profile: string;
  preset_id: string;
  image_family: string;
  status: string;
  web_port?: number | null;
  rdp_port?: number | null;
  ssh_port?: number | null;
  last_health_status?: string | null;
  last_health_summary?: string | null;
  updated_at: string;
};

export type WorkspaceSolutionProject = {
  name: string;
  path_hint: string;
  remote_url?: string | null;
  remotes: Array<{ name: string; url: string }>;
  branch?: string | null;
  upstream?: string | null;
};

export type WorkspaceSolutionImportReport = {
  workspace: Workspace;
  manifest: WorkspaceSolutionManifest;
  projects: Array<{
    name: string;
    remote_url?: string | null;
    path?: string | null;
    status: string;
    message: string;
  }>;
};

export type WorkflowGate = {
  label: string;
  state: string;
  path?: string | null;
  detail: string;
};

export type WorkflowResumeEntry = {
  kind: string;
  label: string;
  command: string;
  path: string;
  status: string;
};

export type WorkflowStateSummary = {
  stages: Array<{
    id: string;
    label: string;
    command: string;
    state: WorkflowStageState | string;
    detail: string;
  }>;
  gates: WorkflowGate[];
  resume_entries: WorkflowResumeEntry[];
};
