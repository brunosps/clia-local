import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import CodeMirror from "@uiw/react-codemirror";
import { listen } from "@tauri-apps/api/event";
import {
  Archive,
  Bot,
  Boxes,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronsDownUp,
  FilePlus,
  FolderGit2,
  FolderPlus,
  ChevronRight,
  CircleDot,
  Code2,
  Columns3,
  Copy,
  Download,
  Eye,
  FileText,
  Folder,
  FolderOpen,
  ListChecks,
  Pencil,
  GitFork,
  GitBranch,
  GitCommitHorizontal,
  GitPullRequestArrow,
  History,
  Play,
  Plus,
  RefreshCw,
  Search,
  Send,
  Settings,
  Sparkles,
  Square,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";
import { artifactCounts, artifactLanguage, formatArtifactSize } from "./artifacts";
import {
  agentSessionsForProfile,
  agentStatusLabel,
  hasRunningAgentSession,
  isAgentRunning,
  resolveActiveAgentSession,
  shouldAppendAgentMessage,
  upsertAgentSession,
} from "./agents";
import {
  evidenceCounts,
  evidenceKindLabel,
  evidenceStatusLabel,
  evidenceSummary,
  parseEvidenceLinks,
} from "./evidence";
import { type InterviewOptionKey, type InterviewQuestion } from "./interview";
import {
  flowInterviewPrompt,
  parseFlowInterviewResponse,
  type FlowInterviewTurn,
} from "./flowInterview";
import {
  analyzeInterviewPrompt,
  analyzeAnswerPrompt,
  parseAnalyzeInterviewResponse,
} from "./analyzeInterview";
import {
  suggestionInterviewPrompt,
  suggestionAnswerPrompt,
  parseSuggestionInterviewResponse,
  type SuggestionItem,
} from "./suggestionInterview";
import {
  buildLocalProjectBlueprintPlan,
  buildProjectBlueprintPrompt,
  parseProjectBlueprintAgentResponse,
  projectBlueprintQuestionBatch,
  PROJECT_BLUEPRINT_BATCH_SIZE,
  PROJECT_BLUEPRINT_QUESTION_BANK,
  type ProjectBlueprintAnswer,
  type ProjectBlueprintQuestion,
} from "./projectBlueprint";
import {
  aiCommitProfileKey,
  buildCommitMessagePrompt,
  cleanCommitMessage,
  composeCommitMessage,
  latestAssistantCommitMessage,
  latestSystemMessage,
  resolveAiCommitProfileId,
  splitCommitMessage,
} from "./aiCommit";
import {
  buildFileTree,
  canStageHunks,
  countPatchAreas,
  fileActionLabel,
  fileTreeDirPaths,
  formatPatchStats,
  groupChangedFiles,
  hunkActionLabel,
  patchAreaLabel,
  type FileTreeNode,
} from "./patches";
import { formatSourceSize, monacoLanguage, sourceLanguage, type SourceLanguage } from "./source";
import { EditorTabs } from "./source/EditorTabs";
import { FileHistoryPanel } from "./source/FileHistoryPanel";
import { fileIcon } from "./source/fileIcons";
import { parsePatchToBlocks, tokenize } from "./diff";
import cliaMarkUrl from "./assets/brand/clia-dev-mark.svg";
import cliaSplashLogoUrl from "./assets/brand/clia-dev-splash.svg";
import packageInfo from "../package.json";
import {
  ContextMenu,
  useConfirm,
  useContextMenu,
  useNotice,
  usePrompt,
  type MenuItem,
} from "./ContextMenu";
import { QuickSwitch } from "./QuickSwitch";
import { FilePalette } from "./FilePalette";
import { SearchPanel } from "./SearchPanel";
import { externalFormatterLanguage, formatWithPrettier, prettierParser } from "./format";
import { fileUriFor, lspController } from "./lsp/controller";
import { translate, useI18n, type Locale, type TranslationKey } from "./i18n";
import { filterCommits } from "./commitSearch";
import {
  gitAutoFetchKey,
  gitSnapshotCacheKey,
  parseGitSnapshotCache,
  reconcileChangedFileSelection,
  resolveAutoFetchRemote,
  serializeGitSnapshotCache,
  shouldAutoFetch,
  shouldLoadWorktreeSnapshot,
} from "./gitSnapshot";
import {
  clampNumberPreference,
  normalizeWorkspaceAccentColor,
  parseGitWorkbenchPreference,
  parseSourceWorkspacePreference,
  parseTabPreference,
  parseThemePreference,
  projectUiPreferenceKey,
  serializeGitWorkbenchPreference,
  serializeSourceWorkspacePreference,
  WORKSPACE_COLOR_PRESETS,
  workspaceAccentCssVariables,
  workspaceUiPreferenceKey,
  type SourceSideTabPreference,
  type ThemeMode,
} from "./uiPreferences";
import { gravatarUrl } from "./gravatar";
import { AgentMarkdown, AgentMessageContent } from "./agentMarkdown";
import {
  applySkillAutocomplete,
  composeSkillPrompt,
  filterSkillSuggestions,
  groupSkillSuggestions,
  isSkillSlashCommand,
  resolveSkillSlashCommand,
  skillAutocompleteQuery,
  workspaceSkillFilePath,
} from "./workspaceSkills";
import { api } from "./tauri";
import type {
  AgentMessage,
  AgentMetricEvent,
  AgentProviderHealth,
  AgentProfile,
  AgentProvider,
  AgentRunEvent,
  AgentSession,
  AgentSkillInvocation,
  AgentStatusEvent,
  AgentUsage,
  AgentStreamEvent,
  AttachmentPreview,
  BlameLine,
  Branch,
  ChangedFile,
  Commit,
  CommitDetail,
  CommitFile,
  DwCommand,
  DwArtifact,
  DwSkill,
  EvidenceEntry,
  FilePatch,
  GitRepoSnapshot,
  WorktreeCounts,
  LspServerStatus,
  PatchCheckResult,
  PatchHunk,
  PreflightReport,
  Project,
  KnowledgeSource,
  ProjectBlueprint,
  ProjectBlueprintMaterialization,
  RebaseAction,
  RebaseStep,
  RemoteBranch,
  RepoState,
  Submodule,
  RtkStatus,
  SourceEntry,
  SourceFile,
  StashEntry,
  TagEntry,
  WorkflowStateSummary,
  Workspace,
  WorkspaceSkill,
  WorkspaceSkillTarget,
  WorkspaceSolutionImportReport,
  RequirementCard,
  ChecklistItem,
  RequirementAttachment,
} from "./types";
import { computeGraph } from "./gitGraph";
import {
  composeDwPlanCommand,
  stageCounts,
  workflowStages,
  workflowStateLabel,
  type DwPlanMode,
} from "./workflow";
import {
  DEFAULT_WORKBENCH_SCHEMA,
  parseWorkbenchSchema,
  type StageKind,
  type WorkbenchAction,
  type WorkbenchPhase,
  type WorkbenchSchema,
} from "./workbenchSchema";
import { loadFlowRegistry, parseFlowIndex, singleFlowRegistry, type FlowRegistry } from "./flows";
import {
  createDefaultWorkspaceRoot,
  defaultWorkspaceName,
  parseStateId,
  pickActiveProject,
  pickActiveWorkspace,
  projectDisplayName,
} from "./workspace";
import {
  buildQueue,
  bucketCanonicalStatus,
  parseChecklist,
  serializeChecklist,
  QUEUE_BUCKETS,
  type QueueBucket,
  type QueueCard,
} from "./queue";

const DeployPackagesPanel = lazy(() => import("./DeployPackagesPanel"));
const MonacoSource = lazy(() =>
  import("./source/MonacoSource").then((module) => ({ default: module.MonacoSource })),
);
const DiffCompare = lazy(() =>
  import("./source/DiffCompare").then((module) => ({ default: module.DiffCompare })),
);
const MarkdownPreview = lazy(() =>
  import("./source/MarkdownPreview").then((module) => ({ default: module.MarkdownPreview })),
);

const defaultProjectPath = import.meta.env.VITE_DEFAULT_PROJECT_PATH || "/home/bruno/code/clia-wks";

type Tab = "queue" | "code" | "git" | "deploy" | "agents" | "settings";
type LocalGitRefreshState = "idle" | "cached" | "checking" | "loading" | "stale" | "error";
type DiffRefreshOptions = {
  background?: boolean;
  autoSelect?: boolean;
  untrackedLimit?: number;
};
const DEFAULT_UNTRACKED_LIMIT = 500;
const EMPTY_PALETTE_COMMANDS: DwCommand[] = [];
const EMPTY_PALETTE_SKILLS: DwSkill[] = [];
type DwArtifactPreview = { relativePath: string; content: string };
type AgentProfileDraft = {
  name: string;
  provider: AgentProvider;
  model: string | null;
  reasoning_effort: string | null;
  sandbox: string;
  context_mode: "auto_lean" | "full";
  rtk_enabled: boolean;
};
const agentProviderOptions: Array<{ value: AgentProvider; label: string; enabled: boolean }> = [
  { value: "codex", label: "Codex", enabled: true },
  { value: "claude", label: "Claude Code", enabled: true },
  { value: "copilot", label: "Copilot", enabled: true },
];

const agentModelOptionsByProvider: Record<
  AgentProvider,
  Array<{ value: string; label: string }>
> = {
  codex: [
    { value: "default", label: "Default do Codex" },
    { value: "gpt-5.5", label: "gpt-5.5" },
    { value: "gpt-5.4", label: "gpt-5.4" },
    { value: "gpt-5.4-mini", label: "gpt-5.4-mini" },
    { value: "gpt-5.4-nano", label: "gpt-5.4-nano" },
    { value: "gpt-5.2", label: "gpt-5.2" },
    { value: "gpt-5.2-codex", label: "gpt-5.2-codex" },
    { value: "gpt-5.1", label: "gpt-5.1" },
    { value: "gpt-5.1-codex", label: "gpt-5.1-codex" },
    { value: "gpt-5.1-codex-max", label: "gpt-5.1-codex-max" },
    { value: "gpt-5.1-codex-mini", label: "gpt-5.1-codex-mini" },
    { value: "gpt-5", label: "gpt-5" },
    { value: "gpt-5-codex", label: "gpt-5-codex" },
    { value: "gpt-5-mini", label: "gpt-5-mini" },
    { value: "gpt-5-nano", label: "gpt-5-nano" },
    { value: "gpt-4.1", label: "gpt-4.1" },
    { value: "gpt-4.1-mini", label: "gpt-4.1-mini" },
    { value: "gpt-4.1-nano", label: "gpt-4.1-nano" },
    { value: "o3", label: "o3" },
    { value: "o4-mini", label: "o4-mini" },
    { value: "custom", label: "Custom" },
  ],
  claude: [
    { value: "default", label: "Default do Claude" },
    { value: "opus", label: "opus (alias)" },
    { value: "sonnet", label: "sonnet (alias)" },
    { value: "haiku", label: "haiku (alias)" },
    { value: "claude-opus-4-7", label: "claude-opus-4-7" },
    { value: "claude-sonnet-4-6", label: "claude-sonnet-4-6" },
    { value: "claude-haiku-4-5", label: "claude-haiku-4-5" },
    { value: "claude-haiku-4-5-20251001", label: "claude-haiku-4-5-20251001" },
    { value: "claude-opus-4-6", label: "claude-opus-4-6" },
    { value: "claude-sonnet-4-5", label: "claude-sonnet-4-5" },
    { value: "claude-sonnet-4-5-20250929", label: "claude-sonnet-4-5-20250929" },
    { value: "claude-opus-4-5", label: "claude-opus-4-5" },
    { value: "custom", label: "Custom" },
  ],
  copilot: [
    { value: "default", label: "auto" },
    { value: "claude-sonnet-4.6", label: "claude-sonnet-4.6" },
    { value: "claude-sonnet-4.5", label: "claude-sonnet-4.5" },
    { value: "claude-haiku-4.5", label: "claude-haiku-4.5" },
    { value: "claude-opus-4.7", label: "claude-opus-4.7" },
    { value: "claude-opus-4.6", label: "claude-opus-4.6" },
    { value: "claude-opus-4.6-fast", label: "claude-opus-4.6-fast" },
    { value: "claude-opus-4.5", label: "claude-opus-4.5" },
    { value: "gpt-5.5", label: "gpt-5.5" },
    { value: "gpt-5.4", label: "gpt-5.4" },
    { value: "gpt-5.3-codex", label: "gpt-5.3-codex" },
    { value: "gpt-5.2-codex", label: "gpt-5.2-codex" },
    { value: "gpt-5.2", label: "gpt-5.2" },
    { value: "gpt-5.4-mini", label: "gpt-5.4-mini" },
    { value: "gpt-5-mini", label: "gpt-5-mini" },
    { value: "gpt-4.1", label: "gpt-4.1" },
    { value: "custom", label: "Custom" },
  ],
};

const agentSandboxOptions = [
  { value: "read-only", label: "read-only" },
  { value: "workspace-write", label: "workspace-write" },
  { value: "danger-full-access", label: "YOLO (danger-full-access)" },
];

const agentContextModeOptions = [
  { value: "auto_lean", label: "Auto Lean" },
  { value: "full", label: "Full context" },
];

const agentEffortOptionsByProvider: Record<
  AgentProvider,
  Array<{ value: string; label: string }>
> = {
  codex: [
    { value: "default", label: "Default do Codex" },
    { value: "none", label: "none" },
    { value: "low", label: "low" },
    { value: "medium", label: "medium" },
    { value: "high", label: "high" },
    { value: "xhigh", label: "xhigh" },
  ],
  claude: [
    { value: "default", label: "Default do Claude" },
    { value: "low", label: "low" },
    { value: "medium", label: "medium" },
    { value: "high", label: "high" },
    { value: "xhigh", label: "xhigh" },
    { value: "max", label: "max" },
  ],
  copilot: [
    { value: "default", label: "Default do Copilot" },
    { value: "none", label: "none" },
    { value: "low", label: "low" },
    { value: "medium", label: "medium" },
    { value: "high", label: "high" },
    { value: "xhigh", label: "xhigh" },
    { value: "max", label: "max" },
  ],
};

const navItems: Array<{ id: Tab; labelKey: TranslationKey; icon: typeof CircleDot }> = [
  { id: "queue", labelKey: "nav.queue", icon: ListChecks },
  { id: "code", labelKey: "nav.code", icon: Code2 },
  { id: "git", labelKey: "nav.git", icon: GitBranch },
  { id: "deploy", labelKey: "nav.deploy", icon: Upload },
  { id: "agents", labelKey: "nav.agents", icon: Bot },
  { id: "settings", labelKey: "nav.settings", icon: Settings },
];

const LSP_ENABLED_KEY = "dw.lspEnabled";
const EDITOR_FONT_SIZE_KEY = "dw.editorFontSize";
const THEME_APP_STATE_KEY = "ui.theme";
const DEFAULT_EDITOR_FONT_SIZE = 15;
const MIN_EDITOR_FONT_SIZE = 10;
const MAX_EDITOR_FONT_SIZE = 28;
const DEFAULT_EXPLORER_WIDTH = 300;
const DEFAULT_GIT_SIDEBAR_WIDTH = 240;
const DEFAULT_PATCH_LIST_WIDTH = 320;
const APP_VERSION = packageInfo.version;
const PANE_WIDTH_BOUNDS = {
  explorer: { min: 200, max: 560 },
  gitSidebar: { min: 200, max: 560 },
  patchList: { min: 200, max: 620 },
};

function clampEditorFontSize(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_EDITOR_FONT_SIZE;
  return Math.min(MAX_EDITOR_FONT_SIZE, Math.max(MIN_EDITOR_FONT_SIZE, Math.round(value)));
}

function readEditorFontSize(): number {
  const stored = Number(localStorage.getItem(EDITOR_FONT_SIZE_KEY));
  return stored ? clampEditorFontSize(stored) : DEFAULT_EDITOR_FONT_SIZE;
}

export function App() {
  const { locale, setLocale, t } = useI18n();
  const [themeMode, setThemeModeState] = useState<ThemeMode>("clia");
  const [activeTab, setActiveTab] = useState<Tab>("queue");
  const [flowRegistry, setFlowRegistry] = useState<FlowRegistry>(() =>
    singleFlowRegistry(DEFAULT_WORKBENCH_SCHEMA),
  );
  const [activeFlowId, setActiveFlowId] = useState<string>("dev-workflow");
  const workbench = flowRegistry.schemas[activeFlowId] ?? DEFAULT_WORKBENCH_SCHEMA;
  const activeFlowMeta = flowRegistry.flows.find((flow) => flow.id === activeFlowId) ?? null;
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [activeWorkspace, setActiveWorkspace] = useState<Workspace | null>(null);
  const activeWorkspaceRef = useRef<Workspace | null>(null);
  const [activeProject, setActiveProject] = useState<Project | null>(null);
  const [projectPath, setProjectPath] = useState(defaultProjectPath);
  const [newWorkspaceName, setNewWorkspaceName] = useState(defaultWorkspaceName);
  const [newWorkspaceRoot, setNewWorkspaceRoot] = useState(
    createDefaultWorkspaceRoot(defaultProjectPath),
  );
  const [workspaceModalOpen, setWorkspaceModalOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [workspaceAccent, setWorkspaceAccent] = useState<{
    workspaceId: number;
    color: string | null;
  } | null>(null);
  const [quickSwitchOpen, setQuickSwitchOpen] = useState(false);
  const [filePaletteOpen, setFilePaletteOpen] = useState(false);
  const [sourceSideTab, setSourceSideTab] = useState<SourceSideTabPreference>("explorer");
  const [sourcePreview, setSourcePreview] = useState(false);
  // One-shot "AI Commit" channel: resolves with the generated message when the
  // agent replies on this dedicated session.
  const aiCommitSessionIdRef = useRef<number | null>(null);
  const aiCommitResolveRef = useRef<((message: string | null) => void) | null>(null);
  const agentSessionsRef = useRef<AgentSession[]>([]);
  const [lspEnabled, setLspEnabled] = useState<boolean>(
    () => localStorage.getItem(LSP_ENABLED_KEY) === "1",
  );
  const [lspStatus, setLspStatus] = useState<LspServerStatus | null>(null);
  const [lspInstalling, setLspInstalling] = useState(false);
  const [searchFocusSeed, setSearchFocusSeed] = useState(0);
  const [revealLine, setRevealLine] = useState<number | null>(null);
  const { menu: headerMenu, open: openHeaderMenu, close: closeHeaderMenu } = useContextMenu();
  const { notice, dialog: noticeDialog } = useNotice();
  const { confirm: appConfirm, dialog: appConfirmDialog } = useConfirm();
  const { prompt: appPrompt, dialog: appPromptDialog } = usePrompt();
  const [cards, setCards] = useState<RequirementCard[]>([]);
  const [cardsLoaded, setCardsLoaded] = useState(false);
  const [cardsError, setCardsError] = useState("");
  const [queueProjectFilter, setQueueProjectFilter] = useState<number | null>(null);
  const [openTaskId, setOpenTaskId] = useState<number | null>(null);
  const [localRegistryChecked, setLocalRegistryChecked] = useState(false);
  const settleAiCommit = useCallback(
    (message: string | null, failure?: string, options: { stopSession?: boolean } = {}) => {
      const resolve = aiCommitResolveRef.current;
      if (!resolve) return false;
      const sessionId = aiCommitSessionIdRef.current;
      aiCommitResolveRef.current = null;
      aiCommitSessionIdRef.current = null;
      if (options.stopSession && sessionId != null) void api.stopAgentSession(sessionId);
      if (failure) {
        void notice({
          title: "AI Commit não gerou mensagem",
          body: failure,
        });
      }
      resolve(message);
      return true;
    },
    [notice],
  );
  // clia-local: load the active workspace's tasks (requirement cards) from the
  // LOCAL store. The board is scoped to the active workspace; projects come from
  // the existing `projects` state.
  const loadWorkspaceTasks = useCallback(async (): Promise<RequirementCard[] | null> => {
    if (!activeWorkspace) {
      setCards([]);
      setCardsLoaded(true);
      return [];
    }
    setCardsError("");
    const result = await api.listRequirementCards(activeWorkspace.id);
    if (!result.ok) {
      setCardsError(result.error);
      setCardsLoaded(true);
      return null;
    }
    setCards(result.value);
    setCardsLoaded(true);
    return result.value;
  }, [activeWorkspace]);
  const [flowBuilder, setFlowBuilder] = useState<{ mode: "new" | "edit"; flowId: string } | null>(
    null,
  );
  const paletteCommands = EMPTY_PALETTE_COMMANDS;
  const paletteSkills = EMPTY_PALETTE_SKILLS;
  const [workspaceSkills, setWorkspaceSkills] = useState<WorkspaceSkill[]>([]);
  const [workspaceSkillsBusy, setWorkspaceSkillsBusy] = useState(false);
  const [workspaceSkillsError, setWorkspaceSkillsError] = useState("");
  const [workspaceImportOpen, setWorkspaceImportOpen] = useState(false);
  const [workspaceImportSource, setWorkspaceImportSource] = useState("");
  const [workspaceImportRoot, setWorkspaceImportRoot] = useState(
    createDefaultWorkspaceRoot(defaultProjectPath),
  );
  const [workspaceImportName, setWorkspaceImportName] = useState("");
  const [workspaceImportReport, setWorkspaceImportReport] =
    useState<WorkspaceSolutionImportReport | null>(null);
  // URL→flow interview: an agent asks H1-H4 questions then emits a flow schema,
  // which seeds the flow builder for review. Workspace-scoped (not per-card).
  const [flowInterviewOpen, setFlowInterviewOpen] = useState(false);
  const [flowInterviewUrl, setFlowInterviewUrl] = useState("");
  const [flowInterview, setFlowInterview] = useState<{
    sessionId: number | null;
    profileId: number | null;
    status: "idle" | "asking" | "generating" | "ready" | "error";
    question: InterviewQuestion | null;
    turns: FlowInterviewTurn[];
    error?: string;
  }>({ sessionId: null, profileId: null, status: "idle", question: null, turns: [] });
  const [flowInterviewMessages, setFlowInterviewMessages] = useState<AgentMessage[]>([]);
  const [flowBuilderSeed, setFlowBuilderSeed] = useState<{
    schemaText: string;
    id: string;
    label: string;
  } | null>(null);
  const flowInterviewSessionIdRef = useRef<number | null>(null);
  const parsedFlowMessageIdRef = useRef<number | null>(null);
  // clia-local: per-project analysis (dw-analyze-project) + opportunities, run via
  // the local agent. `projectAnalyzed` derives from the flow's analyzeMarker on disk.
  const [analysisModalOpen, setAnalysisModalOpen] = useState(false);
  const [projectAnalyzed, setProjectAnalyzed] = useState<boolean | null>(null);
  const [analysisMessages, setAnalysisMessages] = useState<AgentMessage[]>([]);
  const analysisSessionIdRef = useRef<number | null>(null);
  const [attachmentPreview, setAttachmentPreview] = useState<AttachmentPreview | null>(null);
  const [dwArtifactPreview, setDwArtifactPreview] = useState<DwArtifactPreview | null>(null);
  const [knowledgeSources, setKnowledgeSources] = useState<KnowledgeSource[]>([]);
  const [projectBlueprints, setProjectBlueprints] = useState<ProjectBlueprint[]>([]);
  const [projectBlueprintModalOpen, setProjectBlueprintModalOpen] = useState(false);
  const [addProjectModalOpen, setAddProjectModalOpen] = useState(false);
  const [projectBlueprintTitle, setProjectBlueprintTitle] = useState("");
  const [projectBlueprintIdea, setProjectBlueprintIdea] = useState("");
  const [projectBlueprintSourceIds, setProjectBlueprintSourceIds] = useState<number[]>([]);
  const [projectBlueprintBusy, setProjectBlueprintBusy] = useState(false);
  const [projectBlueprintMessages, setProjectBlueprintMessages] = useState<AgentMessage[]>([]);
  const [projectBlueprintInterview, setProjectBlueprintInterview] = useState<{
    blueprint: ProjectBlueprint | null;
    status: "idle" | "asking" | "waiting" | "planned" | "error";
    questions: ProjectBlueprintQuestion[];
    answers: ProjectBlueprintAnswer[];
    currentAnswers: Record<string, string>;
    note?: string;
    error?: string;
  }>({
    blueprint: null,
    status: "idle",
    questions: [],
    answers: [],
    currentAnswers: {},
  });
  const projectBlueprintSessionIdRef = useRef<number | null>(null);
  const parsedProjectBlueprintMessageIdRef = useRef<number | null>(null);
  const [agentProfiles, setAgentProfiles] = useState<AgentProfile[]>([]);
  const [agentSessions, setAgentSessions] = useState<AgentSession[]>([]);
  const [agentMessages, setAgentMessages] = useState<AgentMessage[]>([]);
  const [agentRunMetrics, setAgentRunMetrics] = useState<AgentRunEvent[]>([]);
  const [agentHealthByProfile, setAgentHealthByProfile] = useState<
    Record<number, AgentProviderHealth>
  >({});
  const [activeAgentProfileId, setActiveAgentProfileId] = useState<number | null>(null);
  const [aiCommitProfileId, setAiCommitProfileId] = useState<number | null>(null);
  const [activeAgentSessionId, setActiveAgentSessionId] = useState<number | null>(null);
  const [agentComposer, setAgentComposer] = useState("");
  const [agentBusy, setAgentBusy] = useState(false);
  const [agentError, setAgentError] = useState("");
  const [, setReport] = useState<PreflightReport | null>(null);
  // Legacy text status/graph (kept warm for other views; GitWorkbench fetches its own structured data).
  const [, setGitStatus] = useState("");
  const [, setGitGraph] = useState("");
  const [changedFiles, setChangedFiles] = useState<ChangedFile[]>([]);
  const [selectedChangedFile, setSelectedChangedFile] = useState<ChangedFile | null>(null);
  const [selectedPatch, setSelectedPatch] = useState<FilePatch | null>(null);
  const [diffBusy, setDiffBusy] = useState(false);
  const [localGitRefresh, setLocalGitRefresh] = useState<LocalGitRefreshState>("idle");
  const [worktreeCounts, setWorktreeCounts] = useState<WorktreeCounts | null>(null);
  const [untrackedTruncated, setUntrackedTruncated] = useState(false);
  const [untrackedLimit, setUntrackedLimit] = useState(DEFAULT_UNTRACKED_LIMIT);
  const [importedPatch, setImportedPatch] = useState("");
  const [patchCheck, setPatchCheck] = useState<PatchCheckResult | null>(null);
  const [patchBusy, setPatchBusy] = useState(false);
  const [sourceTree, setSourceTree] = useState<SourceEntry[]>([]);
  const [selectedSourceFile, setSelectedSourceFile] = useState<SourceFile | null>(null);
  const [openFiles, setOpenFiles] = useState<SourceFile[]>([]);
  const [sourceExpandedPaths, setSourceExpandedPaths] = useState<string[]>([]);
  const [sourceContent, setSourceContent] = useState("");
  const [sourceBlame, setSourceBlame] = useState<BlameLine[]>([]);
  const [sourceHistory, setSourceHistory] = useState<Commit[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [editorFontSize, setEditorFontSize] = useState<number>(readEditorFontSize);
  const [explorerWidth, setExplorerWidth] = useState<number>(DEFAULT_EXPLORER_WIDTH);
  const [gitSidebarWidth, setGitSidebarWidth] = useState<number>(DEFAULT_GIT_SIDEBAR_WIDTH);
  const [patchListWidth, setPatchListWidth] = useState<number>(DEFAULT_PATCH_LIST_WIDTH);
  const [timeTravel, setTimeTravel] = useState<{
    sha: string;
    content: string;
    language: string;
    path: string;
  } | null>(null);
  const [compareView, setCompareView] = useState<{
    leftLabel: string;
    rightLabel: string;
    original: string;
    modified: string;
    language: string;
    path: string;
  } | null>(null);
  const [compareBase, setCompareBase] = useState<string | null>(null);
  const activeSourcePathRef = useRef<string | null>(null);
  const selectedChangedFileRef = useRef<ChangedFile | null>(null);
  const worktreeFingerprintRef = useRef<string | null>(null);
  const diffRefreshSeqRef = useRef(0);
  const lspChangeTimer = useRef<number | null>(null);
  const blameChangeTimer = useRef<number | null>(null);
  const sourceBlameSeqRef = useRef(0);
  const [sourceBusy, setSourceBusy] = useState(false);
  const [sourceSaving, setSourceSaving] = useState(false);
  const [busy, setBusy] = useState(false);
  const [registryBusy, setRegistryBusy] = useState(false);
  const [error, setError] = useState("");
  const resettingAgentSessionIds = useRef<Set<number>>(new Set());
  const activeAgentSessionIdRef = useRef<number | null>(null);
  const activeTabPreferenceReadyWorkspaceRef = useRef<number | null>(null);
  const activeFlowPreferenceReadyWorkspaceRef = useRef<number | null>(null);
  const activeAgentProfilePreferenceReadyScopeRef = useRef<string | null>(null);
  const activeAgentSessionPreferenceReadyScopeRef = useRef<string | null>(null);
  const sourcePreferenceReadyProjectRef = useRef<number | null>(null);

  const patchCounts = useMemo(() => countPatchAreas(changedFiles), [changedFiles]);
  const gitPendingCount = worktreeCounts?.total ?? changedFiles.length;
  const sourceDirty = Boolean(selectedSourceFile && sourceContent !== selectedSourceFile.content);
  const activeAgentProfile =
    agentProfiles.find((profile) => profile.id === activeAgentProfileId) ??
    agentProfiles[0] ??
    null;
  const selectedAiCommitProfileId = useMemo(
    () =>
      resolveAiCommitProfileId(agentProfiles, aiCommitProfileId, activeAgentProfile?.id ?? null),
    [agentProfiles, aiCommitProfileId, activeAgentProfile?.id],
  );
  const activeAgentSessions = useMemo(
    () => agentSessionsForProfile(agentSessions, activeAgentProfile?.id ?? null),
    [agentSessions, activeAgentProfile?.id],
  );
  const activeAgentSession = useMemo(
    () =>
      resolveActiveAgentSession(
        agentSessions,
        activeAgentProfile?.id ?? null,
        activeAgentSessionId,
      ),
    [agentSessions, activeAgentProfile?.id, activeAgentSessionId],
  );
  const activeAgentWorking = hasRunningAgentSession(agentSessions);
  const currentPath = projectPath.trim();
  const workspaceAccentColor =
    activeWorkspace && workspaceAccent?.workspaceId === activeWorkspace.id
      ? workspaceAccent.color
      : null;
  const workspaceAccentStyle = useMemo(
    () => workspaceAccentCssVariables(workspaceAccentColor) as CSSProperties,
    [workspaceAccentColor],
  );
  const appShellClassName = [
    "app-shell",
    `theme-${themeMode}`,
    workspaceAccentColor ? "workspace-tinted" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const refreshKnowledgeBase = useCallback(
    async (workspace: Workspace | null, project: Project | null = null) => {
      if (!workspace) {
        setKnowledgeSources([]);
        setProjectBlueprints([]);
        return;
      }
      const [sourcesResult, blueprintsResult] = await Promise.all([
        api.listKnowledgeSources(workspace.id, project?.id ?? null),
        api.listProjectBlueprints(workspace.id),
      ]);
      if (sourcesResult.ok) {
        setKnowledgeSources(sourcesResult.value);
      } else {
        setError(sourcesResult.error);
      }
      if (blueprintsResult.ok) {
        setProjectBlueprints(blueprintsResult.value);
      } else {
        setError(blueprintsResult.error);
      }
    },
    [],
  );

  useEffect(() => {
    let cancelled = false;
    const workspace = activeWorkspace;
    const project = activeProject;
    void (async () => {
      await Promise.resolve();
      if (!cancelled) await refreshKnowledgeBase(workspace, project);
    })();
    return () => {
      cancelled = true;
    };
  }, [activeProject, activeWorkspace, refreshKnowledgeBase]);

  useEffect(() => {
    activeAgentSessionIdRef.current = activeAgentSession?.id ?? null;
  }, [activeAgentSession?.id]);

  useEffect(() => {
    if (!activeWorkspace || !activeProject || !activeAgentProfile) return;
    let disposed = false;
    void api
      .warmAgentRuntime({
        profile_id: activeAgentProfile.id,
        workspace_id: activeWorkspace.id,
        project_id: activeProject.id,
        project_path: activeProject.path,
        scope: "chat",
        provider_session_id:
          activeAgentSession?.provider_session_id ?? activeAgentSession?.codex_session_id ?? null,
      })
      .then((result) => {
        if (disposed) return;
        if (result.ok) {
          setAgentHealthByProfile((current) => ({
            ...current,
            [activeAgentProfile.id]: result.value,
          }));
        } else {
          setAgentHealthByProfile((current) => ({
            ...current,
            [activeAgentProfile.id]: {
              provider: activeAgentProfile.provider,
              ok: false,
              supported: true,
              program: activeAgentProfile.provider,
              version: null,
              message: result.error,
              details: {},
            },
          }));
        }
      });
    return () => {
      disposed = true;
    };
  }, [
    activeWorkspace,
    activeProject,
    activeAgentProfile,
    activeAgentSession?.provider_session_id,
    activeAgentSession?.codex_session_id,
  ]);

  useEffect(() => {
    const workspaceId = activeWorkspace?.id;
    if (workspaceId == null) return;
    let cancelled = false;
    void api.getAppState(workspaceUiPreferenceKey(workspaceId, "accent_color")).then((result) => {
      if (cancelled) return;
      setWorkspaceAccent({
        workspaceId,
        color: normalizeWorkspaceAccentColor(result.ok ? result.value : null),
      });
    });
    return () => {
      cancelled = true;
    };
  }, [activeWorkspace?.id]);

  useEffect(() => {
    let cancelled = false;
    void api.getAppState(THEME_APP_STATE_KEY).then((result) => {
      if (cancelled) return;
      setThemeModeState(parseThemePreference(result.ok ? result.value : null));
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const setThemeMode = useCallback((nextThemeMode: ThemeMode) => {
    setThemeModeState(nextThemeMode);
    void api.setAppState(THEME_APP_STATE_KEY, nextThemeMode);
  }, []);

  // clia-local: load the active workspace's tasks on mount and whenever the
  // active workspace changes (loadWorkspaceTasks depends on activeWorkspace).
  useEffect(() => {
    void loadWorkspaceTasks();
  }, [loadWorkspaceTasks]);

  // clia-local: a project is "analyzed" when the active flow's analyzeMarker
  // (e.g. .dw/rules/index.md) exists on disk. Re-checked on project/flow change.
  async function refreshAnalysisStatus() {
    const analyzeCommand = activeFlowMeta?.analyzeCommand;
    if (!activeProject || !analyzeCommand) {
      setProjectAnalyzed(null);
      return;
    }
    const marker = activeFlowMeta?.analyzeMarker ?? ".dw/rules/index.md";
    const sep = activeProject.path.includes("\\") ? "\\" : "/";
    const markerPath = `${activeProject.path.replace(/[\\/]+$/, "")}${sep}${marker}`;
    const result = await api.readTextFile(markerPath);
    setProjectAnalyzed(result.ok);
  }

  useEffect(() => {
    void refreshAnalysisStatus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeProject?.id, activeFlowId]);

  useEffect(() => {
    selectedChangedFileRef.current = selectedChangedFile;
  }, [selectedChangedFile]);

  useEffect(() => {
    worktreeFingerprintRef.current = null;
  }, [currentPath]);

  useEffect(() => {
    localStorage.setItem(EDITOR_FONT_SIZE_KEY, String(editorFontSize));
  }, [editorFontSize]);

  useEffect(() => {
    const workspaceId = activeWorkspace?.id;
    if (workspaceId == null) {
      activeTabPreferenceReadyWorkspaceRef.current = null;
      return;
    }
    activeTabPreferenceReadyWorkspaceRef.current = null;
    let cancelled = false;
    void api.getAppState(workspaceUiPreferenceKey(workspaceId, "active_tab")).then((result) => {
      if (cancelled) return;
      const stored = result.ok ? parseTabPreference(result.value) : null;
      if (stored) setActiveTab(stored);
      activeTabPreferenceReadyWorkspaceRef.current = workspaceId;
    });
    return () => {
      cancelled = true;
    };
  }, [activeWorkspace?.id]);

  useEffect(() => {
    const workspaceId = activeWorkspace?.id;
    if (workspaceId == null || activeTabPreferenceReadyWorkspaceRef.current !== workspaceId) {
      return;
    }
    void api.setAppState(workspaceUiPreferenceKey(workspaceId, "active_tab"), activeTab);
  }, [activeWorkspace?.id, activeTab]);

  useEffect(() => {
    const workspaceId = activeWorkspace?.id;
    if (
      workspaceId == null ||
      activeFlowPreferenceReadyWorkspaceRef.current !== workspaceId ||
      !flowRegistry.schemas[activeFlowId]
    ) {
      return;
    }
    void api.setAppState(workspaceUiPreferenceKey(workspaceId, "active_flow"), activeFlowId);
  }, [activeWorkspace?.id, activeFlowId, flowRegistry.schemas]);

  // Load the per-project file-explorer width (persisted in app_state).
  useEffect(() => {
    const projectId = activeProject?.id;
    if (projectId == null) return;
    let cancelled = false;
    void (async () => {
      const preferred = await api.getAppState(projectUiPreferenceKey(projectId, "explorer_width"));
      const legacy =
        preferred.ok && preferred.value != null
          ? preferred
          : await api.getAppState(`explorer_width:${projectId}`);
      if (cancelled) return;
      const stored = legacy.ok ? legacy.value : null;
      setExplorerWidth(
        clampNumberPreference(stored, DEFAULT_EXPLORER_WIDTH, PANE_WIDTH_BOUNDS.explorer),
      );
    })();
    return () => {
      cancelled = true;
    };
  }, [activeProject?.id]);

  // Load the per-project Git sidebar width (persisted in app_state).
  useEffect(() => {
    const projectId = activeProject?.id;
    if (projectId == null) return;
    let cancelled = false;
    void (async () => {
      const preferred = await api.getAppState(
        projectUiPreferenceKey(projectId, "git_sidebar_width"),
      );
      const legacy =
        preferred.ok && preferred.value != null
          ? preferred
          : await api.getAppState(`git_sidebar_width:${projectId}`);
      if (cancelled) return;
      const stored = legacy.ok ? legacy.value : null;
      setGitSidebarWidth(
        clampNumberPreference(stored, DEFAULT_GIT_SIDEBAR_WIDTH, PANE_WIDTH_BOUNDS.gitSidebar),
      );
    })();
    return () => {
      cancelled = true;
    };
  }, [activeProject?.id]);

  useEffect(() => {
    const projectId = activeProject?.id;
    if (projectId == null) {
      return;
    }
    let cancelled = false;
    void api.getAppState(projectUiPreferenceKey(projectId, "patch_list_width")).then((result) => {
      if (cancelled) return;
      setPatchListWidth(
        clampNumberPreference(
          result.ok ? result.value : null,
          DEFAULT_PATCH_LIST_WIDTH,
          PANE_WIDTH_BOUNDS.patchList,
        ),
      );
    });
    return () => {
      cancelled = true;
    };
  }, [activeProject?.id]);

  useEffect(() => {
    const projectId = activeProject?.id;
    if (projectId == null) return;
    let cancelled = false;
    void api.getAppState(aiCommitProfileKey(projectId)).then((result) => {
      if (cancelled) return;
      const stored = result.ok && result.value ? Number(result.value) : NaN;
      setAiCommitProfileId(Number.isFinite(stored) ? stored : null);
    });
    return () => {
      cancelled = true;
    };
  }, [activeProject?.id]);

  useEffect(() => {
    const projectId = activeProject?.id;
    if (projectId == null || sourcePreferenceReadyProjectRef.current !== projectId) return;
    void api.setAppState(
      projectUiPreferenceKey(projectId, "source_workspace"),
      serializeSourceWorkspacePreference({
        openPaths: openFiles.map((file) => file.relative_path),
        expandedPaths: sourceExpandedPaths,
        activePath: selectedSourceFile?.relative_path ?? null,
        sideTab: sourceSideTab,
        preview: sourcePreview,
        showHistory,
      }),
    );
  }, [
    activeProject?.id,
    openFiles,
    sourceExpandedPaths,
    selectedSourceFile?.relative_path,
    sourceSideTab,
    sourcePreview,
    showHistory,
  ]);

  useEffect(() => {
    agentSessionsRef.current = agentSessions;
  }, [agentSessions]);


  useEffect(() => {
    const workspaceId = activeWorkspace?.id;
    if (workspaceId == null || activeAgentProfileId == null) return;
    const scope = `${workspaceId}:${activeProject?.id ?? "workspace"}`;
    if (activeAgentProfilePreferenceReadyScopeRef.current !== scope) return;
    const key =
      activeProject?.id != null
        ? projectUiPreferenceKey(activeProject.id, "active_agent_profile")
        : workspaceUiPreferenceKey(workspaceId, "active_agent_profile");
    void api.setAppState(key, String(activeAgentProfileId));
  }, [activeWorkspace?.id, activeProject?.id, activeAgentProfileId]);

  useEffect(() => {
    const workspaceId = activeWorkspace?.id;
    if (workspaceId == null || activeAgentSessionId == null) return;
    const scope = `${workspaceId}:${activeProject?.id ?? "workspace"}`;
    if (activeAgentSessionPreferenceReadyScopeRef.current !== scope) return;
    const key =
      activeProject?.id != null
        ? projectUiPreferenceKey(activeProject.id, "active_agent_session")
        : workspaceUiPreferenceKey(workspaceId, "active_agent_session");
    void api.setAppState(key, String(activeAgentSessionId));
  }, [activeWorkspace?.id, activeProject?.id, activeAgentSessionId]);

  // Surface a friendly hint once if a language server can't be launched.
  useEffect(() => {
    lspController.setErrorHandler((serverLang, message) => {
      setError(`Language server (${serverLang}) indisponível: ${message}`);
    });
  }, []);

  // Reflect the enable-LSP preference into the controller and persist it.
  useEffect(() => {
    lspController.setEnabled(lspEnabled);
    localStorage.setItem(LSP_ENABLED_KEY, lspEnabled ? "1" : "0");
  }, [lspEnabled]);

  // Stop language servers and clear diagnostics when the active project changes.
  useEffect(() => {
    return () => lspController.reset();
  }, [currentPath]);

  // Detect the language-server status for the active file (drives the footer chip).
  const activeLspLanguage = selectedSourceFile
    ? monacoLanguage(selectedSourceFile.relative_path)
    : null;
  useEffect(() => {
    let cancelled = false;
    const language = activeLspLanguage;
    if (!language || !lspController.supports(language)) {
      void Promise.resolve().then(() => {
        if (!cancelled) setLspStatus(null);
      });
      return () => {
        cancelled = true;
      };
    }
    void api.lspServerStatus(language).then((result) => {
      if (!cancelled) setLspStatus(result.ok ? result.value : null);
    });
    return () => {
      cancelled = true;
    };
  }, [activeLspLanguage, lspInstalling]);

  // Ctrl/Cmd+K opens the quick switcher; Ctrl/Cmd+P the go-to-file palette.
  // Capture phase so we win before Monaco/other handlers can swallow the key.
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey) || event.altKey) return;
      const key = event.key.toLowerCase();
      if (event.shiftKey && (key === "f" || event.code === "KeyF")) {
        event.preventDefault();
        event.stopPropagation();
        setActiveTab("code");
        setSourceSideTab("search");
        setSearchFocusSeed((seed) => seed + 1);
      } else if (event.shiftKey) {
        // Other Shift+Ctrl combos are not ours; let them through.
      } else if (key === "k" || event.code === "KeyK") {
        event.preventDefault();
        event.stopPropagation();
        setQuickSwitchOpen(true);
      } else if (key === "p" || event.code === "KeyP") {
        event.preventDefault();
        event.stopPropagation();
        setFilePaletteOpen(true);
      }
    }
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);

  useEffect(() => {
    if (!error) return;
    const handle = window.setTimeout(() => setError(""), 6000);
    return () => window.clearTimeout(handle);
  }, [error]);

  useEffect(() => {
    activeWorkspaceRef.current = activeWorkspace;
  }, [activeWorkspace]);

  const ensureWorkspaceFlowsFromProject = useCallback(
    async (workspace: Workspace | null, projectRoot: string) => {
      if (!workspace) return;
      const workspaceIndex = await api.readWorkspaceFlowArtifact(
        workspace.root_path,
        "flows/index.json",
      );
      if (workspaceIndex.ok) return;

      const projectIndex = await api.readDwArtifact(projectRoot, "flows/index.json");
      if (!projectIndex.ok) return;
      const parsed = parseFlowIndex(projectIndex.value);
      if (!parsed) return;

      await api.writeWorkspaceFlowArtifact(
        workspace.root_path,
        "flows/index.json",
        projectIndex.value,
      );
      await Promise.all(
        parsed.flows.map(async (flow) => {
          const projectFlow = await api.readDwArtifact(projectRoot, `flows/${flow.id}.json`);
          if (projectFlow.ok) {
            await api.writeWorkspaceFlowArtifact(
              workspace.root_path,
              `flows/${flow.id}.json`,
              projectFlow.value,
            );
          }
        }),
      );
    },
    [],
  );

  const restoreSourceWorkspace = useCallback(async (projectId: number, path: string) => {
    const preferenceResult = await api.getAppState(
      projectUiPreferenceKey(projectId, "source_workspace"),
    );
    const preference = parseSourceWorkspacePreference(
      preferenceResult.ok ? preferenceResult.value : null,
    );

    setSourceSideTab(preference.sideTab);
    setSourcePreview(preference.preview);
    setShowHistory(preference.showHistory);
    setSourceExpandedPaths(preference.expandedPaths);
    setOpenFiles([]);
    setSelectedSourceFile(null);
    setSourceContent("");
    setSourceBlame([]);
    setSourceHistory([]);
    activeSourcePathRef.current = null;
    sourceBlameSeqRef.current += 1;

    if (!preference.openPaths.length) {
      sourcePreferenceReadyProjectRef.current = projectId;
      return;
    }

    const loadedFiles: SourceFile[] = [];
    for (const relativePath of preference.openPaths) {
      const fileResult = await api.readSourceFile(path, relativePath);
      if (fileResult.ok) loadedFiles.push(fileResult.value);
    }

    const activeFile =
      loadedFiles.find((file) => file.relative_path === preference.activePath) ??
      loadedFiles[0] ??
      null;
    if (!activeFile) {
      sourcePreferenceReadyProjectRef.current = projectId;
      return;
    }

    setOpenFiles(loadedFiles);
    setSelectedSourceFile(activeFile);
    setSourceContent(activeFile.content);
    activeSourcePathRef.current = activeFile.relative_path;
    void refreshSourceBlame(path, activeFile.relative_path, activeFile.content, { silent: true });
    void api.gitLogFile(path, activeFile.relative_path).then((history) => {
      if (activeSourcePathRef.current === activeFile.relative_path && history.ok) {
        setSourceHistory(history.value);
      }
    });
    void lspController.openFile(
      monacoLanguage(activeFile.relative_path),
      path,
      fileUriFor(`${path}/${activeFile.relative_path}`),
      activeFile.content,
    );
    sourcePreferenceReadyProjectRef.current = projectId;
  }, []);

  const refreshProject = useCallback(
    async (path: string, projectOverride?: Project | null) => {
      const trimmedPath = path.trim();
      if (!trimmedPath) {
        setError("Project path is required.");
        return;
      }
      const project =
        projectOverride ??
        projects.find((item) => item.path === trimmedPath) ??
        (activeProject?.path === trimmedPath ? activeProject : null);
      const projectId = project?.id ?? null;

      setBusy(true);
      setError("");
      sourcePreferenceReadyProjectRef.current = null;

      // Load the workspace's flow registry (.dw/flows/index.json + per-flow
      // schemas), falling back to the legacy single .dw/workbench.json (or the
      // bundled default) as one "dev-workflow" flow.
      const workspace = activeWorkspaceRef.current;
      await ensureWorkspaceFlowsFromProject(workspace, trimmedPath);
      const registry = await loadFlowRegistry(async (relativePath) => {
        if (workspace) {
          const workspaceFlow = await api.readWorkspaceFlowArtifact(
            workspace.root_path,
            relativePath,
          );
          if (workspaceFlow.ok) return workspaceFlow;
        }
        return api.readDwArtifact(trimmedPath, relativePath);
      });
      const flowPreference = workspace
        ? await api.getAppState(workspaceUiPreferenceKey(workspace.id, "active_flow"))
        : null;
      const preferredFlowId = flowPreference?.ok ? flowPreference.value : null;
      setFlowRegistry(registry);
      setActiveFlowId((prev) => {
        const candidate =
          preferredFlowId && registry.schemas[preferredFlowId] ? preferredFlowId : prev;
        return registry.schemas[candidate] ? candidate : registry.defaultFlowId;
      });
      activeFlowPreferenceReadyWorkspaceRef.current = workspace?.id ?? null;

      const [preflight, sources, changed] = await Promise.all([
        api.preflight(trimmedPath),
        api.listSourceTree(trimmedPath),
        api.gitWorktreeSnapshot(trimmedPath, { untrackedLimit: DEFAULT_UNTRACKED_LIMIT }),
      ]);

      if (preflight.ok) setReport(preflight.value);
      else setError(preflight.error);

      if (changed.ok) {
        worktreeFingerprintRef.current = changed.value.fingerprint;
        setWorktreeCounts(changed.value.counts);
        setUntrackedTruncated(changed.value.untracked_truncated);
        setChangedFiles(changed.value.files);
        setSelectedChangedFile(null);
        setSelectedPatch(null);
        setLocalGitRefresh("cached");
      }
      if (sources.ok) {
        setSourceTree(sources.value);
        if (projectId != null) {
          await restoreSourceWorkspace(projectId, trimmedPath);
        } else {
          setSelectedSourceFile(null);
          setOpenFiles([]);
          setSourceExpandedPaths([]);
          setSourceContent("");
          setSourceBlame([]);
          setSourceHistory([]);
          activeSourcePathRef.current = null;
        }
      }
      setBusy(false);
    },
    [activeProject, ensureWorkspaceFlowsFromProject, projects, restoreSourceWorkspace],
  );

  const refreshAgents = useCallback(
    async (workspace: Workspace | null, project: Project | null) => {
      if (!workspace) {
        setAgentProfiles([]);
        setAgentSessions([]);
        setAgentMessages([]);
        setActiveAgentProfileId(null);
        setActiveAgentSessionId(null);
        activeAgentProfilePreferenceReadyScopeRef.current = null;
        activeAgentSessionPreferenceReadyScopeRef.current = null;
        return;
      }
      const preferenceScope = `${workspace.id}:${project?.id ?? "workspace"}`;
      activeAgentProfilePreferenceReadyScopeRef.current = null;
      activeAgentSessionPreferenceReadyScopeRef.current = null;
      const [profilesResult, sessionsResult, profilePreferenceResult, sessionPreferenceResult] =
        await Promise.all([
          api.listAgentProfiles(workspace.id, project?.id ?? null),
          api.listAgentSessions(workspace.id, project?.id ?? null),
          api.getAppState(
            project
              ? projectUiPreferenceKey(project.id, "active_agent_profile")
              : workspaceUiPreferenceKey(workspace.id, "active_agent_profile"),
          ),
          api.getAppState(
            project
              ? projectUiPreferenceKey(project.id, "active_agent_session")
              : workspaceUiPreferenceKey(workspace.id, "active_agent_session"),
          ),
        ]);
      if (profilesResult.ok) {
        const storedProfileId = profilePreferenceResult.ok
          ? parseStateId(profilePreferenceResult.value)
          : null;
        setAgentProfiles(profilesResult.value);
        setActiveAgentProfileId((current) =>
          profilesResult.value.some((profile) => profile.id === storedProfileId)
            ? storedProfileId
            : profilesResult.value.some((profile) => profile.id === current)
              ? current
              : (profilesResult.value[0]?.id ?? null),
        );
        activeAgentProfilePreferenceReadyScopeRef.current = preferenceScope;
      } else {
        setAgentError(profilesResult.error);
      }
      if (sessionsResult.ok) {
        const storedSessionId = sessionPreferenceResult.ok
          ? parseStateId(sessionPreferenceResult.value)
          : null;
        setAgentSessions(sessionsResult.value);
        setActiveAgentSessionId((current) =>
          sessionsResult.value.some((session) => session.id === storedSessionId)
            ? storedSessionId
            : sessionsResult.value.some((session) => session.id === current)
              ? current
              : (sessionsResult.value[0]?.id ?? null),
        );
        activeAgentSessionPreferenceReadyScopeRef.current = preferenceScope;
      } else {
        setAgentError(sessionsResult.error);
      }
    },
    [],
  );

  useEffect(() => {
    if (!activeAgentWorking || !activeWorkspace) return;
    let cancelled = false;
    const workspaceId = activeWorkspace.id;
    const projectId = activeProject?.id ?? null;

    const reconcileRunningSessions = async () => {
      const result = await api.listAgentSessions(workspaceId, projectId);
      if (cancelled || !result.ok) return;
      setAgentSessions(result.value);
      setActiveAgentSessionId((current) =>
        result.value.some((session) => session.id === current)
          ? current
          : (result.value[0]?.id ?? null),
      );
    };

    void reconcileRunningSessions();
    const interval = window.setInterval(() => void reconcileRunningSessions(), 3000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [activeAgentWorking, activeProject?.id, activeWorkspace]);

  const refreshWorkspaceSkills = useCallback(
    async (workspace: Workspace | null, project?: Project | null) => {
      if (!workspace) {
        setWorkspaceSkills([]);
        setWorkspaceSkillsError("");
        return;
      }
      setWorkspaceSkillsBusy(true);
      setWorkspaceSkillsError("");
      const result = await api.listWorkspaceCapabilities(workspace.id, project?.id ?? null);
      setWorkspaceSkillsBusy(false);
      if (result.ok) {
        setWorkspaceSkills(result.value.skills);
      } else {
        setWorkspaceSkillsError(result.error);
      }
    },
    [],
  );

  async function syncWorkspaceSkill(name: string) {
    if (!activeWorkspace) return;
    const targets: WorkspaceSkillTarget[] = ["workspace", "codex", "claude", "copilot"];
    setWorkspaceSkillsBusy(true);
    setWorkspaceSkillsError("");
    const result = await api.syncWorkspaceSkill(activeWorkspace.root_path, name, targets);
    setWorkspaceSkillsBusy(false);
    if (result.ok) {
      setWorkspaceSkills(result.value);
      void refreshWorkspaceSkills(activeWorkspace, activeProject);
      void notice({ title: "Skill sincronizada", body: name });
    } else {
      setWorkspaceSkillsError(result.error);
    }
  }

  async function exportWorkspaceSolution() {
    if (!activeWorkspace) return;
    const destination = await api.pickSavePath(`${activeWorkspace.name}.wksdw`);
    if (!destination.ok) {
      setWorkspaceSkillsError(destination.error);
      return;
    }
    if (!destination.value) return;
    setWorkspaceSkillsBusy(true);
    setWorkspaceSkillsError("");
    const result = await api.exportWorkspaceSolution(activeWorkspace.id, destination.value);
    setWorkspaceSkillsBusy(false);
    if (result.ok) {
      void notice({
        title: "Workspace exportado",
        body: `${result.value.projects?.length ?? 0} projeto(s), ${result.value.skills.length} skill(s), ${result.value.flows.files.length} arquivo(s) de fluxo.`,
      });
    } else {
      setWorkspaceSkillsError(result.error);
    }
  }

  async function importWorkspaceSolution() {
    if (!activeWorkspace) return;
    const selected = await api.pickFiles();
    if (!selected.ok) {
      setWorkspaceSkillsError(selected.error);
      return;
    }
    const source = selected.value[0];
    if (!source) return;
    setWorkspaceSkillsBusy(true);
    setWorkspaceSkillsError("");
    const result = await api.importWorkspaceSolution(activeWorkspace.root_path, source);
    if (result.ok && activeProject) {
      await api.syncWorkspaceFlows(activeWorkspace.root_path, activeProject.path);
      await reloadFlowRegistry(activeProject.path);
    }
    await refreshWorkspaceSkills(activeWorkspace, activeProject);
    setWorkspaceSkillsBusy(false);
    if (result.ok) {
      void notice({
        title: "Workspace importado",
        body: `${result.value.projects?.length ?? 0} projeto(s), ${result.value.skills.length} skill(s), ${result.value.flows.files.length} arquivo(s) de fluxo.`,
      });
    } else {
      setWorkspaceSkillsError(result.error);
    }
  }

  async function pickWorkspaceSolutionSource() {
    setError("");
    const selected = await api.pickFiles();
    if (!selected.ok) {
      setError(selected.error);
      return;
    }
    const source = selected.value[0];
    if (!source) return;
    setWorkspaceImportSource(source);
    const preview = await api.previewWorkspaceSolution(source);
    if (preview.ok) {
      setWorkspaceImportName(preview.value.workspace.name);
      setWorkspaceImportReport(null);
    } else {
      setError(preview.error);
    }
  }

  function openWorkspaceImport() {
    setWorkspaceImportSource("");
    setWorkspaceImportRoot(createDefaultWorkspaceRoot(defaultProjectPath));
    setWorkspaceImportName("");
    setWorkspaceImportReport(null);
    setWorkspaceImportOpen(true);
  }

  async function importWorkspaceFromSolution(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const source = workspaceImportSource.trim();
    const root = workspaceImportRoot.trim();
    if (!source || !root) {
      setError("Informe o arquivo .wksdw e a pasta destino.");
      return;
    }

    setRegistryBusy(true);
    setError("");
    const result = await api.importWorkspaceSolutionAsWorkspace(
      source,
      root,
      workspaceImportName.trim(),
    );
    setRegistryBusy(false);
    if (!result.ok) {
      setError(result.error);
      return;
    }

    setWorkspaceImportReport(result.value);
    setWorkspaces((items) => [
      result.value.workspace,
      ...items.filter((item) => item.id !== result.value.workspace.id),
    ]);
    await selectWorkspace(result.value.workspace);
    void notice({
      title: "Workspace importado",
      body: workspaceImportSummary(result.value),
    });
  }

  const loadRegistry = useCallback(async () => {
    setRegistryBusy(true);
    setError("");
    setLocalRegistryChecked(false);

    const workspaceResult = await api.listWorkspaces();
    if (!workspaceResult.ok) {
      setError(workspaceResult.error);
      setLocalRegistryChecked(true);
      setRegistryBusy(false);
      return;
    }

    const nextWorkspaces = workspaceResult.value;
    const lastWorkspaceState = await api.getAppState("last_workspace_id");
    const lastWorkspaceId = lastWorkspaceState.ok ? parseStateId(lastWorkspaceState.value) : null;
    const selectedWorkspace = pickActiveWorkspace(nextWorkspaces, lastWorkspaceId);

    setWorkspaces(nextWorkspaces);
    setLocalRegistryChecked(true);
    if (!selectedWorkspace) {
      setActiveWorkspace(null);
      activeWorkspaceRef.current = null;
      void refreshWorkspaceSkills(null);
      setActiveProject(null);
      setProjects([]);
      setProjectPath("");
      clearProjectState();
      setRegistryBusy(false);
      return;
    }

    setActiveWorkspace(selectedWorkspace);
    activeWorkspaceRef.current = selectedWorkspace;
    setNewWorkspaceName(selectedWorkspace.name);
    setNewWorkspaceRoot(selectedWorkspace.root_path);

    const projectResult = await api.listProjects(selectedWorkspace.id);
    if (!projectResult.ok) {
      setError(projectResult.error);
      setRegistryBusy(false);
      return;
    }

    setProjects(projectResult.value);
    const lastProjectState = await api.getAppState(`last_project_id:${selectedWorkspace.id}`);
    const lastProjectId = lastProjectState.ok ? parseStateId(lastProjectState.value) : null;
    const selectedProject = pickActiveProject(projectResult.value, lastProjectId);
    setActiveProject(selectedProject);
    void refreshWorkspaceSkills(selectedWorkspace, selectedProject);

    if (selectedProject) {
      setProjectPath(selectedProject.path);
      await refreshProject(selectedProject.path, selectedProject);
    }
    await refreshAgents(selectedWorkspace, selectedProject);

    setRegistryBusy(false);
  }, [refreshAgents, refreshProject, refreshWorkspaceSkills]);

  // Load the registry once on mount. We intentionally exclude `loadRegistry` from
  // the deps: refreshProject() calls setWorkbench() with a fresh schema object on
  // every load, which would otherwise re-fire this effect and refetch forever.
  // Explicit reloads happen via selectWorkspace/selectProject/refresh actions.
  useEffect(() => {
    if (!cardsLoaded) {
      return;
    }
    const handle = window.setTimeout(() => {
      void loadRegistry();
    }, 0);
    return () => window.clearTimeout(handle);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cardsLoaded]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    void listen<AgentStreamEvent>("agent://event", (event) => {
      const nextMessage = event.payload.message;
      const isActiveSession = shouldAppendAgentMessage(
        activeAgentSessionIdRef.current,
        event.payload.session_id,
      );
      const isFlowSession = shouldAppendAgentMessage(
        flowInterviewSessionIdRef.current,
        event.payload.session_id,
      );
      const isProjectBlueprintSession = shouldAppendAgentMessage(
        projectBlueprintSessionIdRef.current,
        event.payload.session_id,
      );
      const isAiCommitSession = shouldAppendAgentMessage(
        aiCommitSessionIdRef.current,
        event.payload.session_id,
      );
      if (
        event.payload.kind === "assistant_delta" &&
        event.payload.content &&
        isActiveSession &&
        !resettingAgentSessionIds.current.has(event.payload.session_id)
      ) {
        setAgentMessages((messages) =>
          appendAgentStreamingDelta(messages, event.payload.session_id, event.payload.content),
        );
      }
      // AI Commit: first assistant reply on the dedicated session resolves the
      // pending promise with the cleaned commit message, then closes the session.
      if (
        nextMessage &&
        nextMessage.role === "assistant" &&
        nextMessage.content.trim() &&
        isAiCommitSession
      ) {
        settleAiCommit(cleanCommitMessage(nextMessage.content), undefined, { stopSession: true });
      }
      if (event.payload.kind === "error" && isAiCommitSession) {
        settleAiCommit(
          null,
          event.payload.content || "O agente terminou com erro antes de gerar a mensagem.",
        );
      }
      if (
        nextMessage &&
        isActiveSession &&
        !resettingAgentSessionIds.current.has(event.payload.session_id)
      ) {
        setAgentMessages((messages) =>
          messages.some((message) => message.id === nextMessage.id)
            ? messages
            : [
                ...messages.filter(
                  (message) =>
                    nextMessage.role !== "assistant" ||
                    message.id !== streamingAssistantMessageId(event.payload.session_id),
                ),
                nextMessage,
              ],
        );
      }
      if (nextMessage && isFlowSession) {
        setFlowInterviewMessages((messages) =>
          messages.some((message) => message.id === nextMessage.id)
            ? messages
            : [...messages, nextMessage],
        );
      }
      if (nextMessage && isProjectBlueprintSession) {
        setProjectBlueprintMessages((messages) =>
          messages.some((message) => message.id === nextMessage.id)
            ? messages
            : [...messages, nextMessage],
        );
      }
      if (
        nextMessage &&
        shouldAppendAgentMessage(analysisSessionIdRef.current, event.payload.session_id)
      ) {
        setAnalysisMessages((messages) =>
          messages.some((message) => message.id === nextMessage.id)
            ? messages
            : [...messages, nextMessage],
        );
      }
      if (event.payload.kind === "error" && isActiveSession) {
        setAgentError(event.payload.content);
      }
      if (event.payload.kind === "error" && isFlowSession) {
        setFlowInterview((state) => ({ ...state, status: "error", error: event.payload.content }));
      }
      if (event.payload.kind === "error" && isProjectBlueprintSession) {
        setProjectBlueprintInterview((state) => ({
          ...state,
          status: "error",
          error: event.payload.content,
        }));
      }
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });

    void listen<AgentStatusEvent>("agent://status", (event) => {
      if (
        event.payload.session.id === aiCommitSessionIdRef.current &&
        ["done", "failed"].includes(event.payload.session.status)
      ) {
        const sessionId = event.payload.session.id;
        const status = event.payload.session.status;
        void api.listAgentMessages(sessionId).then((result) => {
          if (aiCommitSessionIdRef.current !== sessionId || !aiCommitResolveRef.current) return;
          if (!result.ok) {
            settleAiCommit(null, result.error);
            return;
          }
          const message = latestAssistantCommitMessage(result.value);
          if (message) {
            settleAiCommit(message);
            return;
          }
          const failure =
            latestSystemMessage(result.value) ??
            (status === "failed"
              ? "O agente terminou com erro antes de gerar a mensagem."
              : "O agente terminou sem retornar uma mensagem de commit.");
          settleAiCommit(null, failure);
        });
      }
      setAgentSessions((sessions) => upsertAgentSession(sessions, event.payload.session));
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });

    void listen<AgentMetricEvent>("agent://metric", (event) => {
      if (!shouldAppendAgentMessage(activeAgentSessionIdRef.current, event.payload.session_id)) {
        return;
      }
      setAgentRunMetrics((metrics) => [
        ...metrics,
        {
          id: -Date.now() - metrics.length,
          session_id: event.payload.session_id,
          run_id: event.payload.run_id,
          provider: event.payload.provider,
          phase: event.payload.phase,
          elapsed_ms: event.payload.elapsed_ms,
          details_json: JSON.stringify(event.payload.details ?? {}),
          created_at: new Date().toISOString(),
        },
      ]);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [settleAiCommit]);

  useEffect(() => {
    let disposed = false;
    if (!activeAgentSession) {
      window.setTimeout(() => {
        if (!disposed) {
          setAgentMessages([]);
          setAgentRunMetrics([]);
        }
      }, 0);
      return () => {
        disposed = true;
      };
    }
    void api.listAgentMessages(activeAgentSession.id).then((result) => {
      if (disposed) return;
      if (result.ok) setAgentMessages(result.value);
      else if (!resettingAgentSessionIds.current.has(activeAgentSession.id)) {
        setAgentError(result.error);
      }
    });
    void api.listAgentRunMetrics(activeAgentSession.id).then((result) => {
      if (disposed) return;
      if (result.ok) setAgentRunMetrics(result.value);
    });
    return () => {
      disposed = true;
    };
  }, [activeAgentSession]);

  // Route flow-interview agent replies: a question updates the panel; a final
  // flow is validated and seeded into the flow builder for review. State writes
  // are deferred (setTimeout 0) to avoid setState-in-effect.
  useEffect(() => {
    const latest = [...flowInterviewMessages]
      .reverse()
      .find((message) => message.role === "assistant" && message.content.trim());
    if (!latest || parsedFlowMessageIdRef.current === latest.id) return;
    const parsed = parseFlowInterviewResponse(latest.content);
    if (!parsed) return;
    parsedFlowMessageIdRef.current = latest.id;

    if (parsed.state === "question") {
      window.setTimeout(() => {
        setFlowInterview((state) => ({
          ...state,
          status: "asking",
          question: {
            question_number: parsed.question_number,
            question: parsed.question,
            options: parsed.options,
            running_summary: parsed.running_summary,
          },
          error: undefined,
        }));
      }, 0);
      return;
    }

    const result = parseWorkbenchSchema(parsed.flow);
    window.setTimeout(() => {
      if (result.usedDefault) {
        setFlowInterview((state) => ({
          ...state,
          status: "error",
          error: "Fluxo gerado inválido — peça ao agente para corrigir.",
        }));
        return;
      }
      setFlowInterview((state) => ({ ...state, status: "ready", question: null }));
      setFlowInterviewOpen(false);
      setFlowBuilderSeed({
        schemaText: parsed.flow,
        id: parsed.id ?? "",
        label: parsed.label ?? "",
      });
      setFlowBuilder({ mode: "new", flowId: "" });
    }, 0);
  }, [flowInterviewMessages]);

  // Drive the agent-led project-analysis interview from its message stream.
  // Drive the agent-led suggestion interview from its message stream.
  useEffect(() => {
    const latest = [...projectBlueprintMessages]
      .reverse()
      .find((message) => message.role === "assistant" && message.content.trim());
    if (!latest || parsedProjectBlueprintMessageIdRef.current === latest.id) return;
    const parsed = parseProjectBlueprintAgentResponse(latest.content);
    if (!parsed) return;
    parsedProjectBlueprintMessageIdRef.current = latest.id;

    if (parsed.state === "question_batch") {
      window.setTimeout(() => {
        setProjectBlueprintInterview((state) => ({
          ...state,
          status: "asking",
          questions: parsed.questions,
          currentAnswers: {},
          note: parsed.running_summary || state.note,
          error: undefined,
        }));
      }, 0);
      const blueprintId = projectBlueprintInterview.blueprint?.id;
      if (blueprintId) {
        void api.updateProjectBlueprint({
          id: blueprintId,
          status: "interviewing",
          running_summary: parsed.running_summary ?? null,
          detected_subprojects_json: JSON.stringify(parsed.detected_subprojects ?? []),
        });
      }
      return;
    }

    const blueprintId = projectBlueprintInterview.blueprint?.id;
    const answers = projectBlueprintInterview.answers;
    if (blueprintId) {
      void api
        .updateProjectBlueprint({
          id: blueprintId,
          status: "planned",
          answers_json: JSON.stringify(answers),
          running_summary: parsed.running_summary,
          detected_subprojects_json: JSON.stringify(parsed.detected_subprojects),
          prd: parsed.prd,
          techspec: parsed.techspec,
          tasks_json: JSON.stringify(parsed.tasks),
          definition_of_done: parsed.definition_of_done,
        })
        .then((result) => {
          if (result.ok) {
            setProjectBlueprints((items) => [
              result.value,
              ...items.filter((item) => item.id !== result.value.id),
            ]);
            setProjectBlueprintInterview((state) => ({
              ...state,
              blueprint: result.value,
              status: "planned",
              questions: [],
              currentAnswers: {},
              note: parsed.running_summary,
              error: undefined,
            }));
          } else {
            setProjectBlueprintInterview((state) => ({
              ...state,
              status: "error",
              error: result.error,
            }));
          }
        });
    }
    const blueprintSessionId = projectBlueprintSessionIdRef.current;
    if (blueprintSessionId != null) void api.stopAgentSession(blueprintSessionId);
  }, [
    projectBlueprintInterview.answers,
    projectBlueprintInterview.blueprint?.id,
    projectBlueprintMessages,
  ]);

  async function sendAgentPrompt(
    prompt: string,
    sessionId?: number | null,
    profileOverride?: AgentProfile | null,
    options?: { stayOnTab?: boolean },
    scope?: string,
    requirementCardId?: number | null,
  ) {
    if (!activeWorkspace || !activeProject) {
      setError("Selecione um workspace e um projeto antes de chamar um agente.");
      return null;
    }
    let message = prompt;
    let skillInvocation: AgentSkillInvocation | null = null;
    if (isSkillSlashCommand(prompt)) {
      let skills = workspaceSkills;
      if (!skills.length) {
        const skillsResult = await api.listWorkspaceCapabilities(
          activeWorkspace.id,
          activeProject.id,
        );
        if (skillsResult.ok) {
          skills = skillsResult.value.skills;
          setWorkspaceSkills(skillsResult.value.skills);
        }
      }
      const resolved = resolveSkillSlashCommand(prompt, skills);
      if (!resolved) {
        // Unknown slash command; let the selected agent decide how to handle it.
      } else if (!resolved.ok) {
        setAgentError(resolved.error);
        setError(resolved.error);
        return null;
      } else {
        const skillPath = workspaceSkillFilePath(activeWorkspace.root_path, resolved.skill);
        const skillContent = await api.readTextFile(skillPath);
        if (!skillContent.ok) {
          setAgentError(skillContent.error);
          setError(skillContent.error);
          return null;
        }
        skillInvocation = {
          name: resolved.skill.name,
          scope: resolved.skill.scope ?? null,
          scope_label: resolved.skill.scope_label ?? null,
          framework_id: resolved.skill.framework_id ?? null,
          framework_label: resolved.skill.framework_label ?? null,
          source: resolved.skill.source ?? null,
          path: resolved.skill.path ?? null,
          byte_count: resolved.skill.byte_count ?? skillContent.value.length,
        };
        message = composeSkillPrompt(resolved.skill, skillContent.value, resolved.request);
      }
    }
    let profile = profileOverride ?? activeAgentProfile;
    if (!profile) {
      const created = await api.createAgentProfile({
        workspace_id: activeWorkspace.id,
        project_id: activeProject.id,
        name: "Codex",
        provider: "codex",
        model: null,
        reasoning_effort: null,
        sandbox: "read-only",
        context_mode: "auto_lean",
      });
      if (!created.ok) {
        setAgentError(created.error);
        setError(created.error);
        return null;
      }
      profile = created.value;
      setAgentProfiles((profiles) => [created.value, ...profiles]);
      setActiveAgentProfileId(created.value.id);
      activeAgentSessionIdRef.current = null;
      setActiveAgentSessionId(null);
      setAgentMessages([]);
    }

    const targetSessionId =
      sessionId !== undefined
        ? sessionId
        : profileOverride
          ? (resolveActiveAgentSession(agentSessions, profile.id, activeAgentSessionId)?.id ?? null)
          : (activeAgentSession?.id ?? null);

    setAgentBusy(true);
    setAgentError("");
    setError("");
    const result = await api.sendAgentMessage({
      profile_id: profile.id,
      session_id: targetSessionId,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      project_path: activeProject.path,
      requirement_card_id: requirementCardId ?? null,
      scope: scope ?? null,
      message,
      skill: skillInvocation,
    });
    setAgentBusy(false);
    if (!result.ok) {
      setAgentError(result.error);
      setError(result.error);
      return null;
    }
    setAgentSessions((sessions) => upsertAgentSession(sessions, result.value));
    activeAgentSessionIdRef.current = result.value.id;
    setActiveAgentProfileId(profile.id);
    setActiveAgentSessionId(result.value.id);
    if (!options?.stayOnTab) setActiveTab("agents");
    return result.value;
  }

  async function createCodexProfile(draft: AgentProfileDraft) {
    if (!activeWorkspace) {
      setError("Selecione um workspace antes de criar um agente.");
      return;
    }
    setAgentBusy(true);
    setAgentError("");
    const result = await api.createAgentProfile({
      workspace_id: activeWorkspace.id,
      project_id: activeProject?.id ?? null,
      name: draft.name,
      provider: draft.provider,
      model: draft.model,
      reasoning_effort: draft.reasoning_effort,
      sandbox: draft.sandbox,
      context_mode: draft.context_mode,
      rtk_enabled: draft.rtk_enabled,
    });
    setAgentBusy(false);
    if (!result.ok) {
      setAgentError(result.error);
      setError(result.error);
      return;
    }
    setAgentProfiles((profiles) => [result.value, ...profiles]);
    setActiveAgentProfileId(result.value.id);
    activeAgentSessionIdRef.current = null;
    setActiveAgentSessionId(null);
    setAgentMessages([]);
  }

  async function updateCodexProfile(id: number, draft: AgentProfileDraft) {
    setAgentBusy(true);
    setAgentError("");
    const result = await api.updateAgentProfile({
      id,
      name: draft.name,
      provider: draft.provider,
      model: draft.model,
      reasoning_effort: draft.reasoning_effort,
      sandbox: draft.sandbox,
      context_mode: draft.context_mode,
      rtk_enabled: draft.rtk_enabled,
    });
    setAgentBusy(false);
    if (!result.ok) {
      setAgentError(result.error);
      setError(result.error);
      return;
    }
    setAgentProfiles((profiles) =>
      profiles.map((profile) => (profile.id === result.value.id ? result.value : profile)),
    );
    setActiveAgentProfileId(result.value.id);
    if (activeWorkspace) void refreshAgents(activeWorkspace, activeProject);
  }

  async function updateAgentRtk(profile: AgentProfile, enabled: boolean) {
    const result = await api.updateAgentProfile({
      id: profile.id,
      name: profile.name,
      provider: safeAgentProvider(profile.provider),
      model: profile.model ?? null,
      reasoning_effort: profile.reasoning_effort ?? null,
      sandbox: profile.sandbox,
      context_mode: profile.context_mode,
      rtk_enabled: enabled,
    });
    if (!result.ok) {
      setAgentError(result.error);
      setError(result.error);
      return null;
    }
    setAgentProfiles((profiles) =>
      profiles.map((item) => (item.id === result.value.id ? result.value : item)),
    );
    return result.value;
  }

  async function stopAgentSession(sessionId: number) {
    setAgentBusy(true);
    const result = await api.stopAgentSession(sessionId);
    setAgentBusy(false);
    if (result.ok) {
      setAgentSessions((sessions) => upsertAgentSession(sessions, result.value));
    } else {
      setAgentError(result.error);
    }
  }

  async function resetAgentChat() {
    if (!activeWorkspace || !activeProject || !activeAgentProfile) {
      setError("Selecione um workspace, projeto e agente antes de limpar o chat.");
      return;
    }
    const staleSessionIds = agentSessions
      .filter((session) => session.profile_id === activeAgentProfile.id)
      .map((session) => session.id);
    const runningSessionIds = agentSessions
      .filter((session) => session.profile_id === activeAgentProfile.id && isAgentRunning(session))
      .map((session) => session.id);
    staleSessionIds.forEach((sessionId) => resettingAgentSessionIds.current.add(sessionId));
    activeAgentSessionIdRef.current = null;
    setActiveAgentSessionId(null);
    setAgentMessages([]);
    setAgentSessions((sessions) =>
      sessions.filter((session) => session.profile_id !== activeAgentProfile.id),
    );
    setAgentBusy(true);
    setAgentError("");
    await Promise.all(runningSessionIds.map((sessionId) => api.stopAgentSession(sessionId)));
    const result = await api.resetAgentChat({
      profile_id: activeAgentProfile.id,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      project_path: activeProject.path,
    });
    setAgentBusy(false);
    window.setTimeout(() => {
      staleSessionIds.forEach((sessionId) => resettingAgentSessionIds.current.delete(sessionId));
    }, 2000);
    if (!result.ok) {
      setAgentError(result.error);
      setError(result.error);
      void refreshAgents(activeWorkspace, activeProject);
      return;
    }
    setAgentSessions((sessions) => [
      result.value,
      ...sessions.filter((session) => session.profile_id !== activeAgentProfile.id),
    ]);
    activeAgentSessionIdRef.current = result.value.id;
    setActiveAgentSessionId(result.value.id);
    setAgentMessages([]);
  }

  // clia-local: adicionar projeto = clonar do GitHub OU registrar uma pasta local.
  // Reaproveita api.cloneGitProject / api.addLocalProject (backend já existente).
  function openAddProjectModal() {
    if (!activeWorkspace) {
      void notice({
        title: "Nenhum workspace",
        body: "Crie ou selecione um workspace antes de adicionar um projeto.",
      });
      return;
    }
    setAddProjectModalOpen(true);
  }

  async function handleProjectAdded(project: Project) {
    if (activeWorkspace) {
      const result = await api.listProjects(activeWorkspace.id);
      if (result.ok) setProjects(result.value);
    }
    selectProject(project);
    setActiveTab("code");
  }

  function openProjectBlueprintModal(blueprint?: ProjectBlueprint | null) {
    parsedProjectBlueprintMessageIdRef.current = null;
    setProjectBlueprintMessages([]);
    if (blueprint) {
      setProjectBlueprintTitle(blueprint.title);
      setProjectBlueprintIdea(blueprint.idea);
      setProjectBlueprintSourceIds(parseNumberArray(blueprint.knowledge_source_ids_json));
      setProjectBlueprintInterview({
        blueprint,
        status:
          blueprint.status === "planned" || blueprint.status === "materialized"
            ? "planned"
            : "asking",
        questions: projectBlueprintQuestionBatch(
          parseProjectBlueprintAnswers(blueprint.answers_json).length,
        ),
        answers: parseProjectBlueprintAnswers(blueprint.answers_json),
        currentAnswers: {},
        note: blueprint.running_summary || undefined,
      });
    } else {
      setProjectBlueprintTitle("");
      setProjectBlueprintIdea("");
      setProjectBlueprintSourceIds(knowledgeSources.map((source) => source.id));
      setProjectBlueprintInterview({
        blueprint: null,
        status: "idle",
        questions: [],
        answers: [],
        currentAnswers: {},
      });
    }
    setProjectBlueprintModalOpen(true);
  }

  async function requestProjectBlueprintAgent(
    blueprint: ProjectBlueprint,
    answers: ProjectBlueprintAnswer[],
  ) {
    const workspace = activeWorkspace;
    const profile = activeAgentProfile;
    if (!workspace || !profile) {
      const questions = projectBlueprintQuestionBatch(answers.length);
      setProjectBlueprintInterview((state) => ({
        ...state,
        status: questions.length ? "asking" : "planned",
        questions,
        currentAnswers: {},
        note: profile ? state.note : "Sem agente ativo: usando entrevista local.",
      }));
      return;
    }
    setProjectBlueprintInterview((state) => ({ ...state, status: "waiting", error: undefined }));
    const selectedSources = knowledgeSources.filter((source) =>
      projectBlueprintSourceIds.includes(source.id),
    );
    const result = await api.sendAgentMessage({
      profile_id: profile.id,
      session_id: blueprint.agent_session_id ?? projectBlueprintSessionIdRef.current,
      workspace_id: workspace.id,
      project_id: activeProject?.id ?? null,
      scope: "project_blueprint",
      title: `Novo projeto: ${blueprint.title}`,
      project_path: activeProject?.path ?? workspace.root_path,
      message: buildProjectBlueprintPrompt({
        title: blueprint.title,
        idea: blueprint.idea,
        answers,
        knowledgeSources: selectedSources,
      }),
    });
    if (!result.ok) {
      const questions = projectBlueprintQuestionBatch(answers.length);
      setProjectBlueprintInterview((state) => ({
        ...state,
        status: questions.length ? "asking" : "error",
        questions,
        currentAnswers: {},
        error: result.error,
      }));
      return;
    }
    projectBlueprintSessionIdRef.current = result.value.id;
    setProjectBlueprintInterview((state) => ({
      ...state,
      blueprint: { ...blueprint, agent_session_id: result.value.id },
    }));
    void api.updateProjectBlueprint({
      id: blueprint.id,
      status: "interviewing",
      agent_session_id: result.value.id,
      answers_json: JSON.stringify(answers),
      knowledge_source_ids: projectBlueprintSourceIds,
    });
  }

  async function startProjectBlueprintInterview(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const workspace = activeWorkspace;
    if (!workspace) return;
    const title = projectBlueprintTitle.trim();
    if (!title) {
      setError("Informe o nome do novo projeto.");
      return;
    }
    setProjectBlueprintBusy(true);
    const result = await api.createProjectBlueprint({
      workspace_id: workspace.id,
      title,
      idea: projectBlueprintIdea,
      agent_profile_id: activeAgentProfile?.id ?? null,
      knowledge_source_ids: projectBlueprintSourceIds,
    });
    setProjectBlueprintBusy(false);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setProjectBlueprints((items) => [
      result.value,
      ...items.filter((item) => item.id !== result.value.id),
    ]);
    const questions = projectBlueprintQuestionBatch(0);
    setProjectBlueprintInterview({
      blueprint: result.value,
      status: "asking",
      questions,
      answers: [],
      currentAnswers: {},
    });
    await requestProjectBlueprintAgent(result.value, []);
  }

  async function answerProjectBlueprintBatch(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const blueprint = projectBlueprintInterview.blueprint;
    if (!blueprint) return;
    const nextAnswers = [
      ...projectBlueprintInterview.answers,
      ...projectBlueprintInterview.questions
        .map((question) => ({
          ...question,
          answer: projectBlueprintInterview.currentAnswers[question.id]?.trim() ?? "",
        }))
        .filter((answer) => answer.answer),
    ];
    if (nextAnswers.length === projectBlueprintInterview.answers.length) {
      setError("Responda pelo menos uma pergunta do lote.");
      return;
    }
    setProjectBlueprintInterview((state) => ({
      ...state,
      answers: nextAnswers,
      currentAnswers: {},
      questions: [],
      status: "waiting",
    }));
    await api.updateProjectBlueprint({
      id: blueprint.id,
      status: "interviewing",
      answers_json: JSON.stringify(nextAnswers),
      knowledge_source_ids: projectBlueprintSourceIds,
    });
    await requestProjectBlueprintAgent(blueprint, nextAnswers);
  }

  async function finalizeProjectBlueprintLocally() {
    const blueprint = projectBlueprintInterview.blueprint;
    if (!blueprint) return;
    const selectedSources = knowledgeSources.filter((source) =>
      projectBlueprintSourceIds.includes(source.id),
    );
    const plan = buildLocalProjectBlueprintPlan({
      title: blueprint.title,
      idea: blueprint.idea,
      answers: projectBlueprintInterview.answers,
      knowledgeSources: selectedSources,
    });
    const result = await api.updateProjectBlueprint({
      id: blueprint.id,
      status: "planned",
      answers_json: JSON.stringify(projectBlueprintInterview.answers),
      running_summary: plan.running_summary,
      detected_subprojects_json: JSON.stringify(plan.detected_subprojects),
      prd: plan.prd,
      techspec: plan.techspec,
      tasks_json: JSON.stringify(plan.tasks),
      definition_of_done: plan.definition_of_done,
      knowledge_source_ids: projectBlueprintSourceIds,
    });
    if (!result.ok) {
      setProjectBlueprintInterview((state) => ({ ...state, status: "error", error: result.error }));
      return;
    }
    setProjectBlueprints((items) => [
      result.value,
      ...items.filter((item) => item.id !== result.value.id),
    ]);
    setProjectBlueprintInterview((state) => ({
      ...state,
      blueprint: result.value,
      status: "planned",
      questions: [],
      currentAnswers: {},
      note: plan.running_summary,
      error: undefined,
    }));
  }

  async function materializeProjectBlueprint(blueprint: ProjectBlueprint) {
    setProjectBlueprintBusy(true);
    const result = await api.materializeProjectBlueprint(blueprint.id);
    setProjectBlueprintBusy(false);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    applyProjectBlueprintMaterialization(result.value);
    setProjectBlueprintModalOpen(false);
    setActiveTab("queue");
    void notice({
      title: "Projeto materializado",
      body: `${result.value.project.name}: ${result.value.cards.length} card(s) criados.`,
    });
  }

  function applyProjectBlueprintMaterialization(result: ProjectBlueprintMaterialization) {
    setProjectBlueprints((items) => [
      result.blueprint,
      ...items.filter((item) => item.id !== result.blueprint.id),
    ]);
    setProjects((items) => [
      result.project,
      ...items.filter((project) => project.id !== result.project.id),
    ]);
    selectProject(result.project);
    void refreshProject(result.project.path, result.project);
  }

  // AI Commit: ask the agent for a commit message from the current diff and
  // resolve with it (the git panel fills its textarea). Does NOT commit.
  async function generateCommitMessage(profileId?: number | null): Promise<string | null> {
    if (!activeWorkspace || !activeProject) {
      void notice({
        title: "AI Commit indisponível",
        body: "Selecione um workspace e um projeto.",
      });
      return null;
    }
    const profile =
      agentProfiles.find((item) => item.id === profileId) ?? activeAgentProfile ?? null;
    if (!profile) {
      void notice({
        title: "AI Commit indisponível",
        body: "Configure um agente para gerar a mensagem de commit.",
      });
      return null;
    }
    const diff = await api.gitStagedDiff(currentPath);
    if (!diff.ok) {
      void notice({ title: "AI Commit não leu o staged diff", body: diff.error });
      return null;
    }
    const diffText = diff.value.trim();
    if (!diffText) {
      setError("");
      void notice({
        title: "Nada staged para gerar commit",
        body: "Stage pelo menos uma mudança antes de usar AI Commit. A IA gera a mensagem somente a partir do que já está staged.",
      });
      return null;
    }
    const result = await api.sendAgentMessage({
      profile_id: profile.id,
      session_id: null,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      scope: "chat",
      title: `Mensagem de commit: ${projectDisplayName(activeProject)}`,
      project_path: activeProject.path,
      message: buildCommitMessagePrompt(diffText, {
        projectName: projectDisplayName(activeProject),
        stagedFiles: changedFiles
          .filter((file) => file.area === "staged")
          .map((file) => ({
            path: file.path,
            status: file.status,
            additions: file.additions,
            deletions: file.deletions,
          })),
      }),
    });
    if (!result.ok) {
      void notice({ title: `AI Commit (${profile.name}) falhou`, body: result.error });
      return null;
    }
    aiCommitSessionIdRef.current = result.value.id;
    return new Promise<string | null>((resolve) => {
      aiCommitResolveRef.current = resolve;
      window.setTimeout(() => {
        settleAiCommit(
          null,
          "O agente não retornou uma mensagem de commit dentro de 120 segundos.",
          { stopSession: true },
        );
      }, 120000);
    });
  }

  function selectAiCommitProfile(profileId: number | null) {
    setAiCommitProfileId(profileId);
    const projectId = activeProject?.id;
    if (projectId != null && profileId != null) {
      void api.setAppState(aiCommitProfileKey(projectId), String(profileId));
    }
  }

  async function createWorkspace(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = newWorkspaceName.trim();
    const rootPath = newWorkspaceRoot.trim();
    if (!name || !rootPath) {
      setError("Workspace name and root path are required.");
      return;
    }

    setRegistryBusy(true);
    setError("");
    const result = await api.createWorkspace(name, rootPath);
    if (!result.ok) {
      setError(result.error);
      setRegistryBusy(false);
      return;
    }
    setWorkspaces((items) => [
      result.value,
      ...items.filter((item) => item.id !== result.value.id),
    ]);
    await selectWorkspace(result.value);
    setWorkspaceModalOpen(false);
    setRegistryBusy(false);
  }

  // clia-local: create a requirement card in the active workspace. Public IDs are
  // reserved locally by the SQLite store (no cloud sequence).
  async function createQueueCard() {
    const workspace = activeWorkspace ?? workspaces[0] ?? null;
    if (!workspace) {
      await notice({
        title: "Nenhum workspace",
        body: "Crie ou selecione um workspace antes de adicionar tarefas.",
      });
      return;
    }
    // Open the full task modal right away (no separate title-only prompt):
    // create a draft card with a default title and let the user fill every
    // field (title included) in the modal.
    const projectIds = activeProject ? [activeProject.id] : [];
    const result = await api.createRequirementCard(
      workspace.id,
      activeProject?.id ?? null,
      projectIds,
      "Nova tarefa",
      "",
    );
    if (!result.ok) {
      await notice({ title: "Erro ao criar tarefa", body: result.error });
      return;
    }
    await loadWorkspaceTasks();
    // Open the new task so the user can fill description, checklist, projects,
    // priority, prompt and attachments before working on it.
    setOpenTaskId(result.value.id);
  }

  // clia-local: run a task on a local agent. Assembles prompt upstream (TaskModal);
  // streams into the existing agent message state so the modal can show it inline.
  // scope "card_run" links the new session to the card (history) + keeps it out of the Agents tab.
  async function runTaskWithAgent(
    cardId: number,
    prompt: string,
    profile: AgentProfile | null,
  ) {
    return sendAgentPrompt(prompt, null, profile, { stayOnTab: true }, "card_run", cardId);
  }

  async function pickDirectory(setPath: (value: string) => void) {
    setError("");
    const result = await api.pickDirectory();
    if (result.ok) {
      if (result.value) setPath(result.value);
    } else {
      setError(result.error);
    }
  }

  function updateWorkspaceAccentColor(value: string | null) {
    const workspace = activeWorkspace;
    if (!workspace) return;
    const color = normalizeWorkspaceAccentColor(value);
    setWorkspaceAccent({ workspaceId: workspace.id, color });
    void api.setAppState(workspaceUiPreferenceKey(workspace.id, "accent_color"), color ?? "");
  }

  async function readFlowArtifact(projectRoot: string, relativePath: string) {
    if (activeWorkspace) {
      const workspaceFlow = await api.readWorkspaceFlowArtifact(
        activeWorkspace.root_path,
        relativePath,
      );
      if (workspaceFlow.ok) return workspaceFlow;
    }
    return api.readDwArtifact(projectRoot, relativePath);
  }

  async function writeFlowArtifact(projectRoot: string, relativePath: string, content: string) {
    if (activeWorkspace) {
      const workspaceWrite = await api.writeWorkspaceFlowArtifact(
        activeWorkspace.root_path,
        relativePath,
        content,
      );
      if (!workspaceWrite.ok) return workspaceWrite;
      if (projectRoot) {
        const sync = await api.syncWorkspaceFlows(activeWorkspace.root_path, projectRoot);
        if (!sync.ok) return sync;
      }
      return workspaceWrite;
    }
    return api.writeDwArtifact(projectRoot, relativePath, content);
  }

  async function reloadFlowRegistry(projectRoot: string) {
    const registry = await loadFlowRegistry((relativePath) =>
      readFlowArtifact(projectRoot, relativePath),
    );
    setFlowRegistry(registry);
    return registry;
  }

  async function selectWorkspace(workspace: Workspace) {
    setRegistryBusy(true);
    setError("");
    setActiveWorkspace(workspace);
    activeWorkspaceRef.current = workspace;
    setNewWorkspaceName(workspace.name);
    setNewWorkspaceRoot(workspace.root_path);
    void api.setAppState("last_workspace_id", String(workspace.id));

    const projectResult = await api.listProjects(workspace.id);
    if (!projectResult.ok) {
      setError(projectResult.error);
      setRegistryBusy(false);
      return;
    }

    setProjects(projectResult.value);
    const selectedProject = projectResult.value[0] ?? null;
    setActiveProject(selectedProject);
    void refreshWorkspaceSkills(workspace, selectedProject);
    if (selectedProject) {
      selectProject(selectedProject);
      await refreshProject(selectedProject.path, selectedProject);
    } else {
      setProjectPath("");
      clearProjectState();
    }
    await refreshAgents(workspace, selectedProject);
    setRegistryBusy(false);
  }

  // Persist a custom/edited flow: validate JSON, write .dw/flows/<id>.json,
  // merge into the index (preserving currently-loaded flows), reload + switch.
  async function saveFlowSchema(args: {
    id: string;
    label: string;
    schemaText: string;
    analyzeCommand?: string;
    analyzeMarker?: string;
  }) {
    const root = currentPath || activeProject?.path || "";
    if (!root) {
      setError("Selecione um projeto antes de salvar o fluxo.");
      return;
    }
    const id = args.id.trim();
    if (!id || !/^[a-z0-9-]+$/.test(id)) {
      setError("Id do fluxo inválido (use minúsculas, números e hífen).");
      return;
    }
    if (parseWorkbenchSchema(args.schemaText).usedDefault) {
      setError("JSON do fluxo inválido — sem fases válidas.");
      return;
    }

    const schemaWrite = await writeFlowArtifact(root, `flows/${id}.json`, args.schemaText);
    if (!schemaWrite.ok) {
      setError(schemaWrite.error);
      return;
    }

    const indexRead = await readFlowArtifact(root, "flows/index.json");
    const parsedIndex = indexRead.ok ? parseFlowIndex(indexRead.value) : null;
    const baseMetas = parsedIndex?.flows ?? flowRegistry.flows;
    const analyzeCommand = args.analyzeCommand?.trim();
    const analyzeMarker = args.analyzeMarker?.trim();
    const meta = {
      id,
      label: args.label.trim() || id,
      ...(analyzeCommand ? { analyzeCommand } : {}),
      ...(analyzeMarker ? { analyzeMarker } : {}),
    };
    const flows = [...baseMetas.filter((flow) => flow.id !== id), meta];
    const keepDefault =
      parsedIndex?.default && flows.some((flow) => flow.id === parsedIndex.default);
    const index = {
      flows,
      default: keepDefault ? parsedIndex?.default : flows[0].id,
      ...(parsedIndex?.intake ? { intake: parsedIndex.intake } : {}),
    };
    const indexWrite = await writeFlowArtifact(
      root,
      "flows/index.json",
      JSON.stringify(index, null, 2),
    );
    if (!indexWrite.ok) {
      setError(indexWrite.error);
      return;
    }

    await reloadFlowRegistry(root);
    setActiveFlowId(id);
    setFlowBuilder(null);
    setFlowBuilderSeed(null);
  }

  // Start the URL→flow interview: fetch the docs (Rust, with agent WebFetch
  // fallback) and ask the agent to drive an H1-H4 interview toward a flow JSON.
  // clia-local: analysis/opportunities interview — start a fresh agent session or
  // answer the current one. The buffer (analysisMessages) is fed by the agent
  // event listener and parsed in ProjectAnalysisModal.
  async function startAnalysisSession(message: string): Promise<boolean> {
    if (!activeWorkspace || !activeProject || !activeAgentProfile) return false;
    setAnalysisMessages([]);
    analysisSessionIdRef.current = null;
    const result = await api.sendAgentMessage({
      profile_id: activeAgentProfile.id,
      session_id: null,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      scope: "chat",
      title: `Análise: ${projectDisplayName(activeProject)}`,
      project_path: activeProject.path,
      message,
    });
    if (!result.ok) {
      setError(result.error);
      return false;
    }
    analysisSessionIdRef.current = result.value.id;
    return true;
  }

  async function answerAnalysisSession(message: string): Promise<boolean> {
    if (!activeWorkspace || !activeProject || !activeAgentProfile) return false;
    const sessionId = analysisSessionIdRef.current;
    if (!sessionId) return false;
    const result = await api.sendAgentMessage({
      profile_id: activeAgentProfile.id,
      session_id: sessionId,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      scope: "chat",
      title: `Análise: ${projectDisplayName(activeProject)}`,
      project_path: activeProject.path,
      message,
    });
    if (!result.ok) {
      setError(result.error);
      return false;
    }
    return true;
  }

  async function createTasksFromSuggestions(items: SuggestionItem[]): Promise<number> {
    if (!activeWorkspace || !activeProject) return 0;
    let created = 0;
    for (const item of items) {
      const body = item.kind ? `[${item.kind}] ${item.body}` : item.body;
      const result = await api.createRequirementCard(
        activeWorkspace.id,
        activeProject.id,
        [activeProject.id],
        item.title,
        body,
      );
      if (result.ok) created += 1;
    }
    await loadWorkspaceTasks();
    return created;
  }

  async function startFlowInterview(profileId: number, url: string) {
    if (!activeWorkspace || !activeProject) {
      setError("Selecione um workspace e um projeto antes de criar o fluxo.");
      return;
    }
    const trimmedUrl = url.trim();
    if (!trimmedUrl) {
      setError("Informe a URL da documentação da ferramenta.");
      return;
    }
    setFlowInterviewMessages([]);
    parsedFlowMessageIdRef.current = null;
    flowInterviewSessionIdRef.current = null;
    setFlowInterview({ sessionId: null, profileId, status: "asking", question: null, turns: [] });

    const fetched = await api.fetchUrl(trimmedUrl);
    const pageContent = fetched.ok ? fetched.value : undefined;
    const result = await api.sendAgentMessage({
      profile_id: profileId,
      session_id: null,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      scope: "chat",
      title: `Fluxo de URL: ${trimmedUrl}`,
      project_path: activeProject.path,
      message: flowInterviewPrompt({ url: trimmedUrl, pageContent, turns: [] }),
    });
    if (!result.ok) {
      setFlowInterview((state) => ({ ...state, status: "error", error: result.error }));
      return;
    }
    flowInterviewSessionIdRef.current = result.value.id;
    setFlowInterview((state) => ({ ...state, sessionId: result.value.id }));
  }

  async function answerFlowInterview(selected: InterviewOptionKey, note: string) {
    if (!activeWorkspace || !activeProject) return;
    const current = flowInterview.question;
    const profileId = flowInterview.profileId;
    const sessionId = flowInterview.sessionId;
    if (!current || !profileId || !sessionId) {
      setError("Inicie a criação do fluxo escolhendo um agente.");
      return;
    }
    const nextTurns: FlowInterviewTurn[] = [
      ...flowInterview.turns,
      {
        question: current.question,
        selected,
        answer: current.options[selected],
        note: note || undefined,
      },
    ];
    parsedFlowMessageIdRef.current = null;
    setFlowInterview((state) => ({ ...state, status: "asking", question: null, turns: nextTurns }));
    const result = await api.sendAgentMessage({
      profile_id: profileId,
      session_id: sessionId,
      workspace_id: activeWorkspace.id,
      project_id: activeProject.id,
      scope: "chat",
      title: `Fluxo de URL: ${flowInterviewUrl}`,
      project_path: activeProject.path,
      message: flowInterviewPrompt({ url: flowInterviewUrl, turns: nextTurns }),
    });
    if (!result.ok) {
      setFlowInterview((state) => ({ ...state, status: "error", error: result.error }));
    }
  }

  // Route an intake card into a flow: bind flow_id + jump to that flow's first
  // phase, then switch the board to that flow. Pass flowId="" to unroute.

  function selectProject(project: Project) {
    setActiveProject(project);
    setProjectPath(project.path);
    clearProjectState();
    if (activeWorkspace) {
      void api.setAppState(`last_project_id:${activeWorkspace.id}`, String(project.id));
      void refreshWorkspaceSkills(activeWorkspace, project);
    }
    void refreshAgents(activeWorkspace, project);
  }

  function clearProjectState() {
    sourcePreferenceReadyProjectRef.current = null;
    setReport(null);
    setGitStatus("");
    setGitGraph("");
    setChangedFiles([]);
    setSelectedChangedFile(null);
    setSelectedPatch(null);
    setWorktreeCounts(null);
    setUntrackedTruncated(false);
    setUntrackedLimit(DEFAULT_UNTRACKED_LIMIT);
    setLocalGitRefresh("idle");
    setImportedPatch("");
    setPatchCheck(null);
    setSourceTree([]);
    setSelectedSourceFile(null);
    setOpenFiles([]);
    setSourceExpandedPaths([]);
    setSourceContent("");
    setSourceBlame([]);
    setSourceHistory([]);
    setSourceSideTab("explorer");
    setSourcePreview(false);
    setShowHistory(false);
    activeSourcePathRef.current = null;
    sourceBlameSeqRef.current += 1;
    if (lspChangeTimer.current) window.clearTimeout(lspChangeTimer.current);
    if (blameChangeTimer.current) window.clearTimeout(blameChangeTimer.current);
  }

  async function refreshSourceBlame(
    path: string,
    relativePath: string,
    content: string,
    options: { silent?: boolean } = {},
  ) {
    if (activeSourcePathRef.current !== relativePath) return;
    const sequence = ++sourceBlameSeqRef.current;
    const result = await api.gitBlamePorcelainForContent(path, relativePath, content);
    if (sequence !== sourceBlameSeqRef.current || activeSourcePathRef.current !== relativePath) {
      return;
    }
    if (result.ok) {
      setSourceBlame(result.value);
      return;
    }

    const fallback = await api.gitBlamePorcelain(path, relativePath);
    if (sequence !== sourceBlameSeqRef.current || activeSourcePathRef.current !== relativePath) {
      return;
    }
    if (fallback.ok) {
      setSourceBlame(fallback.value);
    } else {
      setSourceBlame([]);
      if (!options.silent) setError(result.error || fallback.error);
    }
  }

  async function openSourcePath(path: string, relativePath: string) {
    setSourceBusy(true);
    setError("");
    if (blameChangeTimer.current) window.clearTimeout(blameChangeTimer.current);
    activeSourcePathRef.current = relativePath;
    setSourceBlame([]);
    setSourceHistory([]);
    setCompareBase(null);
    setRevealLine(null);
    setSourcePreview(false);
    const result = await api.readSourceFile(path, relativePath);
    if (result.ok) {
      const file = result.value;
      setSelectedSourceFile(file);
      setSourceContent(file.content);
      setOpenFiles((files) =>
        files.some((item) => item.relative_path === relativePath)
          ? files.map((item) => (item.relative_path === relativePath ? file : item))
          : [...files, file],
      );
      // Load GitLens blame + file history async; ignore if the user switched files.
      void refreshSourceBlame(path, relativePath, file.content);
      void api.gitLogFile(path, relativePath).then((history) => {
        if (activeSourcePathRef.current === relativePath && history.ok) {
          setSourceHistory(history.value);
        }
      });
      // Hand the file to its language server (no-op when no server is wired/installed).
      void lspController.openFile(
        monacoLanguage(relativePath),
        path,
        fileUriFor(`${path}/${relativePath}`),
        file.content,
      );
    } else {
      setError(result.error);
    }
    setSourceBusy(false);
  }

  async function loadSourceFile(path: string, entry: SourceEntry) {
    if (entry.kind !== "file") return;
    await openSourcePath(path, entry.relative_path);
  }

  // Editor edits: update the buffer immediately, push to the LSP debounced.
  function handleSourceContentChange(content: string) {
    setSourceContent(content);
    const relativePath = activeSourcePathRef.current;
    if (!relativePath) return;
    const uri = fileUriFor(`${currentPath}/${relativePath}`);
    if (lspChangeTimer.current) window.clearTimeout(lspChangeTimer.current);
    lspChangeTimer.current = window.setTimeout(() => {
      lspController.changeFile(uri, content);
    }, 350);
    if (blameChangeTimer.current) window.clearTimeout(blameChangeTimer.current);
    blameChangeTimer.current = window.setTimeout(() => {
      void refreshSourceBlame(currentPath, relativePath, content, { silent: true });
    }, 500);
  }

  // Open a file from a search result and jump to the matched line.
  async function openSourceAt(relativePath: string, line: number) {
    await openSourcePath(currentPath, relativePath);
    setRevealLine(line);
  }

  async function reloadSourceTree() {
    if (!currentPath) return;
    const sources = await api.listSourceTree(currentPath);
    if (sources.ok) setSourceTree(sources.value);
    else setError(sources.error);
  }

  async function createNewSourceFile(relativePath: string) {
    const result = await api.createSourceFile(currentPath, relativePath);
    if (result.ok) {
      await reloadSourceTree();
      await openSourcePath(currentPath, relativePath);
    } else {
      setError(result.error);
    }
  }

  async function createNewSourceDir(relativePath: string) {
    const result = await api.createSourceDir(currentPath, relativePath);
    if (result.ok) await reloadSourceTree();
    else setError(result.error);
  }

  // Beautifier (Alt+Shift+F): Prettier in-process, or an external CLI per dw language.
  async function formatBuffer(text: string, language: string): Promise<string | null> {
    const parser = prettierParser(language);
    if (parser) {
      try {
        return await formatWithPrettier(text, parser);
      } catch (error) {
        setError(`Falha ao formatar: ${error instanceof Error ? error.message : String(error)}`);
        return null;
      }
    }
    if (externalFormatterLanguage(language)) {
      const result = await api.formatExternal(language, text);
      if (result.ok) return result.value;
      setError(result.error);
      return null;
    }
    setError(`Sem formatter para ${language}.`);
    return null;
  }

  // Push the active file into its (now enabled/installed) language server.
  function syncActiveFileToLsp() {
    if (!selectedSourceFile) return;
    const relativePath = selectedSourceFile.relative_path;
    void lspController.openFile(
      monacoLanguage(relativePath),
      currentPath,
      fileUriFor(`${currentPath}/${relativePath}`),
      sourceContent,
    );
  }

  async function installLspServer(language: string) {
    setLspEnabled(true);
    lspController.setEnabled(true);
    setLspInstalling(true);
    const result = await api.lspInstall(language);
    setLspInstalling(false);
    if (result.ok) {
      setLspStatus((status) => (status ? { ...status, installed: true } : status));
      syncActiveFileToLsp();
    } else {
      setError(result.error);
    }
  }

  function enableLsp() {
    setLspEnabled(true);
    lspController.setEnabled(true);
    if (!activeLspLanguage) return;
    if (lspStatus && !lspStatus.installed) {
      if (lspStatus.can_install) void installLspServer(activeLspLanguage);
      else setError(`Instale ${lspStatus.program} (ou seu pré-requisito) para ativar o LSP.`);
    } else {
      syncActiveFileToLsp();
    }
  }

  async function openTimeTravel(sha: string) {
    if (!selectedSourceFile) return;
    const relativePath = selectedSourceFile.relative_path;
    const result = await api.gitShowFile(currentPath, sha, relativePath);
    if (result.ok) {
      setTimeTravel({
        sha,
        content: result.value,
        language: monacoLanguage(relativePath),
        path: relativePath,
      });
    } else {
      setError(result.error);
    }
  }

  // Resolve one side of a compare: "WORKING" = working copy, otherwise a commit sha.
  async function resolveCompareSide(
    id: string,
    relativePath: string,
  ): Promise<{ content: string; label: string } | null> {
    if (id === "WORKING") {
      return { content: selectedSourceFile?.content ?? "", label: "Cópia de trabalho" };
    }
    const result = await api.gitShowFile(currentPath, id, relativePath);
    if (!result.ok) {
      setError(result.error);
      return null;
    }
    return { content: result.value, label: id.slice(0, 7) };
  }

  // First pick marks the base (A); second pick opens the A↔B diff.
  function pickCompare(id: string) {
    if (compareBase === null) {
      setCompareBase(id);
      return;
    }
    if (id === compareBase) {
      setCompareBase(null);
      return;
    }
    void openCompare(compareBase, id);
    setCompareBase(null);
  }

  async function openCompare(a: string, b: string) {
    if (!selectedSourceFile) return;
    const relativePath = selectedSourceFile.relative_path;
    const [left, right] = await Promise.all([
      resolveCompareSide(a, relativePath),
      resolveCompareSide(b, relativePath),
    ]);
    if (!left || !right) return;
    setCompareView({
      leftLabel: left.label,
      rightLabel: right.label,
      original: left.content,
      modified: right.content,
      language: monacoLanguage(relativePath),
      path: relativePath,
    });
  }

  function closeSourceTab(relativePath: string) {
    lspController.closeFile(fileUriFor(`${currentPath}/${relativePath}`));
    const remaining = openFiles.filter((item) => item.relative_path !== relativePath);
    setOpenFiles(remaining);
    if (selectedSourceFile?.relative_path === relativePath) {
      const neighbor = remaining[remaining.length - 1];
      if (neighbor) {
        void openSourcePath(currentPath, neighbor.relative_path);
      } else {
        setSelectedSourceFile(null);
        setSourceContent("");
      }
    }
  }

  async function saveSourceFile(path: string, sourceFile: SourceFile | null) {
    if (!sourceFile) return;

    // Use a dedicated saving flag (not sourceBusy) so the editor content/cursor
    // is never blanked and reset on save.
    setSourceSaving(true);
    setError("");
    const result = await api.writeSourceFile(path, sourceFile.relative_path, sourceContent);
    if (result.ok) {
      setSelectedSourceFile(result.value);
      setOpenFiles((files) =>
        files.map((file) =>
          file.relative_path === result.value.relative_path ? result.value : file,
        ),
      );
      // Only reconcile if the bytes on disk differ from the buffer (avoids a
      // needless model replace that would move the cursor).
      if (result.value.content !== sourceContent) {
        setSourceContent(result.value.content);
      }
      void refreshDiffReview(path, { background: true, autoSelect: false });
    } else {
      setError(result.error);
    }
    setSourceSaving(false);
  }

  async function refreshDiffReview(path: string, options: DiffRefreshOptions = {}) {
    if (!path) return;
    const requestId = ++diffRefreshSeqRef.current;
    const background = Boolean(options.background);
    const autoSelect = options.autoSelect ?? !background;
    const nextUntrackedLimit = options.untrackedLimit ?? untrackedLimit;

    if (background) {
      setLocalGitRefresh("checking");
      const fingerprint = await api.gitWorktreeFingerprint(path);
      if (requestId !== diffRefreshSeqRef.current) return;
      if (!fingerprint.ok) {
        setLocalGitRefresh("error");
        setError(fingerprint.error);
        return;
      }
      setWorktreeCounts(fingerprint.value.counts);
      if (
        !shouldLoadWorktreeSnapshot(worktreeFingerprintRef.current, fingerprint.value.fingerprint)
      ) {
        setLocalGitRefresh("cached");
        return;
      }
      setLocalGitRefresh("stale");
    } else {
      setLocalGitRefresh("loading");
    }

    const changed = await api.gitWorktreeSnapshot(path, { untrackedLimit: nextUntrackedLimit });
    if (requestId !== diffRefreshSeqRef.current) return;
    if (changed.ok) {
      worktreeFingerprintRef.current = changed.value.fingerprint;
      setWorktreeCounts(changed.value.counts);
      setUntrackedTruncated(changed.value.untracked_truncated);
      setChangedFiles(changed.value.files);
      const nextSelected = reconcileChangedFileSelection({
        files: changed.value.files,
        selected: selectedChangedFileRef.current,
        autoSelect,
      });
      setSelectedChangedFile(nextSelected);
      selectedChangedFileRef.current = nextSelected;
      if (nextSelected) {
        await loadFilePatch(path, nextSelected, {
          reconcileEmpty: !background,
          showBusy: !background,
        });
      } else {
        setSelectedPatch(null);
      }
      setLocalGitRefresh("cached");
    } else {
      setLocalGitRefresh("error");
      setError(changed.error);
    }
  }

  async function loadFilePatch(
    path: string,
    file: ChangedFile,
    options: { reconcileEmpty?: boolean; showBusy?: boolean } = {},
  ) {
    const { reconcileEmpty = true, showBusy = true } = options;
    if (showBusy) setDiffBusy(true);
    setError("");
    const result = await api.readFilePatch(path, file.path, file.area);
    if (result.ok) {
      setSelectedChangedFile(file);
      selectedChangedFileRef.current = file;
      setSelectedPatch(result.value);
      // Empty patch for a "changed" file means the list is stale (e.g. it was
      // committed outside the app) — reconcile it instead of showing nothing.
      if (reconcileEmpty && !result.value.patch && result.value.hunks.length === 0) {
        if (showBusy) setDiffBusy(false);
        await refreshDiffReview(path);
        return;
      }
    } else {
      setError(result.error);
    }
    if (showBusy) setDiffBusy(false);
  }

  // Pick up changes made outside the app (e.g. a terminal commit) when the
  // window regains focus, so the Local Changes list never goes stale.
  useEffect(() => {
    let handle: number | null = null;
    const onFocus = () => {
      if (!currentPath || activeTab !== "git") return;
      if (handle != null) window.clearTimeout(handle);
      handle = window.setTimeout(() => {
        void refreshDiffReview(currentPath, { background: true, autoSelect: false });
      }, 150);
    };
    window.addEventListener("focus", onFocus);
    return () => {
      if (handle != null) window.clearTimeout(handle);
      window.removeEventListener("focus", onFocus);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentPath, activeTab, untrackedLimit]);

  async function toggleFileStage(file: ChangedFile) {
    setDiffBusy(true);
    setError("");
    const result =
      file.area === "staged"
        ? await api.unstageFile(currentPath, file.path)
        : await api.stageFile(currentPath, file.path);
    if (!result.ok) setError(result.error);
    await refreshDiffReview(currentPath);
    setDiffBusy(false);
  }

  async function toggleHunkStage(file: ChangedFile, hunk: PatchHunk) {
    setDiffBusy(true);
    setError("");
    const result =
      file.area === "staged"
        ? await api.unstageHunk(currentPath, hunk.patch)
        : await api.stageHunk(currentPath, hunk.patch);
    if (result.ok) {
      if (!result.value.ok) setError(result.value.output || "Git could not apply this hunk.");
    } else {
      setError(result.error);
    }
    await refreshDiffReview(currentPath);
    setDiffBusy(false);
  }

  async function stageAllChanges() {
    setDiffBusy(true);
    setError("");
    const result = await api.gitStageAll(currentPath);
    if (!result.ok) setError(result.error);
    await refreshDiffReview(currentPath);
    setDiffBusy(false);
  }

  async function unstageAllChanges() {
    setDiffBusy(true);
    setError("");
    const result = await api.gitUnstageAll(currentPath);
    if (!result.ok) setError(result.error);
    await refreshDiffReview(currentPath);
    setDiffBusy(false);
  }

  async function discardChangedFile(file: ChangedFile) {
    const ok = await appConfirm({
      title: "Descartar mudanças?",
      body: `Descartar todas as mudanças de ${file.path}? Esta ação não pode ser desfeita.`,
      confirmLabel: "Descartar",
      danger: true,
    });
    if (!ok) return;
    setDiffBusy(true);
    setError("");
    try {
      const result = await api.gitDiscardFile(currentPath, file.path);
      if (!result.ok) setError(result.error);
      await refreshDiffReview(currentPath);
    } finally {
      setDiffBusy(false);
    }
  }

  async function discardChangedHunk(file: ChangedFile, hunk: PatchHunk) {
    const ok = await appConfirm({
      title: "Descartar hunk?",
      body: `Descartar este trecho de ${file.path}? Esta ação não pode ser desfeita.`,
      confirmLabel: "Descartar",
      danger: true,
    });
    if (!ok) return;
    setDiffBusy(true);
    setError("");
    try {
      const result = await api.gitDiscardHunk(currentPath, hunk.patch);
      if (result.ok) {
        if (!result.value.ok)
          setError(result.value.output || "Git não conseguiu descartar este hunk.");
      } else {
        setError(result.error);
      }
      await refreshDiffReview(currentPath);
    } finally {
      setDiffBusy(false);
    }
  }

  async function openChangedFile(file: ChangedFile, showHistoryPanel = false) {
    setActiveTab("code");
    setSourceSideTab("explorer");
    setShowHistory(showHistoryPanel);
    await openSourcePath(currentPath, file.path);
  }

  async function revealChangedFile(file: ChangedFile) {
    const result = await api.revealProjectFile(currentPath, file.path);
    if (!result.ok) {
      void notice({ title: "Não foi possível revelar o arquivo", body: result.error });
    }
  }

  async function openExternalDiff(file: ChangedFile) {
    const result = await api.gitExternalDiff(currentPath, file.path, file.area);
    if (!result.ok) {
      void notice({ title: "External Diff não abriu", body: result.error });
    }
  }

  async function stashChangedFile(file: ChangedFile) {
    const message = await appPrompt({
      title: "Stash 1 arquivo",
      label: "Mensagem do stash",
      initial: `WIP ${file.path}`,
      confirmLabel: "Stash",
    });
    if (!message) return;
    setDiffBusy(true);
    try {
      const result = await api.gitStashFile(currentPath, file.path, message);
      if (!result.ok) {
        void notice({ title: "Stash do arquivo falhou", body: result.error });
      }
      await refreshDiffReview(currentPath);
    } finally {
      setDiffBusy(false);
    }
  }

  async function ignoreChangedFile(file: ChangedFile, target: "info_exclude" | "gitignore") {
    const result = await api.gitIgnoreFile(currentPath, file.path, target);
    if (!result.ok) {
      void notice({ title: "Não foi possível ignorar o arquivo", body: result.error });
      return;
    }
    await refreshDiffReview(currentPath);
  }

  async function saveChangedFilePatch(file: ChangedFile) {
    const patch = await api.gitFilePatchText(currentPath, file.path, file.area);
    if (!patch.ok) {
      void notice({ title: "Não foi possível gerar o patch", body: patch.error });
      return;
    }
    const filename = `${file.path.split(/[\\/]/).pop() ?? "change"}.patch`;
    const destination = await api.pickSavePath(filename);
    if (!destination.ok) {
      void notice({ title: "Não foi possível escolher o destino", body: destination.error });
      return;
    }
    if (!destination.value) return;
    const written = await api.writeTextFile(destination.value, patch.value);
    if (!written.ok) {
      void notice({ title: "Não foi possível salvar o patch", body: written.error });
    }
  }

  function copyChangedFilePath(file: ChangedFile, fullPath: boolean) {
    const value = fullPath ? `${currentPath.replace(/[\\/]$/, "")}/${file.path}` : file.path;
    void navigator.clipboard?.writeText(value);
  }

  function changedFileMenuItems(file: ChangedFile): MenuItem[] {
    const isStaged = file.area === "staged";
    const canIgnore = !isStaged && file.status === "??";
    const canOpen = !file.status.includes("D");
    return [
      {
        label: "Open",
        icon: <Eye aria-hidden="true" size={14} />,
        shortcut: "Ctrl+Shift+Alt+O",
        disabled: !canOpen,
        onSelect: () => void openChangedFile(file),
      },
      {
        label: "External Diff",
        icon: <Columns3 aria-hidden="true" size={14} />,
        onSelect: () => void openExternalDiff(file),
      },
      {
        label: "Show in File Explorer",
        icon: <FolderOpen aria-hidden="true" size={14} />,
        onSelect: () => void revealChangedFile(file),
      },
      { separator: true },
      {
        label: "Blame/Timeline...",
        icon: <History aria-hidden="true" size={14} />,
        disabled: !canOpen,
        onSelect: () => void openChangedFile(file, false),
      },
      {
        label: "History...",
        icon: <History aria-hidden="true" size={14} />,
        disabled: !canOpen,
        onSelect: () => void openChangedFile(file, true),
      },
      { separator: true },
      {
        label: isStaged ? "Unstage" : "Stage",
        icon: <Plus aria-hidden="true" size={14} />,
        shortcut: "Enter",
        onSelect: () => void toggleFileStage(file),
      },
      {
        label: "Discard changes...",
        icon: <Trash2 aria-hidden="true" size={14} />,
        shortcut: "Delete",
        danger: true,
        disabled: isStaged,
        onSelect: () => void discardChangedFile(file),
      },
      {
        label: isStaged ? "Unstage All" : "Stage All",
        shortcut: "Ctrl+Shift+Alt+S",
        onSelect: () => void (isStaged ? unstageAllChanges() : stageAllChanges()),
      },
      {
        label: "Ignore",
        disabled: !canIgnore,
        submenu: canIgnore
          ? [
              {
                label: "Local exclude",
                onSelect: () => void ignoreChangedFile(file, "info_exclude"),
              },
              {
                label: ".gitignore",
                onSelect: () => void ignoreChangedFile(file, "gitignore"),
              },
            ]
          : undefined,
      },
      { separator: true },
      {
        label: "Stash 1 File...",
        icon: <Archive aria-hidden="true" size={14} />,
        onSelect: () => void stashChangedFile(file),
      },
      {
        label: "Save as Patch...",
        icon: <Download aria-hidden="true" size={14} />,
        onSelect: () => void saveChangedFilePatch(file),
      },
      { separator: true },
      {
        label: "Copy Path",
        icon: <Copy aria-hidden="true" size={14} />,
        shortcut: "Ctrl+C",
        onSelect: () => copyChangedFilePath(file, false),
      },
      {
        label: "Copy Full Path",
        icon: <Copy aria-hidden="true" size={14} />,
        shortcut: "Ctrl+Shift+C",
        onSelect: () => copyChangedFilePath(file, true),
      },
    ];
  }

  function openChangedFileMenu(file: ChangedFile, event: ReactMouseEvent) {
    openHeaderMenu(event, changedFileMenuItems(file));
  }

  async function checkPatch() {
    const patch = importedPatch.trim();
    if (!patch) {
      setError("Paste a unified diff before checking it.");
      return;
    }

    setPatchBusy(true);
    setError("");
    const result = await api.checkImportedPatch(currentPath, patch);
    if (result.ok) {
      setPatchCheck(result.value);
      if (!result.value.ok) setError(result.value.output || "Patch check failed.");
    } else {
      setPatchCheck(null);
      setError(result.error);
    }
    setPatchBusy(false);
  }

  async function applyPatch() {
    const patch = importedPatch.trim();
    if (!patchCheck?.ok || !patch) return;
    const ok = await appConfirm({
      title: "Aplicar patch?",
      body: "Aplicar este patch na working tree?",
      confirmLabel: "Aplicar patch",
    });
    if (!ok) return;

    setPatchBusy(true);
    setError("");
    const result = await api.applyImportedPatch(currentPath, patch);
    if (result.ok) {
      setPatchCheck(result.value);
      if (result.value.ok) {
        setImportedPatch("");
        await refreshDiffReview(currentPath);
      } else {
        setError(result.value.output || "Patch apply failed.");
      }
    } else {
      setError(result.error);
    }
    setPatchBusy(false);
  }

  function rejectPatch() {
    setImportedPatch("");
    setPatchCheck(null);
  }

  function loadAllUntracked() {
    setUntrackedLimit(0);
    void refreshDiffReview(currentPath, { autoSelect: false, untrackedLimit: 0 });
  }

  // clia-local: no cloud login or cloud-install gates. The app goes straight to the
  // workspace UI; creating/opening workspaces is fully local.
  const activeTabLabel = t(navItems.find((item) => item.id === activeTab)?.labelKey ?? "nav.queue");

  return (
    <>
      <main className={appShellClassName} style={workspaceAccentStyle}>
        <aside className="sidebar" aria-label="Primary navigation">
          <button
            className="brand"
            type="button"
            title={t("about.openHint")}
            aria-label={t("about.open")}
            onDoubleClick={() => setAboutOpen(true)}
            onKeyDown={(event) => {
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              setAboutOpen(true);
            }}
          >
            <img className="brand-logo" src={cliaMarkUrl} alt="" aria-hidden="true" />
            <div>
              <strong>{t("app.name")}</strong>
              <span>{t("app.tagline")}</span>
            </div>
          </button>

          <nav className="nav-list">
            {navItems.map((item) => {
              const Icon = item.icon;
              const label = t(item.labelKey);
              return (
                <button
                  key={item.id}
                  className={activeTab === item.id ? "nav-item active" : "nav-item"}
                  type="button"
                  onClick={() => setActiveTab(item.id)}
                  aria-label={label}
                  title={label}
                >
                  <Icon aria-hidden="true" size={32} />
                  {item.id === "git" && gitPendingCount > 0 ? (
                    <span className="nav-badge" aria-label={`${gitPendingCount} changes`}>
                      {gitPendingCount > 99 ? "99+" : gitPendingCount}
                    </span>
                  ) : null}
                  <span>{label}</span>
                </button>
              );
            })}
          </nav>
        </aside>

        <section className="main-column">
          <header className="topbar">
            <div className="topbar-screen-title">{activeTabLabel}</div>
            <nav className="context-breadcrumb" aria-label={t("topbar.activeContext")}>
              <button
                className="context-crumb"
                type="button"
                disabled={registryBusy}
                aria-haspopup="menu"
                title={t("topbar.switchWorkspace")}
                onClick={(event) => {
                  const rect = event.currentTarget.getBoundingClientRect();
                  const items: MenuItem[] = [
                    ...workspaces.map((workspace) => ({
                      label: workspace.name,
                      icon:
                        workspace.id === activeWorkspace?.id ? (
                          <Check aria-hidden="true" size={14} />
                        ) : undefined,
                      onSelect: () => void selectWorkspace(workspace),
                    })),
                    ...(workspaces.length ? [{ separator: true }] : []),
                    {
                      label: t("topbar.newWorkspace"),
                      icon: <Plus aria-hidden="true" size={14} />,
                      onSelect: () => setWorkspaceModalOpen(true),
                    },
                  ];
                  openHeaderMenu(
                    { preventDefault() {}, clientX: rect.left, clientY: rect.bottom },
                    items,
                  );
                }}
              >
                <Boxes aria-hidden="true" size={16} />
                <span className="context-crumb-text">
                  <span className="context-crumb-label">Workspace</span>
                  <span className="context-crumb-name">
                    {activeWorkspace?.name ?? t("topbar.noWorkspace")}
                  </span>
                </span>
                <ChevronDown aria-hidden="true" size={14} />
              </button>
              <span className="context-crumb-sep" aria-hidden="true">
                /
              </span>
              <button
                className="context-crumb"
                type="button"
                disabled={!activeWorkspace}
                aria-haspopup="menu"
                title={t("topbar.switchProject")}
                onClick={(event) => {
                  const rect = event.currentTarget.getBoundingClientRect();
                  const items: MenuItem[] = [
                    ...projects.map((project) => ({
                      label: project.is_submodule
                        ? `↳ ${projectDisplayName(project)}  ·submodule`
                        : projectDisplayName(project),
                      icon:
                        project.id === activeProject?.id ? (
                          <Check aria-hidden="true" size={14} />
                        ) : undefined,
                      onSelect: () => {
                        selectProject(project);
                        void refreshProject(project.path, project);
                      },
                    })),
                    ...(projects.length ? [{ separator: true }] : []),
                    {
                      label: t("project.modal.title"),
                      icon: <Plus aria-hidden="true" size={14} />,
                      onSelect: () => openAddProjectModal(),
                    },
                  ];
                  openHeaderMenu(
                    { preventDefault() {}, clientX: rect.left, clientY: rect.bottom },
                    items,
                  );
                }}
              >
                <FolderGit2 aria-hidden="true" size={16} />
                <span className="context-crumb-text">
                  <span className="context-crumb-label">Projeto</span>
                  <span className="context-crumb-name">
                    {activeProject ? projectDisplayName(activeProject) : t("topbar.noProject")}
                  </span>
                </span>
                <ChevronDown aria-hidden="true" size={14} />
              </button>
              {activeProject && activeFlowMeta?.analyzeCommand ? (
                <button
                  type="button"
                  className={
                    projectAnalyzed === false
                      ? "analysis-pill pending"
                      : projectAnalyzed
                        ? "analysis-pill done"
                        : "analysis-pill"
                  }
                  onClick={() => setAnalysisModalOpen(true)}
                  title="Análise do projeto"
                >
                  <Sparkles aria-hidden="true" size={13} />
                  {projectAnalyzed === false
                    ? "Análise pendente"
                    : projectAnalyzed
                      ? "Analisado"
                      : "Análise…"}
                </button>
              ) : null}
            </nav>
            <div className="topbar-actions">
              {activeWorkspace ? (
                <>
                  <button className="secondary-button" type="button" onClick={openAddProjectModal}>
                    <FolderPlus aria-hidden="true" size={17} />
                    {t("topbar.newProject")}
                  </button>
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => openProjectBlueprintModal(null)}
                    disabled={projectBlueprintBusy}
                    title={t("topbar.generateProject")}
                  >
                    <Sparkles aria-hidden="true" size={17} />
                    {t("topbar.generateProject")}
                  </button>
                </>
              ) : null}
              {activeTab === "queue" && activeAgentWorking ? (
                <span className="status-pill ready agent-working-pill">
                  <span className="agent-working-dot" aria-hidden="true" />
                  {t("topbar.agentWorking")}
                </span>
              ) : null}
              <button
                className="secondary-button icon-button"
                type="button"
                onClick={() =>
                  activeTab === "queue"
                    ? void loadWorkspaceTasks()
                    : void refreshProject(currentPath, activeProject)
                }
                disabled={busy || registryBusy || (!activeWorkspace && !currentPath)}
                aria-label={t("common.refresh")}
                title={t("common.refresh")}
              >
                <RefreshCw aria-hidden="true" size={17} />
              </button>
            </div>
          </header>

          {error ? (
            <div className="error-banner" role="status">
              <span>{error}</span>
              <button
                className="banner-close"
                type="button"
                onClick={() => setError("")}
                aria-label={t("common.close")}
              >
                <X aria-hidden="true" size={16} />
              </button>
            </div>
          ) : null}

          {headerMenu ? <ContextMenu {...headerMenu} onClose={closeHeaderMenu} /> : null}
          {quickSwitchOpen ? (
            <QuickSwitch
              workspaces={workspaces}
              projects={projects}
              activeWorkspaceId={activeWorkspace?.id ?? null}
              activeProjectId={activeProject?.id ?? null}
              onPickWorkspace={(workspace) => {
                void selectWorkspace(workspace);
                setQuickSwitchOpen(false);
              }}
              onPickProject={(project) => {
                selectProject(project);
                void refreshProject(project.path, project);
                setQuickSwitchOpen(false);
              }}
              onAddWorkspace={() => {
                setQuickSwitchOpen(false);
                setWorkspaceModalOpen(true);
              }}
              onImportWorkspace={() => {
                setQuickSwitchOpen(false);
                openWorkspaceImport();
              }}
              onAddProject={() => {
                setQuickSwitchOpen(false);
                openAddProjectModal();
              }}
              onClose={() => setQuickSwitchOpen(false)}
            />
          ) : null}

          {filePaletteOpen ? (
            <FilePalette
              entries={sourceTree}
              openFiles={openFiles}
              onOpen={(relativePath) => {
                setActiveTab("code");
                void openSourcePath(currentPath, relativePath);
                setFilePaletteOpen(false);
              }}
              onClose={() => setFilePaletteOpen(false)}
            />
          ) : null}

          {workspaceModalOpen ? (
            <WorkspaceModal
              busy={registryBusy}
              newWorkspaceName={newWorkspaceName}
              newWorkspaceRoot={newWorkspaceRoot}
              onClose={() => setWorkspaceModalOpen(false)}
              onCreateWorkspace={(event) => void createWorkspace(event)}
              onPickWorkspaceRoot={() => void pickDirectory(setNewWorkspaceRoot)}
              setNewWorkspaceName={setNewWorkspaceName}
              setNewWorkspaceRoot={setNewWorkspaceRoot}
              t={t}
            />
          ) : null}

          {workspaceImportOpen ? (
            <WorkspaceImportModal
              busy={registryBusy}
              importName={workspaceImportName}
              importRoot={workspaceImportRoot}
              importSource={workspaceImportSource}
              onClose={() => setWorkspaceImportOpen(false)}
              onImport={(event) => void importWorkspaceFromSolution(event)}
              onPickRoot={() => void pickDirectory(setWorkspaceImportRoot)}
              onPickSource={() => void pickWorkspaceSolutionSource()}
              report={workspaceImportReport}
              setImportName={setWorkspaceImportName}
              setImportRoot={setWorkspaceImportRoot}
              setImportSource={setWorkspaceImportSource}
              t={t}
            />
          ) : null}

          {addProjectModalOpen && activeWorkspace ? (
            <AddProjectModal
              workspaceId={activeWorkspace.id}
              onClose={() => setAddProjectModalOpen(false)}
              onAdded={(project) => {
                setAddProjectModalOpen(false);
                void handleProjectAdded(project);
              }}
              t={t}
            />
          ) : null}

          {analysisModalOpen && activeProject && activeFlowMeta?.analyzeCommand ? (
            <ProjectAnalysisModal
              projectName={projectDisplayName(activeProject)}
              analyzeCommand={activeFlowMeta.analyzeCommand}
              suggestCommand={activeFlowMeta.suggestCommand}
              analyzed={projectAnalyzed}
              hasAgent={Boolean(activeAgentProfile)}
              messages={analysisMessages}
              onClose={() => setAnalysisModalOpen(false)}
              onConfigureAgent={() => {
                setAnalysisModalOpen(false);
                setActiveTab("agents");
              }}
              onStart={startAnalysisSession}
              onAnswer={answerAnalysisSession}
              onAnalyzed={() => void refreshAnalysisStatus()}
              onCreateTasks={createTasksFromSuggestions}
            />
          ) : null}

          {projectBlueprintModalOpen && activeWorkspace ? (
            <ProjectBlueprintModal
              activeAgentProfile={activeAgentProfile}
              busy={projectBlueprintBusy}
              idea={projectBlueprintIdea}
              interview={projectBlueprintInterview}
              knowledgeSources={knowledgeSources}
              onAnswerChange={(questionId, value) =>
                setProjectBlueprintInterview((state) => ({
                  ...state,
                  currentAnswers: { ...state.currentAnswers, [questionId]: value },
                }))
              }
              onClose={() => setProjectBlueprintModalOpen(false)}
              onFinalizeLocal={() => void finalizeProjectBlueprintLocally()}
              onMaterialize={(blueprint) => void materializeProjectBlueprint(blueprint)}
              onSourceToggle={(sourceId) =>
                setProjectBlueprintSourceIds((ids) => toggleNumber(ids, sourceId))
              }
              onStart={(event) => void startProjectBlueprintInterview(event)}
              onSubmitAnswers={(event) => void answerProjectBlueprintBatch(event)}
              onTitleChange={setProjectBlueprintTitle}
              onIdeaChange={setProjectBlueprintIdea}
              selectedSourceIds={projectBlueprintSourceIds}
              title={projectBlueprintTitle}
              t={t}
            />
          ) : null}

          {aboutOpen ? (
            <AboutCliaModal
              activeAgentWorking={activeAgentWorking}
              activeFlowLabel={
                flowRegistry.flows.find((flow) => flow.id === activeFlowId)?.label ?? activeFlowId
              }
              activeProject={activeProject}
              activeWorkspace={activeWorkspace}
              locale={locale}
              onClose={() => setAboutOpen(false)}
              t={t}
              version={APP_VERSION}
            />
          ) : null}

          {flowBuilder ? (
            <FlowBuilderModal
              key={
                flowBuilderSeed
                  ? `seed-${flowBuilderSeed.id || "new"}`
                  : `${flowBuilder.mode}-${flowBuilder.flowId}`
              }
              busy={registryBusy}
              commands={paletteCommands}
              mode={flowBuilder.mode}
              initialId={
                flowBuilderSeed?.id ?? (flowBuilder.mode === "edit" ? flowBuilder.flowId : "")
              }
              initialLabel={
                flowBuilderSeed?.label ??
                (flowBuilder.mode === "edit"
                  ? (flowRegistry.flows.find((flow) => flow.id === flowBuilder.flowId)?.label ?? "")
                  : "")
              }
              initialSchemaText={
                flowBuilderSeed?.schemaText ??
                (flowBuilder.mode === "edit" && flowRegistry.schemas[flowBuilder.flowId]
                  ? JSON.stringify(flowRegistry.schemas[flowBuilder.flowId], null, 2)
                  : STARTER_FLOW_SCHEMA_TEXT)
              }
              initialAnalyzeCommand={
                flowRegistry.flows.find((flow) => flow.id === flowBuilder.flowId)?.analyzeCommand ??
                ""
              }
              initialAnalyzeMarker={
                flowRegistry.flows.find((flow) => flow.id === flowBuilder.flowId)?.analyzeMarker ??
                ""
              }
              onClose={() => {
                setFlowBuilder(null);
                setFlowBuilderSeed(null);
              }}
              onSave={(args) => void saveFlowSchema(args)}
              skills={paletteSkills}
            />
          ) : null}

          {flowInterviewOpen ? (
            <FlowInterviewModal
              agentProfiles={agentProfiles}
              onAnswer={(selected, note) => void answerFlowInterview(selected, note)}
              onClose={() => setFlowInterviewOpen(false)}
              onStart={(profileId, url) => void startFlowInterview(profileId, url)}
              onUrlChange={setFlowInterviewUrl}
              state={flowInterview}
              url={flowInterviewUrl}
            />
          ) : null}

          {attachmentPreview ? (
            <AttachmentPreviewModal
              preview={attachmentPreview}
              onClose={() => setAttachmentPreview(null)}
            />
          ) : null}

          {dwArtifactPreview ? (
            <DwArtifactPreviewModal
              preview={dwArtifactPreview}
              onClose={() => setDwArtifactPreview(null)}
            />
          ) : null}
          {openTaskId != null ? (
            <TaskModal
              cardId={openTaskId}
              workspaceId={activeWorkspace?.id ?? 0}
              cards={cards}
              projects={projects}
              agentProfiles={agentProfiles}
              activeProfileId={activeAgentProfile?.id ?? null}
              activeProjectId={activeProject?.id ?? null}
              agentMessages={agentMessages}
              agentBusy={agentBusy}
              onClose={() => setOpenTaskId(null)}
              onSaved={() => void loadWorkspaceTasks()}
              onRunAgent={(prompt, profile) => runTaskWithAgent(openTaskId, prompt, profile)}
            />
          ) : null}

          <section
            className={activeTab === "agents" ? "work-surface agents-surface" : "work-surface"}
            aria-label="Active workspace"
          >
            {!activeWorkspace ? (
              <WelcomeScreen
                onCreateWorkspace={() => setWorkspaceModalOpen(true)}
                onImportWorkspace={openWorkspaceImport}
                t={t}
                workspaceCount={workspaces.length}
              />
            ) : activeTab === "queue" ? (
              <QueuePanel
                cards={cards}
                projects={projects}
                loaded={cardsLoaded}
                error={cardsError}
                projectFilter={queueProjectFilter}
                onChangeProjectFilter={setQueueProjectFilter}
                onCreateCard={() => void createQueueCard()}
                onRefresh={() => void loadWorkspaceTasks()}
                onOpenTask={(cardId) => setOpenTaskId(cardId)}
                onMoveCard={async (card, bucket) => {
                  const result = await api.updateRequirementCardStatus(
                    card.cardId,
                    bucketCanonicalStatus(bucket),
                  );
                  if (!result.ok) throw new Error(result.error);
                  await loadWorkspaceTasks();
                }}
                onArchive={async (card) => {
                  const result = await api.archiveRequirementCard(card.cardId);
                  if (!result.ok) throw new Error(result.error);
                  await loadWorkspaceTasks();
                }}
              />
            ) : activeTab === "code" ? (
              <>
                <SourcePanel
                  blame={sourceBlame}
                  history={sourceHistory}
                  showHistory={showHistory}
                  compareBase={compareBase}
                  onToggleHistory={() => setShowHistory((value) => !value)}
                  onSelectHistory={(sha) => void openTimeTravel(sha)}
                  onPickCompare={pickCompare}
                  onContentChange={handleSourceContentChange}
                  onSave={() => void saveSourceFile(currentPath, selectedSourceFile)}
                  onQuickOpen={() => setFilePaletteOpen(true)}
                  onSelect={(entry) => void loadSourceFile(currentPath, entry)}
                  openFiles={openFiles}
                  expandedPaths={sourceExpandedPaths}
                  onExpandedPathsChange={setSourceExpandedPaths}
                  onSelectTab={(relativePath) => void openSourcePath(currentPath, relativePath)}
                  onCloseTab={closeSourceTab}
                  selectedFile={selectedSourceFile}
                  sourceDirty={sourceDirty}
                  sourceBusy={sourceBusy}
                  sourceSaving={sourceSaving}
                  sourceContent={sourceContent}
                  sourceTree={sourceTree}
                  editorFontSize={editorFontSize}
                  revealLine={revealLine}
                  sideTab={sourceSideTab}
                  onSideTabChange={setSourceSideTab}
                  searchFocusSeed={searchFocusSeed}
                  searchPath={currentPath}
                  onOpenResult={(relativePath, line) => void openSourceAt(relativePath, line)}
                  onFindInFiles={() => {
                    setSourceSideTab("search");
                    setSearchFocusSeed((seed) => seed + 1);
                  }}
                  previewActive={sourcePreview}
                  onTogglePreview={() => setSourcePreview((value) => !value)}
                  onFormat={formatBuffer}
                  editorPath={
                    selectedSourceFile
                      ? fileUriFor(`${currentPath}/${selectedSourceFile.relative_path}`)
                      : undefined
                  }
                  onRefreshTree={() => void reloadSourceTree()}
                  onCreateFile={(relativePath) => void createNewSourceFile(relativePath)}
                  onCreateDir={(relativePath) => void createNewSourceDir(relativePath)}
                  lsp={
                    activeLspLanguage && lspController.supports(activeLspLanguage)
                      ? {
                          enabled: lspEnabled,
                          installed: lspStatus?.installed ?? false,
                          canInstall: lspStatus?.can_install ?? false,
                          installing: lspInstalling,
                          program: lspStatus?.program ?? "language server",
                        }
                      : null
                  }
                  onEnableLsp={enableLsp}
                  onInstallLsp={() => activeLspLanguage && void installLspServer(activeLspLanguage)}
                  explorerWidth={explorerWidth}
                  onExplorerResize={(width) => {
                    setExplorerWidth(width);
                    const projectId = activeProject?.id;
                    if (projectId != null) {
                      void api.setAppState(
                        projectUiPreferenceKey(projectId, "explorer_width"),
                        String(width),
                      );
                      void api.setAppState(`explorer_width:${projectId}`, String(width));
                    }
                  }}
                />
                {timeTravel ? (
                  <div
                    className="source-overlay"
                    role="dialog"
                    aria-modal="true"
                    aria-label="Versão histórica"
                  >
                    <div className="source-overlay-card wide">
                      <div className="source-overlay-head">
                        <span>
                          Vendo <code>{timeTravel.sha}</code> · {timeTravel.path.split("/").pop()}
                        </span>
                        <button
                          className="secondary-button icon-button"
                          type="button"
                          onClick={() => setTimeTravel(null)}
                          aria-label="Fechar"
                        >
                          <X aria-hidden="true" size={14} />
                        </button>
                      </div>
                      <div className="source-overlay-body">
                        <Suspense fallback={<PanelLoading label="Editor" />}>
                          <MonacoSource
                            value={timeTravel.content}
                            language={timeTravel.language}
                            readOnly
                            height="100%"
                            fontSize={editorFontSize}
                          />
                        </Suspense>
                      </div>
                    </div>
                  </div>
                ) : null}
                {compareView ? (
                  <Suspense fallback={<PanelLoading label="Diff" />}>
                    <DiffCompare
                      path={compareView.path}
                      leftLabel={compareView.leftLabel}
                      rightLabel={compareView.rightLabel}
                      original={compareView.original}
                      modified={compareView.modified}
                      language={compareView.language}
                      fontSize={editorFontSize}
                      onClose={() => setCompareView(null)}
                    />
                  </Suspense>
                ) : null}
              </>
            ) : activeTab === "git" ? (
              <GitWorkbench
                key={currentPath}
                path={currentPath}
                projectId={activeProject?.id ?? null}
                activeAgentProfileId={activeAgentProfile?.id ?? null}
                aiCommitProfileId={selectedAiCommitProfileId}
                agentProfiles={agentProfiles}
                changedCount={gitPendingCount}
                onRefreshLocal={() => void refreshDiffReview(currentPath)}
                onRequestAiCommit={generateCommitMessage}
                onSelectAiCommitProfile={selectAiCommitProfile}
                sidebarWidth={gitSidebarWidth}
                onSidebarResize={(width) => {
                  setGitSidebarWidth(width);
                  const projectId = activeProject?.id;
                  if (projectId != null) {
                    void api.setAppState(
                      projectUiPreferenceKey(projectId, "git_sidebar_width"),
                      String(width),
                    );
                    void api.setAppState(`git_sidebar_width:${projectId}`, String(width));
                  }
                }}
                diffProps={{
                  changedFiles,
                  diffBusy,
                  importedPatch,
                  onApplyPatch: applyPatch,
                  onCheckPatch: checkPatch,
                  onPatchChange: (value) => {
                    setImportedPatch(value);
                    setPatchCheck(null);
                  },
                  onRefresh: () => void refreshDiffReview(currentPath),
                  onLoadAllUntracked: loadAllUntracked,
                  onRejectPatch: rejectPatch,
                  onSelect: (file) => void loadFilePatch(currentPath, file),
                  onToggleFile: (file) => void toggleFileStage(file),
                  onToggleHunk: (file, hunk) => void toggleHunkStage(file, hunk),
                  onDiscardFile: (file) => void discardChangedFile(file),
                  onDiscardHunk: (file, hunk) => void discardChangedHunk(file, hunk),
                  onStageAll: () => void stageAllChanges(),
                  onUnstageAll: () => void unstageAllChanges(),
                  onContextFile: openChangedFileMenu,
                  patchBusy,
                  patchCheck,
                  patchCounts,
                  refreshState: localGitRefresh,
                  selectedFile: selectedChangedFile,
                  selectedPatch,
                  untrackedTruncated,
                  worktreeCounts,
                  listWidth: patchListWidth,
                  onListResize: (width) => {
                    setPatchListWidth(width);
                    const projectId = activeProject?.id;
                    if (projectId != null) {
                      void api.setAppState(
                        projectUiPreferenceKey(projectId, "patch_list_width"),
                        String(width),
                      );
                    }
                  },
                }}
              />
            ) : activeTab === "deploy" ? (
              <Suspense fallback={<PanelLoading label="Deploy" />}>
                <DeployPackagesPanel
                  activeProject={activeProject}
                  confirm={appConfirm}
                  workspace={activeWorkspace}
                  projects={projects}
                />
              </Suspense>
            ) : activeTab === "settings" ? (
              <WorkspaceSettingsPanel
                busy={registryBusy}
                workspace={activeWorkspace}
                activeProject={activeProject}
                agentProfiles={agentProfiles}
                onAgentRtkChange={(profile, enabled) => updateAgentRtk(profile, enabled)}
                editorFontSize={editorFontSize}
                onEditorFontSizeChange={(value) => setEditorFontSize(clampEditorFontSize(value))}
                locale={locale}
                onLocaleChange={setLocale}
                themeMode={themeMode}
                onThemeModeChange={setThemeMode}
                workspaceAccentColor={workspaceAccentColor}
                onWorkspaceAccentColorChange={updateWorkspaceAccentColor}
                t={t}
              />
            ) : activeTab === "agents" ? (
              <AgentsPanel
                activeProfile={activeAgentProfile}
                activeSession={activeAgentSession}
                activeSessions={activeAgentSessions}
                busy={agentBusy}
                composer={agentComposer}
                error={agentError}
                health={
                  activeAgentProfile ? (agentHealthByProfile[activeAgentProfile.id] ?? null) : null
                }
                messages={agentMessages}
                metrics={agentRunMetrics}
                onCreateProfile={(draft) => void createCodexProfile(draft)}
                onUpdateProfile={(id, draft) => void updateCodexProfile(id, draft)}
                onSelectProfile={(profileId) => {
                  setActiveAgentProfileId(profileId);
                  const nextSessionId =
                    agentSessions.find((session) => session.profile_id === profileId)?.id ?? null;
                  activeAgentSessionIdRef.current = nextSessionId;
                  setActiveAgentSessionId(nextSessionId);
                  setAgentMessages([]);
                  setAgentRunMetrics([]);
                }}
                onSelectSession={(sessionId) => {
                  activeAgentSessionIdRef.current = sessionId;
                  setActiveAgentSessionId(sessionId);
                  setAgentMessages([]);
                  setAgentRunMetrics([]);
                }}
                onSend={(message) => void sendAgentPrompt(message)}
                onResetChat={() => void resetAgentChat()}
                onStop={(sessionId) => void stopAgentSession(sessionId)}
                profiles={agentProfiles}
                project={activeProject}
                sessions={agentSessions}
                setComposer={setAgentComposer}
                skills={workspaceSkills}
              />
            ) : null}
          </section>
        </section>
      </main>
      {appConfirmDialog}
      {appPromptDialog}
      {noticeDialog}
    </>
  );
}

function formatCountdown(totalSeconds: number) {
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function WelcomeScreen({
  onCreateWorkspace,
  onImportWorkspace,
  t,
  workspaceCount,
}: {
  onCreateWorkspace: () => void;
  onImportWorkspace: () => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
  workspaceCount: number;
}) {
  return (
    <div className="welcome-screen">
      <section className="welcome-panel">
        <div className="welcome-copy">
          <div className="section-label">{t("app.name")}</div>
          <h1>{t("welcome.title")}</h1>
          <p>{t("welcome.body")}</p>
        </div>
        <div className="welcome-actions">
          <button className="primary-button welcome-cta" type="button" onClick={onCreateWorkspace}>
            <FolderOpen aria-hidden="true" size={22} />
            {t("welcome.createWorkspace")}
          </button>
          <button
            className="secondary-button welcome-cta"
            type="button"
            onClick={onImportWorkspace}
          >
            <Upload aria-hidden="true" size={22} />
            {t("welcome.importWorkspace")}
          </button>
        </div>
        <div className="welcome-notes" aria-label="Como funciona">
          <span>
            {workspaceCount
              ? t("welcome.savedWorkspaces", { count: workspaceCount })
              : t("welcome.noWorkspace")}
          </span>
          <span>{t("welcome.noteFlow")}</span>
          <span>{t("welcome.noteKnowledge")}</span>
        </div>
      </section>
    </div>
  );
}

function workspaceImportSummary(report: WorkspaceSolutionImportReport) {
  const cloned = report.projects.filter((project) => project.status === "cloned").length;
  const warnings = report.projects.filter((project) => project.status === "warning").length;
  const failed = report.projects.filter((project) => project.status === "failed").length;
  const skipped = report.projects.filter((project) => project.status === "skipped").length;
  return `${cloned} clonado(s), ${warnings} com aviso, ${failed} falhou(aram), ${skipped} sem remote.`;
}

function WorkspaceModal({
  busy,
  newWorkspaceName,
  newWorkspaceRoot,
  onClose,
  onCreateWorkspace,
  onPickWorkspaceRoot,
  setNewWorkspaceName,
  setNewWorkspaceRoot,
  t,
}: {
  busy: boolean;
  newWorkspaceName: string;
  newWorkspaceRoot: string;
  onClose: () => void;
  onCreateWorkspace: (event: React.FormEvent<HTMLFormElement>) => void;
  onPickWorkspaceRoot: () => void;
  setNewWorkspaceName: (value: string) => void;
  setNewWorkspaceRoot: (value: string) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}) {
  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="workspace-modal-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Workspace</div>
            <h2 id="workspace-modal-title">{t("workspace.modal.title")}</h2>
            <p>{t("workspace.modal.description")}</p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>
        <form className="setup-form" onSubmit={onCreateWorkspace}>
          <label>
            <span>{t("workspace.modal.name")}</span>
            <input
              value={newWorkspaceName}
              onChange={(event) => setNewWorkspaceName(event.target.value)}
              disabled={busy}
            />
          </label>
          <label>
            <span>{t("workspace.modal.root")}</span>
            <div className="path-picker">
              <input
                value={newWorkspaceRoot}
                onChange={(event) => setNewWorkspaceRoot(event.target.value)}
                disabled={busy}
                spellCheck={false}
              />
              <button
                className="secondary-button"
                type="button"
                onClick={onPickWorkspaceRoot}
                disabled={busy}
              >
                <FolderOpen aria-hidden="true" size={16} />
                {t("workspace.modal.pick")}
              </button>
            </div>
          </label>
          <button className="primary-button" type="submit" disabled={busy}>
            <Plus aria-hidden="true" size={17} />
            {t("workspace.modal.save")}
          </button>
        </form>
      </section>
    </div>
  );
}

function WorkspaceImportModal({
  busy,
  importName,
  importRoot,
  importSource,
  onClose,
  onImport,
  onPickRoot,
  onPickSource,
  report,
  setImportName,
  setImportRoot,
  setImportSource,
  t,
}: {
  busy: boolean;
  importName: string;
  importRoot: string;
  importSource: string;
  onClose: () => void;
  onImport: (event: React.FormEvent<HTMLFormElement>) => void;
  onPickRoot: () => void;
  onPickSource: () => void;
  report: WorkspaceSolutionImportReport | null;
  setImportName: (value: string) => void;
  setImportRoot: (value: string) => void;
  setImportSource: (value: string) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}) {
  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel wide"
        role="dialog"
        aria-modal="true"
        aria-labelledby="workspace-import-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Workspace</div>
            <h2 id="workspace-import-title">{t("workspace.import.title")}</h2>
            <p>{t("workspace.import.description")}</p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>
        <form className="setup-form" onSubmit={onImport}>
          <label>
            <span>{t("workspace.import.file")}</span>
            <div className="path-picker">
              <input
                value={importSource}
                onChange={(event) => setImportSource(event.target.value)}
                disabled={busy}
                spellCheck={false}
              />
              <button
                className="secondary-button"
                type="button"
                onClick={onPickSource}
                disabled={busy}
              >
                <Upload aria-hidden="true" size={16} />
                {t("workspace.modal.pick")}
              </button>
            </div>
          </label>
          <label>
            <span>{t("workspace.import.name")}</span>
            <input
              value={importName}
              onChange={(event) => setImportName(event.target.value)}
              disabled={busy}
              placeholder={t("workspace.import.placeholderName")}
            />
          </label>
          <label>
            <span>{t("workspace.import.destination")}</span>
            <div className="path-picker">
              <input
                value={importRoot}
                onChange={(event) => setImportRoot(event.target.value)}
                disabled={busy}
                spellCheck={false}
              />
              <button
                className="secondary-button"
                type="button"
                onClick={onPickRoot}
                disabled={busy}
              >
                <FolderOpen aria-hidden="true" size={16} />
                {t("workspace.modal.pick")}
              </button>
            </div>
          </label>
          <button
            className="primary-button"
            type="submit"
            disabled={busy || !importSource || !importRoot}
          >
            <GitFork aria-hidden="true" size={17} />
            {t("workspace.import.submit")}
          </button>
        </form>
        {report ? (
          <div className="workspace-import-report">
            <strong>{workspaceImportSummary(report)}</strong>
            <div className="workspace-import-projects">
              {report.projects.length ? (
                report.projects.map((project) => (
                  <div className={`workspace-import-project ${project.status}`} key={project.name}>
                    <span>{project.name}</span>
                    <small>{project.message}</small>
                  </div>
                ))
              ) : (
                <small>{t("workspace.import.empty")}</small>
              )}
            </div>
          </div>
        ) : null}
      </section>
    </div>
  );
}

function AboutCliaModal({
  activeAgentWorking,
  activeFlowLabel,
  activeProject,
  activeWorkspace,
  locale,
  onClose,
  t,
  version,
}: {
  activeAgentWorking: boolean;
  activeFlowLabel: string;
  activeProject: Project | null;
  activeWorkspace: Workspace | null;
  locale: Locale;
  onClose: () => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
  version: string;
}) {
  const statusRows = [
    { label: t("about.version"), value: version },
    { label: t("about.workspace"), value: activeWorkspace?.name ?? t("topbar.noWorkspace") },
    {
      label: t("about.project"),
      value: activeProject ? projectDisplayName(activeProject) : t("topbar.noProject"),
    },
    { label: t("about.flow"), value: activeFlowLabel },
    { label: t("about.language"), value: locale },
    {
      label: t("about.agent"),
      value: activeAgentWorking ? t("about.agentWorking") : t("about.agentIdle"),
    },
  ];

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel about-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-clia-title"
      >
        <div className="about-hero">
          <img className="about-logo" src={cliaSplashLogoUrl} alt="" aria-hidden="true" />
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>
        <div className="about-copy">
          <div>
            <div className="section-label">{t("about.eyebrow")}</div>
            <h2 id="about-clia-title">{t("about.title")}</h2>
          </div>
          <p>{t("about.description")}</p>
        </div>
        <div className="about-status-grid">
          {statusRows.map((row) => (
            <div className="about-status-item" key={row.label}>
              <span>{row.label}</span>
              <strong>{row.value}</strong>
            </div>
          ))}
        </div>
        <div className="about-footer">
          <span>{t("about.runtime")}</span>
          <code>dev-workflow · dw-* · Tauri</code>
        </div>
      </section>
    </div>
  );
}

function PanelLoading({ label }: { label: string }) {
  return (
    <div className="lazy-panel-loading" role="status" aria-live="polite">
      <RefreshCw aria-hidden="true" size={18} />
      <span>Carregando {label}...</span>
    </div>
  );
}

function FlowBuilderModal({
  busy,
  commands,
  initialId,
  initialLabel,
  initialSchemaText,
  initialAnalyzeCommand,
  initialAnalyzeMarker,
  mode,
  onClose,
  onSave,
  skills,
}: {
  busy: boolean;
  commands: DwCommand[];
  initialId: string;
  initialLabel: string;
  initialSchemaText: string;
  initialAnalyzeCommand: string;
  initialAnalyzeMarker: string;
  mode: "new" | "edit";
  onClose: () => void;
  onSave: (args: {
    id: string;
    label: string;
    schemaText: string;
    analyzeCommand?: string;
    analyzeMarker?: string;
  }) => void;
  skills: DwSkill[];
}) {
  const [id, setId] = useState(initialId);
  const [label, setLabel] = useState(initialLabel);
  const [text, setText] = useState(initialSchemaText);
  const [analyzeCommand, setAnalyzeCommand] = useState(initialAnalyzeCommand);
  const [analyzeMarker, setAnalyzeMarker] = useState(initialAnalyzeMarker);
  const parsed = parseWorkbenchSchema(text);
  const invalid = parsed.usedDefault;

  function appendPhase(action: WorkbenchAction, baseLabel: string) {
    if (invalid) return;
    const schema = parsed.schema;
    const existing = new Set(schema.phases.map((phase) => phase.id));
    const base = slugifyFlowId(baseLabel) || "phase";
    let pid = base;
    let n = 2;
    while (existing.has(pid)) pid = `${base}-${n++}`;
    const phase: WorkbenchPhase = {
      id: pid,
      label: baseLabel,
      status: pid.replace(/-/g, "_"),
      description: "",
      fields: [],
      action,
    };
    setText(JSON.stringify({ ...schema, phases: [...schema.phases, phase] }, null, 2));
  }

  function patchPhase(index: number, patch: Partial<WorkbenchPhase>) {
    if (invalid) return;
    const phases = parsed.schema.phases.map((phase, i) =>
      i === index ? { ...phase, ...patch } : phase,
    );
    setText(JSON.stringify({ ...parsed.schema, phases }, null, 2));
  }

  function removePhase(index: number) {
    if (invalid) return;
    const phases = parsed.schema.phases.filter((_, i) => i !== index);
    setText(JSON.stringify({ ...parsed.schema, phases }, null, 2));
  }

  const effectiveId = mode === "edit" ? initialId : slugifyFlowId(id);
  const canSave = !busy && !invalid && Boolean(effectiveId);

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel flow-builder-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="flow-builder-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Fluxos</div>
            <h2 id="flow-builder-title">
              {mode === "edit" ? `Editar fluxo · ${initialId}` : "Novo fluxo custom"}
            </h2>
            <p>Clique numa skill/command para anexar uma fase, ou edite o JSON à mão.</p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar modal"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        <div className="flow-builder-body">
          <aside className="flow-builder-palette" aria-label="Paleta de skills e commands">
            <div className="section-label">Commands</div>
            <div className="flow-palette-list">
              {commands.length ? (
                commands.map((command) => (
                  <button
                    key={command.name}
                    type="button"
                    className="flow-palette-item"
                    disabled={invalid}
                    title={command.description ?? command.command}
                    onClick={() =>
                      appendPhase(
                        {
                          type: "command",
                          base: command.command,
                          promptParts: [{ template: "{{card.title}}" }],
                        },
                        command.title || command.name,
                      )
                    }
                  >
                    <code>{command.command}</code>
                    {command.description ? <small>{command.description}</small> : null}
                  </button>
                ))
              ) : (
                <span className="flow-palette-empty">Nenhum command em .dw/commands/</span>
              )}
            </div>
            <div className="section-label">Skills</div>
            <div className="flow-palette-list">
              {skills.length ? (
                skills.map((skill) => (
                  <button
                    key={skill.name}
                    type="button"
                    className="flow-palette-item"
                    disabled={invalid}
                    title={skill.description ?? skill.name}
                    onClick={() =>
                      appendPhase(
                        {
                          type: "skill",
                          skill:
                            skill.name.startsWith("/") || skill.name.startsWith("$")
                              ? skill.name
                              : `$${skill.name}`,
                        },
                        skill.name,
                      )
                    }
                  >
                    <code>{skill.name}</code>
                    {skill.description ? <small>{skill.description}</small> : null}
                  </button>
                ))
              ) : (
                <span className="flow-palette-empty">Nenhuma skill encontrada</span>
              )}
            </div>
          </aside>

          <div className="flow-builder-editor">
            <div className="flow-builder-meta">
              <label>
                <span>Id do fluxo</span>
                <input
                  value={id}
                  disabled={mode === "edit"}
                  placeholder="meu-fluxo"
                  onChange={(event) => setId(event.target.value)}
                />
              </label>
              <label>
                <span>Nome</span>
                <input
                  value={label}
                  placeholder="Meu fluxo"
                  onChange={(event) => setLabel(event.target.value)}
                />
              </label>
              <label>
                <span>Comando de análise do projeto</span>
                <input
                  value={analyzeCommand}
                  placeholder="/dw-analyze-project"
                  onChange={(event) => setAnalyzeCommand(event.target.value)}
                />
              </label>
              <label>
                <span>Marcador de análise (caminho)</span>
                <input
                  value={analyzeMarker}
                  placeholder=".dw/rules/index.md"
                  onChange={(event) => setAnalyzeMarker(event.target.value)}
                />
              </label>
            </div>
            {invalid ? null : (
              <div className="flow-stage-editor" aria-label="Etapas do fluxo">
                {parsed.schema.phases.map((phase, index) => (
                  <FlowStageEditor
                    key={`${phase.id}-${index}`}
                    phase={phase}
                    onChange={(patch) => patchPhase(index, patch)}
                    onRemove={() => removePhase(index)}
                  />
                ))}
              </div>
            )}
            <details className="flow-builder-advanced">
              <summary>JSON avançado</summary>
              <textarea
                className="flow-builder-json"
                spellCheck={false}
                value={text}
                onChange={(event) => setText(event.target.value)}
              />
            </details>
            <div className="flow-builder-status">
              {invalid ? (
                <span className="flow-builder-error">JSON inválido — sem fases válidas.</span>
              ) : (
                <span className="flow-builder-ok">
                  {parsed.schema.phases.length} fase(s)
                  {parsed.warnings.length ? ` · ${parsed.warnings.length} aviso(s)` : ""}
                </span>
              )}
            </div>
            {parsed.warnings.length ? (
              <ul className="flow-builder-warnings">
                {parsed.warnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            ) : null}
            <div className="flow-builder-actions">
              <button className="secondary-button" type="button" onClick={onClose}>
                Cancelar
              </button>
              <button
                className="primary-button"
                type="button"
                disabled={!canSave}
                onClick={() =>
                  onSave({
                    id: effectiveId,
                    label,
                    schemaText: text,
                    analyzeCommand,
                    analyzeMarker,
                  })
                }
              >
                Salvar fluxo
              </button>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

function ProjectBlueprintModal({
  activeAgentProfile,
  busy,
  idea,
  interview,
  knowledgeSources,
  onAnswerChange,
  onClose,
  onFinalizeLocal,
  onIdeaChange,
  onMaterialize,
  onSourceToggle,
  onStart,
  onSubmitAnswers,
  onTitleChange,
  selectedSourceIds,
  t,
  title,
}: {
  activeAgentProfile: AgentProfile | null;
  busy: boolean;
  idea: string;
  interview: {
    blueprint: ProjectBlueprint | null;
    status: "idle" | "asking" | "waiting" | "planned" | "error";
    questions: ProjectBlueprintQuestion[];
    answers: ProjectBlueprintAnswer[];
    currentAnswers: Record<string, string>;
    note?: string;
    error?: string;
  };
  knowledgeSources: KnowledgeSource[];
  onAnswerChange: (questionId: string, value: string) => void;
  onClose: () => void;
  onFinalizeLocal: () => void;
  onIdeaChange: (value: string) => void;
  onMaterialize: (blueprint: ProjectBlueprint) => void;
  onSourceToggle: (sourceId: number) => void;
  onStart: (event: React.FormEvent<HTMLFormElement>) => void;
  onSubmitAnswers: (event: React.FormEvent<HTMLFormElement>) => void;
  onTitleChange: (value: string) => void;
  selectedSourceIds: number[];
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
  title: string;
}) {
  const blueprint = interview.blueprint;
  const planned = blueprint?.status === "planned" || blueprint?.status === "materialized";
  const subprojects = blueprint
    ? projectBlueprintSubprojects(blueprint.detected_subprojects_json)
    : [];
  const taskCount = blueprint ? projectBlueprintTaskCount(blueprint.tasks_json) : 0;

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel project-blueprint-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="project-blueprint-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">{t("blueprint.new")}</div>
            <h2 id="project-blueprint-title">
              {blueprint ? blueprint.title : t("blueprint.initialInterview")}
            </h2>
            <p>
              {blueprint
                ? `${projectBlueprintStatusLabel(blueprint.status, t)} · ${t(
                    "blueprint.answerCount",
                    {
                      count: interview.answers.length,
                    },
                  )}`
                : activeAgentProfile
                  ? t("blueprint.agent", { name: activeAgentProfile.name })
                  : t("blueprint.noAgent")}
            </p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        {!blueprint ? (
          <form className="project-blueprint-form" onSubmit={onStart}>
            <label>
              <span>{t("blueprint.name")}</span>
              <input
                value={title}
                onChange={(event) => onTitleChange(event.target.value)}
                disabled={busy}
                placeholder={t("blueprint.namePlaceholder")}
              />
            </label>
            <label>
              <span>{t("blueprint.idea")}</span>
              <textarea
                value={idea}
                onChange={(event) => onIdeaChange(event.target.value)}
                disabled={busy}
                rows={5}
                placeholder={t("blueprint.ideaPlaceholder")}
              />
            </label>
            <ProjectBlueprintSourcePicker
              knowledgeSources={knowledgeSources}
              selectedSourceIds={selectedSourceIds}
              onSourceToggle={onSourceToggle}
              t={t}
            />
            <div className="modal-actions">
              <button className="secondary-button" type="button" onClick={onClose}>
                {t("common.cancel")}
              </button>
              <button className="primary-button" type="submit" disabled={busy || !title.trim()}>
                <Sparkles aria-hidden="true" size={16} />
                {t("blueprint.startInterview")}
              </button>
            </div>
          </form>
        ) : (
          <div className="project-blueprint-body">
            <div className="project-blueprint-stats">
              <span>
                <strong>{interview.answers.length}</strong>
                {t("blueprint.answers")}
              </span>
              <span>
                <strong>{selectedSourceIds.length}</strong>
                {t("blueprint.sources")}
              </span>
              <span>
                <strong>{subprojects.length || 1}</strong>
                {t("blueprint.projects")}
              </span>
              <span>
                <strong>{taskCount || "-"}</strong>
                {t("blueprint.tasks")}
              </span>
            </div>

            <ProjectBlueprintSourcePicker
              knowledgeSources={knowledgeSources}
              selectedSourceIds={selectedSourceIds}
              onSourceToggle={onSourceToggle}
              t={t}
            />

            {interview.error ? (
              <div className="form-error" role="status">
                {interview.error}
              </div>
            ) : null}
            {interview.note ? <div className="project-blueprint-note">{interview.note}</div> : null}

            {interview.status === "waiting" ? (
              <div className="project-blueprint-waiting">
                <RefreshCw aria-hidden="true" size={18} />
                {t("blueprint.waiting")}
              </div>
            ) : null}

            {interview.status === "asking" && interview.questions.length ? (
              <form className="project-blueprint-question-form" onSubmit={onSubmitAnswers}>
                {interview.questions.map((question) => (
                  <label key={question.id}>
                    <span>{question.area}</span>
                    <strong>{question.question}</strong>
                    <textarea
                      value={interview.currentAnswers[question.id] ?? ""}
                      onChange={(event) => onAnswerChange(question.id, event.target.value)}
                      disabled={busy}
                      rows={3}
                    />
                  </label>
                ))}
                <div className="modal-actions">
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={onFinalizeLocal}
                    disabled={busy}
                  >
                    {t("blueprint.generateNow")}
                  </button>
                  <button className="primary-button" type="submit" disabled={busy}>
                    <Send aria-hidden="true" size={16} />
                    {t("blueprint.sendBatch")}
                  </button>
                </div>
              </form>
            ) : null}

            {planned && blueprint ? (
              <div className="project-blueprint-plan">
                <section>
                  <span>PRD</span>
                  <p>{truncateText(blueprint.prd.replace(/\s+/g, " "), 220)}</p>
                </section>
                <section>
                  <span>TechSpec</span>
                  <p>{truncateText(blueprint.techspec.replace(/\s+/g, " "), 220)}</p>
                </section>
                <section>
                  <span>Definition of Done</span>
                  <p>{truncateText(blueprint.definition_of_done.replace(/\s+/g, " "), 220)}</p>
                </section>
                <div className="modal-actions">
                  <button className="secondary-button" type="button" onClick={onClose}>
                    {t("common.close")}
                  </button>
                  <button
                    className="primary-button"
                    type="button"
                    disabled={busy || blueprint.status === "materialized"}
                    onClick={() => onMaterialize(blueprint)}
                  >
                    <CheckCircle2 aria-hidden="true" size={16} />
                    {t("blueprint.materialize")}
                  </button>
                </div>
              </div>
            ) : null}
          </div>
        )}
      </section>
    </div>
  );
}

function ProjectBlueprintSourcePicker({
  knowledgeSources,
  onSourceToggle,
  selectedSourceIds,
  t,
}: {
  knowledgeSources: KnowledgeSource[];
  onSourceToggle: (sourceId: number) => void;
  selectedSourceIds: number[];
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}) {
  return (
    <section className="project-blueprint-sources" aria-label={t("knowledge.title")}>
      <div className="section-label">{t("knowledge.title")}</div>
      <div className="project-blueprint-source-list">
        {knowledgeSources.length ? (
          knowledgeSources.map((source) => (
            <label key={source.id}>
              <input
                type="checkbox"
                checked={selectedSourceIds.includes(source.id)}
                onChange={() => onSourceToggle(source.id)}
              />
              <span>
                <strong>{source.name}</strong>
                <small>{knowledgeSourceScopeLabel(source)}</small>
              </span>
            </label>
          ))
        ) : (
          <small>{t("blueprint.noSource")}</small>
        )}
      </div>
    </section>
  );
}


const STARTER_FLOW_SCHEMA_TEXT = JSON.stringify(
  {
    version: 1,
    groups: [],
    phases: [
      {
        id: "backlog",
        label: "Backlog",
        status: "draft",
        description: "",
        fields: [],
        action: { type: "none" },
      },
    ],
  },
  null,
  2,
);

function slugifyFlowId(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** Compose a custom flow: a JSON editor (validated live) + a palette of the
 * project's commands/skills that append phase stubs. */
const STAGE_KIND_OPTIONS: StageKind[] = [
  "planning",
  "execution",
  "review",
  "delivery",
  "approval",
  "status",
];

function FlowStageEditor({
  phase,
  onChange,
  onRemove,
}: {
  phase: WorkbenchPhase;
  onChange: (patch: Partial<WorkbenchPhase>) => void;
  onRemove: () => void;
}) {
  const output = phase.output;
  const inputsText = (phase.inputs ?? []).map((input) => input.path).join("\n");
  return (
    <div className="flow-stage-card">
      <div className="flow-stage-head">
        <input
          value={phase.label}
          onChange={(event) => onChange({ label: event.target.value })}
          aria-label="Nome da etapa"
        />
        <code>{phase.id}</code>
        <button className="ghost-button" type="button" onClick={onRemove}>
          Remover
        </button>
      </div>
      <div className="flow-stage-grid">
        <label>
          <span>Tipo</span>
          <select
            value={phase.kind ?? ""}
            onChange={(event) =>
              onChange({ kind: (event.target.value || undefined) as StageKind | undefined })
            }
          >
            <option value="">(auto)</option>
            {STAGE_KIND_OPTIONS.map((kind) => (
              <option key={kind} value={kind}>
                {kind}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Banda</span>
          <input
            value={phase.group ?? ""}
            placeholder="planejamento / execucao / …"
            onChange={(event) => onChange({ group: event.target.value || undefined })}
          />
        </label>
        <label>
          <span>Saída</span>
          <select
            value={output?.policy ?? "none"}
            onChange={(event) => {
              const policy = event.target.value as "none" | "optional" | "required";
              if (policy === "none") onChange({ output: undefined });
              else
                onChange({
                  output: {
                    path: output?.path || `{{cardBase}}/${phase.id}.md`,
                    policy,
                    capture: output?.capture ?? true,
                  },
                });
            }}
          >
            <option value="none">nenhuma</option>
            <option value="optional">opcional</option>
            <option value="required">obrigatória</option>
          </select>
        </label>
      </div>
      {output ? (
        <>
          <label>
            <span>Caminho da saída</span>
            <input
              value={output.path}
              onChange={(event) => onChange({ output: { ...output, path: event.target.value } })}
            />
          </label>
          <label className="flow-stage-check">
            <input
              type="checkbox"
              checked={output.capture ?? false}
              onChange={(event) =>
                onChange({ output: { ...output, capture: event.target.checked } })
              }
            />
            <span>Capturar a saída final do agente automaticamente</span>
          </label>
        </>
      ) : null}
      <label>
        <span>Insumos (um caminho por linha)</span>
        <textarea
          className="flow-stage-inputs"
          value={inputsText}
          placeholder="{{cardBase}}/plan.md"
          onChange={(event) => {
            const paths = event.target.value
              .split("\n")
              .map((line) => line.trim())
              .filter(Boolean);
            onChange({ inputs: paths.length ? paths.map((path) => ({ path })) : undefined });
          }}
        />
      </label>
    </div>
  );
}


function priorityLabel(priority: string): string {
  if (priority === "high") return "Alta";
  if (priority === "low") return "Baixa";
  return "Média";
}

function priorityCode(priority: string): string {
  if (priority === "high") return "P0";
  if (priority === "low") return "P2";
  return "P1";
}

function priorityClass(priority: string): string {
  if (priority === "high") return "p0";
  if (priority === "low") return "p2";
  return "p1";
}

function queueAttachmentCount(card: QueueCard): number {
  const raw = card.raw as RequirementCard & {
    attachment_count?: number;
    attachments_count?: number;
    attachments?: unknown[];
  };
  if (typeof raw.attachment_count === "number") return raw.attachment_count;
  if (typeof raw.attachments_count === "number") return raw.attachments_count;
  return Array.isArray(raw.attachments) ? raw.attachments.length : 0;
}

function QueuePanel({
  cards,
  projects,
  loaded,
  error,
  projectFilter,
  onChangeProjectFilter,
  onCreateCard,
  onRefresh,
  onOpenTask,
  onMoveCard,
  onArchive,
}: {
  cards: RequirementCard[];
  projects: Project[];
  loaded: boolean;
  error: string;
  projectFilter: number | null;
  onChangeProjectFilter: (projectId: number | null) => void;
  onCreateCard: () => void;
  onRefresh: () => void;
  onOpenTask: (cardId: number) => void;
  onMoveCard: (card: QueueCard, bucket: QueueBucket) => Promise<void>;
  onArchive: (card: QueueCard) => Promise<void>;
}) {
  const [pendingId, setPendingId] = useState<number | null>(null);
  const [statusError, setStatusError] = useState("");
  const queue = useMemo(
    () => buildQueue(cards, projects, { projectId: projectFilter }),
    [cards, projects, projectFilter],
  );

  async function run(card: QueueCard, action: () => Promise<void>) {
    setStatusError("");
    setPendingId(card.cardId);
    try {
      await action();
    } catch (error) {
      setStatusError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingId(null);
    }
  }
  function handleDrop(cardId: number, bucket: QueueBucket) {
    const card = queue.items.find((item) => item.cardId === cardId);
    if (!card || card.bucket === bucket) return;
    void run(card, () => onMoveCard(card, bucket));
  }

  return (
    <section className="queue-panel" aria-labelledby="queue-title">
      <header className="queue-header">
        <div>
          <h2 className="queue-title" id="queue-title">
            Minha Fila
          </h2>
          <p>Quadro local do workspace ativo.</p>
        </div>
        <div className="queue-stats">
          <span className="queue-stat">
            <span className="queue-stat-num">{queue.items.length}</span> cards
          </span>
          <span className="queue-stat">
            <span className="queue-stat-num">{projects.length}</span> projetos
          </span>
        </div>
        <div className="queue-card-actions">
          <button className="primary-button" type="button" onClick={onCreateCard}>
            <Plus aria-hidden="true" size={16} />
            Nova tarefa
          </button>
          <button className="secondary-button" type="button" onClick={onRefresh} disabled={!loaded}>
            <RefreshCw aria-hidden="true" size={16} />
            Atualizar
          </button>
        </div>
      </header>

      {projects.length ? (
        <div className="queue-filter detail-tabs" role="group" aria-label="Filtrar por projeto">
          <button
            className={projectFilter == null ? "detail-tab active" : "detail-tab"}
            type="button"
            onClick={() => onChangeProjectFilter(null)}
          >
            Todos
          </button>
          {projects.map((project) => (
            <button
              key={project.id}
              className={projectFilter === project.id ? "detail-tab active" : "detail-tab"}
              type="button"
              onClick={() => onChangeProjectFilter(project.id)}
            >
              {project.name}
            </button>
          ))}
        </div>
      ) : null}

      {error || statusError ? (
        <div className="error-banner compact">{statusError || error}</div>
      ) : null}

      {!loaded ? (
        <PanelLoading label="Tarefas" />
      ) : (
        <div className="queue-kanban kanban-board">
          {QUEUE_BUCKETS.map((bucket) => (
            <QueueColumn
              key={bucket.id}
              bucket={bucket.id}
              label={bucket.label}
              cards={queue.buckets[bucket.id]}
              pendingId={pendingId}
              onDropCardId={handleDrop}
              onOpenTask={onOpenTask}
              onArchive={(card) => void run(card, () => onArchive(card))}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function QueueColumn({
  bucket,
  label,
  cards,
  pendingId,
  onDropCardId,
  onOpenTask,
  onArchive,
}: {
  bucket: QueueBucket;
  label: string;
  cards: QueueCard[];
  pendingId: number | null;
  onDropCardId: (cardId: number, bucket: QueueBucket) => void;
  onOpenTask: (cardId: number) => void;
  onArchive: (card: QueueCard) => void;
}) {
  const [over, setOver] = useState(false);
  return (
    <div
      className={["queue-column", bucket, "kanban-column", `kanban-${bucket}`, over ? "over" : ""]
        .filter(Boolean)
        .join(" ")}
      onDragOver={(event) => {
        event.preventDefault();
        if (!over) setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(event) => {
        event.preventDefault();
        setOver(false);
        const cardId = Number(event.dataTransfer.getData("text/plain"));
        if (Number.isFinite(cardId)) onDropCardId(cardId, bucket);
      }}
    >
      <header className="queue-column-header kanban-column-head">
        <span className="queue-column-title kanban-column-title">
          <span className="queue-column-dot kanban-column-dot" aria-hidden="true" />
          {label}
        </span>
        <span className="queue-column-count kanban-count">{cards.length}</span>
      </header>
      <div className="queue-column-cards kanban-column-body">
        {cards.map((card) => (
          <TaskCardView
            key={card.cardId}
            card={card}
            pending={pendingId === card.cardId}
            onOpen={onOpenTask}
            onArchive={onArchive}
          />
        ))}
        {cards.length === 0 ? <p className="kanban-empty">—</p> : null}
      </div>
    </div>
  );
}

function TaskCardView({
  card,
  pending,
  onOpen,
  onArchive,
}: {
  card: QueueCard;
  pending: boolean;
  onOpen: (cardId: number) => void;
  onArchive: (card: QueueCard) => void;
}) {
  const attachmentCount = queueAttachmentCount(card);
  return (
    <article
      className={pending ? "queue-card kanban-card pending" : "queue-card kanban-card"}
      draggable
      onDragStart={(event) => event.dataTransfer.setData("text/plain", String(card.cardId))}
      onClick={() => onOpen(card.cardId)}
    >
      <div className="queue-card-header kanban-card-meta">
        <span className="queue-card-id kanban-card-id">{card.publicId}</span>
        <span
          className={`queue-card-priority ${priorityClass(card.priority)} priority-pill ${card.priority}`}
          title={priorityLabel(card.priority)}
        >
          {priorityCode(card.priority)}
        </span>
      </div>
      <h3 className="queue-card-title">{card.title}</h3>
      <div className="queue-card-meta">
        <span className="queue-card-meta-item">
          <FolderGit2 aria-hidden="true" size={12} />
          {card.projectNames[0] ?? "Sem projeto"}
        </span>
        {card.checklistTotal ? (
          <span className="queue-card-meta-item kanban-card-checklist">
            <ListChecks aria-hidden="true" size={12} />
            {card.checklistDone}/{card.checklistTotal}
          </span>
        ) : null}
        {attachmentCount ? (
          <span className="queue-card-meta-item">
            <FileText aria-hidden="true" size={12} />
            {attachmentCount} docs
          </span>
        ) : null}
        {card.agentPrompt.trim() ? (
          <span className="queue-card-meta-item">
            <Bot aria-hidden="true" size={12} />
            agente
          </span>
        ) : null}
      </div>
      <div className="kanban-card-foot">
        <span />
        <button
          className="ghost-button"
          type="button"
          title="Arquivar"
          onClick={(event) => {
            event.stopPropagation();
            onArchive(card);
          }}
        >
          <Archive aria-hidden="true" size={14} />
        </button>
      </div>
    </article>
  );
}

function runStatusLabel(status: string): string {
  switch (status) {
    case "done":
      return "concluído";
    case "failed":
      return "falhou";
    case "running":
      return "rodando";
    case "stopped":
      return "parado";
    case "idle":
      return "ocioso";
    default:
      return status;
  }
}

function TaskModal({
  cardId,
  workspaceId,
  cards,
  projects,
  agentProfiles,
  activeProfileId,
  activeProjectId,
  agentMessages,
  agentBusy,
  onClose,
  onSaved,
  onRunAgent,
}: {
  cardId: number;
  workspaceId: number;
  cards: RequirementCard[];
  projects: Project[];
  agentProfiles: AgentProfile[];
  activeProfileId: number | null;
  activeProjectId: number | null;
  agentMessages: AgentMessage[];
  agentBusy: boolean;
  onClose: () => void;
  onSaved: () => void;
  onRunAgent: (prompt: string, profile: AgentProfile | null) => Promise<{ id: number } | null>;
}) {
  const card = cards.find((item) => item.id === cardId) ?? null;
  const initialProjectIds = card
    ? card.project_ids.length
      ? card.project_ids
      : card.project_id != null
        ? [card.project_id]
        : []
    : [];
  const [title, setTitle] = useState(card?.title ?? "");
  const [description, setDescription] = useState(card?.body ?? "");
  const [priority, setPriority] = useState(card?.priority || "medium");
  const [status, setStatus] = useState(card?.status ?? "todo");
  const [projectIds, setProjectIds] = useState<number[]>(initialProjectIds);
  const [checklist, setChecklist] = useState<ChecklistItem[]>(parseChecklist(card?.checklist_json));
  const [agentPrompt, setAgentPrompt] = useState(card?.agent_prompt ?? "");
  const [checklistDraft, setChecklistDraft] = useState("");
  const [attachments, setAttachments] = useState<RequirementAttachment[]>([]);
  const [profileId, setProfileId] = useState<number | null>(activeProfileId);
  const [runningSessionId, setRunningSessionId] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [modalError, setModalError] = useState("");
  const titleRef = useRef<HTMLInputElement>(null);
  // Per-task agent-run history (sessions launched from this card via scope "card_run").
  const [runHistory, setRunHistory] = useState<AgentSession[]>([]);
  const [runTranscripts, setRunTranscripts] = useState<Record<number, AgentMessage[]>>({});

  // Focus + select the title when the modal opens so a freshly created task
  // ("Nova tarefa") can be renamed by just typing.
  useEffect(() => {
    const el = titleRef.current;
    if (el) {
      el.focus();
      el.select();
    }
  }, [cardId]);

  useEffect(() => {
    const next = cards.find((item) => item.id === cardId);
    if (!next) return;
    setTitle(next.title);
    setDescription(next.body);
    setPriority(next.priority || "medium");
    setStatus(next.status);
    setProjectIds(
      next.project_ids.length
        ? next.project_ids
        : next.project_id != null
          ? [next.project_id]
          : [],
    );
    setChecklist(parseChecklist(next.checklist_json));
    setAgentPrompt(next.agent_prompt ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cardId]);

  useEffect(() => {
    let cancelled = false;
    void api.listRequirementAttachments(cardId).then((result) => {
      if (!cancelled && result.ok) setAttachments(result.value);
    });
    return () => {
      cancelled = true;
    };
  }, [cardId]);

  // Load the agent-run history for this card. Re-fetches when a run starts/ends
  // (agentBusy) so a freshly launched run — and its final status — show up.
  useEffect(() => {
    if (!workspaceId) return;
    let cancelled = false;
    void api.listAgentSessionsForCard(workspaceId, cardId).then((result) => {
      if (!cancelled && result.ok) setRunHistory(result.value);
    });
    return () => {
      cancelled = true;
    };
  }, [cardId, workspaceId, agentBusy]);

  // Lazy-load a run's transcript the first time its <details> is expanded.
  function loadTranscript(sessionId: number) {
    if (runTranscripts[sessionId]) return;
    void api.listAgentMessages(sessionId).then((result) => {
      if (result.ok) setRunTranscripts((prev) => ({ ...prev, [sessionId]: result.value }));
    });
  }

  const statusOptions = [
    { value: "todo", label: "A fazer" },
    { value: "doing", label: "Fazendo" },
    { value: "validating", label: "Validando" },
    { value: "done", label: "Feito" },
  ];

  function toggleProject(id: number) {
    setProjectIds((ids) => (ids.includes(id) ? ids.filter((value) => value !== id) : [...ids, id]));
  }
  function addChecklistItem() {
    const text = checklistDraft.trim();
    if (!text) return;
    setChecklist((items) => [
      ...items,
      { id: `item-${items.length}-${text.slice(0, 12)}`, text, done: false },
    ]);
    setChecklistDraft("");
  }
  function toggleChecklistItem(id: string) {
    setChecklist((items) =>
      items.map((item) => (item.id === id ? { ...item, done: !item.done } : item)),
    );
  }
  function removeChecklistItem(id: string) {
    setChecklist((items) => items.filter((item) => item.id !== id));
  }

  async function addAttachment() {
    setModalError("");
    const picked = await api.pickFile();
    if (!picked.ok || !picked.value) return;
    const result = await api.addRequirementAttachment(cardId, picked.value);
    if (result.ok) setAttachments((items) => [result.value, ...items]);
    else setModalError(result.error);
  }
  async function removeAttachment(id: number) {
    const result = await api.removeRequirementAttachment(id);
    if (result.ok) setAttachments((items) => items.filter((item) => item.id !== id));
    else setModalError(result.error);
  }

  async function save() {
    setSaving(true);
    setModalError("");
    const updated = await api.updateRequirementCard({
      id: cardId,
      title: title.trim() || "Sem título",
      body: description,
      priority,
      checklist_json: serializeChecklist(checklist),
      agent_prompt: agentPrompt,
    });
    if (!updated.ok) {
      setModalError(updated.error);
      setSaving(false);
      return false;
    }
    const projectsResult = await api.setRequirementCardProjects(cardId, projectIds);
    if (!projectsResult.ok) {
      setModalError(projectsResult.error);
      setSaving(false);
      return false;
    }
    if (card && status !== card.status) {
      const statusResult = await api.updateRequirementCardStatus(cardId, status);
      if (!statusResult.ok) {
        setModalError(statusResult.error);
        setSaving(false);
        return false;
      }
    }
    setSaving(false);
    onSaved();
    return true;
  }

  function buildPrompt(): string {
    const parts: string[] = [`# ${title.trim() || "Tarefa"}`];
    if (description.trim()) parts.push(description.trim());
    if (checklist.length) {
      parts.push(
        "## Checklist\n" +
          checklist.map((item) => `- [${item.done ? "x" : " "}] ${item.text}`).join("\n"),
      );
    }
    if (agentPrompt.trim()) parts.push("## Instruções\n" + agentPrompt.trim());
    if (attachments.length) {
      parts.push(
        "## Anexos (leia estes arquivos)\n" +
          attachments.map((item) => `- ${item.name}: ${item.file_path}`).join("\n"),
      );
    }
    return parts.join("\n\n");
  }

  async function runAgent() {
    setModalError("");
    const saved = await save();
    if (!saved) return;
    const profile = agentProfiles.find((item) => item.id === profileId) ?? null;
    const session = await onRunAgent(buildPrompt(), profile);
    if (session) setRunningSessionId(session.id);
  }

  const runMessages = runningSessionId
    ? agentMessages.filter((message) => message.session_id === runningSessionId)
    : [];
  const canRun = activeProjectId != null;

  return (
    <div className="modal-backdrop elevated" role="presentation" onClick={onClose}>
      <section
        className="modal-panel task-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="task-modal-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="modal-heading">
          <div>
            <span className="section-label">{card?.public_id ?? "TAREFA"}</span>
            <h2 id="task-modal-title">Editar tarefa</h2>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        {modalError ? <div className="error-banner compact">{modalError}</div> : null}

        <div className="task-modal-body">
          <label className="field">
            <span>Título</span>
            <input
              ref={titleRef}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>

          <label className="field">
            <span>Descrição</span>
            <textarea
              value={description}
              rows={4}
              onChange={(event) => setDescription(event.target.value)}
            />
          </label>

          <div className="task-modal-row">
            <label className="field">
              <span>Prioridade</span>
              <select value={priority} onChange={(event) => setPriority(event.target.value)}>
                <option value="high">Alta</option>
                <option value="medium">Média</option>
                <option value="low">Baixa</option>
              </select>
            </label>
            <label className="field">
              <span>Status</span>
              <select value={status} onChange={(event) => setStatus(event.target.value)}>
                {statusOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="field">
            <span>Projetos</span>
            <div className="chip-row">
              {projects.length ? (
                projects.map((project) => (
                  <button
                    key={project.id}
                    type="button"
                    className={projectIds.includes(project.id) ? "chip active" : "chip"}
                    onClick={() => toggleProject(project.id)}
                  >
                    {project.name}
                  </button>
                ))
              ) : (
                <span className="empty-note">Nenhum projeto neste workspace.</span>
              )}
            </div>
          </div>

          <div className="field">
            <span>
              Checklist ({checklist.filter((item) => item.done).length}/{checklist.length})
            </span>
            <ul className="checklist">
              {checklist.map((item) => (
                <li key={item.id}>
                  <label>
                    <input
                      type="checkbox"
                      checked={item.done}
                      onChange={() => toggleChecklistItem(item.id)}
                    />
                    <span className={item.done ? "done" : ""}>{item.text}</span>
                  </label>
                  <button
                    className="ghost-button"
                    type="button"
                    onClick={() => removeChecklistItem(item.id)}
                    aria-label="Remover item"
                  >
                    <Trash2 aria-hidden="true" size={13} />
                  </button>
                </li>
              ))}
            </ul>
            <div className="checklist-add">
              <input
                value={checklistDraft}
                placeholder="Nova subtarefa"
                onChange={(event) => setChecklistDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    addChecklistItem();
                  }
                }}
              />
              <button className="secondary-button" type="button" onClick={addChecklistItem}>
                <Plus aria-hidden="true" size={14} />
              </button>
            </div>
          </div>

          <div className="field">
            <span>Anexos</span>
            <ul className="attachment-list">
              {attachments.map((attachment) => (
                <li key={attachment.id}>
                  <FileText aria-hidden="true" size={14} />
                  <span>{attachment.name}</span>
                  <button
                    className="ghost-button"
                    type="button"
                    onClick={() => void removeAttachment(attachment.id)}
                    aria-label="Remover anexo"
                  >
                    <Trash2 aria-hidden="true" size={13} />
                  </button>
                </li>
              ))}
              {attachments.length === 0 ? <li className="empty-note">Sem anexos.</li> : null}
            </ul>
            <button className="secondary-button" type="button" onClick={() => void addAttachment()}>
              <Plus aria-hidden="true" size={14} />
              Anexar arquivo
            </button>
          </div>

          <label className="field">
            <span>Prompt do agente</span>
            <textarea
              value={agentPrompt}
              rows={3}
              placeholder="Instruções extras para o agente ao executar esta tarefa"
              onChange={(event) => setAgentPrompt(event.target.value)}
            />
          </label>

          <div className="field">
            <span>Executar com agente</span>
            <div className="task-run-row">
              {agentProfiles.length > 1 ? (
                <select
                  value={profileId ?? ""}
                  onChange={(event) =>
                    setProfileId(event.target.value ? Number(event.target.value) : null)
                  }
                >
                  {agentProfiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>
                      {profile.name} ({profile.provider})
                    </option>
                  ))}
                </select>
              ) : null}
              <button
                className="primary-button"
                type="button"
                onClick={() => void runAgent()}
                disabled={agentBusy || !canRun}
                title={canRun ? undefined : "Selecione um projeto ativo para executar"}
              >
                <Play aria-hidden="true" size={15} />
                Executar com agente
              </button>
            </div>
            {!canRun ? (
              <p className="empty-note">
                Selecione um projeto ativo no app para executar a tarefa.
              </p>
            ) : null}
            {runningSessionId ? (
              <div className="agent-inline-stream">
                {runMessages.length === 0 ? (
                  <p className="empty-note">Aguardando o agente…</p>
                ) : (
                  runMessages.map((message) => (
                    <div key={message.id} className={`agent-msg ${message.role}`}>
                      <span className="agent-msg-role">{message.role}</span>
                      <pre>{message.content}</pre>
                    </div>
                  ))
                )}
              </div>
            ) : null}
          </div>

          <div className="field">
            <span>Histórico do agente</span>
            <ul className="agent-history-list">
              {runHistory.length === 0 ? (
                <li className="empty-note">
                  Sem execuções ainda. Use “Executar com agente” acima.
                </li>
              ) : (
                runHistory.map((run) => {
                  const msgs = runTranscripts[run.id] ?? [];
                  const promptMsg = msgs.find((message) => message.role === "user");
                  const resultMsg = [...msgs]
                    .reverse()
                    .find((message) => message.role === "assistant");
                  return (
                    <li key={run.id} className="agent-run-item">
                      <details
                        onToggle={(event) => {
                          if ((event.target as HTMLDetailsElement).open) loadTranscript(run.id);
                        }}
                      >
                        <summary>
                          <span className="run-time">
                            {new Date(run.created_at).toLocaleString()}
                          </span>
                          <span className="run-agent">{run.provider}</span>
                          <span className={`run-status status-${run.status}`}>
                            {runStatusLabel(run.status)}
                          </span>
                        </summary>
                        <div className="run-detail">
                          <div className="run-section">
                            <span className="run-section-label">Prompt</span>
                            {promptMsg ? (
                              <div className="run-result">
                                <AgentMarkdown text={promptMsg.content} />
                              </div>
                            ) : (
                              <p className="empty-note">—</p>
                            )}
                          </div>
                          <div className="run-section">
                            <span className="run-section-label">Resultado</span>
                            {resultMsg ? (
                              <div className="run-result">
                                <AgentMessageContent message={resultMsg} />
                              </div>
                            ) : (
                              <p className="empty-note">
                                {msgs.length
                                  ? "(sem resposta do assistente)"
                                  : "Abra para carregar…"}
                              </p>
                            )}
                          </div>
                        </div>
                      </details>
                    </li>
                  );
                })
              )}
            </ul>
          </div>
        </div>

        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={onClose}>
            Fechar
          </button>
          <button
            className="primary-button"
            type="button"
            onClick={() => void save()}
            disabled={saving}
          >
            <Check aria-hidden="true" size={16} />
            Salvar
          </button>
        </div>
      </section>
    </div>
  );
}

function AddProjectModal({
  workspaceId,
  onClose,
  onAdded,
  t,
}: {
  workspaceId: number;
  onClose: () => void;
  onAdded: (project: Project) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}) {
  const [mode, setMode] = useState<"clone" | "local">("clone");
  const [remoteUrl, setRemoteUrl] = useState("");
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [progress, setProgress] = useState<string[]>([]);
  const [needAuth, setNeedAuth] = useState(false);
  const [username, setUsername] = useState("");
  const [token, setToken] = useState("");
  const cloneIdRef = useRef<string>("");

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void listen<{ clone_id: string; line: string }>("clone://progress", (event) => {
      if (disposed || event.payload.clone_id !== cloneIdRef.current) return;
      setProgress((lines) => [...lines.slice(-200), event.payload.line]);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  function basename(value: string): string {
    return (
      value
        .replace(/[\\/]+$/, "")
        .split(/[\\/]/)
        .pop() ?? ""
    );
  }

  async function pickFolder() {
    setError("");
    const result = await api.pickDirectory();
    if (!result.ok) {
      setError(result.error);
      return;
    }
    if (result.value) {
      setPath(result.value);
      if (!name.trim()) setName(basename(result.value));
    }
  }

  async function submit() {
    setError("");
    setBusy(true);
    try {
      if (mode === "clone") {
        const url = remoteUrl.trim();
        if (!url) {
          setError("Informe a URL do repositório.");
          return;
        }
        setProgress([]);
        const cloneId = crypto.randomUUID();
        cloneIdRef.current = cloneId;
        const result = await api.cloneGitProjectStreamed({
          workspace_id: workspaceId,
          remote_url: url,
          name: name.trim() || null,
          clone_id: cloneId,
          username: needAuth ? username.trim() || null : null,
          token: needAuth ? token.trim() || null : null,
        });
        if (!result.ok) {
          if (result.error.includes("AUTH_REQUIRED")) {
            setNeedAuth(true);
            setError("Repositório privado: informe usuário e token e clone de novo.");
          } else {
            setError(result.error);
          }
          return;
        }
        onAdded(result.value);
      } else {
        const folder = path.trim();
        if (!folder) {
          setError("Selecione a pasta do projeto.");
          return;
        }
        const projectName = name.trim() || basename(folder) || "projeto";
        const result = await api.addLocalProject(workspaceId, projectName, folder);
        if (!result.ok) {
          setError(result.error);
          return;
        }
        onAdded(result.value);
      }
    } finally {
      setBusy(false);
      cloneIdRef.current = "";
    }
  }

  return (
    <div className="modal-backdrop elevated" role="presentation" onClick={onClose}>
      <section
        className="modal-panel task-modal add-project-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="add-project-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="modal-heading">
          <div>
            <span className="section-label">CLIA LOCAL</span>
            <h2 id="add-project-title">{t("project.modal.title")}</h2>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        {error ? <div className="error-banner compact">{error}</div> : null}

        <div className="chip-row" role="tablist">
          <button
            type="button"
            className={mode === "clone" ? "chip active" : "chip"}
            onClick={() => setMode("clone")}
          >
            {t("project.modal.clone")}
          </button>
          <button
            type="button"
            className={mode === "local" ? "chip active" : "chip"}
            onClick={() => setMode("local")}
          >
            {t("project.modal.local")}
          </button>
        </div>

        <div className="task-modal-body">
          {mode === "clone" ? (
            <>
              <label className="field">
                <span>{t("project.modal.remote")}</span>
                <input
                  value={remoteUrl}
                  placeholder="https://github.com/usuario/repo.git"
                  autoFocus
                  onChange={(event) => setRemoteUrl(event.target.value)}
                />
              </label>
              {needAuth ? (
                <div className="task-modal-row">
                  <label className="field">
                    <span>Usuário</span>
                    <input value={username} onChange={(event) => setUsername(event.target.value)} />
                  </label>
                  <label className="field">
                    <span>Token / senha</span>
                    <input
                      type="password"
                      value={token}
                      onChange={(event) => setToken(event.target.value)}
                    />
                  </label>
                </div>
              ) : null}
              {progress.length ? (
                <div className="agent-inline-stream">
                  {progress.slice(-12).map((line, index) => (
                    <div key={index} className="agent-msg">
                      <pre>{line}</pre>
                    </div>
                  ))}
                </div>
              ) : null}
            </>
          ) : (
            <div className="field">
              <span>{t("project.modal.path")}</span>
              <div className="task-run-row">
                <input
                  value={path}
                  placeholder="/home/voce/code/projeto"
                  style={{ flex: 1 }}
                  onChange={(event) => setPath(event.target.value)}
                />
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => void pickFolder()}
                >
                  <FolderOpen aria-hidden="true" size={15} />
                  Selecionar
                </button>
              </div>
            </div>
          )}

          <label className="field">
            <span>{t("project.modal.name")}</span>
            <input
              value={name}
              placeholder={mode === "clone" ? "(opcional — derivado da URL)" : ""}
              onChange={(event) => setName(event.target.value)}
            />
          </label>
        </div>

        <div className="modal-actions">
          {busy && mode === "clone" ? (
            <button
              className="secondary-button"
              type="button"
              onClick={() => void api.cancelClone(cloneIdRef.current)}
            >
              <X aria-hidden="true" size={16} />
              Cancelar
            </button>
          ) : (
            <button className="secondary-button" type="button" onClick={onClose} disabled={busy}>
              {t("common.close")}
            </button>
          )}
          <button
            className="primary-button"
            type="button"
            onClick={() => void submit()}
            disabled={busy}
          >
            {mode === "clone" ? (
              <Download aria-hidden="true" size={16} />
            ) : (
              <FolderPlus aria-hidden="true" size={16} />
            )}
            {busy
              ? mode === "clone"
                ? "Clonando…"
                : "Adicionando…"
              : mode === "clone"
                ? t("project.modal.cloneSubmit")
                : t("project.modal.addLocal")}
          </button>
        </div>
      </section>
    </div>
  );
}

function ProjectAnalysisModal({
  projectName,
  analyzeCommand,
  suggestCommand,
  analyzed,
  hasAgent,
  messages,
  onClose,
  onConfigureAgent,
  onStart,
  onAnswer,
  onAnalyzed,
  onCreateTasks,
}: {
  projectName: string;
  analyzeCommand: string;
  suggestCommand?: string;
  analyzed: boolean | null;
  hasAgent: boolean;
  messages: AgentMessage[];
  onClose: () => void;
  onConfigureAgent: () => void;
  onStart: (message: string) => Promise<boolean>;
  onAnswer: (message: string) => Promise<boolean>;
  onAnalyzed: () => void;
  onCreateTasks: (items: SuggestionItem[]) => Promise<number>;
}) {
  const [mode, setMode] = useState<"idle" | "analyze" | "opportunities">("idle");
  const [question, setQuestion] = useState<{ question: string; options?: string[] } | null>(null);
  const [working, setWorking] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [suggestions, setSuggestions] = useState<SuggestionItem[] | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [analyzeDone, setAnalyzeDone] = useState(false);
  const [createdCount, setCreatedCount] = useState<number | null>(null);
  const parsedIdRef = useRef<number | null>(null);

  useEffect(() => {
    if (mode === "idle") return;
    const latest = [...messages]
      .reverse()
      .find((message) => message.role === "assistant" && message.content.trim());
    if (!latest || parsedIdRef.current === latest.id) return;
    if (mode === "analyze") {
      const parsed = parseAnalyzeInterviewResponse(latest.content);
      if (!parsed) return;
      parsedIdRef.current = latest.id;
      if (parsed.state === "question") {
        setQuestion({ question: parsed.question, options: parsed.options });
        setWorking("");
      } else if (parsed.state === "working") {
        setWorking(parsed.message ?? "Trabalhando…");
        setQuestion(null);
      } else {
        setMode("idle");
        setQuestion(null);
        setWorking("");
        setAnalyzeDone(true);
        onAnalyzed();
      }
    } else {
      const parsed = parseSuggestionInterviewResponse(latest.content);
      if (!parsed) return;
      parsedIdRef.current = latest.id;
      if (parsed.state === "question") {
        setQuestion({ question: parsed.question, options: parsed.options });
        setWorking("");
      } else if (parsed.state === "working") {
        setWorking(parsed.message ?? "Trabalhando…");
        setQuestion(null);
      } else {
        setMode("idle");
        setQuestion(null);
        setWorking("");
        setSuggestions(parsed.suggestions);
        setSelected(new Set(parsed.suggestions.map((_, index) => index)));
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages, mode]);

  async function run(kind: "analyze" | "opportunities") {
    const command = kind === "analyze" ? analyzeCommand : suggestCommand;
    if (!command) return;
    parsedIdRef.current = null;
    setSuggestions(null);
    setCreatedCount(null);
    setAnalyzeDone(false);
    setQuestion(null);
    setWorking("Iniciando…");
    setMode(kind);
    setBusy(true);
    const message =
      kind === "analyze"
        ? analyzeInterviewPrompt({ projectName, analyzeCommand: command })
        : suggestionInterviewPrompt({ projectName, suggestCommand: command });
    const ok = await onStart(message);
    setBusy(false);
    if (!ok) {
      setMode("idle");
      setWorking("");
    }
  }

  async function answerWith(value: string) {
    const answer = value.trim();
    if (!answer || mode === "idle") return;
    parsedIdRef.current = null;
    setQuestion(null);
    setNote("");
    setWorking("Enviando…");
    setBusy(true);
    const message =
      mode === "analyze" ? analyzeAnswerPrompt(answer) : suggestionAnswerPrompt(answer);
    await onAnswer(message);
    setBusy(false);
  }

  async function addSelected() {
    if (!suggestions) return;
    const chosen = suggestions.filter((_, index) => selected.has(index));
    if (!chosen.length) return;
    setBusy(true);
    const created = await onCreateTasks(chosen);
    setBusy(false);
    setCreatedCount(created);
    setSuggestions(null);
  }

  const running = mode !== "idle";

  return (
    <div className="modal-backdrop elevated" role="presentation" onClick={onClose}>
      <section
        className="modal-panel task-modal analysis-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="analysis-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="modal-heading">
          <div>
            <span className="section-label">{projectName}</span>
            <h2 id="analysis-title">Análise do projeto</h2>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        {!hasAgent ? (
          <div className="task-modal-body">
            <p className="empty-note">
              Configure um agente antes de rodar a análise. Ela executa{" "}
              <code>{analyzeCommand}</code> pelo agente local (use um perfil com permissão de
              escrita para gravar <code>.dw/rules/</code>).
            </p>
            <div className="modal-actions">
              <button className="secondary-button" type="button" onClick={onClose}>
                Fechar
              </button>
              <button className="primary-button" type="button" onClick={onConfigureAgent}>
                <Bot aria-hidden="true" size={16} />
                Configurar agente
              </button>
            </div>
          </div>
        ) : (
          <div className="task-modal-body">
            <div className="field">
              <span>
                Status: {analyzed === false ? "pendente" : analyzed ? "analisado" : "verificando…"}
              </span>
              <div className="task-run-row">
                <button
                  className="primary-button"
                  type="button"
                  onClick={() => void run("analyze")}
                  disabled={busy || running}
                >
                  <Sparkles aria-hidden="true" size={15} />
                  {analyzed ? "Reanalisar" : "Analisar projeto"}
                </button>
                {suggestCommand ? (
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => void run("opportunities")}
                    disabled={busy || running}
                  >
                    <Sparkles aria-hidden="true" size={15} />
                    Mapear oportunidades
                  </button>
                ) : null}
              </div>
            </div>

            {analyzeDone ? (
              <p className="empty-note">Análise concluída — as rules do projeto foram geradas.</p>
            ) : null}
            {createdCount != null ? (
              <p className="empty-note">{createdCount} tarefa(s) criada(s) no kanban.</p>
            ) : null}

            {running ? (
              <div className="agent-inline-stream">
                {working ? <p className="empty-note">{working}</p> : null}
                {question ? (
                  <div className="analysis-question">
                    <p>{question.question}</p>
                    {question.options?.length ? (
                      <div className="chip-row">
                        {question.options.map((option) => (
                          <button
                            key={option}
                            type="button"
                            className="chip"
                            disabled={busy}
                            onClick={() => void answerWith(option)}
                          >
                            {option}
                          </button>
                        ))}
                      </div>
                    ) : null}
                    <div className="task-run-row">
                      <input
                        value={note}
                        placeholder="Sua resposta…"
                        style={{ flex: 1 }}
                        disabled={busy}
                        onChange={(event) => setNote(event.target.value)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") {
                            event.preventDefault();
                            void answerWith(note);
                          }
                        }}
                      />
                      <button
                        className="secondary-button"
                        type="button"
                        disabled={busy || !note.trim()}
                        onClick={() => void answerWith(note)}
                      >
                        <Send aria-hidden="true" size={15} />
                        Responder
                      </button>
                    </div>
                  </div>
                ) : null}
              </div>
            ) : null}

            {suggestions ? (
              <div className="field">
                <span>
                  Oportunidades ({selected.size}/{suggestions.length} selecionadas)
                </span>
                <ul className="checklist">
                  {suggestions.map((item, index) => (
                    <li key={`${item.title}-${index}`}>
                      <label>
                        <input
                          type="checkbox"
                          checked={selected.has(index)}
                          onChange={() =>
                            setSelected((current) => {
                              const next = new Set(current);
                              if (next.has(index)) next.delete(index);
                              else next.add(index);
                              return next;
                            })
                          }
                        />
                        <span>
                          {item.kind ? `[${item.kind}] ` : ""}
                          {item.title}
                          {item.body ? ` — ${item.body}` : ""}
                        </span>
                      </label>
                    </li>
                  ))}
                </ul>
                <button
                  className="primary-button"
                  type="button"
                  disabled={busy || selected.size === 0}
                  onClick={() => void addSelected()}
                >
                  <Plus aria-hidden="true" size={15} />
                  Adicionar selecionadas como tarefas
                </button>
              </div>
            ) : null}
          </div>
        )}
      </section>
    </div>
  );
}

function WorkspaceSettingsPanel({
  activeProject,
  agentProfiles,
  busy,
  locale,
  onAgentRtkChange,
  onLocaleChange,
  workspace,
  editorFontSize,
  onEditorFontSizeChange,
  themeMode,
  onThemeModeChange,
  workspaceAccentColor,
  onWorkspaceAccentColorChange,
  t,
}: {
  activeProject: Project | null;
  agentProfiles: AgentProfile[];
  busy: boolean;
  locale: Locale;
  onAgentRtkChange: (profile: AgentProfile, enabled: boolean) => Promise<AgentProfile | null>;
  onLocaleChange: (locale: Locale) => void;
  workspace: Workspace;
  editorFontSize: number;
  onEditorFontSizeChange: (value: number) => void;
  themeMode: ThemeMode;
  onThemeModeChange: (value: ThemeMode) => void;
  workspaceAccentColor: string | null;
  onWorkspaceAccentColorChange: (value: string | null) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}) {
  const rtkProjectPath = activeProject?.path ?? workspace.root_path;
  const [rtkStatuses, setRtkStatuses] = useState<Record<number, RtkStatus>>({});
  const [rtkBusyProfileId, setRtkBusyProfileId] = useState<number | null>(null);
  const [rtkError, setRtkError] = useState("");
  const [activeSettingsSection, setActiveSettingsSection] = useState<
    "theme" | "accent" | "language" | "editor" | "rtk"
  >("theme");

  async function refreshRtkStatus(profile: AgentProfile) {
    const result = await api.getRtkStatus(profile.id, rtkProjectPath);
    if (!result.ok) {
      setRtkError(result.error);
      return null;
    }
    setRtkStatuses((current) => ({ ...current, [profile.id]: result.value }));
    return result.value;
  }

  async function toggleRtk(profile: AgentProfile) {
    setRtkError("");
    setRtkBusyProfileId(profile.id);
    const nextEnabled = !profile.rtk_enabled;
    const updated = await onAgentRtkChange(profile, nextEnabled);
    if (updated && nextEnabled) {
      const install = await api.installRtk(updated.id, rtkProjectPath);
      if (!install.ok) {
        setRtkError(install.error);
        setRtkBusyProfileId(null);
        return;
      }
      setRtkStatuses((current) => ({ ...current, [updated.id]: install.value.status }));
    } else if (updated) {
      await refreshRtkStatus(updated);
    }
    setRtkBusyProfileId(null);
  }

  async function installRtk(profile: AgentProfile) {
    setRtkError("");
    setRtkBusyProfileId(profile.id);
    const result = await api.installRtk(profile.id, rtkProjectPath);
    if (!result.ok) {
      setRtkError(result.error);
      setRtkBusyProfileId(null);
      return;
    }
    setRtkStatuses((current) => ({ ...current, [profile.id]: result.value.status }));
    setRtkBusyProfileId(null);
  }

  async function configureRtk(profile: AgentProfile) {
    setRtkError("");
    setRtkBusyProfileId(profile.id);
    const currentStatus = rtkStatuses[profile.id] ?? (await refreshRtkStatus(profile));
    if (!currentStatus?.available) {
      const install = await api.installRtk(profile.id, rtkProjectPath);
      if (!install.ok) {
        setRtkError(install.error);
        setRtkBusyProfileId(null);
        return;
      }
      setRtkStatuses((current) => ({ ...current, [profile.id]: install.value.status }));
    }
    const preview = await api.configureRtk(profile.id, rtkProjectPath, false);
    if (!preview.ok) {
      setRtkError(preview.error);
      setRtkBusyProfileId(null);
      return;
    }
    const commands = preview.value.commands
      .map((command) => `${command.cwd}\n$ ${command.command}\n${command.description}`)
      .join("\n\n");
    const confirmed = window.confirm(
      `RTK setup preview for ${profile.name}\n\n${commands}\n\nApply these changes now?`,
    );
    if (!confirmed) {
      setRtkBusyProfileId(null);
      return;
    }
    const applied = await api.configureRtk(profile.id, rtkProjectPath, true);
    if (!applied.ok) {
      setRtkError(applied.error);
      setRtkBusyProfileId(null);
      return;
    }
    setRtkStatuses((current) => ({ ...current, [profile.id]: applied.value.status }));
    setRtkBusyProfileId(null);
  }

  useEffect(() => {
    let cancelled = false;
    async function load() {
      const entries = await Promise.all(
        agentProfiles.map(async (profile) => {
          const result = await api.getRtkStatus(profile.id, rtkProjectPath);
          return result.ok ? ([profile.id, result.value] as const) : null;
        }),
      );
      if (!cancelled) {
        setRtkStatuses(Object.fromEntries(entries.filter(Boolean) as Array<[number, RtkStatus]>));
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [agentProfiles, rtkProjectPath]);

  const settingsNavGroups: Array<{
    title: string;
    items: Array<{
      id: typeof activeSettingsSection;
      label: string;
      icon: ReactNode;
    }>;
  }> = [
    {
      title: "Aparência",
      items: [
        { id: "theme", label: "Tema", icon: <CircleDot aria-hidden="true" size={16} /> },
        { id: "accent", label: "Cor de destaque", icon: <Pencil aria-hidden="true" size={16} /> },
        { id: "language", label: "Idioma", icon: <Settings aria-hidden="true" size={16} /> },
        { id: "editor", label: "Tipografia", icon: <Code2 aria-hidden="true" size={16} /> },
      ],
    },
    {
      title: "Agentes",
      items: [{ id: "rtk", label: "Agent RTK", icon: <Bot aria-hidden="true" size={16} /> }],
    },
  ];

  return (
    <section className="settings-panel" aria-labelledby="workspace-settings-title">
      <header className="settings-topbar screen-topbar">
        <span className="topbar-title" id="workspace-settings-title">
          Settings
        </span>
        <span className="topbar-path">{workspace.name}</span>
      </header>

      <div className="settings-main">
        <aside className="settings-nav" aria-label="Seções de configurações">
          {settingsNavGroups.map((group) => (
            <div className="settings-nav-section" key={group.title}>
              <div className="settings-nav-title">{group.title}</div>
              <div className="settings-nav-items">
                {group.items.map((item) => (
                  <button
                    className={
                      activeSettingsSection === item.id
                        ? "settings-nav-item active"
                        : "settings-nav-item"
                    }
                    key={item.id}
                    type="button"
                    aria-current={activeSettingsSection === item.id ? "page" : undefined}
                    onClick={() => setActiveSettingsSection(item.id)}
                  >
                    {item.icon}
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </aside>

        <div className="workspace-settings-form settings-page-form settings-content">
          {activeSettingsSection === "theme" ? (
            <section className="settings-section" id="settings-appearance">
              <h2 className="settings-section-title">{t("workspace.settings.theme.title")}</h2>
              <p className="settings-section-desc">{t("workspace.settings.theme.description")}</p>
              <div className="settings-group">
                <div className="settings-row">
                  <div className="settings-row-info">
                    <div className="settings-row-label">{t("workspace.settings.theme.title")}</div>
                    <div className="settings-row-hint">
                      {t("workspace.settings.theme.description")}
                    </div>
                  </div>
                  <div
                    className="settings-row-control theme-control"
                    role="radiogroup"
                    aria-label={t("workspace.settings.theme.title")}
                  >
                    <button
                      className={themeMode === "clia" ? "theme-choice active" : "theme-choice"}
                      type="button"
                      aria-pressed={themeMode === "clia"}
                      onClick={() => onThemeModeChange("clia")}
                    >
                      <span className="theme-preview theme-preview-clia" aria-hidden="true" />
                      {t("workspace.settings.theme.clia")}
                    </button>
                    <button
                      className={themeMode === "black" ? "theme-choice active" : "theme-choice"}
                      type="button"
                      aria-pressed={themeMode === "black"}
                      onClick={() => onThemeModeChange("black")}
                    >
                      <span className="theme-preview theme-preview-black" aria-hidden="true" />
                      {t("workspace.settings.theme.black")}
                    </button>
                  </div>
                </div>
              </div>
            </section>
          ) : null}

          {activeSettingsSection === "accent" ? (
            <section className="settings-section" id="settings-accent">
              <h2 className="settings-section-title">{t("workspace.settings.color.title")}</h2>
              <p className="settings-section-desc">{t("workspace.settings.color.description")}</p>
              <div className="settings-group">
                <div className="settings-row">
                  <div className="settings-row-info">
                    <div className="settings-row-label">
                      {t("workspace.settings.color.title")}
                    </div>
                    <div className="settings-row-hint">
                      {t("workspace.settings.color.description")}
                    </div>
                  </div>
                  <div className="settings-row-control workspace-color-control">
                    <div className="workspace-color-preview">
                      <span
                        className="workspace-color-preview-swatch"
                        style={{
                          background: workspaceAccentColor ?? "transparent",
                          borderColor: workspaceAccentColor ?? "#3a4656",
                        }}
                        aria-hidden="true"
                      />
                      <span>{workspaceAccentColor ?? t("common.noColor")}</span>
                    </div>
                    <div
                      className="workspace-color-swatch-grid"
                      aria-label={t("workspace.settings.color.title")}
                    >
                      {WORKSPACE_COLOR_PRESETS.map((preset) => (
                        <button
                          key={preset.color}
                          className={
                            workspaceAccentColor === preset.color
                              ? "workspace-color-swatch active"
                              : "workspace-color-swatch"
                          }
                          type="button"
                          style={{ background: preset.color }}
                          title={preset.label}
                          aria-label={preset.label}
                          aria-pressed={workspaceAccentColor === preset.color}
                          onClick={() => onWorkspaceAccentColorChange(preset.color)}
                        />
                      ))}
                    </div>
                    <button
                      className="secondary-button workspace-color-reset"
                      type="button"
                      onClick={() => onWorkspaceAccentColorChange(null)}
                      disabled={!workspaceAccentColor}
                    >
                      {t("workspace.settings.color.remove")}
                    </button>
                  </div>
                </div>
              </div>
            </section>
          ) : null}

          {activeSettingsSection === "language" ? (
            <section className="settings-section" id="settings-language">
              <h2 className="settings-section-title">{t("workspace.settings.language.title")}</h2>
              <p className="settings-section-desc">
                {t("workspace.settings.language.description")}
              </p>
              <div className="settings-group">
                <div className="settings-row">
                  <div className="settings-row-info">
                    <div className="settings-row-label">
                      {t("workspace.settings.language.title")}
                    </div>
                    <div className="settings-row-hint">
                      {t("workspace.settings.language.description")}
                    </div>
                  </div>
                  <div
                    className="settings-row-control language-control"
                    role="radiogroup"
                    aria-label={t("workspace.settings.language.title")}
                  >
                    <button
                      className={locale === "en" ? "language-choice active" : "language-choice"}
                      type="button"
                      aria-pressed={locale === "en"}
                      onClick={() => onLocaleChange("en")}
                    >
                      {t("workspace.settings.language.en")}
                    </button>
                    <button
                      className={
                        locale === "pt-BR" ? "language-choice active" : "language-choice"
                      }
                      type="button"
                      aria-pressed={locale === "pt-BR"}
                      onClick={() => onLocaleChange("pt-BR")}
                    >
                      {t("workspace.settings.language.ptBR")}
                    </button>
                  </div>
                </div>
              </div>
            </section>
          ) : null}

          {activeSettingsSection === "editor" ? (
            <section className="settings-section" id="settings-editor">
              <h2 className="settings-section-title">{t("workspace.settings.editor.title")}</h2>
              <p className="settings-section-desc">{t("workspace.settings.editor.description")}</p>
              <div className="settings-group">
                <div className="settings-row">
                  <div className="settings-row-info">
                    <div className="settings-row-label">
                      {t("workspace.settings.editor.title")}
                    </div>
                    <div className="settings-row-hint">
                      {t("workspace.settings.editor.description")}
                    </div>
                  </div>
                  <div className="settings-row-control font-size-control">
                    <input
                      className="settings-slider"
                      type="range"
                      min={MIN_EDITOR_FONT_SIZE}
                      max={MAX_EDITOR_FONT_SIZE}
                      step={1}
                      value={editorFontSize}
                      onChange={(event) => onEditorFontSizeChange(Number(event.target.value))}
                      aria-label={t("workspace.settings.editor.title")}
                    />
                    <input
                      className="settings-number-input"
                      type="number"
                      min={MIN_EDITOR_FONT_SIZE}
                      max={MAX_EDITOR_FONT_SIZE}
                      value={editorFontSize}
                      onChange={(event) => onEditorFontSizeChange(Number(event.target.value))}
                      aria-label={t("workspace.settings.editorPx")}
                    />
                    <span className="settings-slider-value">{editorFontSize}px</span>
                  </div>
                </div>
              </div>
            </section>
          ) : null}

          {activeSettingsSection === "rtk" ? (
            <section className="settings-section rtk-settings-section" id="settings-rtk">
              <h2 className="settings-section-title">{t("workspace.settings.rtk.title")}</h2>
              <p className="settings-section-desc">{t("workspace.settings.rtk.description")}</p>
              {rtkError ? <div className="error-banner compact">{rtkError}</div> : null}
              <div className="settings-group rtk-agent-list">
                {agentProfiles.length ? (
                  agentProfiles.map((profile) => {
                    const status = rtkStatuses[profile.id];
                    const busyProfile = rtkBusyProfileId === profile.id;
                    const available = Boolean(status?.available);
                    return (
                      <article className="settings-row rtk-agent-row" key={profile.id}>
                        <div className="rtk-agent-main">
                          <Sparkles aria-hidden="true" size={18} />
                          <span>
                            <strong>{profile.name}</strong>
                            <small>
                              {agentProviderLabel(profile.provider)} ·{" "}
                              {profile.rtk_enabled
                                ? t("workspace.settings.rtk.enabled")
                                : t("workspace.settings.rtk.disabled")}
                            </small>
                          </span>
                        </div>
                        <span className={available ? "status-pill ready" : "status-pill"}>
                          {status?.setup_state ?? t("common.loading")}
                        </span>
                        <small className="rtk-agent-message">
                          {status?.version ??
                            status?.message ??
                            t("workspace.settings.rtk.checking")}
                        </small>
                        <div className="rtk-agent-actions">
                          <button
                            className="secondary-button"
                            type="button"
                            onClick={() => void toggleRtk(profile)}
                            disabled={busy || busyProfile}
                          >
                            {profile.rtk_enabled
                              ? t("workspace.settings.rtk.disable")
                              : t("workspace.settings.rtk.enable")}
                          </button>
                          <button
                            className="secondary-button"
                            type="button"
                            onClick={() => void installRtk(profile)}
                            disabled={busy || busyProfile}
                            title={status?.message}
                          >
                            {available
                              ? t("workspace.settings.rtk.checkInstall")
                              : t("workspace.settings.rtk.install")}
                          </button>
                          <button
                            className="secondary-button"
                            type="button"
                            onClick={() => void configureRtk(profile)}
                            disabled={busy || busyProfile || !profile.rtk_enabled || !available}
                            title={status?.message}
                          >
                            {t("workspace.settings.rtk.setup")}
                          </button>
                          <button
                            className="secondary-button icon-button"
                            type="button"
                            onClick={() => void refreshRtkStatus(profile)}
                            disabled={busy || busyProfile}
                            aria-label={t("workspace.settings.rtk.refresh")}
                            title={t("workspace.settings.rtk.refresh")}
                          >
                            <RefreshCw aria-hidden="true" size={14} />
                          </button>
                        </div>
                      </article>
                    );
                  })
                ) : (
                  <div className="settings-row">
                    <div className="empty-note">{t("workspace.settings.rtk.emptyAgents")}</div>
                  </div>
                )}
              </div>
            </section>
          ) : null}
        </div>
      </div>
    </section>
  );
}

function FlowInterviewModal({
  agentProfiles,
  onAnswer,
  onClose,
  onStart,
  onUrlChange,
  state,
  url,
}: {
  agentProfiles: AgentProfile[];
  onAnswer: (selected: InterviewOptionKey, note: string) => void;
  onClose: () => void;
  onStart: (profileId: number, url: string) => void;
  onUrlChange: (url: string) => void;
  state: {
    sessionId: number | null;
    profileId: number | null;
    status: "idle" | "asking" | "generating" | "ready" | "error";
    question: InterviewQuestion | null;
    turns: FlowInterviewTurn[];
    error?: string;
  };
  url: string;
}) {
  const [selectedProfileId, setSelectedProfileId] = useState(agentProfiles[0]?.id ?? 0);
  const [note, setNote] = useState("");
  const started = state.sessionId !== null || state.status !== "idle";
  const waiting = state.status === "asking" && !state.question;
  const canStart = Boolean(url.trim() && selectedProfileId && agentProfiles.length);

  return (
    <div className="modal-backdrop elevated" role="presentation">
      <section
        className="modal-panel interview-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="flow-interview-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Novo fluxo</div>
            <h2 id="flow-interview-title">Criar fluxo de URL (com agente)</h2>
            <p>
              Cole a doc da ferramenta; o agente faz algumas perguntas e monta o fluxo. Você revisa
              no editor antes de salvar.
            </p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        <div className="interview-modal-body">
          {!agentProfiles.length ? (
            <div className="form-error" role="status">
              Nenhum agente configurado. Crie um agente na aba Agents antes de continuar.
            </div>
          ) : null}

          {!started ? (
            <div className="interview-start flow-interview-start">
              <label>
                <span>URL da documentação</span>
                <input
                  type="url"
                  value={url}
                  placeholder="https://github.com/org/ferramenta"
                  onChange={(event) => onUrlChange(event.target.value)}
                />
              </label>
              <label>
                <span>Agente</span>
                <select
                  value={selectedProfileId}
                  onChange={(event) => setSelectedProfileId(Number(event.target.value))}
                  disabled={!agentProfiles.length}
                >
                  {agentProfiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>
                      {profile.name} · {agentProviderLabel(profile.provider)}
                    </option>
                  ))}
                </select>
              </label>
              <button
                className="primary-button"
                type="button"
                onClick={() => onStart(selectedProfileId, url)}
                disabled={!canStart}
              >
                <Bot aria-hidden="true" size={17} />
                Começar
              </button>
            </div>
          ) : null}

          {state.error ? (
            <div className="form-error" role="status">
              {state.error}
            </div>
          ) : null}

          {started && state.question ? (
            <div className="interview-question">
              <div className="interview-progress">
                Pergunta {state.question.question_number} · {state.turns.length} respondida(s)
              </div>
              <h4>{state.question.question}</h4>
              <div className="interview-options">
                {(Object.keys(state.question.options) as InterviewOptionKey[]).map((key) => (
                  <button
                    className="interview-option"
                    key={key}
                    type="button"
                    onClick={() => {
                      onAnswer(key, note);
                      setNote("");
                    }}
                  >
                    <strong>{key}</strong>
                    <span>{state.question?.options[key]}</span>
                  </button>
                ))}
              </div>
              <label>
                <span>Observação opcional</span>
                <textarea
                  rows={3}
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                  placeholder="Complemente sua escolha se faltar algum detalhe."
                />
              </label>
            </div>
          ) : null}

          {waiting ? (
            <div className="interview-waiting" role="status">
              O agente está{" "}
              {state.turns.length ? "preparando a próxima pergunta" : "lendo a documentação"}…
            </div>
          ) : null}
        </div>
      </section>
    </div>
  );
}

// Generic, schema-driven renderer for a phase's declared fields.

function AttachmentPreviewModal({
  onClose,
  preview,
}: {
  onClose: () => void;
  preview: AttachmentPreview;
}) {
  const source = `data:${preview.mime_type};base64,${preview.data_base64}`;
  const isPdf = preview.mime_type === "application/pdf";

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel attachment-preview-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="attachment-preview-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Anexo</div>
            <h2 id="attachment-preview-title">{preview.name}</h2>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar preview"
          >
            <X aria-hidden="true" size={17} />
          </button>
        </div>
        <div className="attachment-preview-body">
          {isPdf ? (
            <iframe src={source} title={preview.name} />
          ) : (
            <img src={source} alt={preview.name} />
          )}
        </div>
      </section>
    </div>
  );
}

function DwArtifactPreviewModal({
  onClose,
  preview,
}: {
  onClose: () => void;
  preview: DwArtifactPreview;
}) {
  return (
    <div className="modal-backdrop elevated" role="presentation">
      <section
        className="modal-panel dw-artifact-preview-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dw-artifact-preview-title"
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Artefato DW</div>
            <h2 id="dw-artifact-preview-title">{preview.relativePath}</h2>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar artefato"
          >
            <X aria-hidden="true" size={17} />
          </button>
        </div>
        <article
          className="markdown-preview-body"
          dangerouslySetInnerHTML={{ __html: renderMarkdown(preview.content) }}
        />
      </section>
    </div>
  );
}

function parseNumberArray(value?: string | null) {
  if (!value) return [];
  try {
    const parsed = JSON.parse(value) as unknown;
    return Array.isArray(parsed)
      ? parsed.filter((item): item is number => typeof item === "number" && Number.isFinite(item))
      : [];
  } catch {
    return [];
  }
}

function parseProjectBlueprintAnswers(value?: string | null): ProjectBlueprintAnswer[] {
  if (!value) return [];
  try {
    const parsed = JSON.parse(value) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object")
      .map((item) => {
        const id = typeof item.id === "string" ? item.id : "";
        const area = typeof item.area === "string" ? item.area : "";
        const question = typeof item.question === "string" ? item.question : "";
        const answer = typeof item.answer === "string" ? item.answer : "";
        return id && area && question && answer ? { id, area, question, answer } : null;
      })
      .filter((item): item is ProjectBlueprintAnswer => item !== null);
  } catch {
    return [];
  }
}

function projectBlueprintTaskCount(value?: string | null) {
  if (!value) return 0;
  try {
    const parsed = JSON.parse(value) as unknown;
    return Array.isArray(parsed) ? parsed.length : 0;
  } catch {
    return 0;
  }
}

function projectBlueprintSubprojects(value?: string | null) {
  if (!value) return [];
  try {
    const parsed = JSON.parse(value) as unknown;
    return Array.isArray(parsed)
      ? parsed.filter((item): item is string => typeof item === "string" && item.trim() !== "")
      : [];
  } catch {
    return [];
  }
}

function projectBlueprintStatusLabel(
  status: string,
  t?: (key: TranslationKey, params?: Record<string, string | number>) => string,
): string {
  const labelKeys: Record<string, TranslationKey> = {
    draft: "blueprint.status.draft",
    interviewing: "blueprint.status.interviewing",
    planned: "blueprint.status.planned",
    materialized: "blueprint.status.materialized",
    archived: "blueprint.status.archived",
  };
  const labelKey = labelKeys[status];
  return labelKey && t ? t(labelKey) : labelKey ? translate("pt-BR", labelKey) : status;
}

function knowledgeSourceScopeLabel(source: KnowledgeSource) {
  if (source.scope === "workspace") return "workspace";
  if (source.scope === "project") return "projeto";
  if (source.scope === "blueprint") return "blueprint";
  return source.scope || "fonte";
}

function toggleNumber(values: number[], value: number) {
  return values.includes(value) ? values.filter((item) => item !== value) : [...values, value];
}

function renderMarkdown(content: string) {
  const lines = content.replace(/\r\n/g, "\n").split("\n");
  const html: string[] = [];
  let codeLines: string[] = [];
  let inCodeBlock = false;
  let listType: "ul" | "ol" | null = null;

  function closeList() {
    if (!listType) return;
    html.push(`</${listType}>`);
    listType = null;
  }

  function openList(type: "ul" | "ol") {
    if (listType === type) return;
    closeList();
    html.push(`<${type}>`);
    listType = type;
  }

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      if (inCodeBlock) {
        html.push(`<pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
        codeLines = [];
        inCodeBlock = false;
      } else {
        closeList();
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeLines.push(line);
      continue;
    }

    if (!trimmed) {
      closeList();
      continue;
    }

    const heading = /^(#{1,6})\s+(.+)$/.exec(trimmed);
    if (heading) {
      closeList();
      const level = Math.min(heading[1].length + 1, 6);
      html.push(`<h${level}>${renderInlineMarkdown(heading[2])}</h${level}>`);
      continue;
    }

    if (/^---+$/.test(trimmed)) {
      closeList();
      html.push("<hr />");
      continue;
    }

    const quote = /^>\s?(.+)$/.exec(trimmed);
    if (quote) {
      closeList();
      html.push(`<blockquote>${renderInlineMarkdown(quote[1])}</blockquote>`);
      continue;
    }

    const unordered = /^[-*]\s+(?:\[([ xX])\]\s+)?(.+)$/.exec(trimmed);
    if (unordered) {
      openList("ul");
      const checkbox = unordered[1]
        ? `<span class="markdown-checkbox ${unordered[1].trim() ? "checked" : ""}"></span>`
        : "";
      html.push(`<li>${checkbox}${renderInlineMarkdown(unordered[2])}</li>`);
      continue;
    }

    const ordered = /^\d+\.\s+(.+)$/.exec(trimmed);
    if (ordered) {
      openList("ol");
      html.push(`<li>${renderInlineMarkdown(ordered[1])}</li>`);
      continue;
    }

    closeList();
    html.push(`<p>${renderInlineMarkdown(trimmed)}</p>`);
  }

  closeList();
  if (inCodeBlock) {
    html.push(`<pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
  }

  return html.join("\n");
}

function renderInlineMarkdown(value: string) {
  const links: string[] = [];
  let output = value.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label: string, href: string) => {
    const index = links.length;
    links.push(
      `<a href="${escapeAttribute(safeMarkdownHref(href))}" target="_blank" rel="noreferrer">${escapeHtml(label)}</a>`,
    );
    return `\u0000LINK${index}\u0000`;
  });

  const codes: string[] = [];
  output = output.replace(/`([^`]+)`/g, (_match, code: string) => {
    const index = codes.length;
    codes.push(`<code>${escapeHtml(code)}</code>`);
    return `\u0000CODE${index}\u0000`;
  });

  output = escapeHtml(output)
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\*([^*]+)\*/g, "<em>$1</em>");

  codes.forEach((code, index) => {
    output = output.replace(`\u0000CODE${index}\u0000`, code);
  });
  links.forEach((link, index) => {
    output = output.replace(`\u0000LINK${index}\u0000`, link);
  });

  return output;
}

function safeMarkdownHref(href: string) {
  const trimmed = href.trim();
  if (/^(https?:|mailto:)/i.test(trimmed)) return trimmed;
  if (/^[./#][^\s]*$/.test(trimmed)) return trimmed;
  return "#";
}

function escapeHtml(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function escapeAttribute(value: string) {
  return escapeHtml(value).replace(/`/g, "&#96;");
}

function agentProviderLabel(provider: string) {
  return agentProviderOptions.find((option) => option.value === provider)?.label ?? provider;
}

function defaultAgentName(provider: AgentProvider) {
  return agentProviderLabel(provider);
}

function agentModelOptions(provider: AgentProvider) {
  return agentModelOptionsByProvider[provider] ?? agentModelOptionsByProvider.codex;
}

function agentEffortOptions(provider: AgentProvider) {
  return agentEffortOptionsByProvider[provider] ?? agentEffortOptionsByProvider.codex;
}

function safeAgentProvider(provider?: string | null): AgentProvider {
  return agentProviderOptions.some((option) => option.value === provider)
    ? (provider as AgentProvider)
    : "codex";
}

function AgentProfileModal({
  busy,
  onClose,
  onCreate,
  profile,
}: {
  busy: boolean;
  onClose: () => void;
  onCreate: (draft: AgentProfileDraft) => void;
  profile?: AgentProfile | null;
}) {
  const initialProvider = safeAgentProvider(profile?.provider);
  const initialModel = profile?.model ?? null;
  const initialModelOptions = agentModelOptions(initialProvider);
  const knownModel = initialModel
    ? initialModelOptions.some((option) => option.value === initialModel)
    : false;
  const [name, setName] = useState(profile?.name ?? defaultAgentName(initialProvider));
  const [provider, setProvider] = useState<AgentProvider>(initialProvider);
  const [modelChoice, setModelChoice] = useState(
    !initialModel ? "default" : knownModel ? initialModel : "custom",
  );
  const [customModel, setCustomModel] = useState(initialModel && !knownModel ? initialModel : "");
  const [reasoningEffort, setReasoningEffort] = useState(profile?.reasoning_effort ?? "default");
  const [sandbox, setSandbox] = useState(profile?.sandbox ?? "read-only");
  const [contextMode, setContextMode] = useState<"auto_lean" | "full">(
    profile?.context_mode === "full" ? "full" : "auto_lean",
  );
  const [rtkEnabled, setRtkEnabled] = useState(Boolean(profile?.rtk_enabled));
  const modelOptions = agentModelOptions(provider);
  const effortOptions = agentEffortOptions(provider);
  const selectedModel =
    modelChoice === "default" ? null : modelChoice === "custom" ? customModel.trim() : modelChoice;
  const selectedEffort = reasoningEffort === "default" ? null : reasoningEffort;
  const editing = Boolean(profile);

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className="modal-panel agent-profile-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="agent-profile-title"
      >
        <div className="modal-heading">
          <div>
            <h2 id="agent-profile-title">{editing ? "Editar agente" : "Novo agente"}</h2>
            <p>
              {editing
                ? "Ajuste nome, tipo, modelo, effort e sandbox usados nas próximas execuções."
                : "Configure o agente que vai receber as mensagens e etapas do fluxo."}
            </p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar modal"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        <form
          className="agent-profile-form"
          onSubmit={(event) => {
            event.preventDefault();
            onCreate({
              name: name.trim(),
              provider,
              model: selectedModel || null,
              reasoning_effort: selectedEffort,
              sandbox,
              context_mode: contextMode,
              rtk_enabled: rtkEnabled,
            });
          }}
        >
          <label>
            <span>Nome</span>
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder={defaultAgentName(provider)}
              disabled={busy}
            />
          </label>

          <label>
            <span>Tipo do agente</span>
            <select
              value={provider}
              onChange={(event) => {
                const nextProvider = event.target.value as AgentProvider;
                setProvider(nextProvider);
                setModelChoice("default");
                setCustomModel("");
                setReasoningEffort("default");
                setName((current) => {
                  const trimmed = current.trim();
                  const isDefaultName = agentProviderOptions.some(
                    (option) => option.label === trimmed,
                  );
                  return !trimmed || isDefaultName ? defaultAgentName(nextProvider) : current;
                });
              }}
              disabled={busy}
            >
              {agentProviderOptions.map((option) => (
                <option key={option.value} value={option.value} disabled={!option.enabled}>
                  {option.enabled ? option.label : `${option.label} - em breve`}
                </option>
              ))}
            </select>
          </label>

          <label>
            <span>Modelo</span>
            <select
              value={modelChoice}
              onChange={(event) => setModelChoice(event.target.value)}
              disabled={busy}
            >
              {modelOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          {modelChoice === "custom" ? (
            <label>
              <span>Modelo customizado</span>
              <input
                value={customModel}
                onChange={(event) => setCustomModel(event.target.value)}
                placeholder="ex: gpt-5.3-codex"
                disabled={busy}
              />
            </label>
          ) : null}

          <label>
            <span>Reasoning effort</span>
            <select
              value={reasoningEffort}
              onChange={(event) => setReasoningEffort(event.target.value)}
              disabled={busy}
            >
              {effortOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          <label>
            <span>Sandbox</span>
            <select
              value={sandbox}
              onChange={(event) => setSandbox(event.target.value)}
              disabled={busy}
            >
              {agentSandboxOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          <label>
            <span>Contexto</span>
            <select
              value={contextMode}
              onChange={(event) => setContextMode(event.target.value as "auto_lean" | "full")}
              disabled={busy}
            >
              {agentContextModeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          <label className="agent-profile-checkbox">
            <input
              type="checkbox"
              checked={rtkEnabled}
              onChange={(event) => setRtkEnabled(event.target.checked)}
              disabled={busy}
            />
            <span>
              <strong>Token savings with RTK</strong>
              <small>
                Disponibiliza o RTK para comandos shell do agente. Configure os hooks em Workspace
                settings.
              </small>
            </span>
          </label>

          <div className="modal-actions">
            <button className="secondary-button" type="button" onClick={onClose}>
              Cancelar
            </button>
            <button
              className="primary-button"
              type="submit"
              disabled={busy || !name.trim() || (modelChoice === "custom" && !customModel.trim())}
            >
              <Bot aria-hidden="true" size={17} />
              {editing ? "Salvar agente" : "Criar agente"}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}

function AgentsPanel({
  activeProfile,
  activeSession,
  activeSessions,
  busy,
  composer,
  error,
  health,
  messages,
  metrics,
  onCreateProfile,
  onUpdateProfile,
  onSelectProfile,
  onSelectSession,
  onSend,
  onResetChat,
  onStop,
  profiles,
  project,
  sessions,
  setComposer,
  skills,
}: {
  activeProfile: AgentProfile | null;
  activeSession: AgentSession | null;
  activeSessions: AgentSession[];
  busy: boolean;
  composer: string;
  error: string;
  health: AgentProviderHealth | null;
  messages: AgentMessage[];
  metrics: AgentRunEvent[];
  onCreateProfile: (draft: AgentProfileDraft) => void;
  onUpdateProfile: (id: number, draft: AgentProfileDraft) => void;
  onSelectProfile: (id: number) => void;
  onSelectSession: (id: number) => void;
  onSend: (message: string) => void;
  onResetChat: () => void;
  onStop: (sessionId: number) => void;
  profiles: AgentProfile[];
  project: Project | null;
  sessions: AgentSession[];
  setComposer: (value: string) => void;
  skills: WorkspaceSkill[];
}) {
  const [profileModalOpen, setProfileModalOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<AgentProfile | null>(null);
  const [rawOpen, setRawOpen] = useState(false);
  const [sessionsOpen, setSessionsOpen] = useState(false);
  const [usageByProfile, setUsageByProfile] = useState<Record<number, AgentUsage>>({});
  const [usageBusy, setUsageBusy] = useState(false);
  const usage = activeProfile ? (usageByProfile[activeProfile.id] ?? null) : null;
  const [rawPage, setRawPage] = useState(0);
  const [rawPageSize, setRawPageSize] = useState(10);
  const [skillSuggestionIndex, setSkillSuggestionIndex] = useState(0);
  const [skillAutocompleteHidden, setSkillAutocompleteHidden] = useState(false);
  const timelineRef = useRef<HTMLDivElement | null>(null);
  const rawBodyRef = useRef<HTMLDivElement | null>(null);
  const working = isAgentRunning(activeSession);
  const rawEvents = messages.filter((message) => message.raw_json);
  const latestMetric = metrics.at(-1) ?? null;
  const latestRunMetrics = latestMetric
    ? metrics.filter((metric) => metric.run_id === latestMetric.run_id)
    : [];
  const metricSummary = summarizeAgentMetrics(latestRunMetrics);
  // Newest first (chronological, recent → old).
  const formattedRawEvents = rawEvents
    .map((message) => ({
      id: message.id,
      type: rawEventType(message.raw_json),
      summary: rawEventSummary(message.raw_json),
      value: formatRawJson(message.raw_json),
    }))
    .reverse();
  const visibleMessages = messages.filter((message) => message.role !== "event");
  const sessionStatus = activeSession ? agentStatusLabel(activeSession.status) : "Sem conversa";
  const rawPayload = formattedRawEvents.map((event) => event.value).join("\n\n");
  const rawPageCount = Math.max(1, Math.ceil(formattedRawEvents.length / rawPageSize));
  const safeRawPage = Math.min(rawPage, rawPageCount - 1);
  const rawPageStart = safeRawPage * rawPageSize;
  const pagedRawEvents = formattedRawEvents.slice(rawPageStart, rawPageStart + rawPageSize);
  const lastVisibleMessage = visibleMessages.at(-1);
  const skillQuery = skillAutocompleteQuery(composer);
  const skillSuggestions = useMemo(() => {
    return filterSkillSuggestions(skills, skillQuery);
  }, [skillQuery, skills]);
  const skillSuggestionRows = useMemo(() => {
    let index = 0;
    const groups = groupSkillSuggestions(skillSuggestions);
    return groups.flatMap((group, groupIndex) => {
      const rows: Array<
        | { type: "scope"; label: string }
        | { type: "group"; label: string }
        | { type: "skill"; skill: WorkspaceSkill; index: number }
      > = [];
      if (groupIndex === 0 || groups[groupIndex - 1]?.scopeLabel !== group.scopeLabel) {
        rows.push({ type: "scope", label: group.scopeLabel });
      }
      rows.push({ type: "group", label: group.frameworkLabel });
      rows.push(
        ...group.skills.map((skill) => ({
          type: "skill" as const,
          skill,
          index: index++,
        })),
      );
      return rows;
    });
  }, [skillSuggestions]);
  const skillAutocompleteOpen = !skillAutocompleteHidden && Boolean(project) && skillQuery != null;
  const activeSkillSuggestionIndex = skillSuggestions.length
    ? Math.min(skillSuggestionIndex, skillSuggestions.length - 1)
    : 0;

  async function copyRawEvents() {
    if (!rawPayload) return;
    await navigator.clipboard.writeText(rawPayload);
  }

  async function refreshUsage() {
    if (!activeProfile || !project) return;
    const profileId = activeProfile.id;
    setUsageBusy(true);
    const result = await api.agentUsage(activeProfile.provider, project.path);
    const value: AgentUsage = result.ok
      ? result.value
      : { provider: activeProfile.provider, supported: false, raw: result.error, windows: [] };
    setUsageByProfile((prev) => ({ ...prev, [profileId]: value }));
    setUsageBusy(false);
  }

  useEffect(() => {
    const timeline = timelineRef.current;
    if (!timeline) return;
    window.requestAnimationFrame(() => {
      timeline.scrollTop = timeline.scrollHeight;
    });
  }, [
    activeSession?.id,
    lastVisibleMessage?.content,
    lastVisibleMessage?.id,
    visibleMessages.length,
    working,
  ]);

  useEffect(() => {
    const rawBody = rawBodyRef.current;
    if (!rawBody) return;
    // Newest events are at the top now, so reset to the top.
    window.requestAnimationFrame(() => {
      rawBody.scrollTop = 0;
    });
  }, [rawOpen, safeRawPage, rawPageSize, pagedRawEvents.length]);

  function applyAgentSkillSuggestion(skill: WorkspaceSkill) {
    setComposer(applySkillAutocomplete(composer, skill.name));
    setSkillAutocompleteHidden(true);
  }

  return (
    <section className="agents-panel">
      {profileModalOpen ? (
        <AgentProfileModal
          busy={busy}
          profile={editingProfile}
          onClose={() => {
            setProfileModalOpen(false);
            setEditingProfile(null);
          }}
          onCreate={(draft) => {
            if (editingProfile) onUpdateProfile(editingProfile.id, draft);
            else onCreateProfile(draft);
            setProfileModalOpen(false);
            setEditingProfile(null);
          }}
        />
      ) : null}
      <header className="agents-topbar screen-topbar">
        <span className="topbar-title">Agents</span>
        <div className="topbar-actions">
          <button
            className={rawOpen ? "topbar-btn active" : "topbar-btn"}
            type="button"
            onClick={() => {
              setRawOpen((open) => !open);
              setSessionsOpen(false);
            }}
            aria-pressed={rawOpen}
          >
            <FileText aria-hidden="true" size={14} />
            Eventos
          </button>
          <button
            className="topbar-btn"
            type="button"
            onClick={() => {
              setEditingProfile(activeProfile);
              setProfileModalOpen(true);
            }}
            disabled={!activeProfile}
          >
            <Settings aria-hidden="true" size={14} />
            Perfis
          </button>
        </div>
      </header>
      <aside className="agents-sidebar" aria-label="Agentes">
        <section className="sidebar-section">
          <div className="sidebar-section-title">
            Perfis
            <button
              className="sidebar-section-btn"
              type="button"
              onClick={() => {
                setEditingProfile(null);
                setProfileModalOpen(true);
              }}
              disabled={busy}
              aria-label="Novo agente"
              title="Novo agente"
            >
              <Plus aria-hidden="true" size={14} />
            </button>
          </div>

          <div className="profile-list agent-list">
            {profiles.length ? (
              profiles.map((profile) => {
                const profileWorking = sessions.some(
                  (session) => session.profile_id === profile.id && isAgentRunning(session),
                );
                return (
                  <article
                    className={
                      activeProfile?.id === profile.id
                        ? "profile-card agent-row active"
                        : "profile-card agent-row"
                    }
                    key={profile.id}
                  >
                    <button
                      className="profile-card-main agent-row-main"
                      type="button"
                      onClick={() => onSelectProfile(profile.id)}
                    >
                      <span className="profile-header">
                        <span className="profile-avatar agent-profile-avatar" aria-hidden="true">
                          {authorInitials(profile.name).slice(0, 2)}
                        </span>
                        <span className="profile-name">{profile.name}</span>
                      </span>
                      <span className="profile-meta">
                        <span className="profile-tag model">
                          {profile.model || agentProviderLabel(profile.provider)}
                        </span>
                        <span className="profile-tag">
                          {profile.reasoning_effort || "default"}
                        </span>
                        <span className="profile-tag sandbox">{profile.sandbox}</span>
                        {profile.rtk_enabled ? <span className="profile-tag">RTK</span> : null}
                      </span>
                    </button>
                    {profileWorking ? (
                      <span className="agent-list-status" aria-label="Agente em trabalho">
                        <span className="agent-spinner compact" aria-hidden="true" />
                      </span>
                    ) : null}
                    <button
                      className="secondary-button icon-button agent-edit-button"
                      type="button"
                      onClick={() => {
                        setEditingProfile(profile);
                        setProfileModalOpen(true);
                      }}
                      aria-label={`Editar ${profile.name}`}
                      title="Editar agente"
                    >
                      <Pencil aria-hidden="true" size={15} />
                    </button>
                  </article>
                );
              })
            ) : (
              <button
                className="agent-empty-cta"
                type="button"
                onClick={() => {
                  setEditingProfile(null);
                  setProfileModalOpen(true);
                }}
                disabled={busy}
              >
                <Plus aria-hidden="true" size={18} />
                <span>
                  <strong>Criar agente</strong>
                  <small>Configure um agente para executar o fluxo.</small>
                </span>
              </button>
            )}
          </div>
        </section>

        <section className="sidebar-section agent-session-section">
          <div className="sidebar-section-title">
            Sessões
            <button
              className="sidebar-section-btn"
              type="button"
              onClick={onResetChat}
              disabled={busy || !activeProfile}
              aria-label="Nova sessão"
              title="Nova sessão"
            >
              <Plus aria-hidden="true" size={14} />
            </button>
          </div>
          <div className="session-list">
            {activeSessions.length ? (
              activeSessions.map((session) => (
                <button
                  key={session.id}
                  type="button"
                  className={session.id === activeSession?.id ? "session-item active" : "session-item"}
                  onClick={() => onSelectSession(session.id)}
                >
                  <span className="session-title">{session.title || `#${session.id}`}</span>
                  <span className="session-meta">
                    <span>{agentStatusLabel(session.status)}</span>
                    <span>{new Date(session.updated_at).toLocaleString()}</span>
                  </span>
                </button>
              ))
            ) : (
              <div className="empty-note">Nenhuma conversa ainda.</div>
            )}
          </div>
        </section>

        {activeProfile ? (
          <section className="sidebar-section agent-usage">
            <div className="agent-usage-head">
              <span>Uso · {agentProviderLabel(activeProfile.provider)}</span>
              <button
                className="sidebar-section-btn"
                type="button"
                onClick={() => void refreshUsage()}
                disabled={usageBusy || !project}
                aria-label="Atualizar uso"
                title="Atualizar uso"
              >
                {usageBusy ? (
                  <span className="flow-run-spinner" aria-hidden="true" />
                ) : (
                  <RefreshCw aria-hidden="true" size={14} />
                )}
              </button>
            </div>
            {usage == null ? (
              <span className="agent-usage-hint">Clique em atualizar para consultar.</span>
            ) : !usage.supported ? (
              <span className="agent-usage-hint">Uso indisponível para este provider.</span>
            ) : usage.windows.length ? (
              <div className="agent-usage-windows">
                {usage.windows.map((window, index) => (
                  <div className="agent-usage-window" key={index} title={window.label}>
                    <span className="agent-usage-label">{window.label}</span>
                    {typeof window.pct === "number" ? (
                      <span className="agent-usage-bar">
                        <span style={{ width: `${Math.max(0, Math.min(100, window.pct))}%` }} />
                      </span>
                    ) : null}
                  </div>
                ))}
              </div>
            ) : (
              <span className="agent-usage-hint" title={usage.raw}>
                Uso indisponível (sem dados legíveis).
              </span>
            )}
          </section>
        ) : null}
      </aside>

      <section className="agent-workspace">
        <div className="chat-header agent-session-bar">
          <div className="chat-profile-avatar" aria-hidden="true">
            {activeProfile ? authorInitials(activeProfile.name).slice(0, 2) : "AI"}
          </div>
          <div className="chat-profile-info agent-chat-title">
            <div className="chat-profile-name">{activeProfile?.name || "Selecione um agente"}</div>
            <div className="chat-profile-status">
              <span className={working ? "chat-profile-dot" : "chat-profile-dot idle"} />
              {activeProfile
                ? `Ativo · ${activeSession?.title || sessionStatus}`
                : project
                  ? projectDisplayName(project)
                  : "Sem projeto"}
            </div>
          </div>
          <div className="agent-session-actions">
            {working ? <span className="status-pill ready">Working</span> : null}
            <button
              className="secondary-button agent-raw-toggle"
              type="button"
              onClick={() => {
                setSessionsOpen((open) => !open);
                setRawOpen(false);
              }}
              aria-pressed={sessionsOpen}
              aria-label={sessionsOpen ? "Ocultar conversas" : "Mostrar conversas"}
            >
              <span>Conversas</span>
              <strong>{activeSessions.length}</strong>
            </button>
            {activeSession ? (
              <button
                className="secondary-button"
                type="button"
                onClick={() => onStop(activeSession.id)}
                disabled={activeSession.status !== "running"}
              >
                <Square aria-hidden="true" size={16} />
                Stop
              </button>
            ) : null}
            <button
              className="secondary-button agent-raw-toggle"
              type="button"
              onClick={() => {
                setRawOpen((open) => !open);
                setSessionsOpen(false);
              }}
              aria-pressed={rawOpen}
              aria-label={rawOpen ? "Ocultar eventos raw" : "Mostrar eventos raw"}
            >
              <span>Raw</span>
              <strong>{rawEvents.length}</strong>
            </button>
            <button
              className="secondary-button"
              type="button"
              onClick={onResetChat}
              disabled={busy || !activeProfile}
            >
              <Trash2 aria-hidden="true" size={16} />
              Limpar chat
            </button>
          </div>
        </div>

        {error ? <div className="error-banner">{error}</div> : null}

        <div className="agent-diagnostics" aria-label="Diagnóstico do agente">
          <span className={health?.ok ? "status-pill ready" : "status-pill"}>
            {health
              ? health.ok
                ? "Provider pronto"
                : "Provider com alerta"
              : "Verificando provider"}
          </span>
          {health ? (
            <span title={health.message}>
              {health.program}
              {health.version ? ` · ${health.version}` : ""}
            </span>
          ) : null}
          {latestMetric ? (
            <span>
              {agentMetricPhaseLabel(latestMetric.phase)} · {latestMetric.elapsed_ms}ms
            </span>
          ) : null}
          {metricSummary ? <span>{metricSummary}</span> : null}
        </div>

        <div className={rawOpen || sessionsOpen ? "agent-chat-shell raw-open" : "agent-chat-shell"}>
          <div className="chat-messages agent-timeline" ref={timelineRef} aria-label="Histórico do agente">
            {visibleMessages.length ? (
              visibleMessages.map((message) => (
                <article
                  className={`message agent-message ${
                    message.role === "user" ? "user" : "assistant"
                  }`}
                  key={message.id}
                >
                  <div className="message-header agent-message-meta">
                    <span className="message-author">{message.role === "user" ? "Você" : activeProfile?.name || "Agente"}</span>
                    <time>{new Date(message.created_at).toLocaleString()}</time>
                  </div>
                  <div className="message-content">
                    <AgentMessageContent message={message} />
                  </div>
                </article>
              ))
            ) : (
              <div className="terminal-empty">
                <Bot aria-hidden="true" />
                <span>Envie uma mensagem ou delegue uma etapa do Kanban.</span>
              </div>
            )}
            {working ? (
              <div className="typing-indicator agent-working">
                <span className="typing-dot" aria-hidden="true" />
                <span className="typing-dot" aria-hidden="true" />
                <span className="typing-dot" aria-hidden="true" />
              </div>
            ) : null}
          </div>

          {sessionsOpen ? (
            <aside
              className="agent-raw-panel agent-sessions-drawer"
              aria-label="Conversas do agente"
            >
              <div>
                <span>
                  <strong>Conversas</strong>
                  <small>{activeSessions.length}</small>
                </span>
              </div>
              <div className="agent-raw-body">
                {activeSessions.length ? (
                  activeSessions.map((session) => (
                    <button
                      key={session.id}
                      type="button"
                      className={
                        session.id === activeSession?.id
                          ? "agent-session-item active"
                          : "agent-session-item"
                      }
                      onClick={() => {
                        onSelectSession(session.id);
                        setSessionsOpen(false);
                      }}
                    >
                      <span className="agent-session-item-title">
                        {session.title || `#${session.id}`}
                      </span>
                      <span className="agent-session-item-meta">
                        {agentStatusLabel(session.status)} ·{" "}
                        {new Date(session.updated_at).toLocaleString()}
                      </span>
                    </button>
                  ))
                ) : (
                  <div className="empty-note">Nenhuma conversa ainda.</div>
                )}
              </div>
            </aside>
          ) : null}

          {rawOpen ? (
            <aside className="agent-raw-panel" aria-label="Eventos raw do agente">
              <div>
                <span>
                  <strong>Raw events</strong>
                  <small>{rawEvents.length}</small>
                </span>
                <button
                  className="secondary-button icon-button"
                  type="button"
                  onClick={() => void copyRawEvents()}
                  disabled={!rawEvents.length}
                  aria-label="Copiar eventos raw"
                  title="Copiar raw"
                >
                  <Copy aria-hidden="true" size={15} />
                </button>
              </div>
              <div className="agent-raw-body" ref={rawBodyRef}>
                {pagedRawEvents.map((event) => (
                  <article className="agent-raw-event" key={event.id}>
                    <header>
                      <span>
                        <strong>{event.type}</strong>
                        {event.summary !== "Abrir JSON" ? <em>{event.summary}</em> : null}
                      </span>
                      <small>#{event.id}</small>
                    </header>
                    <pre className="agent-raw-json">
                      <code>{highlightJson(event.value)}</code>
                    </pre>
                  </article>
                ))}
              </div>
              <div className="agent-raw-controls">
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setRawPage((page) => Math.max(0, page - 1))}
                  disabled={safeRawPage === 0}
                >
                  Anterior
                </button>
                <span>
                  {rawEvents.length ? rawPageStart + 1 : 0}-
                  {Math.min(rawPageStart + rawPageSize, rawEvents.length)} de {rawEvents.length}
                </span>
                <select
                  value={rawPageSize}
                  onChange={(event) => {
                    setRawPageSize(Number(event.target.value));
                    setRawPage(0);
                  }}
                  aria-label="Eventos raw por página"
                >
                  <option value={10}>10</option>
                  <option value={20}>20</option>
                  <option value={50}>50</option>
                </select>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setRawPage((page) => Math.min(rawPageCount - 1, page + 1))}
                  disabled={safeRawPage >= rawPageCount - 1}
                >
                  Próxima
                </button>
              </div>
            </aside>
          ) : null}
        </div>

        <form
          className="chat-composer agent-composer"
          onSubmit={(event) => {
            event.preventDefault();
            const message = composer.trim();
            if (!message) return;
            onSend(message);
            setComposer("");
          }}
        >
          <div className="composer-input-wrapper agent-composer-input">
            {skillAutocompleteOpen ? (
              <div className="agent-skill-menu" role="listbox" aria-label="Skills">
                {skillSuggestionRows.length ? (
                  skillSuggestionRows.map((row) =>
                    row.type === "scope" ? (
                      <div className="agent-skill-scope-label" key={`scope:${row.label}`}>
                        {row.label}
                      </div>
                    ) : row.type === "group" ? (
                      <div className="agent-skill-group-label" key={`group:${row.label}`}>
                        {row.label}
                      </div>
                    ) : (
                      <button
                        key={`${row.skill.scope}:${row.skill.scope_label}:${row.skill.name}:${row.skill.path ?? ""}`}
                        type="button"
                        className={
                          row.index === activeSkillSuggestionIndex
                            ? "agent-skill-option active"
                            : "agent-skill-option"
                        }
                        role="option"
                        aria-selected={row.index === activeSkillSuggestionIndex}
                        onMouseEnter={() => setSkillSuggestionIndex(row.index)}
                        onMouseDown={(event) => {
                          event.preventDefault();
                          applyAgentSkillSuggestion(row.skill);
                        }}
                      >
                        <span>
                          <strong>/{row.skill.name}</strong>
                          <small>
                            {row.skill.description || row.skill.owner || row.skill.source}
                          </small>
                        </span>
                        <em>{row.skill.kind || row.skill.scope_label || row.skill.source}</em>
                      </button>
                    ),
                  )
                ) : (
                  <div className="agent-skill-empty">Nenhuma skill encontrada.</div>
                )}
              </div>
            ) : null}
            <textarea
              className="composer-input"
              value={composer}
              onChange={(event) => {
                setComposer(event.target.value);
                setSkillSuggestionIndex(0);
                setSkillAutocompleteHidden(false);
              }}
              onKeyDown={(event) => {
                if (!skillAutocompleteOpen) return;
                if (!skillSuggestions.length) {
                  if (event.key === "Escape") {
                    event.preventDefault();
                    setSkillAutocompleteHidden(true);
                  }
                  return;
                }
                if (event.key === "ArrowDown") {
                  event.preventDefault();
                  setSkillSuggestionIndex((current) => (current + 1) % skillSuggestions.length);
                } else if (event.key === "ArrowUp") {
                  event.preventDefault();
                  setSkillSuggestionIndex(
                    (current) => (current - 1 + skillSuggestions.length) % skillSuggestions.length,
                  );
                } else if (event.key === "Enter" || event.key === "Tab") {
                  event.preventDefault();
                  const skill = skillSuggestions[activeSkillSuggestionIndex] ?? skillSuggestions[0];
                  if (skill) applyAgentSkillSuggestion(skill);
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  setSkillAutocompleteHidden(true);
                }
              }}
              placeholder="Digite sua mensagem… (Ctrl+K para skills)"
              disabled={busy || !project}
            />
            <div className="composer-actions">
              <button className="composer-btn" type="button" title="Anexar arquivo" disabled>
                <Upload aria-hidden="true" size={18} />
              </button>
              <button
                className="composer-btn send"
                type="submit"
                disabled={busy || !project || !composer.trim()}
                title="Enviar"
              >
                <Send aria-hidden="true" size={18} />
              </button>
            </div>
          </div>
          <div className="composer-hints">
            <span className="composer-hint">
              <kbd>Enter</kbd> Enviar
            </span>
            <span className="composer-hint">
              <kbd>Shift</kbd>+<kbd>Enter</kbd> Nova linha
            </span>
            <span className="composer-hint">
              <kbd>Ctrl</kbd>+<kbd>K</kbd> Skills
            </span>
          </div>
        </form>
      </section>
    </section>
  );
}

function formatRawJson(value?: string | null) {
  if (!value) return "";
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

function rawEventType(value?: string | null) {
  if (!value) return "event";
  try {
    const parsed = JSON.parse(value) as { type?: unknown; kind?: unknown };
    const type = typeof parsed.type === "string" ? parsed.type : parsed.kind;
    return typeof type === "string" && type.trim() ? type : "event";
  } catch {
    return "event";
  }
}

function rawEventSummary(value?: string | null) {
  if (!value) return "Sem payload";
  try {
    const parsed = JSON.parse(value) as {
      data?: { content?: unknown; deltaContent?: unknown; status?: unknown };
      response?: { output_text?: unknown };
      result?: unknown;
      sessionId?: unknown;
    };
    const summary =
      parsed.data?.content ??
      parsed.data?.deltaContent ??
      parsed.response?.output_text ??
      parsed.result ??
      parsed.data?.status ??
      parsed.sessionId;
    if (typeof summary === "string" && summary.trim()) {
      return truncateText(summary.replace(/\s+/g, " "), 110);
    }
  } catch {
    return truncateText(value.replace(/\s+/g, " "), 110);
  }
  return "Abrir JSON";
}

function agentMetricPhaseLabel(phase: string) {
  const labels: Record<string, string> = {
    started: "Iniciando",
    command_built: "Comando pronto",
    process_spawned: "Processo iniciado",
    context_policy: "Política de contexto",
    first_event: "Primeiro evento",
    provider_init: "Contexto carregado",
    provider_session: "Sessão",
    model_start: "Modelo iniciou",
    assistant_output: "Respondendo",
    assistant_output_fallback: "Resposta capturada",
    result: "Resultado",
    finished: "Finalizado",
    failed: "Falhou",
    provider_error: "Erro do provider",
  };
  return labels[phase] ?? phase;
}

function summarizeAgentMetrics(metrics: AgentRunEvent[]) {
  if (!metrics.length) return "";
  const result = [...metrics].reverse().find((metric) => metric.phase === "result");
  const contextPolicy = [...metrics].reverse().find((metric) => metric.phase === "context_policy");
  const providerInit = [...metrics]
    .reverse()
    .find((metric) => metric.phase === "provider_init" && metric.details_json.includes("tools"));
  const parts: string[] = [];
  if (contextPolicy) {
    const details = parseMetricDetails(contextPolicy.details_json);
    const effectiveMode = stringDetail(details, "effective_mode");
    if (effectiveMode) parts.push(effectiveMode === "lean" ? "lean" : "full");
  }
  if (providerInit) {
    const details = parseMetricDetails(providerInit.details_json);
    const tools = numberDetail(details, "tools");
    const mcpServers = numberDetail(details, "mcp_servers");
    const skills = numberDetail(details, "skills");
    const slashCommands = numberDetail(details, "slash_commands");
    const contextParts = [
      tools != null ? `${tools} tools` : "",
      mcpServers != null ? `${mcpServers} MCPs` : "",
      skills != null ? `${skills} skills` : "",
      slashCommands != null ? `${slashCommands} slash` : "",
    ].filter(Boolean);
    if (contextParts.length) parts.push(contextParts.join("/"));
  }
  if (result) {
    const details = parseMetricDetails(result.details_json);
    const duration =
      numberDetail(details, "duration_ms") ?? numberDetail(details, "duration_api_ms");
    const inputTokens = numberDetail(details, "input_tokens");
    const outputTokens = numberDetail(details, "output_tokens");
    const cacheCreate = numberDetail(details, "cache_creation_input_tokens");
    const cacheRead = numberDetail(details, "cache_read_input_tokens");
    const cachedInput = numberDetail(details, "cached_input_tokens");
    const cost = numberDetail(details, "total_cost_usd");
    if (duration != null) parts.push(`${duration}ms modelo`);
    if (inputTokens != null || outputTokens != null) {
      parts.push(`${inputTokens ?? 0}/${outputTokens ?? 0} tokens`);
    }
    if (cacheCreate != null) parts.push(`${cacheCreate} cache create`);
    if (cacheRead != null) parts.push(`${cacheRead} cache read`);
    if (cachedInput != null) parts.push(`${cachedInput} cached`);
    if (cost != null) parts.push(`$${cost.toFixed(4)}`);
  }
  return parts.join(" · ");
}

function parseMetricDetails(value: string) {
  try {
    return JSON.parse(value) as Record<string, unknown>;
  } catch {
    return {};
  }
}

function numberDetail(details: Record<string, unknown>, key: string) {
  const value = details[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function stringDetail(details: Record<string, unknown>, key: string) {
  const value = details[key];
  return typeof value === "string" ? value : null;
}

function streamingAssistantMessageId(sessionId: number) {
  return -1_000_000_000 - sessionId;
}

function appendAgentStreamingDelta(
  messages: AgentMessage[],
  sessionId: number,
  delta: string,
): AgentMessage[] {
  const id = streamingAssistantMessageId(sessionId);
  const existing = messages.find((message) => message.id === id);
  if (existing) {
    return messages.map((message) =>
      message.id === id ? { ...message, content: `${message.content}${delta}` } : message,
    );
  }
  return [
    ...messages,
    {
      id,
      session_id: sessionId,
      role: "assistant",
      content: delta,
      raw_json: null,
      created_at: new Date().toISOString(),
    },
  ];
}

function truncateText(value: string, maxLength: number) {
  return value.length > maxLength ? `${value.slice(0, maxLength - 1)}...` : value;
}

function highlightJson(value: string) {
  const tokenPattern =
    /("(?:\\u[\da-fA-F]{4}|\\[^u]|[^"\\])*"(?=\s*:)|"(?:\\u[\da-fA-F]{4}|\\[^u]|[^"\\])*"|\btrue\b|\bfalse\b|\bnull\b|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g;
  const parts: ReactNode[] = [];
  let lastIndex = 0;

  for (const match of value.matchAll(tokenPattern)) {
    const token = match[0];
    const index = match.index ?? 0;
    if (index > lastIndex) {
      parts.push(value.slice(lastIndex, index));
    }

    const className = jsonTokenClass(token, value.slice(index + token.length));
    parts.push(
      <span className={className} key={`${index}-${token}`}>
        {token}
      </span>,
    );
    lastIndex = index + token.length;
  }

  if (lastIndex < value.length) {
    parts.push(value.slice(lastIndex));
  }

  return parts;
}

function jsonTokenClass(token: string, rest: string) {
  if (token.startsWith('"')) {
    return rest.trimStart().startsWith(":") ? "json-key" : "json-string";
  }
  if (token === "true" || token === "false") return "json-boolean";
  if (token === "null") return "json-null";
  return "json-number";
}

// Kept temporarily as hidden advanced surface while the primary navigation is simplified.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function WorkflowPanel({
  commands,
  report,
  counts,
  onRunCommand,
  runningTerminal,
  skills,
  stages,
  workflowState,
}: {
  commands: DwCommand[];
  report: PreflightReport | null;
  counts: ReturnType<typeof stageCounts>;
  onRunCommand: (command: string) => void;
  runningTerminal: boolean;
  skills: DwSkill[];
  stages: typeof workflowStages;
  workflowState: WorkflowStateSummary | null;
}) {
  const [planMode, setPlanMode] = useState<DwPlanMode>("default");
  const [planInput, setPlanInput] = useState("");
  const [selectedSkills, setSelectedSkills] = useState<string[]>([]);
  const planCommand = commands.find((command) => command.command === "/dw-plan");
  const selectedSkillModels = skills.filter((skill) => selectedSkills.includes(skill.name));
  const composedCommand = composeDwPlanCommand(planMode, planInput, selectedSkillModels);
  const visibleSkills = skills.slice(0, 12);

  function toggleSkill(name: string) {
    setSelectedSkills((current) =>
      current.includes(name) ? current.filter((item) => item !== name) : [...current, name],
    );
  }

  return (
    <div className="panel-stack">
      <div className="surface-heading">
        <div>
          <h1>Workflow runner</h1>
          <p>
            Run dev-workflow stages through the active agent and inspect gates as artifacts change.
          </p>
        </div>
        <div className="count-strip" aria-label="Workflow stage counts">
          <span>{counts.ready} ready</span>
          <span>{counts.active + counts.pending} active</span>
          <span>{counts.blocked} blocked</span>
        </div>
      </div>

      <div className="pipeline">
        {stages.map((stage) => (
          <article className={`stage ${stage.state}`} key={stage.id}>
            <div className="stage-title">
              <CheckCircle2 aria-hidden="true" size={18} />
              <strong>{stage.label}</strong>
              <code>{stage.command}</code>
            </div>
            <span className="stage-state">{workflowStateLabel(stage.state)}</span>
            <p>{stage.description}</p>
          </article>
        ))}
      </div>

      <div className="runner-layout">
        <section className="runner-command-panel" aria-label="Run dev-workflow command">
          <div className="runner-panel-heading">
            <div>
              <h2>/dw-plan</h2>
              <p>{planCommand?.description ?? "Create or continue PRD, TechSpec, and Tasks."}</p>
            </div>
            <span className={runningTerminal ? "status-pill ready" : "status-pill blocked"}>
              {runningTerminal ? "agent ready" : "agent required"}
            </span>
          </div>

          <div className="runner-controls">
            <label>
              <span>Mode</span>
              <select
                value={planMode}
                onChange={(event) => setPlanMode(event.target.value as DwPlanMode)}
              >
                <option value="default">default</option>
                <option value="prd">prd</option>
                <option value="techspec">techspec</option>
                <option value="tasks">tasks</option>
              </select>
            </label>
            <label>
              <span>Idea or slug</span>
              <input
                value={planInput}
                onChange={(event) => setPlanInput(event.target.value)}
                placeholder="prd-dev-workflow-runner"
                spellCheck={false}
              />
            </label>
          </div>

          <section className="skill-picker" aria-label="Skill context">
            <div className="section-label">Skill context</div>
            <div className="skill-list">
              {visibleSkills.length ? (
                visibleSkills.map((skill) => (
                  <label className="skill-choice" key={skill.name}>
                    <input
                      type="checkbox"
                      checked={selectedSkills.includes(skill.name)}
                      onChange={() => toggleSkill(skill.name)}
                    />
                    <span>{skill.name}</span>
                    <small>{skill.source}</small>
                  </label>
                ))
              ) : (
                <div className="empty-note">No skills discovered for this project.</div>
              )}
            </div>
          </section>

          <div className="command-preview">
            <div className="section-label">Command preview</div>
            <pre className="terminal-output">{composedCommand}</pre>
          </div>

          <button
            className="primary-button runner-send"
            type="button"
            onClick={() => onRunCommand(composedCommand)}
            disabled={!runningTerminal || !composedCommand}
          >
            <Send aria-hidden="true" size={17} />
            Delegar ao agente
          </button>
        </section>

        <section className="runner-state-panel" aria-label="Workflow gates and resume entries">
          <div className="runner-panel-heading">
            <div>
              <h2>Gates and resume</h2>
              <p>Derived from `.dw` artifacts in the active project.</p>
            </div>
            <Play aria-hidden="true" size={20} />
          </div>

          <div className="gate-list">
            {workflowState?.gates.length ? (
              workflowState.gates.map((gate) => (
                <article className="gate-row" key={`${gate.label}-${gate.path ?? gate.state}`}>
                  <strong>{gate.label}</strong>
                  <span>{gate.state}</span>
                  <small>{gate.detail}</small>
                </article>
              ))
            ) : (
              <div className="empty-note">No workflow gates detected.</div>
            )}
          </div>

          <div className="resume-list">
            <div className="section-label">Resume entries</div>
            {workflowState?.resume_entries.length ? (
              workflowState.resume_entries.slice(0, 8).map((entry) => (
                <button
                  className="resume-row"
                  key={`${entry.kind}-${entry.path}`}
                  type="button"
                  onClick={() => onRunCommand(entry.command)}
                  disabled={!runningTerminal}
                >
                  <strong>{entry.label}</strong>
                  <code>{entry.command}</code>
                  <small>{entry.status}</small>
                </button>
              ))
            ) : (
              <div className="empty-note">Refresh after creating `.dw` artifacts.</div>
            )}
          </div>
        </section>
      </div>

      <section className="evidence-band">
        <h2>Project readiness</h2>
        <CodeMirror
          value={JSON.stringify(report ?? {}, null, 2)}
          height="220px"
          extensions={[json()]}
          editable={false}
          basicSetup={{ lineNumbers: false, foldGutter: false }}
        />
      </section>
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
function EvidencePanel({
  busy,
  counts,
  entries,
  onCompleteRun,
  onCreateManual,
  onOpenArtifact,
  onRefresh,
}: {
  busy: boolean;
  counts: ReturnType<typeof evidenceCounts>;
  entries: EvidenceEntry[];
  onCompleteRun: (runId: number, status: string, summary: string) => void;
  onCreateManual: (input: {
    title: string;
    status: string;
    summary: string;
    relative_paths: string[];
  }) => void;
  onOpenArtifact: (relativePath: string) => void;
  onRefresh: () => void;
}) {
  const [manualTitle, setManualTitle] = useState("");
  const [manualStatus, setManualStatus] = useState("unknown");
  const [manualSummary, setManualSummary] = useState("");
  const [manualLinks, setManualLinks] = useState("");
  const [completionRunId, setCompletionRunId] = useState<number | null>(null);
  const [completionStatus, setCompletionStatus] = useState("passed");
  const [completionSummary, setCompletionSummary] = useState("");

  function submitManual(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onCreateManual({
      title: manualTitle,
      status: manualStatus,
      summary: manualSummary,
      relative_paths: parseEvidenceLinks(manualLinks),
    });
    setManualTitle("");
    setManualSummary("");
    setManualLinks("");
  }

  function submitCompletion(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (completionRunId == null) return;
    onCompleteRun(completionRunId, completionStatus, completionSummary);
    setCompletionRunId(null);
    setCompletionSummary("");
    setCompletionStatus("passed");
  }

  return (
    <div className="panel-stack">
      <div className="surface-heading">
        <div>
          <h1>Evidence</h1>
          <p>Audit trail for workflow runs, QA outputs, review notes, scripts, and logs.</p>
        </div>
        <div className="count-strip compact" aria-label="Evidence counts">
          <span>{counts.runs} runs</span>
          <span>{counts.items} artifacts</span>
          <span>{counts.submitted} submitted</span>
        </div>
      </div>

      <div className="evidence-layout">
        <section className="evidence-list" aria-label="Evidence records">
          <div className="evidence-toolbar">
            <button className="secondary-button" type="button" onClick={onRefresh} disabled={busy}>
              <RefreshCw aria-hidden="true" size={16} />
              Refresh
            </button>
            {counts.stale ? (
              <span className="status-pill blocked">{counts.stale} stale</span>
            ) : null}
          </div>

          {entries.length ? (
            entries.map((entry) => (
              <article className="evidence-row" key={entry.id}>
                <div className="evidence-row-main">
                  <div>
                    <strong>{entry.title}</strong>
                    <span>
                      {evidenceKindLabel(entry.kind)} · {evidenceStatusLabel(entry.status)}
                    </span>
                  </div>
                  <span className={`evidence-status ${entry.status}`}>
                    {entry.stale ? "Stale" : evidenceStatusLabel(entry.status)}
                  </span>
                </div>
                <p>{evidenceSummary(entry)}</p>
                <div className="evidence-meta">
                  <span>{entry.created_at}</span>
                  {entry.prd_slug ? <span>{entry.prd_slug}</span> : null}
                  {entry.terminal_log_path ? <span>{entry.terminal_log_path}</span> : null}
                </div>
                <div className="evidence-actions">
                  {entry.relative_path ? (
                    <button
                      className="secondary-button inline-action"
                      type="button"
                      onClick={() => onOpenArtifact(entry.relative_path ?? "")}
                      disabled={entry.stale}
                    >
                      <FileText aria-hidden="true" size={16} />
                      Open artifact
                    </button>
                  ) : null}
                  {entry.record_type === "run" &&
                  entry.run_id != null &&
                  entry.status === "submitted" ? (
                    <button
                      className="secondary-button inline-action"
                      type="button"
                      onClick={() => {
                        setCompletionRunId(entry.run_id ?? null);
                        setCompletionSummary(entry.summary);
                      }}
                    >
                      <CheckCircle2 aria-hidden="true" size={16} />
                      Complete
                    </button>
                  ) : null}
                </div>
              </article>
            ))
          ) : (
            <div className="empty-note">No evidence recorded for this project yet.</div>
          )}
        </section>

        <aside className="evidence-capture" aria-label="Evidence capture">
          <form className="evidence-form" onSubmit={submitManual}>
            <div className="runner-panel-heading">
              <div>
                <h2>Manual evidence</h2>
                <p>Save a note with optional links to existing `.dw` artifacts.</p>
              </div>
            </div>
            <label>
              <span>Title</span>
              <input
                value={manualTitle}
                onChange={(event) => setManualTitle(event.target.value)}
                placeholder="QA walkthrough"
              />
            </label>
            <label>
              <span>Status</span>
              <select
                value={manualStatus}
                onChange={(event) => setManualStatus(event.target.value)}
              >
                <option value="unknown">unknown</option>
                <option value="passed">passed</option>
                <option value="failed">failed</option>
              </select>
            </label>
            <label>
              <span>Summary</span>
              <textarea
                value={manualSummary}
                onChange={(event) => setManualSummary(event.target.value)}
                placeholder="What this evidence proves."
              />
            </label>
            <label>
              <span>.dw links</span>
              <textarea
                value={manualLinks}
                onChange={(event) => setManualLinks(event.target.value)}
                placeholder="spec/prd-demo/QA/qa-report.md"
                spellCheck={false}
              />
            </label>
            <button className="primary-button" type="submit" disabled={busy || !manualTitle.trim()}>
              <Plus aria-hidden="true" size={17} />
              Save evidence
            </button>
          </form>

          {completionRunId != null ? (
            <form className="evidence-form" onSubmit={submitCompletion}>
              <div className="runner-panel-heading">
                <div>
                  <h2>Complete run</h2>
                  <p>Mark the submitted runner command after it finishes in the terminal.</p>
                </div>
              </div>
              <label>
                <span>Status</span>
                <select
                  value={completionStatus}
                  onChange={(event) => setCompletionStatus(event.target.value)}
                >
                  <option value="passed">passed</option>
                  <option value="failed">failed</option>
                  <option value="unknown">unknown</option>
                </select>
              </label>
              <label>
                <span>Summary</span>
                <textarea
                  value={completionSummary}
                  onChange={(event) => setCompletionSummary(event.target.value)}
                  placeholder="Result, verification command, or failure context."
                />
              </label>
              <button
                className="primary-button"
                type="submit"
                disabled={busy || !completionSummary.trim()}
              >
                <CheckCircle2 aria-hidden="true" size={17} />
                Save result
              </button>
            </form>
          ) : null}
        </aside>
      </div>
    </div>
  );
}

function SourcePanel({
  blame,
  history,
  showHistory,
  compareBase,
  onToggleHistory,
  onSelectHistory,
  onPickCompare,
  onContentChange,
  onSave,
  onQuickOpen,
  onSelect,
  openFiles,
  expandedPaths,
  onExpandedPathsChange,
  onSelectTab,
  onCloseTab,
  selectedFile,
  sourceBusy,
  sourceSaving,
  sourceContent,
  sourceDirty,
  sourceTree,
  editorFontSize,
  revealLine,
  sideTab,
  onSideTabChange,
  searchFocusSeed,
  searchPath,
  onOpenResult,
  onFindInFiles,
  previewActive,
  onTogglePreview,
  onFormat,
  onRefreshTree,
  onCreateFile,
  onCreateDir,
  editorPath,
  lsp,
  onEnableLsp,
  onInstallLsp,
  explorerWidth,
  onExplorerResize,
}: {
  blame: BlameLine[];
  history: Commit[];
  showHistory: boolean;
  compareBase: string | null;
  onToggleHistory: () => void;
  onSelectHistory: (sha: string) => void;
  onPickCompare: (id: string) => void;
  onContentChange: (content: string) => void;
  onSave: () => void;
  onQuickOpen: () => void;
  onSelect: (entry: SourceEntry) => void;
  openFiles: SourceFile[];
  expandedPaths: string[];
  onExpandedPathsChange: (paths: string[]) => void;
  onSelectTab: (relativePath: string) => void;
  onCloseTab: (relativePath: string) => void;
  selectedFile: SourceFile | null;
  sourceBusy: boolean;
  sourceSaving: boolean;
  sourceContent: string;
  sourceDirty: boolean;
  sourceTree: SourceEntry[];
  editorFontSize: number;
  revealLine: number | null;
  sideTab: "explorer" | "search";
  onSideTabChange: (tab: "explorer" | "search") => void;
  searchFocusSeed: number;
  searchPath: string;
  onOpenResult: (relativePath: string, line: number) => void;
  onFindInFiles: () => void;
  previewActive: boolean;
  onTogglePreview: () => void;
  onFormat: (text: string, language: string) => Promise<string | null>;
  onRefreshTree: () => void;
  onCreateFile: (relativePath: string) => void;
  onCreateDir: (relativePath: string) => void;
  editorPath: string | undefined;
  lsp: {
    enabled: boolean;
    installed: boolean;
    canInstall: boolean;
    installing: boolean;
    program: string;
  } | null;
  onEnableLsp: () => void;
  onInstallLsp: () => void;
  explorerWidth: number;
  onExplorerResize: (width: number) => void;
}) {
  const language = selectedFile ? monacoLanguage(selectedFile.relative_path) : "plaintext";
  const isMarkdown = language === "markdown";
  const showPreview = previewActive && isMarkdown && Boolean(selectedFile);
  const expanded = useMemo(() => new Set(expandedPaths), [expandedPaths]);
  const [cursor, setCursor] = useState<{ line: number; col: number }>({ line: 1, col: 1 });
  const { prompt, dialog: promptDialog } = usePrompt();
  const layoutRef = useRef<HTMLDivElement>(null);

  // Drag the explorer/editor splitter. To avoid re-rendering the heavy panel on
  // every mousemove we mutate the CSS var directly and only commit (persist) on
  // mouseup; React reconciles the var from `explorerWidth` on the next render.
  function startExplorerResize(event: React.MouseEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = explorerWidth;
    let latest = startWidth;
    const onMove = (move: MouseEvent) => {
      latest = Math.max(200, Math.min(560, startWidth + (move.clientX - startX)));
      layoutRef.current?.style.setProperty("--explorer-w", `${latest}px`);
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      onExplorerResize(latest);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
  }

  const sourcePathLabel = selectedFile?.relative_path
    ? `${selectedFile.relative_path.split("/").join(" / ")}`
    : "Nenhum arquivo";

  function toggleDir(path: string) {
    const next = new Set(expanded);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    onExpandedPathsChange([...next]);
  }

  async function promptNewFile() {
    const relativePath = await prompt({
      title: "Novo arquivo",
      label: "Caminho relativo (ex.: src/novo.ts)",
      confirmLabel: "Criar",
    });
    if (relativePath) onCreateFile(relativePath);
  }

  async function promptNewDir() {
    const relativePath = await prompt({
      title: "Nova pasta",
      label: "Caminho relativo (ex.: src/lib)",
      confirmLabel: "Criar",
    });
    if (relativePath) onCreateDir(relativePath);
  }

  return (
    <div className="source-panel">
      <header className="source-topbar screen-topbar">
        <span className="topbar-title">Source Code</span>
        <span className="topbar-path">
          {selectedFile ? sourcePathLabel : "Selecione um arquivo no explorer"}
        </span>
        <div className="topbar-actions">
          <button className="topbar-btn" type="button" onClick={onQuickOpen}>
            <Search aria-hidden="true" size={14} />
            Buscar
          </button>
          <button
            className={previewActive ? "topbar-btn active" : "topbar-btn"}
            type="button"
            onClick={onTogglePreview}
            disabled={!selectedFile || !isMarkdown}
            aria-pressed={previewActive}
          >
            <Eye aria-hidden="true" size={14} />
            Preview
          </button>
        </div>
      </header>
      <div
        className="source-layout"
        ref={layoutRef}
        style={{ "--explorer-w": `${explorerWidth}px` } as React.CSSProperties}
      >
        <aside className="explorer source-list" aria-label="Source files">
          <div className="explorer-header source-side-tabs" role="tablist">
            <span className="explorer-title">Explorer</span>
            {sideTab === "explorer" ? (
              <div className="explorer-actions">
                <button
                  className="explorer-btn explorer-action-btn"
                  type="button"
                  onClick={() => void promptNewFile()}
                  title="Novo arquivo"
                  aria-label="Novo arquivo"
                >
                  <FilePlus aria-hidden="true" size={15} />
                </button>
                <button
                  className="explorer-btn explorer-action-btn"
                  type="button"
                  onClick={() => void promptNewDir()}
                  title="Nova pasta"
                  aria-label="Nova pasta"
                >
                  <FolderPlus aria-hidden="true" size={15} />
                </button>
                <button
                  className="explorer-btn explorer-action-btn"
                  type="button"
                  onClick={onRefreshTree}
                  title="Atualizar"
                  aria-label="Atualizar"
                >
                  <RefreshCw aria-hidden="true" size={15} />
                </button>
                <button
                  className="explorer-btn explorer-action-btn"
                  type="button"
                  onClick={() => onExpandedPathsChange([])}
                  title="Recolher tudo"
                  aria-label="Recolher tudo"
                >
                  <ChevronsDownUp aria-hidden="true" size={15} />
                </button>
              </div>
            ) : null}
          </div>
          <div className="explorer-search">
            <input
              type="search"
              placeholder="Buscar arquivos..."
              onFocus={() => {
                onSideTabChange("search");
                onFindInFiles();
              }}
            />
          </div>
          <div className="source-side-tab-row" role="tablist" aria-label="Source side tabs">
            <button
              type="button"
              role="tab"
              aria-selected={sideTab === "explorer"}
              className={sideTab === "explorer" ? "source-side-tab active" : "source-side-tab"}
              onClick={() => onSideTabChange("explorer")}
            >
              Explorer
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={sideTab === "search"}
              className={sideTab === "search" ? "source-side-tab active" : "source-side-tab"}
              onClick={() => onSideTabChange("search")}
              title="Buscar nos arquivos (Ctrl+Shift+F)"
            >
              Search
            </button>
          </div>
          {sideTab === "search" ? (
            <SearchPanel
              path={searchPath}
              focusSeed={searchFocusSeed}
              onOpenResult={onOpenResult}
            />
          ) : (
            <>
              {sourceTree.length ? (
                <SourceEntryList
                  entries={sourceTree}
                  expanded={expanded}
                  onSelect={onSelect}
                  onToggle={toggleDir}
                  selectedPath={selectedFile?.relative_path}
                />
              ) : (
                <div className="empty-state empty-state-compact">
                  <span className="empty-state-icon">
                    <FolderGit2 aria-hidden="true" size={20} />
                  </span>
                  <p className="empty-state-desc">
                    Nenhum projeto carregado. Selecione ou adicione um projeto para ver os arquivos.
                  </p>
                </div>
              )}
            </>
          )}
          {promptDialog}
        </aside>

        <div
          className="source-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label="Redimensionar explorer"
          onMouseDown={startExplorerResize}
        />

        <section className="editor-area source-viewer">
          <EditorTabs
            openFiles={openFiles}
            activePath={selectedFile?.relative_path ?? null}
            dirty={sourceDirty}
            previewActive={previewActive}
            onSelect={onSelectTab}
            onClose={onCloseTab}
            onTogglePreview={() => onTogglePreview()}
          />

          <div className="source-editor-row">
            <div className="source-editor-host">
              {selectedFile ? (
                showPreview ? (
                  <Suspense fallback={<PanelLoading label="Preview" />}>
                    <MarkdownPreview text={sourceContent} />
                  </Suspense>
                ) : (
                  <Suspense fallback={<PanelLoading label="Editor" />}>
                    <MonacoSource
                      value={sourceBusy ? "" : sourceContent}
                      language={language}
                      onChange={onContentChange}
                      onSave={() => onSave()}
                      onClose={() => onCloseTab(selectedFile.relative_path)}
                      onQuickOpen={onQuickOpen}
                      onFindInFiles={onFindInFiles}
                      onFormat={onFormat}
                      onCursorChange={(line, col) => setCursor({ line, col })}
                      blame={blame}
                      fontSize={editorFontSize}
                      revealLine={revealLine}
                      path={editorPath}
                    />
                  </Suspense>
                )
              ) : (
                <div className="empty-state">
                  <span className="empty-state-icon">
                    <Code2 aria-hidden="true" size={26} />
                  </span>
                  <h3 className="empty-state-title">Nenhum arquivo aberto</h3>
                  <p className="empty-state-desc">
                    Selecione um arquivo no explorer para ver e editar o código.
                  </p>
                </div>
              )}
            </div>
            {showHistory && selectedFile ? (
              <FileHistoryPanel
                commits={history}
                filePath={selectedFile.relative_path}
                compareBase={compareBase}
                onSelect={onSelectHistory}
                onPickCompare={onPickCompare}
                onClose={onToggleHistory}
              />
            ) : null}
          </div>

          <footer className="source-footer status-bar">
            <div className="source-footer-info">
              <span className="source-footer-branch" title="Branch atual">
                <GitBranch aria-hidden="true" size={13} />
                branch
              </span>
              <span className="source-footer-sep">·</span>
              {selectedFile ? (
                <>
                  <span className="source-footer-path" title={selectedFile.relative_path}>
                    {selectedFile.relative_path}
                  </span>
                  <span className="source-footer-sep">·</span>
                  <span>{sourceLanguage(selectedFile.relative_path)}</span>
                  <span className="source-footer-sep">·</span>
                  <span>{formatSourceSize(selectedFile.bytes)}</span>
                </>
              ) : (
                <span>Nenhum arquivo selecionado</span>
              )}
            </div>
            <div className="source-footer-actions">
              {selectedFile ? (
                <span className="source-footer-cursor">
                  Ln {cursor.line}, Col {cursor.col}
                </span>
              ) : null}
              <span>UTF-8</span>
              <span>Spaces: 2</span>
              <span>LF</span>
              {selectedFile && sourceDirty ? (
                <span className="source-footer-dirty" title="Alterações não salvas">
                  ● não salvo
                </span>
              ) : null}
              {lsp ? (
                lsp.installing ? (
                  <span className="source-footer-lsp" title="Instalando language server">
                    Instalando {lsp.program}…
                  </span>
                ) : !lsp.enabled ? (
                  <button className="source-footer-link" type="button" onClick={onEnableLsp}>
                    Ativar LSP
                  </button>
                ) : !lsp.installed ? (
                  <button
                    className="source-footer-link"
                    type="button"
                    onClick={onInstallLsp}
                    disabled={!lsp.canInstall}
                    title={
                      lsp.canInstall
                        ? `Instalar ${lsp.program}`
                        : `Instale o pré-requisito de ${lsp.program} primeiro`
                    }
                  >
                    Instalar {lsp.program}
                  </button>
                ) : (
                  <span className="source-footer-lsp ok" title={`${lsp.program} ativo`}>
                    LSP ✓
                  </span>
                )
              ) : null}
              <button
                className={showHistory ? "source-footer-btn active" : "source-footer-btn"}
                type="button"
                onClick={onToggleHistory}
                disabled={!selectedFile}
                title="Histórico do arquivo"
                aria-label="Histórico do arquivo"
                aria-pressed={showHistory}
              >
                <History aria-hidden="true" size={14} />
              </button>
              <button
                className="source-footer-btn"
                type="button"
                onClick={onSave}
                disabled={!selectedFile || !sourceDirty || sourceSaving}
                title="Salvar (Ctrl+S)"
                aria-label="Salvar (Ctrl+S)"
              >
                <CheckCircle2 aria-hidden="true" size={14} />
              </button>
            </div>
          </footer>
        </section>
      </div>
    </div>
  );
}

function SourceEntryList({
  entries,
  expanded,
  onSelect,
  onToggle,
  selectedPath,
  depth = 0,
}: {
  entries: SourceEntry[];
  expanded: Set<string>;
  onSelect: (entry: SourceEntry) => void;
  onToggle: (path: string) => void;
  selectedPath?: string;
  depth?: number;
}) {
  // Directories first, then files; each group alphabetical (explorer convention).
  const sorted = [...entries].sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <div className="source-entry-list">
      {sorted.map((entry) => {
        const isDir = entry.kind === "directory";
        const isOpen = isDir && expanded.has(entry.relative_path);
        return (
          <div key={entry.relative_path}>
            <button
              className={[
                "tree-item",
                "source-file-row",
                isDir ? "is-dir" : "",
                selectedPath === entry.relative_path ? "active" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => (isDir ? onToggle(entry.relative_path) : onSelect(entry))}
              style={
                {
                  paddingLeft: `${10 + depth * 14}px`,
                  "--depth": depth,
                } as CSSProperties
              }
              type="button"
              aria-expanded={isDir ? isOpen : undefined}
            >
              {isDir ? (
                isOpen ? (
                  <ChevronDown aria-hidden="true" size={14} className="source-chevron" />
                ) : (
                  <ChevronRight aria-hidden="true" size={14} className="source-chevron" />
                )
              ) : (
                <span className="source-chevron-spacer" />
              )}
              {isDir ? (
                isOpen ? (
                  <FolderOpen aria-hidden="true" size={15} className="tree-icon folder" />
                ) : (
                  <Folder aria-hidden="true" size={15} className="tree-icon folder" />
                )
              ) : (
                (() => {
                  const { Icon, color } = fileIcon(entry.name);
                  return (
                    <Icon
                      aria-hidden="true"
                      className={`tree-icon ${entry.extension ?? ""}`}
                      size={15}
                      style={{ color }}
                    />
                  );
                })()
              )}
              <span className="tree-name">{entry.name}</span>
              {entry.kind === "file" && entry.bytes != null ? (
                <small>{formatSourceSize(entry.bytes)}</small>
              ) : null}
            </button>
            {isDir && isOpen && entry.children.length ? (
              <SourceEntryList
                depth={depth + 1}
                entries={entry.children}
                expanded={expanded}
                onSelect={onSelect}
                onToggle={onToggle}
                selectedPath={selectedPath}
              />
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

const GRAPH_LANE_W = 14;
const GRAPH_ROW_H = 24;

function authorInitials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

/** Gravatar avatar with an initials fallback when the image is missing/offline. */
function Avatar({ name, email, size = 18 }: { name: string; email: string; size?: number }) {
  const [failed, setFailed] = useState(false);
  if (failed || !email.trim()) {
    return (
      <span className="git-avatar" aria-hidden="true">
        {authorInitials(name)}
      </span>
    );
  }
  return (
    <img
      className="git-avatar git-avatar-img"
      src={gravatarUrl(email, size * 2)}
      alt=""
      width={size}
      height={size}
      onError={() => setFailed(true)}
    />
  );
}
const GRAPH_COLORS = [
  "#6ea8fe",
  "#5fd0a8",
  "#b99cff",
  "#f2c879",
  "#f29db4",
  "#7fd1e8",
  "#9ad27f",
  "#e88f8f",
];

function refBadgeClass(kind: string) {
  if (kind === "head") return "git-ref head";
  if (kind === "tag") return "git-ref tag";
  if (kind === "stash") return "git-ref stash";
  if (kind === "remote") return "git-ref remote";
  return "git-ref branch";
}

function CommitGraph({
  commits,
  selectedSha,
  onSelect,
  onContextMenu,
}: {
  commits: Commit[];
  selectedSha: string | null;
  onSelect: (sha: string) => void;
  onContextMenu?: (commit: Commit, event: ReactMouseEvent) => void;
}) {
  const layout = useMemo(
    () => computeGraph(commits.map((c) => ({ sha: c.sha, parents: c.parents }))),
    [commits],
  );
  const rowIndex = useMemo(() => {
    const map = new Map<string, number>();
    commits.forEach((c, i) => map.set(c.sha, i));
    return map;
  }, [commits]);

  const gutter = layout.columns * GRAPH_LANE_W + GRAPH_LANE_W;
  const height = commits.length * GRAPH_ROW_H;
  const cx = (col: number) => col * GRAPH_LANE_W + GRAPH_LANE_W;
  const cy = (row: number) => row * GRAPH_ROW_H + GRAPH_ROW_H / 2;
  const colorOf = (n: number) => GRAPH_COLORS[n % GRAPH_COLORS.length];

  return (
    <div className="commit-graph" style={{ gridTemplateColumns: `${gutter}px minmax(0, 1fr)` }}>
      <svg className="commit-graph-svg" width={gutter} height={height} aria-hidden="true">
        {commits.map((commit, i) => {
          const node = layout.nodes.get(commit.sha);
          if (!node) return null;
          return commit.parents.map((parent) => {
            const pRow = rowIndex.get(parent);
            const pNode = layout.nodes.get(parent);
            if (pRow === undefined || !pNode) return null;
            const x1 = cx(node.column);
            const y1 = cy(i);
            const x2 = cx(pNode.column);
            const y2 = cy(pRow);
            const mid = (y1 + y2) / 2;
            const d =
              x1 === x2
                ? `M ${x1} ${y1} L ${x2} ${y2}`
                : `M ${x1} ${y1} C ${x1} ${mid}, ${x2} ${mid}, ${x2} ${y2}`;
            return (
              <path
                key={`${commit.sha}-${parent}`}
                d={d}
                fill="none"
                stroke={colorOf(node.color)}
                strokeWidth={1.5}
              />
            );
          });
        })}
        {commits.map((commit, i) => {
          const node = layout.nodes.get(commit.sha);
          if (!node) return null;
          return (
            <circle
              key={commit.sha}
              cx={cx(node.column)}
              cy={cy(i)}
              r={selectedSha === commit.sha ? 5.5 : 4}
              fill={colorOf(node.color)}
              stroke={selectedSha === commit.sha ? "#f5f7fb" : "transparent"}
              strokeWidth={1.5}
            />
          );
        })}
      </svg>
      <div className="commit-rows">
        {commits.map((commit) => (
          <button
            key={commit.sha}
            type="button"
            className={selectedSha === commit.sha ? "commit-row active" : "commit-row"}
            style={{ height: GRAPH_ROW_H }}
            onClick={() => onSelect(commit.sha)}
            onContextMenu={onContextMenu ? (event) => onContextMenu(commit, event) : undefined}
          >
            <span className="commit-subject">
              {commit.refs.map((ref) => (
                <span className={refBadgeClass(ref.kind)} key={`${ref.kind}-${ref.name}`}>
                  {ref.name}
                </span>
              ))}
              {commit.subject}
            </span>
            <span className="commit-author">
              <Avatar name={commit.author_name} email={commit.author_email} />
              {commit.author_name}
            </span>
            <span className="commit-date">{commit.date.slice(0, 10)}</span>
            <span className="commit-sha">{commit.short_sha}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

type RefLeaf = { name: string; full: string; isHead?: boolean; ahead?: number; behind?: number };
type RefTreeNode = { segment: string; path: string; leaf?: RefLeaf; children: RefTreeNode[] };

function buildRefTree(leaves: RefLeaf[]): RefTreeNode[] {
  const roots: RefTreeNode[] = [];
  for (const leaf of leaves) {
    const parts = leaf.name.split("/");
    let level = roots;
    let acc = "";
    parts.forEach((seg, index) => {
      acc = acc ? `${acc}/${seg}` : seg;
      if (index === parts.length - 1) {
        level.push({ segment: seg, path: acc, leaf, children: [] });
      } else {
        let folder = level.find((node) => !node.leaf && node.segment === seg);
        if (!folder) {
          folder = { segment: seg, path: acc, children: [] };
          level.push(folder);
        }
        level = folder.children;
      }
    });
  }
  return roots;
}

function GitRefTree({
  nodes,
  depth = 0,
  leafIcon,
  onCheckout,
  onMerge,
  onContext,
}: {
  nodes: RefTreeNode[];
  depth?: number;
  leafIcon: ReactNode;
  onCheckout: (full: string) => void;
  onMerge?: (full: string) => void;
  onContext?: (leaf: RefLeaf, event: ReactMouseEvent) => void;
}) {
  return (
    <>
      {nodes.map((node) =>
        node.leaf ? (
          <div className={node.leaf.isHead ? "git-ref-row head" : "git-ref-row"} key={node.path}>
            <button
              type="button"
              title="Duplo clique para checkout · botão direito para mais ações"
              style={{ paddingLeft: 8 + depth * 12 }}
              onDoubleClick={() => onCheckout(node.leaf!.full)}
              onContextMenu={onContext ? (event) => onContext(node.leaf!, event) : undefined}
            >
              {node.leaf.isHead ? <CheckCircle2 aria-hidden="true" size={13} /> : leafIcon}
              <span className="git-ref-name">{node.segment}</span>
              {node.leaf.ahead || node.leaf.behind ? (
                <small>
                  ↑{node.leaf.ahead ?? 0} ↓{node.leaf.behind ?? 0}
                </small>
              ) : null}
            </button>
            {onMerge && !node.leaf.isHead ? (
              <button
                className="git-ref-action"
                type="button"
                title="Merge na branch atual"
                onClick={() => onMerge(node.leaf!.full)}
              >
                merge
              </button>
            ) : null}
          </div>
        ) : (
          <div key={node.path}>
            <div className="git-ref-folder" style={{ paddingLeft: 8 + depth * 12 }}>
              <FolderOpen aria-hidden="true" size={12} />
              <span>{node.segment}</span>
            </div>
            <GitRefTree
              nodes={node.children}
              depth={depth + 1}
              leafIcon={leafIcon}
              onCheckout={onCheckout}
              onMerge={onMerge}
              onContext={onContext}
            />
          </div>
        ),
      )}
    </>
  );
}

type GitView = "local" | "commits";

/** Create a branch: name + which branch to start from + optional checkout. */
function NewBranchModal({
  initialSource,
  branches,
  onClose,
  onCreate,
}: {
  initialSource: string;
  branches: Branch[];
  onClose: () => void;
  onCreate: (name: string, source: string, checkout: boolean) => void;
}) {
  const [name, setName] = useState("");
  const [source, setSource] = useState(initialSource);
  const [checkout, setCheckout] = useState(true);
  const canCreate = name.trim().length > 0;

  return (
    <div className="modal-backdrop elevated" role="presentation">
      <section className="modal-panel confirm-modal" role="dialog" aria-modal="true">
        <div className="modal-heading">
          <div>
            <div className="section-label">Git</div>
            <h2>Criar branch</h2>
          </div>
        </div>
        <label className="prompt-field">
          <span>Nome da nova branch</span>
          <input
            autoFocus
            value={name}
            placeholder="feat/minha-branch"
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && canCreate) onCreate(name.trim(), source, checkout);
              if (event.key === "Escape") onClose();
            }}
          />
        </label>
        <label className="prompt-field">
          <span>A partir de</span>
          <select value={source} onChange={(event) => setSource(event.target.value)}>
            {!branches.some((branch) => branch.name === source) ? (
              <option value={source}>{source || "HEAD atual"}</option>
            ) : null}
            {branches.map((branch) => (
              <option key={branch.name} value={branch.name}>
                {branch.name}
                {branch.is_head ? " (atual)" : ""}
              </option>
            ))}
          </select>
        </label>
        <label className="git-toggle new-branch-checkout">
          <input
            type="checkbox"
            checked={checkout}
            onChange={(event) => setCheckout(event.target.checked)}
          />
          Fazer checkout após criar
        </label>
        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={onClose}>
            Cancelar
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={!canCreate}
            onClick={() => onCreate(name.trim(), source, checkout)}
          >
            Criar branch
          </button>
        </div>
      </section>
    </div>
  );
}

const REBASE_ACTIONS: RebaseAction[] = ["pick", "reword", "edit", "squash", "fixup", "drop"];

/** Interactive-rebase editor: per-commit action + drag to reorder. */
function RebaseDialog({
  base,
  commits,
  onClose,
  onSubmit,
}: {
  base: string;
  commits: Commit[];
  onClose: () => void;
  onSubmit: (steps: RebaseStep[]) => void;
}) {
  const [rows, setRows] = useState<Array<{ sha: string; subject: string; action: RebaseAction }>>(
    commits.map((commit) => ({ sha: commit.sha, subject: commit.subject, action: "pick" })),
  );
  const dragIndex = useRef<number | null>(null);

  function move(from: number, to: number) {
    if (from === to) return;
    setRows((current) => {
      const next = current.slice();
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return next;
    });
  }

  return (
    <div className="modal-backdrop elevated" role="presentation">
      <section className="modal-panel rebase-dialog" role="dialog" aria-modal="true">
        <div className="modal-heading">
          <div>
            <div className="section-label">Git</div>
            <h2>Rebase interativo</h2>
            <p>
              Replay dos commits sobre <code>{base.slice(0, 8)}</code>. Arraste para reordenar;
              escolha a ação de cada commit. (Mais antigo no topo.)
            </p>
          </div>
          <button
            className="secondary-button icon-button"
            type="button"
            onClick={onClose}
            aria-label="Fechar"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>

        <div className="rebase-rows">
          {rows.map((row, index) => (
            <div
              className="rebase-row"
              key={row.sha}
              draggable
              onDragStart={() => (dragIndex.current = index)}
              onDragOver={(event) => event.preventDefault()}
              onDrop={() => {
                if (dragIndex.current !== null) move(dragIndex.current, index);
                dragIndex.current = null;
              }}
            >
              <span className="rebase-grip" aria-hidden="true">
                ⠿
              </span>
              <select
                value={row.action}
                onChange={(event) =>
                  setRows((current) =>
                    current.map((item, i) =>
                      i === index ? { ...item, action: event.target.value as RebaseAction } : item,
                    ),
                  )
                }
              >
                {REBASE_ACTIONS.map((action) => (
                  <option key={action} value={action}>
                    {action}
                  </option>
                ))}
              </select>
              <code className="rebase-sha">{row.sha.slice(0, 8)}</code>
              <span className={row.action === "drop" ? "rebase-subject dropped" : "rebase-subject"}>
                {row.subject}
              </span>
            </div>
          ))}
        </div>

        <p className="rebase-hint">
          reword/squash mantêm a mensagem padrão; conflitos param o rebase e são resolvidos no
          banner de conflito (Continuar/Abortar).
        </p>
        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={onClose}>
            Cancelar
          </button>
          <button
            className="primary-button"
            type="button"
            onClick={() => onSubmit(rows.map(({ action, sha }) => ({ action, sha })))}
          >
            Iniciar rebase
          </button>
        </div>
      </section>
    </div>
  );
}

function localGitRefreshLabel(state: LocalGitRefreshState) {
  switch (state) {
    case "checking":
      return "checking";
    case "loading":
      return "loading";
    case "stale":
      return "stale";
    case "error":
      return "error";
    case "cached":
      return "cached";
    case "idle":
    default:
      return "idle";
  }
}

function GitWorkbench({
  path,
  projectId,
  diffProps,
  activeAgentProfileId,
  aiCommitProfileId,
  agentProfiles,
  changedCount,
  onRefreshLocal,
  onRequestAiCommit,
  onSelectAiCommitProfile,
  sidebarWidth,
  onSidebarResize,
}: {
  path: string;
  projectId: number | null;
  diffProps: Omit<React.ComponentProps<typeof DiffPanel>, "commitSlot">;
  activeAgentProfileId: number | null;
  aiCommitProfileId: number | null;
  agentProfiles: AgentProfile[];
  changedCount: number;
  onRefreshLocal: () => void;
  onRequestAiCommit: (profileId?: number | null) => Promise<string | null>;
  onSelectAiCommitProfile: (profileId: number | null) => void;
  sidebarWidth: number;
  onSidebarResize: (width: number) => void;
}) {
  const [view, setView] = useState<GitView>("local");
  const [sidebarMode, setSidebarMode] = useState<"commits" | "branches" | "stashes">("commits");
  const [aiCommitBusy, setAiCommitBusy] = useState(false);
  const [commits, setCommits] = useState<Commit[]>([]);
  const [commitQuery, setCommitQuery] = useState("");
  const [rebaseBase, setRebaseBase] = useState<string | null>(null);
  const [branches, setBranches] = useState<Branch[]>([]);
  const [remoteBranches, setRemoteBranches] = useState<RemoteBranch[]>([]);
  const [tags, setTags] = useState<TagEntry[]>([]);
  const [stashes, setStashes] = useState<StashEntry[]>([]);
  const [submodules, setSubmodules] = useState<Submodule[]>([]);
  const [repoState, setRepoState] = useState<RepoState | null>(null);
  const [includeRemotes, setIncludeRemotes] = useState(false);
  const [includeTags, setIncludeTags] = useState(false);
  const [limit, setLimit] = useState(200);
  const [selectedSha, setSelectedSha] = useState<string | null>(null);
  const [selectedStash, setSelectedStash] = useState<StashEntry | null>(null);
  const [detail, setDetail] = useState<CommitDetail | null>(null);
  const [detailTab, setDetailTab] = useState<"commit" | "changes" | "tree">("commit");
  const [detailFile, setDetailFile] = useState<FilePatch | null>(null);
  const [commitSubject, setCommitSubject] = useState("");
  const [commitDescription, setCommitDescription] = useState("");
  const [amend, setAmend] = useState(false);
  const [newBranchModal, setNewBranchModal] = useState<{ source: string } | null>(null);
  const [pendingCheckout, setPendingCheckout] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [autoFetching, setAutoFetching] = useState(false);
  const [error, setError] = useState("");
  const [info, setInfo] = useState("");
  const layoutRef = useRef<HTMLDivElement>(null);
  const commitDescriptionRef = useRef<HTMLTextAreaElement | null>(null);
  const gitOperationBusyRef = useRef(false);
  const gitPreferenceReadyProjectRef = useRef<number | null>(null);
  const aiProfileValue = aiCommitProfileId ?? activeAgentProfileId ?? agentProfiles[0]?.id ?? null;

  useEffect(() => {
    if (projectId == null) {
      gitPreferenceReadyProjectRef.current = null;
      return;
    }
    gitPreferenceReadyProjectRef.current = null;
    let cancelled = false;
    void api.getAppState(projectUiPreferenceKey(projectId, "git_workbench")).then((result) => {
      if (cancelled) return;
      const preference = parseGitWorkbenchPreference(result.ok ? result.value : null);
      setView(preference.view);
      setIncludeRemotes(preference.includeRemotes);
      setIncludeTags(preference.includeTags);
      setLimit(preference.limit);
      setDetailTab(preference.detailTab);
      gitPreferenceReadyProjectRef.current = projectId;
    });
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  useEffect(() => {
    if (projectId == null || gitPreferenceReadyProjectRef.current !== projectId) return;
    void api.setAppState(
      projectUiPreferenceKey(projectId, "git_workbench"),
      serializeGitWorkbenchPreference({
        view,
        includeRemotes,
        includeTags,
        limit,
        detailTab,
      }),
    );
  }, [projectId, view, includeRemotes, includeTags, limit, detailTab]);

  const snapshotKey = useMemo(
    () =>
      projectId == null
        ? null
        : gitSnapshotCacheKey(projectId, { includeRemotes, includeTags, limit }),
    [projectId, includeRemotes, includeTags, limit],
  );

  const applySnapshot = useCallback((snapshot: GitRepoSnapshot) => {
    setCommits(snapshot.commits);
    setBranches(snapshot.branches);
    setRemoteBranches(snapshot.remote_branches);
    setTags(snapshot.tags);
    setStashes(snapshot.stashes);
    setSubmodules(snapshot.submodules);
    setRepoState(snapshot.repo_state);
  }, []);

  const refreshRefs = useCallback(
    async ({ silent = false }: { silent?: boolean } = {}) => {
      if (!path) return null;
      if (!silent) setRefreshing(true);
      const snapshot = await api.gitRepoSnapshot(path, { includeRemotes, includeTags, limit });
      if (snapshot.ok) {
        applySnapshot(snapshot.value);
        if (snapshotKey) {
          void api.setAppState(snapshotKey, serializeGitSnapshotCache(snapshot.value));
        }
      } else if (!silent) {
        setError(snapshot.error);
      }
      if (!silent) setRefreshing(false);
      return snapshot.ok ? snapshot.value : null;
    },
    [path, includeRemotes, includeTags, limit, applySnapshot, snapshotKey],
  );

  useEffect(() => {
    if (!path) return;
    let cancelled = false;
    void (async () => {
      let cached: GitRepoSnapshot | null = null;
      if (snapshotKey) {
        const cachedResult = await api.getAppState(snapshotKey);
        if (cancelled) return;
        cached = cachedResult.ok ? parseGitSnapshotCache(cachedResult.value) : null;
        if (cached) applySnapshot(cached);
      }

      const fresh = await refreshRefs({ silent: Boolean(cached) });
      if (cancelled) return;
      const latest = fresh ?? cached;
      if (!projectId || !latest) return;

      const remote = resolveAutoFetchRemote(latest.repo_state, latest.remote_branches);
      const lastFetchResult = await api.getAppState(gitAutoFetchKey(projectId));
      if (cancelled) return;
      const lastFetchAt =
        lastFetchResult.ok && lastFetchResult.value ? Number(lastFetchResult.value) : null;

      if (
        shouldAutoFetch({
          now: Date.now(),
          lastFetchAt: Number.isFinite(lastFetchAt) ? lastFetchAt : null,
          busy: gitOperationBusyRef.current,
          operation: latest.repo_state.operation,
          remote,
        })
      ) {
        setAutoFetching(true);
        const fetched = await api.gitFetch(path, remote ?? undefined);
        if (cancelled) return;
        if (fetched.ok) {
          void api.setAppState(gitAutoFetchKey(projectId), String(Date.now()));
          await refreshRefs({ silent: true });
        } else {
          setError(`Auto-fetch: ${fetched.error}`);
        }
        setAutoFetching(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [path, projectId, snapshotKey, applySnapshot, refreshRefs]);

  const selectCommit = useCallback(
    (sha: string) => {
      setView("commits");
      setSelectedSha(sha);
      setSelectedStash(null);
      setDetailFile(null);
      setDetailTab("commit");
      void api.gitCommitDetail(path, sha).then((result) => {
        if (!result.ok) {
          setError(result.error);
          return;
        }
        setDetail(result.value);
        // Preload the first file's diff so the Changes tab is never empty.
        const first = result.value.files[0];
        if (first) {
          void api.gitCommitFileDiff(path, sha, first.path).then((diffResult) => {
            if (diffResult.ok) setDetailFile(diffResult.value);
          });
        }
      });
    },
    [path],
  );

  const selectStash = useCallback(
    (stash: StashEntry) => {
      setView("commits");
      setSelectedStash(stash);
      setSelectedSha(null);
      setDetail(null);
      setDetailFile(null);
      setDetailTab("changes");
      void api.gitStashDetail(path, stash.index).then((result) => {
        if (!result.ok) {
          setError(result.error);
          return;
        }
        setDetail(result.value);
        const first = result.value.files[0];
        if (first) {
          void api.gitStashFileDiff(path, stash.index, first.path).then((diffResult) => {
            if (diffResult.ok) setDetailFile(diffResult.value);
          });
        }
      });
    },
    [path],
  );

  const run = useCallback(
    async (label: string, action: () => ReturnType<typeof api.gitFetch>) => {
      gitOperationBusyRef.current = true;
      setBusy(true);
      setError("");
      setInfo("");
      try {
        const result = await action();
        if (result.ok) {
          setInfo(`${label} ✓`);
          onRefreshLocal();
          await refreshRefs();
          return true;
        }
        setError(`${label}: ${result.error}`);
        await refreshRefs();
        return false;
      } catch (error) {
        setError(`${label}: ${error instanceof Error ? error.message : String(error)}`);
        return false;
      } finally {
        setBusy(false);
        gitOperationBusyRef.current = false;
      }
    },
    [onRefreshLocal, refreshRefs],
  );

  // clia-local: pull, then auto-sync submodules — a pull that moves submodule
  // pointers leaves the working tree stale until `submodule update` runs.
  const pullWithSubmodules = (rebase: boolean) => async () => {
    const result = await api.gitPull(path, rebase);
    if (result.ok) await api.gitUpdateAllSubmodules(path, true);
    return result;
  };

  async function doCommit() {
    const message = composeCommitMessage(commitSubject, commitDescription);
    if (!message.trim() && !amend) return;
    const committed = await run("Commit", () => api.gitCommit(path, message, amend));
    if (committed) {
      setCommitSubject("");
      setCommitDescription("");
      setAmend(false);
    }
  }

  async function openDetailFile(file: CommitFile) {
    const result = selectedStash
      ? await api.gitStashFileDiff(path, selectedStash.index, file.path)
      : selectedSha
        ? await api.gitCommitFileDiff(path, selectedSha, file.path)
        : null;
    if (!result) return;
    if (result.ok) setDetailFile(result.value);
    else setError(result.error);
  }

  function requestCheckout(full: string) {
    if (repoState?.dirty) {
      setPendingCheckout(full);
    } else {
      void run(`Checkout ${full}`, () => api.gitCheckoutBranch(path, full, "plain"));
    }
  }

  function doCheckout(full: string, mode: "discard" | "stash" | "stash_apply") {
    setPendingCheckout(null);
    void run(`Checkout ${full}`, () => api.gitCheckoutBranch(path, full, mode));
  }

  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const { confirm, dialog: confirmDialog } = useConfirm();
  const { prompt, dialog: promptDialog } = usePrompt();

  function commitMenuItems(commit: Commit): MenuItem[] {
    return [
      {
        label: "Checkout (detached)",
        onSelect: () => void run("Checkout commit", () => api.gitCheckoutCommit(path, commit.sha)),
      },
      {
        label: "Criar branch aqui…",
        onSelect: () =>
          void prompt({
            title: "Criar branch aqui",
            label: "Nome",
            initial: `branch-${commit.short_sha}`,
          }).then((name) => {
            if (name) void run("Branch criada", () => api.gitCreateBranch(path, name, commit.sha));
          }),
      },
      {
        label: "Criar tag aqui…",
        onSelect: () =>
          void prompt({
            title: "Criar tag aqui",
            label: "Nome",
            initial: `tag-${commit.short_sha}`,
          }).then((name) => {
            if (name) void run("Tag criada", () => api.gitCreateTag(path, name, commit.sha));
          }),
      },
      { separator: true },
      {
        label: "Cherry-pick",
        onSelect: () => void run("Cherry-pick", () => api.gitCherryPick(path, commit.sha)),
      },
      {
        label: "Revert",
        onSelect: () => void run("Revert", () => api.gitRevert(path, commit.sha)),
      },
      {
        label: "Reset",
        submenu: [
          {
            label: "Soft",
            onSelect: () => void run("Reset --soft", () => api.gitReset(path, commit.sha, "soft")),
          },
          {
            label: "Mixed",
            onSelect: () =>
              void run("Reset --mixed", () => api.gitReset(path, commit.sha, "mixed")),
          },
          {
            label: "Hard",
            danger: true,
            onSelect: () =>
              void confirm({
                title: "Reset --hard?",
                body: "Descarta mudanças locais até esse commit.",
                danger: true,
                confirmLabel: "Reset --hard",
              }).then((ok) => {
                if (ok) void run("Reset --hard", () => api.gitReset(path, commit.sha, "hard"));
              }),
          },
        ],
      },
      { label: "Rebase interativo a partir daqui", onSelect: () => openRebase(commit.sha) },
      { separator: true },
      { label: "Copiar SHA", onSelect: () => void navigator.clipboard?.writeText(commit.sha) },
      {
        label: "Copiar mensagem",
        onSelect: () => void navigator.clipboard?.writeText(commit.subject),
      },
    ];
  }

  function branchMenuItems(leaf: RefLeaf): MenuItem[] {
    const isHead = Boolean(leaf.isHead);
    return [
      { label: "Checkout", disabled: isHead, onSelect: () => requestCheckout(leaf.full) },
      {
        label: "Criar branch a partir desta…",
        onSelect: () => setNewBranchModal({ source: leaf.full }),
      },
      {
        label: "Merge na atual",
        disabled: isHead,
        onSelect: () => void run(`Merge ${leaf.full}`, () => api.gitMergeBranch(path, leaf.full)),
      },
      {
        label: "Rebase atual sobre esta",
        disabled: isHead,
        onSelect: () =>
          void run(`Rebase onto ${leaf.full}`, () => api.gitRebaseBranch(path, leaf.full)),
      },
      { separator: true },
      {
        label: "Renomear…",
        onSelect: () =>
          void prompt({ title: "Renomear branch", label: "Novo nome", initial: leaf.full }).then(
            (name) => {
              if (name && name !== leaf.full)
                void run("Branch renomeada", () => api.gitRenameBranch(path, leaf.full, name));
            },
          ),
      },
      {
        label: "Excluir",
        danger: true,
        disabled: isHead,
        onSelect: () =>
          void confirm({
            title: `Excluir branch ${leaf.full}?`,
            danger: true,
            confirmLabel: "Excluir",
          }).then((ok) => {
            if (ok) void run("Branch excluída", () => api.gitDeleteBranch(path, leaf.full));
          }),
      },
      { separator: true },
      {
        label: "Push",
        onSelect: () =>
          void run("Push", () => api.gitPush(path, { setUpstream: !repoState?.upstream })),
      },
      { label: "Pull", onSelect: () => void run("Pull", pullWithSubmodules(false)) },
    ];
  }

  function tagMenuItems(name: string, sha: string): MenuItem[] {
    return [
      {
        label: "Checkout",
        onSelect: () => void run("Checkout tag", () => api.gitCheckoutCommit(path, sha)),
      },
      {
        label: "Excluir",
        danger: true,
        onSelect: () =>
          void confirm({
            title: `Excluir tag ${name}?`,
            danger: true,
            confirmLabel: "Excluir",
          }).then((ok) => {
            if (ok) void run("Tag excluída", () => api.gitDeleteTag(path, name));
          }),
      },
    ];
  }

  function stashMenuItems(index: number): MenuItem[] {
    return [
      {
        label: "Pop (aplicar e remover)",
        onSelect: () => void run("Stash pop", () => api.gitStashPop(path, index)),
      },
      {
        label: "Apply (aplicar e manter)",
        onSelect: () => void run("Stash apply", () => api.gitStashApply(path, index)),
      },
      {
        label: "Drop (descartar)",
        danger: true,
        onSelect: () =>
          void confirm({ title: "Descartar stash?", danger: true, confirmLabel: "Drop" }).then(
            (ok) => {
              if (ok) void run("Stash drop", () => api.gitStashDrop(path, index));
            },
          ),
      },
    ];
  }

  function saveStash() {
    void prompt({ title: "Stash", label: "Mensagem", initial: "WIP" }).then((message) => {
      if (message) void run("Stash criado", () => api.gitStashSave(path, message, true));
    });
  }

  const currentBranch = repoState?.branch ?? null;
  const branchTree = useMemo(
    () =>
      buildRefTree(
        branches
          .slice()
          .sort((a, b) => a.name.localeCompare(b.name))
          .map((b) => ({
            name: b.name,
            full: b.name,
            isHead: b.is_head,
            ahead: b.ahead,
            behind: b.behind,
          })),
      ),
    [branches],
  );
  const remoteTree = useMemo(
    () =>
      buildRefTree(
        remoteBranches
          .slice()
          .sort((a, b) => a.full.localeCompare(b.full))
          .map((r) => ({ name: r.full, full: r.full })),
      ),
    [remoteBranches],
  );
  const visibleCommits = useMemo(() => filterCommits(commits, commitQuery), [commits, commitQuery]);
  // Commits newer than the rebase base (oldest-first) — the ones the interactive
  // rebase will replay. Empty when base is the tip or history isn't linear here.
  const rebaseCommits = useMemo(() => {
    if (!rebaseBase) return [];
    const index = commits.findIndex((commit) => commit.sha === rebaseBase);
    return index > 0 ? commits.slice(0, index).reverse() : [];
  }, [commits, rebaseBase]);

  function openRebase(sha: string) {
    if (repoState?.dirty) {
      setError("Faça commit ou stash das mudanças antes do rebase interativo.");
      return;
    }
    setRebaseBase(sha);
  }

  function startRebase(base: string, steps: RebaseStep[]) {
    setRebaseBase(null);
    void run("Rebase interativo", () => api.gitStartInteractiveRebase(path, base, steps));
  }

  function createNewBranch(name: string, source: string, checkout: boolean) {
    setNewBranchModal(null);
    void run("Branch criada", () => api.gitCreateBranch(path, name, source || undefined, checkout));
  }

  function startSidebarResize(event: React.MouseEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = sidebarWidth;
    let latest = startWidth;
    const onMove = (move: MouseEvent) => {
      latest = Math.max(200, Math.min(560, startWidth + (move.clientX - startX)));
      layoutRef.current?.style.setProperty("--git-sidebar-w", `${latest}px`);
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      onSidebarResize(latest);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
  }

  return (
    <div className="git-workbench">
      <header className="git-topbar screen-topbar">
        <div className="git-toolbar-left">
          <span className="topbar-title">Git Workbench</span>
          <span className="topbar-branch">
            <GitBranch aria-hidden="true" size={14} />
            {currentBranch ?? (repoState?.detached ? "(detached)" : "—")}
          </span>
          {repoState && (repoState.ahead > 0 || repoState.behind > 0) ? (
            <span className="git-tracking">
              ↑{repoState.ahead} ↓{repoState.behind}
            </span>
          ) : null}
          {refreshing || autoFetching ? (
            <span className="git-sync-state">
              <span className="flow-run-spinner" aria-hidden="true" />
              {autoFetching ? "fetch" : "refresh"}
            </span>
          ) : null}
        </div>
        <div className="git-toolbar-actions">
          <button
            className="topbar-btn"
            type="button"
            disabled={busy || autoFetching}
            onClick={() => void run("Fetch", () => api.gitFetch(path))}
          >
            <RefreshCw className="ico-fetch" aria-hidden="true" size={15} /> Fetch
          </button>
          <button
            className="topbar-btn"
            type="button"
            disabled={busy}
            onClick={() => void run("Pull", pullWithSubmodules(false))}
          >
            <Download className="ico-pull" aria-hidden="true" size={15} /> Pull
          </button>
          <button
            className="topbar-btn primary"
            type="button"
            disabled={busy}
            onClick={() =>
              void run("Push", () => api.gitPush(path, { setUpstream: !repoState?.upstream }))
            }
          >
            <GitPullRequestArrow aria-hidden="true" size={15} /> Push
          </button>
          <button
            className="topbar-btn"
            type="button"
            disabled={busy}
            onClick={() => void refreshRefs()}
            title="Refresh"
          >
            <RefreshCw className="ico-fetch" aria-hidden="true" size={15} />
          </button>
          <button
            className="topbar-btn"
            type="button"
            disabled={busy}
            onClick={() => void run("Pull --rebase", pullWithSubmodules(true))}
          >
            Pull rebase
          </button>
          <button
            className="topbar-btn"
            type="button"
            disabled={busy}
            onClick={() =>
              void run("Push --force-with-lease", () => api.gitPush(path, { forceWithLease: true }))
            }
          >
            <GitPullRequestArrow className="ico-push" aria-hidden="true" size={15} /> Force push
          </button>
          <button className="topbar-btn" type="button" disabled={busy} onClick={saveStash}>
            <Archive className="ico-stash" aria-hidden="true" size={15} /> Stash
          </button>
        </div>
      </header>

      {repoState?.operation ? (
        <div className="git-conflict-banner">
          <div className="git-conflict-head">
            <span>
              <strong>{repoState.operation}</strong> em progresso
              {repoState.conflicts.length
                ? ` — ${repoState.conflicts.length} conflito(s)`
                : " — sem conflitos"}
              .
            </span>
            <div>
              <button className="secondary-button" type="button" onClick={() => setView("local")}>
                Local Changes
              </button>
              <button
                className="primary-button"
                type="button"
                disabled={busy || repoState.conflicts.length > 0}
                title={repoState.conflicts.length ? "Resolva os conflitos primeiro" : undefined}
                onClick={() =>
                  void run("Continuar", () =>
                    api.gitContinueOperation(path, repoState.operation as string),
                  )
                }
              >
                Continuar
              </button>
              <button
                className="secondary-button"
                type="button"
                onClick={() =>
                  void run("Abort", () =>
                    api.gitAbortOperation(path, repoState.operation as string),
                  )
                }
              >
                Abortar
              </button>
            </div>
          </div>
          {repoState.conflicts.length ? (
            <ul className="git-conflict-files">
              {repoState.conflicts.map((file) => (
                <li key={file}>
                  <span className="git-fpath">{file}</span>
                  <span className="git-conflict-actions">
                    <button
                      type="button"
                      className="ghost-button"
                      onClick={() => void run("Usar ours", () => api.gitUseOurs(path, file))}
                    >
                      Usar ours
                    </button>
                    <button
                      type="button"
                      className="ghost-button"
                      onClick={() => void run("Usar theirs", () => api.gitUseTheirs(path, file))}
                    >
                      Usar theirs
                    </button>
                    <button
                      type="button"
                      className="ghost-button"
                      onClick={() => void run("Resolvido", () => api.gitMarkResolved(path, file))}
                    >
                      Marcar resolvido
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      {error ? <div className="git-error">{error}</div> : null}
      {info && !error ? <div className="git-info">{info}</div> : null}

      <div
        className="git-body"
        ref={layoutRef}
        style={{ "--git-sidebar-w": `${sidebarWidth}px` } as React.CSSProperties}
      >
        <aside className="git-sidebar">
          <div className="sidebar-tabs" role="tablist" aria-label="Git views">
            <button
              className={sidebarMode === "commits" ? "sidebar-tab active" : "sidebar-tab"}
              type="button"
              role="tab"
              aria-selected={sidebarMode === "commits"}
              onClick={() => setSidebarMode("commits")}
            >
              Commits
            </button>
            <button
              className={sidebarMode === "branches" ? "sidebar-tab active" : "sidebar-tab"}
              type="button"
              role="tab"
              aria-selected={sidebarMode === "branches"}
              onClick={() => setSidebarMode("branches")}
            >
              Branches
            </button>
            <button
              className={sidebarMode === "stashes" ? "sidebar-tab active" : "sidebar-tab"}
              type="button"
              role="tab"
              aria-selected={sidebarMode === "stashes"}
              onClick={() => setSidebarMode("stashes")}
            >
              Stashes
            </button>
          </div>

          <div className="sidebar-content">
            {sidebarMode === "commits" ? (
              <div className="commit-list">
                <button
                  className={view === "local" ? "commit-item active" : "commit-item"}
                  type="button"
                  onClick={() => {
                    setView("local");
                    setSelectedSha(null);
                    setSelectedStash(null);
                  }}
                >
                  <span className="commit-hash">worktree</span>
                  <span className="commit-msg">Local Changes</span>
                  <span className="commit-meta">
                    <span className="commit-author">{changedCount} arquivos</span>
                    <span>{localGitRefreshLabel(diffProps.refreshState ?? "idle")}</span>
                  </span>
                </button>
                <input
                  className="git-commit-search"
                  type="search"
                  value={commitQuery}
                  placeholder="Buscar commits"
                  onChange={(e) => setCommitQuery(e.target.value)}
                />
                {visibleCommits.map((commit) => (
                  <button
                    key={commit.sha}
                    type="button"
                    className={selectedSha === commit.sha ? "commit-item active" : "commit-item"}
                    onClick={() => selectCommit(commit.sha)}
                    onContextMenu={(event) => openMenu(event, commitMenuItems(commit))}
                  >
                    <span className="commit-hash">{commit.short_sha}</span>
                    <span className="commit-msg">{commit.subject}</span>
                    <span className="commit-meta">
                      <span className="commit-author">{commit.author_name}</span>
                      <span>{commit.date.slice(0, 10)}</span>
                    </span>
                  </button>
                ))}
                {commits.length >= limit ? (
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => setLimit((n) => n + 200)}
                  >
                    Carregar mais
                  </button>
                ) : null}
              </div>
            ) : sidebarMode === "branches" ? (
              <>
                <div className="git-section-head">
                  <span>Branches</span>
                  <button
                    className="ghost-button icon-only"
                    type="button"
                    title="Nova branch"
                    aria-label="Nova branch"
                    onClick={() => setNewBranchModal({ source: currentBranch ?? "" })}
                  >
                    <Plus aria-hidden="true" size={14} />
                  </button>
                </div>
                <div className="git-ref-list">
                  <GitRefTree
                    nodes={branchTree}
                    leafIcon={<GitBranch aria-hidden="true" size={13} />}
                    onCheckout={(full) => requestCheckout(full)}
                    onMerge={(full) =>
                      void run(`Merge ${full}`, () => api.gitMergeBranch(path, full))
                    }
                    onContext={(leaf, event) => openMenu(event, branchMenuItems(leaf))}
                  />
                </div>

                {remoteBranches.length ? (
                  <>
                    <div className="git-section-head">
                      <span>Remotes</span>
                    </div>
                    <div className="git-ref-list">
                      <GitRefTree
                        nodes={remoteTree}
                        leafIcon={<GitFork aria-hidden="true" size={13} />}
                        onCheckout={(full) => requestCheckout(full)}
                      />
                    </div>
                  </>
                ) : null}

                {tags.length ? (
                  <>
                    <div className="git-section-head">
                      <span>Tags</span>
                    </div>
                    <div className="git-ref-list">
                      {tags.map((tag) => (
                        <div className="git-ref-row" key={tag.name}>
                          <button
                            type="button"
                            onClick={() => selectCommit(tag.sha)}
                            onContextMenu={(event) =>
                              openMenu(event, tagMenuItems(tag.name, tag.sha))
                            }
                          >
                            {tag.name}
                          </button>
                        </div>
                      ))}
                    </div>
                  </>
                ) : null}

                {submodules.length ? (
                  <>
                    <div className="git-section-head">
                      <span>Submódulos</span>
                      <button
                        className="git-ref-action"
                        type="button"
                        title="submodule sync + update --init --recursive"
                        disabled={busy}
                        onClick={() =>
                          void run("Submódulos atualizados", async () => {
                            await api.gitSyncSubmodules(path);
                            return api.gitUpdateAllSubmodules(path, true);
                          })
                        }
                      >
                        atualizar todos
                      </button>
                    </div>
                    <div className="git-ref-list">
                      {submodules.map((sub) => (
                        <div className="git-ref-row submodule-row" key={sub.path}>
                          <span className="git-ref-name" title={`${sub.status} · ${sub.sha}`}>
                            {sub.path}
                            {sub.status !== "ok" ? (
                              <small className="submodule-flag"> {sub.status}</small>
                            ) : null}
                            {sub.detached ? (
                              <small className="submodule-flag detached"> detached</small>
                            ) : sub.branch ? (
                              <small className="submodule-branch"> {sub.branch}</small>
                            ) : null}
                          </span>
                          <button
                            className="git-ref-action"
                            type="button"
                            title="submodule update --init --recursive"
                            disabled={busy}
                            onClick={() =>
                              void run("Submódulo atualizado", () =>
                                api.gitUpdateSubmodule(path, sub.path, true),
                              )
                            }
                          >
                            update
                          </button>
                          <button
                            className="git-ref-action"
                            type="button"
                            title="submodule update --remote (seguir branch rastreada)"
                            disabled={busy}
                            onClick={() =>
                              void run("Submódulo (branch) atualizado", () =>
                                api.gitUpdateSubmoduleRemote(path, sub.path),
                              )
                            }
                          >
                            remote
                          </button>
                          {sub.detached ? (
                            <button
                              className="git-ref-action"
                              type="button"
                              title="checkout da branch rastreada (sai do detached HEAD)"
                              disabled={busy}
                              onClick={() =>
                                void run("Submódulo na branch", () =>
                                  api.gitCheckoutSubmoduleBranch(path, sub.path),
                                )
                              }
                            >
                              branch
                            </button>
                          ) : null}
                        </div>
                      ))}
                    </div>
                  </>
                ) : null}
              </>
            ) : (
              <div className="commit-list">
                <button className="secondary-button" type="button" onClick={saveStash} disabled={busy}>
                  <Archive aria-hidden="true" size={14} /> Criar stash
                </button>
                {stashes.length ? (
                  stashes.map((stash) => (
                    <button
                      className={
                        selectedStash?.index === stash.index ? "commit-item active" : "commit-item"
                      }
                      key={stash.label}
                      type="button"
                      onClick={() => selectStash(stash)}
                      onContextMenu={(event) => openMenu(event, stashMenuItems(stash.index))}
                    >
                      <span className="commit-hash">{stash.label}</span>
                      <span className="commit-msg">{stash.message}</span>
                      <span className="commit-meta">
                        <span className="commit-author">stash</span>
                      </span>
                    </button>
                  ))
                ) : (
                  <div className="empty-note">Nenhum stash salvo.</div>
                )}
              </div>
            )}
          </div>
        </aside>

        <div
          className="git-sidebar-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label="Redimensionar lista de branches"
          onMouseDown={startSidebarResize}
        />

        <div className="git-main">
          {view === "local" ? (
            <div className="git-local">
              <DiffPanel
                {...diffProps}
                commitSlot={
                  <div className="commit-panel git-commit-bar">
                    <div className="commit-panel-header">
                      <span className="commit-panel-title">Commit Message</span>
                      <button
                        className="ai-commit-btn"
                        type="button"
                        disabled={busy || aiCommitBusy || !aiProfileValue}
                        title="Gerar mensagem de commit com IA a partir das mudanças"
                        onClick={async () => {
                          setAiCommitBusy(true);
                          const message = await onRequestAiCommit(aiProfileValue);
                          if (message) {
                            const split = splitCommitMessage(message);
                            setCommitSubject(split.subject);
                            setCommitDescription(split.description);
                          }
                          setAiCommitBusy(false);
                        }}
                      >
                        {aiCommitBusy ? (
                          <span className="flow-run-spinner" aria-hidden="true" />
                        ) : (
                          <Sparkles aria-hidden="true" size={15} />
                        )}
                        AI Commit
                      </button>
                    </div>
                    <div className="git-commit-fields">
                      <input
                        value={commitSubject}
                        placeholder="Subject"
                        aria-label="Commit subject"
                        onChange={(event) => setCommitSubject(event.target.value)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") {
                            event.preventDefault();
                            commitDescriptionRef.current?.focus();
                          }
                        }}
                      />
                      <textarea
                        className="commit-textarea"
                        ref={commitDescriptionRef}
                        value={commitDescription}
                        placeholder="Description"
                        aria-label="Commit description"
                        onChange={(event) => setCommitDescription(event.target.value)}
                      />
                    </div>
                    <div className="commit-actions git-commit-actions">
                      <label className="git-toggle">
                        <input
                          type="checkbox"
                          checked={amend}
                          onChange={(e) => setAmend(e.target.checked)}
                        />{" "}
                        amend
                      </label>
                      {agentProfiles.length > 1 ? (
                        <select
                          className="git-ai-profile-select"
                          value={aiProfileValue ?? ""}
                          aria-label="Agente do AI Commit"
                          disabled={busy || aiCommitBusy}
                          onChange={(event) => {
                            const value = Number(event.target.value);
                            onSelectAiCommitProfile(Number.isFinite(value) ? value : null);
                          }}
                        >
                          {agentProfiles.map((profile) => (
                            <option key={profile.id} value={profile.id}>
                              {profile.name}
                            </option>
                          ))}
                        </select>
                      ) : null}
                      <button
                        className="topbar-btn"
                        type="button"
                        onClick={() => diffProps.onStageAll?.()}
                      >
                        Stage All
                      </button>
                      <button
                        className="topbar-btn primary"
                        type="button"
                        disabled={busy}
                        onClick={() => void doCommit()}
                      >
                        <GitCommitHorizontal aria-hidden="true" size={15} /> Commit
                      </button>
                    </div>
                  </div>
                }
              />
            </div>
          ) : (
            <div className="git-commits">
              <div className="git-graph-toolbar">
                <input
                  className="git-commit-search"
                  type="search"
                  value={commitQuery}
                  placeholder="Buscar (mensagem, autor, SHA)"
                  onChange={(e) => setCommitQuery(e.target.value)}
                />
                <label className="git-toggle">
                  <input
                    type="checkbox"
                    checked={includeRemotes}
                    onChange={(e) => setIncludeRemotes(e.target.checked)}
                  />{" "}
                  remotas
                </label>
                <label className="git-toggle">
                  <input
                    type="checkbox"
                    checked={includeTags}
                    onChange={(e) => setIncludeTags(e.target.checked)}
                  />{" "}
                  tags
                </label>
                <span className="git-graph-count">
                  {commitQuery.trim()
                    ? `${visibleCommits.length}/${commits.length} commits`
                    : `${commits.length} commits`}
                </span>
              </div>
              <div className="git-graph-scroll">
                <CommitGraph
                  commits={visibleCommits}
                  selectedSha={selectedSha}
                  onSelect={selectCommit}
                  onContextMenu={(commit, event) => openMenu(event, commitMenuItems(commit))}
                />
                {commitQuery.trim() && commits.length >= limit ? (
                  <div className="git-graph-hint">
                    Buscando só nos primeiros {commits.length} commits — use “Carregar mais” para ir
                    mais fundo.
                  </div>
                ) : null}
                {commits.length >= limit ? (
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => setLimit((n) => n + 200)}
                  >
                    Carregar mais
                  </button>
                ) : null}
              </div>
              {detail ? (
                <div className="git-detail">
                  <div className="git-detail-tabs">
                    {(["commit", "changes", "tree"] as const).map((tab) => (
                      <button
                        key={tab}
                        type="button"
                        className={detailTab === tab ? "active" : ""}
                        onClick={() => setDetailTab(tab)}
                      >
                        {tab === "commit"
                          ? selectedStash
                            ? "Stash"
                            : "Commit"
                          : tab === "changes"
                            ? "Changes"
                            : "File Tree"}
                      </button>
                    ))}
                    <div className="git-detail-actions">
                      {selectedStash ? (
                        <>
                          <button
                            type="button"
                            className="secondary-button"
                            onClick={() => void navigator.clipboard?.writeText(selectedStash.label)}
                          >
                            Copiar ref
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Stash apply", () =>
                                api.gitStashApply(path, selectedStash.index),
                              )
                            }
                          >
                            Apply
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Stash pop", () =>
                                api.gitStashPop(path, selectedStash.index),
                              ).then((ok) => {
                                if (ok) {
                                  setSelectedStash(null);
                                  setDetail(null);
                                  setDetailFile(null);
                                }
                              })
                            }
                          >
                            Pop
                          </button>
                          <button
                            type="button"
                            className="secondary-button danger"
                            disabled={busy}
                            onClick={() =>
                              void confirm({
                                title: "Descartar stash?",
                                danger: true,
                                confirmLabel: "Drop",
                              }).then((ok) => {
                                if (!ok) return;
                                void run("Stash drop", () =>
                                  api.gitStashDrop(path, selectedStash.index),
                                ).then((dropped) => {
                                  if (dropped) {
                                    setSelectedStash(null);
                                    setDetail(null);
                                    setDetailFile(null);
                                  }
                                });
                              })
                            }
                          >
                            Drop
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            type="button"
                            className="secondary-button"
                            onClick={() => void navigator.clipboard?.writeText(detail.sha)}
                          >
                            Copiar SHA
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Cherry-pick", () => api.gitCherryPick(path, detail.sha))
                            }
                          >
                            Cherry-pick
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Revert", () => api.gitRevert(path, detail.sha))
                            }
                          >
                            Revert
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Branch criada", () =>
                                api.gitCreateBranch(path, `branch-${detail.short_sha}`, detail.sha),
                              )
                            }
                          >
                            Branch aqui
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Reset --mixed", () =>
                                api.gitReset(path, detail.sha, "mixed"),
                              )
                            }
                          >
                            Reset
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            disabled={busy}
                            onClick={() =>
                              void run("Tag criada", () =>
                                api.gitCreateTag(path, `tag-${detail.short_sha}`, detail.sha),
                              )
                            }
                          >
                            Tag
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                  <div className="git-detail-body">
                    {detailTab === "commit" ? (
                      <div className="git-detail-meta">
                        <div className="git-detail-author">
                          <Avatar name={detail.author_name} email={detail.author_email} size={22} />
                          <span>
                            <strong>{detail.author_name}</strong>{" "}
                            <span className="git-detail-email">&lt;{detail.author_email}&gt;</span>
                            <small> · {detail.date.slice(0, 19).replace("T", " ")}</small>
                          </span>
                        </div>
                        {detail.refs.length ? (
                          <div className="git-detail-line">
                            <span className="git-detail-label">REFS</span>
                            <span className="git-detail-value">
                              {detail.refs.map((ref) => (
                                <span
                                  className={refBadgeClass(ref.kind)}
                                  key={`${ref.kind}-${ref.name}`}
                                >
                                  {ref.name}
                                </span>
                              ))}
                            </span>
                          </div>
                        ) : null}
                        {selectedStash ? (
                          <div className="git-detail-line">
                            <span className="git-detail-label">STASH</span>
                            <span className="git-detail-value mono">{selectedStash.label}</span>
                          </div>
                        ) : null}
                        <div className="git-detail-line">
                          <span className="git-detail-label">SHA</span>
                          <span className="git-detail-value mono">{detail.sha}</span>
                        </div>
                        {detail.parents.length ? (
                          <div className="git-detail-line">
                            <span className="git-detail-label">PARENTS</span>
                            <span className="git-detail-value mono">
                              {detail.parents.map((p) => p.slice(0, 8)).join("  ")}
                            </span>
                          </div>
                        ) : null}
                        <p className="git-detail-message">
                          {detail.subject}
                          {detail.body ? `\n\n${detail.body}` : ""}
                        </p>
                        <ul className="git-detail-files">
                          {detail.files.map((f) => (
                            <li key={f.path}>
                              <button
                                type="button"
                                className="git-detail-file-row"
                                onClick={() => {
                                  setDetailTab("changes");
                                  void openDetailFile(f);
                                }}
                              >
                                <span
                                  className={`git-fsquare s-${f.status[0]}`}
                                  aria-hidden="true"
                                />
                                <span className="git-fpath">{f.path}</span>
                                <small>
                                  +{f.additions} −{f.deletions}
                                </small>
                              </button>
                            </li>
                          ))}
                        </ul>
                      </div>
                    ) : detailTab === "changes" ? (
                      <div className="git-detail-changes">
                        <ul className="git-detail-files">
                          {detail.files.map((f) => (
                            <li key={f.path}>
                              <button type="button" onClick={() => void openDetailFile(f)}>
                                <span className={`git-fstatus s-${f.status[0]}`}>{f.status}</span>{" "}
                                {f.path}
                              </button>
                            </li>
                          ))}
                        </ul>
                        <DiffView
                          patch={detailFile}
                          language={detailFile ? sourceLanguage(detailFile.path) : "plain"}
                        />
                      </div>
                    ) : (
                      <ul className="git-detail-files tree">
                        {detail.files.map((f) => (
                          <li key={f.path}>{f.path}</li>
                        ))}
                      </ul>
                    )}
                  </div>
                </div>
              ) : null}
            </div>
          )}
        </div>
      </div>

      {pendingCheckout ? (
        <div className="modal-backdrop" role="presentation">
          <section
            className="modal-panel"
            role="dialog"
            aria-modal="true"
            aria-labelledby="checkout-confirm-title"
          >
            <div className="modal-heading">
              <div>
                <h2 id="checkout-confirm-title">Mudanças pendentes</h2>
                <p>
                  Há alterações não commitadas. O que fazer ao trocar para{" "}
                  <strong>{pendingCheckout}</strong>?
                </p>
              </div>
            </div>
            <div className="checkout-choices">
              <button
                className="primary-button"
                type="button"
                onClick={() => doCheckout(pendingCheckout, "stash_apply")}
              >
                Stash + aplicar na branch destino
              </button>
              <button
                className="secondary-button"
                type="button"
                onClick={() => doCheckout(pendingCheckout, "stash")}
              >
                Só fazer stash (guardar)
              </button>
              <button
                className="secondary-button checkout-discard"
                type="button"
                onClick={() => doCheckout(pendingCheckout, "discard")}
              >
                Descartar mudanças
              </button>
              <button
                className="secondary-button"
                type="button"
                onClick={() => setPendingCheckout(null)}
              >
                Cancelar
              </button>
            </div>
          </section>
        </div>
      ) : null}

      {rebaseBase && rebaseCommits.length ? (
        <RebaseDialog
          base={rebaseBase}
          commits={rebaseCommits}
          onClose={() => setRebaseBase(null)}
          onSubmit={(steps) => startRebase(rebaseBase, steps)}
        />
      ) : null}

      {newBranchModal ? (
        <NewBranchModal
          initialSource={newBranchModal.source}
          branches={branches}
          onClose={() => setNewBranchModal(null)}
          onCreate={createNewBranch}
        />
      ) : null}

      {menu ? <ContextMenu {...menu} onClose={closeMenu} /> : null}
      {confirmDialog}
      {promptDialog}
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
function ArtifactsPanel({
  artifacts,
  content,
  selectedArtifact,
  summary,
  onSelect,
}: {
  artifacts: DwArtifact[];
  content: string;
  selectedArtifact: DwArtifact | null;
  summary: ReturnType<typeof artifactCounts>;
  onSelect: (artifact: DwArtifact) => void;
}) {
  const language = selectedArtifact ? artifactLanguage(selectedArtifact.relative_path) : "markdown";

  return (
    <div className="panel-stack">
      <div className="surface-heading">
        <div>
          <h1>.dw artifacts</h1>
          <p>
            Inspect project state, specs, rules, commands, and bugfix records from the selected
            project.
          </p>
        </div>
        <div className="count-strip compact" aria-label="Artifact counts">
          <span>{summary.spec} specs</span>
          <span>{summary.command} commands</span>
          <span>{summary.rule} rules</span>
        </div>
      </div>

      <div className="artifact-layout">
        <section className="artifact-list" aria-label="dev-workflow artifacts">
          {artifacts.length ? (
            artifacts.map((artifact) => (
              <button
                key={artifact.relative_path}
                className={
                  selectedArtifact?.relative_path === artifact.relative_path
                    ? "artifact-row active"
                    : "artifact-row"
                }
                type="button"
                onClick={() => onSelect(artifact)}
              >
                <FileText aria-hidden="true" size={16} />
                <span>{artifact.relative_path}</span>
                <small>
                  {artifact.category} · {formatArtifactSize(artifact.bytes)}
                </small>
              </button>
            ))
          ) : (
            <div className="empty-note">No .dw artifacts found for this project.</div>
          )}
        </section>

        <section className="artifact-viewer">
          <div className="artifact-meta">
            <strong>{selectedArtifact?.relative_path ?? "No artifact selected"}</strong>
            <span>
              {selectedArtifact
                ? formatArtifactSize(selectedArtifact.bytes)
                : "Refresh a project first"}
            </span>
          </div>
          <CodeMirror
            value={content || "Select an artifact to inspect its contents."}
            height="620px"
            extensions={language === "json" ? [json()] : [markdown()]}
            editable={false}
            basicSetup={{ lineNumbers: true, foldGutter: true }}
          />
        </section>
      </div>
    </div>
  );
}

function DiffView({
  patch,
  language,
  showHunkActions = false,
  hunkActionLabel,
  onHunkAction,
  onHunkDiscard,
  emptyLabel = "Selecione um arquivo.",
}: {
  patch: FilePatch | null;
  language: SourceLanguage;
  showHunkActions?: boolean;
  hunkActionLabel?: string;
  onHunkAction?: (hunk: PatchHunk) => void;
  onHunkDiscard?: (hunk: PatchHunk) => void;
  emptyLabel?: string;
}) {
  if (!patch || (!patch.patch && patch.hunks.length === 0)) {
    return <div className="diff-empty">{emptyLabel}</div>;
  }
  const blocks = parsePatchToBlocks(patch);
  return (
    <div className="diff-view">
      {blocks.map((block, index) => (
        <div className="diff-block" key={block.hunk?.id ?? index}>
          {block.rows.map((row, rowIndex) => {
            if (row.type === "hunk") {
              return (
                <div className="diff-line hunk" key={`h-${rowIndex}`}>
                  <span className="diff-gutter" aria-hidden="true" />
                  <span className="diff-gutter" aria-hidden="true" />
                  <code className="diff-hunk-head">{row.content}</code>
                  {block.hunk && (showHunkActions || onHunkDiscard) ? (
                    <span className="diff-hunk-actions">
                      {showHunkActions && onHunkAction ? (
                        <button
                          className="ghost-button"
                          type="button"
                          onClick={() => onHunkAction(block.hunk as PatchHunk)}
                        >
                          {hunkActionLabel ?? "Stage hunk"}
                        </button>
                      ) : null}
                      {onHunkDiscard ? (
                        <button
                          className="ghost-button danger"
                          type="button"
                          onClick={() => onHunkDiscard(block.hunk as PatchHunk)}
                        >
                          Descartar
                        </button>
                      ) : null}
                    </span>
                  ) : null}
                </div>
              );
            }
            return (
              <div className={`diff-line ${row.type}`} key={rowIndex}>
                <span className="diff-gutter">{row.oldNo ?? ""}</span>
                <span className="diff-gutter">{row.newNo ?? ""}</span>
                <span className="diff-mark" aria-hidden="true">
                  {row.type === "add" ? "+" : row.type === "del" ? "-" : " "}
                </span>
                <code className="diff-content">
                  {row.type === "meta"
                    ? row.content
                    : tokenize(row.content, language).map((token, tokenIndex) =>
                        token.cls ? (
                          <span className={`tok-${token.cls}`} key={tokenIndex}>
                            {token.text}
                          </span>
                        ) : (
                          <span key={tokenIndex}>{token.text}</span>
                        ),
                      )}
                </code>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}

function DiffPanel({
  changedFiles,
  diffBusy,
  importedPatch,
  onApplyPatch,
  onCheckPatch,
  onLoadAllUntracked,
  onPatchChange,
  onRefresh,
  onRejectPatch,
  onSelect,
  onToggleFile,
  onToggleHunk,
  onDiscardFile,
  onDiscardHunk,
  onStageAll,
  onUnstageAll,
  onContextFile,
  patchBusy,
  patchCheck,
  patchCounts,
  refreshState,
  selectedFile,
  selectedPatch,
  untrackedTruncated,
  worktreeCounts,
  listWidth,
  onListResize,
  commitSlot,
}: {
  changedFiles: ChangedFile[];
  diffBusy: boolean;
  importedPatch: string;
  onApplyPatch: () => void;
  onCheckPatch: () => void;
  onLoadAllUntracked?: () => void;
  onPatchChange: (value: string) => void;
  onRefresh: () => void;
  onRejectPatch: () => void;
  onSelect: (file: ChangedFile) => void;
  onToggleFile: (file: ChangedFile) => void;
  onToggleHunk: (file: ChangedFile, hunk: PatchHunk) => void;
  onDiscardFile?: (file: ChangedFile) => void;
  onDiscardHunk?: (file: ChangedFile, hunk: PatchHunk) => void;
  onStageAll?: () => void;
  onUnstageAll?: () => void;
  onContextFile?: (file: ChangedFile, event: ReactMouseEvent) => void;
  patchBusy: boolean;
  patchCheck: PatchCheckResult | null;
  patchCounts: { staged: number; unstaged: number };
  refreshState?: LocalGitRefreshState;
  selectedFile: ChangedFile | null;
  selectedPatch: FilePatch | null;
  untrackedTruncated?: boolean;
  worktreeCounts?: WorktreeCounts | null;
  listWidth: number;
  onListResize: (width: number) => void;
  commitSlot?: ReactNode;
}) {
  const groups = groupChangedFiles(changedFiles);
  const selectedCanStageHunks = canStageHunks(selectedFile);
  const layoutRef = useRef<HTMLDivElement>(null);
  const counts = worktreeCounts ?? {
    staged: patchCounts.staged,
    unstaged: patchCounts.unstaged,
    untracked: groups.unstaged.filter((file) => file.status === "??").length,
    conflicts: 0,
    total: changedFiles.length,
  };
  const refreshLabel = localGitRefreshLabel(refreshState ?? "idle");
  const refreshBusy = refreshState === "checking" || refreshState === "loading";
  const totalAdditions = changedFiles.reduce((sum, file) => sum + (file.additions ?? 0), 0);
  const totalDeletions = changedFiles.reduce((sum, file) => sum + (file.deletions ?? 0), 0);

  // Drag the file-list/diff splitter (mutates the CSS var directly during drag).
  function startListResize(event: React.MouseEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = listWidth;
    let latest = startWidth;
    const onMove = (move: MouseEvent) => {
      latest = clampNumberPreference(
        startWidth + (move.clientX - startX),
        startWidth,
        PANE_WIDTH_BOUNDS.patchList,
      );
      layoutRef.current?.style.setProperty("--patch-list-w", `${latest}px`);
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      onListResize(latest);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
  }

  return (
    <div className="diff-panel-stack">
      <div className="diff-header">
        <span className="diff-title">Changes</span>
        <div className="diff-stats">
          <span className="diff-stat added">+{totalAdditions}</span>
          <span className="diff-stat removed">-{totalDeletions}</span>
          <span>{counts.total} files</span>
        </div>
        <span className={`git-local-refresh ${refreshState ?? "idle"}`} aria-live="polite">
          {refreshBusy ? <span className="flow-run-spinner" aria-hidden="true" /> : null}
          {refreshLabel}
        </span>
        <button className="topbar-btn" type="button" onClick={onRefresh} disabled={diffBusy}>
          <RefreshCw aria-hidden="true" size={14} />
          Refresh
        </button>
      </div>

      <section className="diff-files" aria-label="Changed files">
        {changedFiles.length ? (
          changedFiles.map((file) => {
            const active = selectedFile?.path === file.path && selectedFile.area === file.area;
            const { Icon, color } = fileIcon(file.path);
            return (
              <button
                key={`${file.area}:${file.path}`}
                className={active ? "diff-file active" : "diff-file"}
                type="button"
                onClick={() => onSelect(file)}
                onContextMenu={(event) => onContextFile?.(file, event)}
                onDoubleClick={() => onToggleFile(file)}
              >
                <span
                  className={file.area === "staged" ? "diff-file-check checked" : "diff-file-check"}
                  aria-hidden="true"
                >
                  {file.area === "staged" ? "✓" : ""}
                </span>
                <Icon
                  aria-hidden="true"
                  className={`diff-file-icon s-${file.status[0]}`}
                  size={16}
                  style={{ color }}
                />
                <span className="diff-file-name">{file.path}</span>
                <span className="diff-file-stats">
                  <span style={{ color: "var(--clia-green)" }}>+{file.additions ?? 0}</span>
                  <span style={{ color: "var(--clia-danger)" }}>-{file.deletions ?? 0}</span>
                </span>
              </button>
            );
          })
        ) : (
          <div className="empty-note">No changed files.</div>
        )}
          {untrackedTruncated && onLoadAllUntracked ? (
            <button
              className="ghost-button git-load-untracked"
              type="button"
              onClick={onLoadAllUntracked}
            >
              Load all untracked ({counts.untracked})
            </button>
          ) : null}
      </section>

      <section className="patch-viewer diff-content">
        <div className="patch-meta">
            <div>
              <strong>{selectedFile?.path ?? "No changed file selected"}</strong>
              <span>
                {selectedFile
                  ? `${patchAreaLabel(selectedFile.area)} · ${selectedFile.status} · ${formatPatchStats(
                      selectedFile,
                    )}`
                  : "Select a changed file to inspect its patch"}
              </span>
            </div>
            {selectedFile ? (
              <div className="diff-file-actions">
                <button
                  className="secondary-button inline-action"
                  type="button"
                  onClick={() => onToggleFile(selectedFile)}
                  disabled={diffBusy}
                >
                  {fileActionLabel(selectedFile)}
                </button>
                {onDiscardFile && selectedFile.area === "unstaged" ? (
                  <button
                    className="secondary-button inline-action danger"
                    type="button"
                    onClick={() => onDiscardFile(selectedFile)}
                    disabled={diffBusy}
                  >
                    Descartar
                  </button>
                ) : null}
              </div>
            ) : null}
          </div>

          <div className="patch-diff-scroll">
            {selectedPatch && (selectedPatch.patch || selectedPatch.hunks.length) ? (
              <DiffView
                patch={selectedPatch}
                language={selectedFile ? sourceLanguage(selectedFile.path) : "plain"}
                showHunkActions={Boolean(selectedFile && selectedCanStageHunks)}
                hunkActionLabel={selectedFile ? hunkActionLabel(selectedFile) : undefined}
                onHunkAction={selectedFile ? (hunk) => onToggleHunk(selectedFile, hunk) : undefined}
                onHunkDiscard={
                  selectedFile && selectedFile.area === "unstaged" && onDiscardHunk
                    ? (hunk) => onDiscardHunk(selectedFile, hunk)
                    : undefined
                }
              />
            ) : (
              <div className="diff-empty">
                {diffBusy
                  ? "Carregando patch..."
                  : "Nenhum patch selecionado. Arquivos staged, untracked ou binários podem precisar de revisão por arquivo."}
              </div>
            )}
          </div>
      </section>
      {commitSlot}

      <details className="imported-patch-details">
        <summary>Importar patch (avançado)</summary>
        <section className="imported-patch-panel" aria-label="Imported patch review">
          <div className="patch-meta">
            <div>
              <h2>Imported patch</h2>
              <span>Paste a unified diff, check it, then explicitly apply or reject it.</span>
            </div>
            <div className="patch-actions">
              <button
                className="secondary-button"
                type="button"
                onClick={onCheckPatch}
                disabled={patchBusy || !importedPatch.trim()}
              >
                Check patch
              </button>
              <button
                className="primary-button"
                type="button"
                onClick={onApplyPatch}
                disabled={patchBusy || !patchCheck?.ok}
              >
                Apply patch
              </button>
              <button
                className="secondary-button"
                type="button"
                onClick={onRejectPatch}
                disabled={patchBusy || !importedPatch}
              >
                Reject patch
              </button>
            </div>
          </div>
          <textarea
            className="patch-input"
            value={importedPatch}
            onChange={(event) => onPatchChange(event.target.value)}
            placeholder="Paste unified diff here."
            spellCheck={false}
          />
          <pre
            className={
              patchCheck?.ok ? "terminal-output patch-check pass" : "terminal-output patch-check"
            }
          >
            {patchCheck
              ? patchCheck.output ||
                (patchCheck.ok ? "Patch applies cleanly." : "Patch check failed.")
              : "Patch check output will appear here."}
          </pre>
        </section>
      </details>
    </div>
  );
}

function ChangedFileNodes({
  nodes,
  depth,
  expanded,
  onToggle,
  onSelect,
  onToggleStage,
  onContextFile,
  selectedFile,
}: {
  nodes: FileTreeNode[];
  depth: number;
  expanded: Set<string>;
  onToggle: (path: string) => void;
  onSelect: (file: ChangedFile) => void;
  onToggleStage: (file: ChangedFile) => void;
  onContextFile?: (file: ChangedFile, event: ReactMouseEvent) => void;
  selectedFile: ChangedFile | null;
}) {
  return (
    <>
      {nodes.map((node) => {
        if (node.file) {
          const file = node.file;
          const active = selectedFile?.path === file.path && selectedFile.area === file.area;
          const { Icon, color } = fileIcon(file.path);
          return (
            <button
              className={active ? "changed-file-row active" : "changed-file-row"}
              key={`${file.area}:${file.path}`}
              type="button"
              style={{ paddingLeft: 8 + depth * 14 }}
              onClick={() => onSelect(file)}
              onContextMenu={(event) => onContextFile?.(file, event)}
              onDoubleClick={() => onToggleStage(file)}
              title={
                file.area === "staged" ? "Duplo clique para unstage" : "Duplo clique para stage"
              }
            >
              <span
                className={file.area === "staged" ? "git-file-check checked" : "git-file-check"}
                aria-hidden="true"
              >
                {file.area === "staged" ? "✓" : ""}
              </span>
              <Icon
                aria-hidden="true"
                className={`diff-file-icon s-${file.status[0]}`}
                size={15}
                style={{ color }}
              />
              <span className={`git-fstatus s-${file.status[0]}`}>{file.status[0]}</span>
              <span className="changed-file-name">{node.segment}</span>
              <small>{formatPatchStats(file)}</small>
            </button>
          );
        }
        const open = expanded.has(node.path);
        return (
          <div key={node.path}>
            <button
              type="button"
              className="changed-file-folder"
              style={{ paddingLeft: 8 + depth * 14 }}
              onClick={() => onToggle(node.path)}
              aria-expanded={open}
            >
              {open ? (
                <ChevronDown aria-hidden="true" size={13} />
              ) : (
                <ChevronRight aria-hidden="true" size={13} />
              )}
              {open ? (
                <FolderOpen aria-hidden="true" size={13} />
              ) : (
                <Folder aria-hidden="true" size={13} />
              )}
              <span className="changed-file-name">{node.segment}</span>
            </button>
            {open ? (
              <ChangedFileNodes
                nodes={node.children}
                depth={depth + 1}
                expanded={expanded}
                onToggle={onToggle}
                onSelect={onSelect}
                onToggleStage={onToggleStage}
                onContextFile={onContextFile}
                selectedFile={selectedFile}
              />
            ) : null}
          </div>
        );
      })}
    </>
  );
}

function ChangedFileGroup({
  files,
  label,
  onSelect,
  onToggleFile,
  onContextFile,
  onStageAll,
  selectedFile,
}: {
  files: ChangedFile[];
  label: string;
  onSelect: (file: ChangedFile) => void;
  onToggleFile: (file: ChangedFile) => void;
  onContextFile?: (file: ChangedFile, event: ReactMouseEvent) => void;
  onStageAll?: () => void;
  selectedFile: ChangedFile | null;
}) {
  const tree = useMemo(() => buildFileTree(files), [files]);
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(fileTreeDirPaths(tree)));
  // Auto-expand folders for any newly-appearing paths (keep user collapses).
  const knownDirs = useRef<Set<string>>(new Set(fileTreeDirPaths(tree)));
  useEffect(() => {
    const dirs = fileTreeDirPaths(tree);
    const added = dirs.filter((dir) => !knownDirs.current.has(dir));
    if (added.length) {
      knownDirs.current = new Set(dirs);
      setExpanded((current) => {
        const next = new Set(current);
        added.forEach((dir) => next.add(dir));
        return next;
      });
    }
  }, [tree]);

  function toggle(path: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }

  const isStaged = label === "Staged";
  return (
    <section className="changed-file-group">
      <div className="changed-file-head">
        <h2>
          {label} <span className="changed-file-count">{files.length}</span>
        </h2>
        {onStageAll && files.length ? (
          <button className="ghost-button" type="button" onClick={onStageAll}>
            {isStaged ? "Unstage all" : "Stage all"}
          </button>
        ) : null}
      </div>
      {files.length ? (
        <div className="changed-file-tree">
          <ChangedFileNodes
            nodes={tree}
            depth={0}
            expanded={expanded}
            onToggle={toggle}
            onSelect={onSelect}
            onToggleStage={onToggleFile}
            onContextFile={onContextFile}
            selectedFile={selectedFile}
          />
        </div>
      ) : (
        <div className="empty-note">No {label.toLowerCase()} files.</div>
      )}
    </section>
  );
}
