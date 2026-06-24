import { listen } from "@tauri-apps/api/event";
import {
  AlertTriangle,
  Check,
  ChevronDown,
  Package,
  Play,
  RefreshCw,
  Search,
  Square,
  Upload,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  activeVersionLabel,
  appendDeployProgress,
  canApproveVersion,
  canDeployVersion,
  canPrepareVersion,
  deployEnvironmentSummary,
  deployEnvironmentValues,
  deployErrorMessage,
  deployFindingPathLabel,
  deployRepairInfo,
  deployReadiness,
  deployStepLabel,
  deployStatusLabel,
  deployStatusTone,
  hasPassedPrepareRun,
  isAutomaticDeployTarget,
  isLegacyDeployPackage,
  latestDeployRunForContext,
  latestVersion,
  parseBlockingFindings,
  parseDeployFindings,
  progressForDeploy,
  retryActionLabel,
  sortDeployStacks,
  type DeployProgressEntry,
} from "./deploy";
import { machineStatusLabel, sortMachines } from "./machines";
import MachinesPanel from "./MachinesPanel";
import { api } from "./tauri";
import type {
  AgentProfile,
  DeployDetectionReport,
  DeployEnvironment,
  DeployPlanReport,
  DeployProgressEvent,
  DeployRun,
  DeployStack,
  DeployStackDetail,
  DeployVersion,
  Project,
  Workspace,
  WorkspaceMachine,
} from "./types";
import { projectDisplayName } from "./workspace";

type DeployNextAction = {
  tone: "neutral" | "warning" | "blocked" | "ready";
  title: string;
  body: string;
  label: string;
  action:
    | "create"
    | "save-environment"
    | "approve"
    | "prepare"
    | "deploy"
    | "repair-version"
    | "refresh"
    | null;
  disabled: boolean;
};

function deployStrategyForVersion(version: DeployVersion | null | undefined) {
  if (!version?.manifest_json) return "unknown";
  try {
    const manifest = JSON.parse(version.manifest_json) as Record<string, unknown>;
    const value = manifest.deploy_strategy;
    return typeof value === "string" && value.trim() ? value : "unknown";
  } catch {
    return "unknown";
  }
}

function deployStrategyLabel(strategy: string) {
  switch (strategy) {
    case "desktop_dev":
      return "Dev VM desktop";
    case "web_service":
      return "Serviço web";
    case "custom_compose":
      return "Compose próprio";
    case "mixed":
      return "Misto";
    case "unsupported":
      return "Não suportado";
    default:
      return "Não identificado";
  }
}

function deployAnalysisForVersion(version: DeployVersion | null | undefined) {
  if (!version?.manifest_json) return null;
  try {
    const manifest = JSON.parse(version.manifest_json) as Record<string, unknown>;
    const analysis = manifest.analysis;
    return analysis && typeof analysis === "object" ? (analysis as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

function deployAnalysisText(analysis: Record<string, unknown> | null, key: string) {
  const value = analysis?.[key];
  return typeof value === "string" && value.trim() ? value : null;
}

function DeploySubnav({
  view,
  onViewChange,
}: {
  view: "deploy" | "machines";
  onViewChange: (view: "deploy" | "machines") => void;
}) {
  return (
    <div className="deploy-subnav" role="tablist" aria-label="Deploy">
      <button
        className={view === "deploy" ? "git-nav active" : "git-nav"}
        type="button"
        role="tab"
        aria-selected={view === "deploy"}
        onClick={() => onViewChange("deploy")}
      >
        Deploy
      </button>
      <button
        className={view === "machines" ? "git-nav active" : "git-nav"}
        type="button"
        role="tab"
        aria-selected={view === "machines"}
        onClick={() => onViewChange("machines")}
      >
        Máquinas
      </button>
    </div>
  );
}

export default function DeployPackagesPanel({
  activeProject,
  confirm,
  workspace,
  projects,
}: {
  activeProject: Project | null;
  confirm: (options: {
    title: string;
    body?: string;
    confirmLabel?: string;
    danger?: boolean;
  }) => Promise<boolean>;
  workspace: Workspace;
  projects: Project[];
}) {
  const [view, setView] = useState<"deploy" | "machines">("deploy");
  const [stacks, setStacks] = useState<DeployStack[]>([]);
  const [detail, setDetail] = useState<DeployStackDetail | null>(null);
  const [machines, setMachines] = useState<WorkspaceMachine[]>([]);
  const [agentProfiles, setAgentProfiles] = useState<AgentProfile[]>([]);
  const [selectedStackId, setSelectedStackId] = useState<string | null>(null);
  const [selectedProjectIds, setSelectedProjectIds] = useState<number[]>(() =>
    projects[0] ? [projects[0].id] : [],
  );
  const [selectedMachineId, setSelectedMachineId] = useState<string>("");
  const [selectedAgentProfileId, setSelectedAgentProfileId] = useState<number | null>(null);
  const [stackName, setStackName] = useState("");
  const [includeDirty, setIncludeDirty] = useState(false);
  const [detection, setDetection] = useState<DeployDetectionReport | null>(null);
  const [planReport, setPlanReport] = useState<DeployPlanReport | null>(null);
  const [artifactPath, setArtifactPath] = useState("manifest.json");
  const [artifact, setArtifact] = useState("");
  const [runs, setRuns] = useState<DeployRun[]>([]);
  const [runLogs, setRunLogs] = useState("");
  const [deployEnvironment, setDeployEnvironment] = useState<DeployEnvironment | null>(null);
  const [environmentDraft, setEnvironmentDraft] = useState<Record<string, string>>({});
  const [environmentSaving, setEnvironmentSaving] = useState(false);
  const [progress, setProgress] = useState<DeployProgressEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [createPackageOpen, setCreatePackageOpen] = useState(false);

  const sortedStacks = useMemo(() => sortDeployStacks(stacks), [stacks]);
  const currentDetail = detail?.stack.id === selectedStackId ? detail : null;
  const versions = currentDetail?.versions ?? [];
  const selectedStack =
    currentDetail?.stack ?? sortedStacks.find((stack) => stack.id === selectedStackId) ?? null;
  const selectedVersion = latestVersion(versions);
  const selectedDeployStrategy = useMemo(
    () => deployStrategyForVersion(selectedVersion),
    [selectedVersion],
  );
  const selectedDeployAnalysis = useMemo(
    () => deployAnalysisForVersion(selectedVersion),
    [selectedVersion],
  );
  const selectedMachine = machines.find((machine) => machine.id === selectedMachineId) ?? null;
  const activeDeployEnvironment =
    deployEnvironment &&
    selectedVersion &&
    deployEnvironment.version_id === selectedVersion.id &&
    deployEnvironment.machine_id === selectedMachineId
      ? deployEnvironment
      : null;
  const latestRun = selectedVersion
    ? latestDeployRunForContext(runs, selectedVersion.id, selectedMachineId || null)
    : null;
  const latestRepair = deployRepairInfo(latestRun);
  const latestRunAgentProfileId = latestRun?.agent_profile_id ?? null;
  const selectedAgentProfileStillExists = Boolean(
    selectedAgentProfileId &&
      agentProfiles.some((profile) => profile.id === selectedAgentProfileId),
  );
  const latestRunAgentProfileStillExists = Boolean(
    latestRunAgentProfileId &&
      agentProfiles.some((profile) => profile.id === latestRunAgentProfileId),
  );
  const effectiveSelectedAgentProfileId = selectedAgentProfileStillExists
    ? selectedAgentProfileId
    : latestRunAgentProfileStillExists
      ? latestRunAgentProfileId
      : agentProfiles.length === 1
        ? agentProfiles[0].id
        : null;
  const selectedAgentProfile =
    agentProfiles.find((profile) => profile.id === effectiveSelectedAgentProfileId) ?? null;
  const visibleProgress = progressForDeploy(progress, {
    stackId: selectedStack?.id,
    versionId: selectedVersion?.id,
    machineId: selectedMachineId || null,
  });
  const deployFindings = parseDeployFindings(selectedVersion);
  const blockingFindings = parseBlockingFindings(selectedVersion);
  const legacyPackage = isLegacyDeployPackage(selectedVersion);
  const defaultStackName =
    projects.length === 1
      ? `${projectDisplayName(projects[0])} deploy`
      : `${workspace.name} deploy`;
  const availableProjectIds = new Set(projects.map((project) => project.id));
  const explicitlyValidProjectIds = selectedProjectIds.filter((id) => availableProjectIds.has(id));
  const validSelectedProjectIds = explicitlyValidProjectIds.length
    ? explicitlyValidProjectIds
    : selectedProjectIds.length
      ? projects[0]
        ? [projects[0].id]
        : []
      : [];
  const readiness = deployReadiness(selectedVersion, selectedMachine, activeDeployEnvironment);
  const agentReady = Boolean(selectedAgentProfile);
  const packagePlanReady = Boolean(
    planReport && planReport.status === "passed" && planReport.deploy_plan_path,
  );
  const deployReady =
    readiness.deployReady &&
    canDeployVersion(selectedVersion, selectedMachine, activeDeployEnvironment) &&
    agentReady;
  const prepareReady =
    readiness.prepareReady &&
    canPrepareVersion(selectedVersion, selectedMachine, activeDeployEnvironment) &&
    agentReady;
  const environmentSummary = deployEnvironmentSummary(activeDeployEnvironment);
  const prepareAlreadyPassed = selectedVersion
    ? hasPassedPrepareRun(runs, selectedVersion.id, selectedMachineId || null)
    : false;
  const latestFailedRun = latestRun?.status === "failed" ? latestRun : null;
  const selectedProjectSummary =
    validSelectedProjectIds.length === 1
      ? projectDisplayName(projects.find((project) => project.id === validSelectedProjectIds[0])!)
      : `${validSelectedProjectIds.length} projetos`;
  let nextAction: DeployNextAction = {
    tone: "neutral",
    title: "Crie o primeiro pacote",
    body: "Escolha os projetos, confirme a VM alvo e gere uma versão para revisão.",
    label: "Criar pacote",
    action: "create",
    disabled: busy || !validSelectedProjectIds.length,
  };
  if (selectedStack && selectedVersion) {
    if (legacyPackage) {
      nextAction = {
        tone: "blocked",
        title: "Crie uma nova versão do pacote",
        body: "Esta versão foi criada antes da validação atual de secrets. Gere outra versão para usar o fluxo corrigido.",
        label: "Criar nova versão",
        action: "create",
        disabled: busy || !validSelectedProjectIds.length,
      };
    } else if (blockingFindings.length) {
      nextAction = {
        tone: "blocked",
        title: "Resolva os bloqueios do review",
        body: `O pacote tem ${blockingFindings.length} bloqueio de review. Corrija a causa e gere uma nova versão; os caminhos ficam em detalhes.`,
        label: "Criar nova versão",
        action: "create",
        disabled: busy || !validSelectedProjectIds.length,
      };
    } else if (!readiness.environmentReady) {
      nextAction = activeDeployEnvironment?.variables.length
        ? {
            tone: "warning",
            title: "Preencha as variáveis do ambiente",
            body: "Os valores ficam locais nesta máquina. Salve o ambiente antes de aprovar e preparar o target.",
            label: "Salvar ambiente",
            action: "save-environment",
            disabled: environmentSaving || !selectedMachineId,
          }
        : {
            tone: "warning",
            title: "Carregue o ambiente do pacote",
            body: "O pacote ainda não terminou de carregar as informações de ambiente para esta VM.",
            label: "Atualizar",
            action: "refresh",
            disabled: busy,
          };
    } else if (!readiness.packageApproved) {
      nextAction = {
        tone: "ready",
        title: "Aprove o pacote",
        body: "Review limpo e ambiente pronto. A aprovação libera o preparo da VM e o deploy.",
        label: "Aprovar pacote",
        action: "approve",
        disabled: busy || !canApproveVersion(selectedVersion),
      };
    } else if (!readiness.targetReady) {
      nextAction = {
        tone: "warning",
        title:
          selectedMachine && !isAutomaticDeployTarget(selectedMachine)
            ? "Escolha uma VM de deploy"
            : "Inicie a VM alvo",
        body:
          selectedMachine && !isAutomaticDeployTarget(selectedMachine)
            ? "VMs manuais não entram no deploy assistido V2. Selecione Ubuntu Server Deploy VM, Ubuntu Desktop Deploy VM ou Windows 11."
            : "O target precisa estar running no WinBox antes do preparo e do deploy.",
        label: "Atualizar status",
        action: "refresh",
        disabled: busy,
      };
    } else if (latestFailedRun) {
      if (latestRepair.patchPending) {
        nextAction = {
          tone: "warning",
          title: "Agente propôs correção",
          body:
            latestRepair.userMessage ??
            latestRepair.patchSummary ??
            "Crie uma nova versão do pacote com o patch proposto, revise e aprove antes de tentar novamente.",
          label: "Criar versão corrigida",
          action: "repair-version",
          disabled: busy,
        };
      } else {
        const retryLabel = retryActionLabel(latestFailedRun) ?? "Tentar novamente";
        nextAction = {
          tone: "blocked",
          title:
            latestFailedRun.operation === "deploy"
              ? "Deploy falhou"
              : latestFailedRun.operation === "prepare"
                ? "Prepare falhou"
                : "Operação falhou",
          body:
            latestRepair.latestTitle && latestRepair.attemptCount
              ? `${latestRepair.latestTitle}: ${
                  latestRepair.latestMessage
                    ? deployErrorMessage(latestRepair.latestMessage)
                    : deployErrorMessage(latestFailedRun.summary || "A última operação falhou.")
                }`
              : deployErrorMessage(latestFailedRun.summary || "A última operação falhou."),
          label: retryLabel,
          action:
            latestFailedRun.operation === "deploy"
              ? "deploy"
              : latestFailedRun.operation === "prepare"
                ? "prepare"
                : "refresh",
          disabled: busy || (latestFailedRun.operation === "deploy" ? !deployReady : !prepareReady),
        };
      }
    } else if (!prepareAlreadyPassed) {
      nextAction = {
        tone: "ready",
        title: agentReady ? "Prepare a VM alvo" : "Escolha um agente",
        body: agentReady
          ? selectedDeployStrategy === "desktop_dev"
            ? "O orquestrador valida o plano desktop_dev e prepara a VM para instalar dependências do projeto."
            : "O orquestrador valida o plano do pacote e deixa a VM pronta para receber a stack."
          : "Selecione o agente que planejou este deploy para orquestrar a execução.",
        label: "Preparar target",
        action: "prepare",
        disabled: busy || !prepareReady,
      };
    } else {
      nextAction = {
        tone: "ready",
        title: agentReady ? "Execute o deploy" : "Escolha um agente",
        body: agentReady
          ? selectedDeployStrategy === "desktop_dev"
            ? "O runbook instala e verifica o pacote dev na VM, sem criar um container web falso."
            : "O runbook executa o plano, valida a execução e coleta logs redigidos."
          : "Selecione o agente que planejou este deploy para orquestrar a execução.",
        label: "Deploy",
        action: "deploy",
        disabled: busy || !deployReady,
      };
    }
  }

  const loadDetail = useCallback(async (stackId: string) => {
    const result = await api.getDeployStack(stackId);
    if (result.ok) setDetail(result.value);
    else setError(result.error);
  }, []);

  const reload = useCallback(async () => {
    setBusy(true);
    setError("");
    const [stackResult, machineResult, agentResult] = await Promise.all([
      api.listDeployStacks(workspace.id),
      api.listWorkspaceMachines(workspace.id),
      api.listAgentProfiles(workspace.id, null),
    ]);
    if (stackResult.ok) {
      const sorted = sortDeployStacks(stackResult.value);
      setStacks(sorted);
      setSelectedStackId((current) => {
        if (current && sorted.some((stack) => stack.id === current)) return current;
        return sorted[0]?.id ?? null;
      });
      if (!sorted.length) {
        setDetail(null);
        setRuns([]);
        setArtifact("");
        setDeployEnvironment(null);
        setEnvironmentDraft({});
      }
    } else {
      setError(stackResult.error);
    }
    if (machineResult.ok) {
      const runningFirst = sortMachines(machineResult.value);
      setMachines(runningFirst);
      setSelectedMachineId((current) => {
        if (current && runningFirst.some((machine) => machine.id === current)) return current;
        const preferred =
          runningFirst.find(
            (machine) => machine.status === "running" && isAutomaticDeployTarget(machine),
          ) ?? runningFirst.find(isAutomaticDeployTarget);
        return (
          preferred?.id ??
          runningFirst.find((machine) => machine.status === "running")?.id ??
          runningFirst[0]?.id ??
          ""
        );
      });
    } else {
      setError(machineResult.error);
    }
    if (agentResult.ok) {
      setAgentProfiles(agentResult.value);
      setSelectedAgentProfileId((current) => {
        if (current && agentResult.value.some((profile) => profile.id === current)) return current;
        return null;
      });
    } else {
      setError(agentResult.error);
    }
    setBusy(false);
  }, [workspace.id]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      void reload();
    }, 0);
    return () => window.clearTimeout(timeout);
  }, [reload]);

  useEffect(() => {
    if (!selectedStackId) {
      return;
    }
    let cancelled = false;
    void api.getDeployStack(selectedStackId).then((result) => {
      if (cancelled) return;
      if (result.ok) setDetail(result.value);
      else {
        setDetail(null);
        setRuns([]);
        setArtifact("");
        setDeployEnvironment(null);
        setEnvironmentDraft({});
        setSelectedStackId(null);
        setError(result.error);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [selectedStackId]);

  useEffect(() => {
    if (!selectedVersion) {
      return;
    }
    let cancelled = false;
    void Promise.all([
      api.listDeployRuns(selectedVersion.id),
      api.readDeployArtifact(selectedVersion.id, artifactPath),
    ]).then(([runsResult, artifactResult]) => {
      if (cancelled) return;
      if (runsResult.ok) setRuns(runsResult.value);
      else setError(runsResult.error);
      if (artifactResult.ok) setArtifact(artifactResult.value);
      else setArtifact(artifactResult.error);
    });
    return () => {
      cancelled = true;
    };
  }, [artifactPath, selectedVersion]);

  useEffect(() => {
    if (!selectedVersion || !selectedMachineId) {
      return;
    }
    let cancelled = false;
    void api.getDeployEnvironment(selectedVersion.id, selectedMachineId).then((result) => {
      if (cancelled) return;
      if (result.ok) {
        setDeployEnvironment(result.value);
        setEnvironmentDraft(
          Object.fromEntries(
            deployEnvironmentValues(result.value).map((item) => [item.key, item.value]),
          ),
        );
      } else {
        setError(deployErrorMessage(result.error));
      }
    });
    return () => {
      cancelled = true;
    };
  }, [selectedMachineId, selectedVersion]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void listen<DeployProgressEvent>("deploy://progress", (event) => {
      if (disposed) return;
      setProgress((current) => appendDeployProgress(current, event.payload));
    }).then((value) => {
      if (disposed) value();
      else unlisten = value;
    });
    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, []);

  function toggleProject(projectId: number) {
    setPlanReport(null);
    setDetection(null);
    setSelectedProjectIds((current) =>
      current.filter((id) => availableProjectIds.has(id)).includes(projectId)
        ? current.filter((id) => id !== projectId && availableProjectIds.has(id))
        : [...current.filter((id) => availableProjectIds.has(id)), projectId],
    );
  }

  async function planPackage() {
    if (!validSelectedProjectIds.length) {
      setError("Selecione pelo menos um projeto.");
      return;
    }
    if (!selectedAgentProfile) {
      setError(deployErrorMessage("deploy_agent_required"));
      return;
    }
    setBusy(true);
    setError("");
    setPlanReport(null);
    const result = await api.planDeployPackage(
      workspace.id,
      validSelectedProjectIds,
      selectedMachineId || null,
      selectedAgentProfile.id,
      includeDirty,
    );
    if (result.ok) {
      setPlanReport(result.value);
      try {
        const parsed = JSON.parse(result.value.project_context_json) as Record<string, unknown>;
        const detector =
          parsed.detector && typeof parsed.detector === "object"
            ? (parsed.detector as Partial<DeployDetectionReport>)
            : {};
        const contextProjects = Array.isArray(parsed.projects)
          ? parsed.projects.map((item) => item as Record<string, unknown>)
          : [];
        setDetection({
          workspace_id: workspace.id,
          services: detector.services ?? [],
          ports: detector.ports ?? [],
          warnings: detector.warnings ?? [],
          projects: contextProjects.map((project) => ({
            project_id: Number(project.project_id ?? 0),
            name: String(project.name ?? "project"),
            path: String(project.path ?? ""),
            language: String(project.language ?? "unknown"),
            framework: typeof project.framework === "string" ? project.framework : null,
            package_manager:
              typeof project.package_manager === "string" ? project.package_manager : null,
            has_dockerfile: Boolean(project.has_dockerfile),
            has_compose: Boolean(project.has_compose),
            services: [],
            ports: [],
            healthcheck: null,
            deploy_strategy: String(project.deploy_strategy ?? "unsupported"),
            strategy_reason: String(project.strategy_reason ?? ""),
            runtime_commands: Array.isArray(project.runtime_commands)
              ? project.runtime_commands.map(String)
              : [],
            requires_desktop_session: Boolean(project.requires_desktop_session),
            warnings: Array.isArray(project.warnings) ? project.warnings.map(String) : [],
          })),
        });
      } catch {
        // The plan report is still usable without detector preview.
      }
      if (result.value.status !== "passed") {
        setError(deployErrorMessage("deploy_plan_validation_failed"));
      }
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function createPackage() {
    if (!validSelectedProjectIds.length) {
      setError("Selecione pelo menos um projeto.");
      return;
    }
    if (!selectedAgentProfile) {
      setError(deployErrorMessage("deploy_agent_required"));
      return;
    }
    if (!packagePlanReady) {
      setError(deployErrorMessage("deploy_plan_required"));
      return;
    }
    const activePlanReport = planReport;
    if (!activePlanReport?.deploy_plan_path) {
      setError(deployErrorMessage("deploy_plan_required"));
      return;
    }
    setBusy(true);
    setError("");
    const result = await api.createDeployPackage({
      workspace_id: workspace.id,
      stack_name: stackName.trim() || defaultStackName,
      project_ids: validSelectedProjectIds,
      target_machine_id: selectedMachineId || null,
      agent_profile_id: selectedAgentProfile.id,
      deploy_plan_path: activePlanReport.deploy_plan_path,
      include_dirty: includeDirty,
    });
    if (result.ok) {
      setArtifactPath("manifest.json");
      setSelectedStackId(result.value.stack_id);
      setCreatePackageOpen(false);
      await reload();
      await loadDetail(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function approveVersion() {
    if (!selectedVersion) return;
    setBusy(true);
    setError("");
    const result = await api.approveDeployVersion(selectedVersion.id);
    if (result.ok) {
      await reload();
      await loadDetail(result.value.stack_id);
      setSelectedStackId(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function saveEnvironment() {
    if (!selectedVersion || !selectedMachineId) return;
    setEnvironmentSaving(true);
    setError("");
    const variables =
      activeDeployEnvironment?.variables.map((variable) => ({
        key: variable.key,
        value: environmentDraft[variable.key] ?? "",
      })) ?? [];
    const result = await api.saveDeployEnvironment(
      selectedVersion.id,
      selectedMachineId,
      variables,
    );
    if (result.ok) {
      setDeployEnvironment(result.value);
      setEnvironmentDraft(
        Object.fromEntries(
          deployEnvironmentValues(result.value).map((item) => [item.key, item.value]),
        ),
      );
    } else {
      setError(deployErrorMessage(result.error));
    }
    setEnvironmentSaving(false);
  }

  async function prepareTarget() {
    if (!selectedVersion || !selectedMachineId || !selectedAgentProfile) {
      setError(deployErrorMessage("deploy_agent_required"));
      return;
    }
    setBusy(true);
    setError("");
    const result = await api.prepareDeployTarget(
      selectedVersion.id,
      selectedMachineId,
      selectedAgentProfile.id,
    );
    if (result.ok) {
      const logs = await api.getDeployRunLogs(result.value.id);
      if (logs.ok) setRunLogs(logs.value);
      await reload();
      await loadDetail(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function deploySelected() {
    if (!selectedVersion || !selectedMachineId || !selectedAgentProfile) {
      setError(deployErrorMessage("deploy_agent_required"));
      return;
    }
    setBusy(true);
    setError("");
    const result = await api.deployVersion(
      selectedVersion.id,
      selectedMachineId,
      selectedAgentProfile.id,
    );
    if (result.ok) {
      const logs = await api.getDeployRunLogs(result.value.id);
      if (logs.ok) setRunLogs(logs.value);
      await reload();
      await loadDetail(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function createRepairVersion() {
    if (!latestRun) {
      return;
    }
    setBusy(true);
    setError("");
    const result = await api.createDeployRepairVersion(latestRun.id);
    if (result.ok) {
      setArtifactPath("RUNBOOK.md");
      setSelectedStackId(result.value.stack_id);
      await reload();
      await loadDetail(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function stopSelected() {
    if (!selectedStack || !selectedMachineId) return;
    setBusy(true);
    setError("");
    const result = await api.stopDeployStack(selectedStack.id, selectedMachineId);
    if (result.ok) {
      const logs = await api.getDeployRunLogs(result.value.id);
      if (logs.ok) setRunLogs(logs.value);
      await reload();
      await loadDetail(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function reactivateVersion(version: DeployVersion) {
    if (!selectedMachineId || !selectedAgentProfile) {
      setError(deployErrorMessage("deploy_agent_required"));
      return;
    }
    setBusy(true);
    setError("");
    const result = await api.reactivateDeployVersion(
      version.id,
      selectedMachineId,
      selectedAgentProfile.id,
    );
    if (result.ok) {
      const logs = await api.getDeployRunLogs(result.value.id);
      if (logs.ok) setRunLogs(logs.value);
      await reload();
      await loadDetail(result.value.stack_id);
    } else {
      setError(deployErrorMessage(result.error));
    }
    setBusy(false);
  }

  async function loadRunLogs(run: DeployRun) {
    setBusy(true);
    const result = await api.getDeployRunLogs(run.id);
    if (result.ok) setRunLogs(result.value);
    else setError(deployErrorMessage(result.error));
    setBusy(false);
  }

  async function runNextAction() {
    switch (nextAction.action) {
      case "create":
        openCreatePackageModal();
        break;
      case "save-environment":
        await saveEnvironment();
        break;
      case "approve":
        await approveVersion();
        break;
      case "prepare":
        await prepareTarget();
        break;
      case "deploy":
        await deploySelected();
        break;
      case "repair-version":
        await createRepairVersion();
        break;
      case "refresh":
        await reload();
        break;
      case null:
        break;
    }
  }

  function openCreatePackageModal() {
    setDetection(null);
    setCreatePackageOpen(true);
  }

  function renderCreatePackageForm() {
    return (
      <form
        className="deploy-create-form"
        onSubmit={(event) => {
          event.preventDefault();
          void createPackage();
        }}
      >
        <label>
          <span>Nome da stack</span>
          <input
            value={stackName}
            placeholder={defaultStackName}
            onChange={(event) => setStackName(event.target.value)}
          />
        </label>
        <label>
          <span>Target</span>
          <select
            value={selectedMachineId}
            onChange={(event) => {
              setSelectedMachineId(event.target.value);
              setPlanReport(null);
            }}
          >
            <option value="">Selecione uma VM</option>
            {machines.map((machine) => (
              <option key={machine.id} value={machine.id}>
                {machine.display_name} · {machineStatusLabel(machine.status)}
                {isAutomaticDeployTarget(machine) ? " · deploy V2" : " · sem deploy V2"}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Agente do planejamento</span>
          <select
            value={effectiveSelectedAgentProfileId ?? ""}
            onChange={(event) => {
              setSelectedAgentProfileId(event.target.value ? Number(event.target.value) : null);
              setPlanReport(null);
            }}
          >
            <option value="">Selecione um agente</option>
            {agentProfiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name} · {profile.provider}
                {profile.model ? ` · ${profile.model}` : ""}
              </option>
            ))}
          </select>
        </label>
        <div className="deploy-project-picker">
          {projects.map((project) => (
            <label key={project.id}>
              <input
                type="checkbox"
                checked={validSelectedProjectIds.includes(project.id)}
                onChange={() => toggleProject(project.id)}
              />
              <span>{projectDisplayName(project)}</span>
            </label>
          ))}
        </div>
        <label className="deploy-check-row">
          <input
            type="checkbox"
            checked={includeDirty}
            onChange={(event) => {
              setIncludeDirty(event.target.checked);
              setPlanReport(null);
            }}
          />
          <span>Incluir snapshot sujo</span>
        </label>
        {planReport ? (
          <section
            className={
              planReport.status === "passed"
                ? "deploy-detection deploy-analysis passed"
                : "deploy-detection deploy-analysis blocked"
            }
          >
            <strong>
              Plano do agente · {planReport.status} · {planReport.confidence}
            </strong>
            <p>{planReport.summary}</p>
            {planReport.agent_session_id ? <p>Sessão do agente #{planReport.agent_session_id}</p> : null}
            {planReport.validation_errors.map((validationError) => (
              <p key={validationError} className="deploy-warning">
                {validationError}
              </p>
            ))}
            {planReport.warnings.map((warning) => (
              <p key={warning} className="deploy-warning">
                {warning}
              </p>
            ))}
          </section>
        ) : null}
        <footer className="modal-actions">
          <button
            className="secondary-button"
            type="button"
            onClick={() => void planPackage()}
            disabled={busy || !validSelectedProjectIds.length || !selectedAgentProfile}
          >
            <Search aria-hidden="true" size={15} /> Analisar
          </button>
          <button
            className="primary-button"
            type="submit"
            disabled={busy || !validSelectedProjectIds.length || !selectedAgentProfile || !packagePlanReady}
          >
            <Package aria-hidden="true" size={15} /> Criar pacote
          </button>
        </footer>
      </form>
    );
  }

  function renderNextActionIcon() {
    if (nextAction.action === "deploy") return <Play aria-hidden="true" size={16} />;
    if (nextAction.action === "prepare") return <Upload aria-hidden="true" size={16} />;
    if (nextAction.action === "approve" || nextAction.action === "save-environment") {
      return <Check aria-hidden="true" size={16} />;
    }
    if (nextAction.action === "refresh") return <RefreshCw aria-hidden="true" size={16} />;
    if (nextAction.tone === "blocked") return <AlertTriangle aria-hidden="true" size={16} />;
    return <Package aria-hidden="true" size={16} />;
  }

  function renderNextActionCard() {
    return (
      <section className={`deploy-next-action ${nextAction.tone}`} aria-live="polite">
        <div>
          <span className="deploy-next-kicker">Próxima ação</span>
          <h3>{nextAction.title}</h3>
          <p>{nextAction.body}</p>
        </div>
        {nextAction.action ? (
          <button
            className={nextAction.tone === "blocked" ? "secondary-button" : "primary-button"}
            type="button"
            onClick={() => void runNextAction()}
            disabled={nextAction.disabled}
          >
            {renderNextActionIcon()} {busy ? "Executando..." : nextAction.label}
          </button>
        ) : null}
      </section>
    );
  }

  if (view === "machines") {
    return (
      <section className="deploy-combined-panel">
        <DeploySubnav view={view} onViewChange={setView} />
        <MachinesPanel activeProject={activeProject} confirm={confirm} workspace={workspace} />
      </section>
    );
  }

  return (
    <div className="deploy-panel">
      <DeploySubnav view={view} onViewChange={setView} />
      <header className="deploy-header">
        <div>
          <p className="eyebrow">Workspace deploy packages</p>
          <h1>Deploy guiado</h1>
          <p>Crie o pacote, configure o ambiente local e publique em uma VM WinBox.</p>
        </div>
        <div className="machines-actions">
          <button className="secondary-button" type="button" onClick={() => void reload()}>
            <RefreshCw aria-hidden="true" size={16} /> Refresh
          </button>
          {sortedStacks.length ? (
            <button className="primary-button" type="button" onClick={openCreatePackageModal}>
              <Package aria-hidden="true" size={16} /> Criar pacote
            </button>
          ) : null}
        </div>
      </header>

      {error ? <div className="git-error">{error}</div> : null}

      {createPackageOpen ? (
        <div className="modal-backdrop elevated" role="presentation">
          <section
            className="modal-panel deploy-create-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="deploy-create-title"
          >
            <header className="modal-heading">
              <div>
                <h2 id="deploy-create-title">Novo pacote</h2>
                <p>Selecione projetos, target e agente; analise o plano antes de gerar a versão.</p>
              </div>
              <button
                className="secondary-button icon-button"
                type="button"
                onClick={() => setCreatePackageOpen(false)}
                aria-label="Fechar modal"
              >
                <X aria-hidden="true" size={16} />
              </button>
            </header>
            {renderCreatePackageForm()}
            {detection ? (
              <section className="deploy-detection">
                <strong>{detection.projects.length} projetos detectados</strong>
                {detection.projects.map((project) => (
                  <p key={project.project_id}>
                    {project.name}: {project.language}
                    {project.framework ? ` · ${project.framework}` : ""}
                    {project.deploy_strategy ? ` · ${deployStrategyLabel(project.deploy_strategy)}` : ""}
                    {project.ports.length
                      ? ` · :${project.ports.map((port) => port.host).join(", :")}`
                      : ""}
                  </p>
                ))}
                {detection.warnings.map((warning) => (
                  <p key={warning} className="deploy-warning">
                    {warning}
                  </p>
                ))}
              </section>
            ) : null}
          </section>
        </div>
      ) : null}

      <div className="deploy-shell">
        <aside className="deploy-setup" aria-label="Deploys do workspace">
          <section className="deploy-stack-picker" aria-label="Stacks de deploy">
            <div className="machines-list-head">
              <strong>{stacks.length} deploys</strong>
              {busy ? <span className="status-pill pending">loading</span> : null}
            </div>
            {!sortedStacks.length ? (
              <div className="terminal-empty">
                <span>Nenhum deploy criado.</span>
              </div>
            ) : (
              <div className="machines-table">
                {sortedStacks.map((stack) => (
                  <button
                    key={stack.id}
                    className={
                      selectedStack?.id === stack.id
                        ? "machine-row deploy-stack-card active"
                        : "machine-row deploy-stack-card"
                    }
                    type="button"
                    onClick={() => setSelectedStackId(stack.id)}
                  >
                    <span className="deploy-stack-main">
                      <strong>{stack.name}</strong>
                      <small>{activeVersionLabel(stack, versions)}</small>
                    </span>
                    <span className={`machine-status ${deployStatusTone(stack.status)}`}>
                      {deployStatusLabel(stack.status)}
                    </span>
                    <span className="deploy-stack-activity">
                      {stack.active_machine_id ? "active" : "idle"}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </section>
        </aside>

        <main className="deploy-wizard" aria-label="Fluxo guiado de deploy">
          {selectedStack && selectedVersion ? (
            <>
              <div className="deploy-wizard-head">
                <div>
                  <h2>{selectedStack.name}</h2>
                  <p>
                    {selectedVersion.label} · {selectedMachine?.display_name ?? "sem target"}
                  </p>
                </div>
                <span className={`machine-status ${deployStatusTone(selectedVersion.status)}`}>
                  {deployStatusLabel(selectedVersion.status)}
                </span>
              </div>

              {renderNextActionCard()}

              {latestRepair.attemptCount || latestRepair.patchPending ? (
                <section className="deploy-wizard-step compact">
                  <div className="machines-list-head">
                    <strong>Deploy Doctor</strong>
                    <span>
                      {latestRepair.attemptCount}/{latestRepair.maxAttempts} tentativas
                    </span>
                  </div>
                  <div className="deploy-summary-grid">
                    <span>
                      <small>Status</small>
                      <strong>{latestRepair.status}</strong>
                    </span>
                    <span>
                      <small>Diagnóstico</small>
                      <strong>{latestRepair.latestTitle ?? "Sem diagnóstico"}</strong>
                    </span>
                    <span>
                      <small>Erro</small>
                      <strong>{latestRepair.latestCode ?? "-"}</strong>
                    </span>
                    <span>
                      <small>Agente</small>
                      <strong>
                        {latestRepair.agentStatus ?? "sem patch"}
                        {latestRepair.adeSafeToApply ? " · validado ADE" : ""}
                      </strong>
                    </span>
                  </div>
                  {latestRepair.latestMessage ? (
                    <div className="deploy-repair-callout">
                      <p>{latestRepair.latestMessage}</p>
                    </div>
                  ) : null}
                  {latestRepair.patchPending ? (
                    <div className="deploy-repair-callout">
                      <p>
                        {latestRepair.userMessage ??
                          latestRepair.patchSummary ??
                          "O agente propôs uma correção de pacote."}
                      </p>
                      <small>
                        Arquivos:{" "}
                        {latestRepair.patches.length
                          ? latestRepair.patches.join(", ")
                          : "scripts/RUNBOOK"}
                      </small>
                    </div>
                  ) : null}
                  {latestRepair.validationErrors.length ? (
                    <div className="deploy-repair-callout">
                      <p>Patch bloqueado pela validação ADE.</p>
                      <small>{latestRepair.validationErrors.join(" · ")}</small>
                    </div>
                  ) : null}
                </section>
              ) : null}

              <ol className="deploy-stepper" aria-label="Etapas do deploy">
                {[
                  { step: "package", label: "Pacote", done: readiness.packageCreated },
                  { step: "analysis", label: "Análise", done: Boolean(selectedDeployAnalysis) },
                  { step: "review", label: "Review", done: readiness.reviewClean },
                  { step: "environment", label: "Ambiente", done: readiness.environmentReady },
                  { step: "approval", label: "Aprovar", done: readiness.packageApproved },
                  { step: "target", label: "Target", done: readiness.targetReady },
                  { step: "deploy", label: "Deploy", done: readiness.deployReady },
                ].map(({ step, label, done }) => (
                  <li
                    key={step as string}
                    className={done ? "done" : readiness.nextStep === step ? "current" : "pending"}
                  >
                    <span>{done ? "✓" : ""}</span>
                    <strong>{label}</strong>
                  </li>
                ))}
              </ol>

              {deployFindings.length ? (
                <details
                  className={
                    blockingFindings.length
                      ? "deploy-review-details blocked"
                      : "deploy-review-details warning"
                  }
                >
                  <summary>
                    <span>
                      {blockingFindings.length ? "Bloqueios do review" : "Avisos do pacote"}
                    </span>
                    <strong>{deployFindings.length}</strong>
                    <ChevronDown aria-hidden="true" size={16} />
                  </summary>
                  <ul>
                    {deployFindings.map((finding, index) => (
                      <li key={`${finding.path}-${finding.reason}-${index}`}>
                        <span
                          className={
                            finding.blocking
                              ? "deploy-finding-severity blocked"
                              : "deploy-finding-severity warning"
                          }
                        >
                          {finding.blocking ? "Bloqueio" : "Aviso"}
                        </span>
                        <code title={finding.path}>
                          {deployFindingPathLabel(finding, selectedVersion.artifact_path)}
                        </code>
                        <small>{finding.reason}</small>
                      </li>
                    ))}
                  </ul>
                </details>
              ) : null}

              <section className="deploy-wizard-step compact">
                <div className="machines-list-head">
                  <strong>Resumo</strong>
                  <span>{selectedProjectSummary}</span>
                </div>
                <div className="deploy-summary-grid">
                  <span>
                    <small>Status</small>
                    <strong>{deployStatusLabel(selectedVersion.status)}</strong>
                  </span>
                  <span>
                    <small>Target</small>
                    <strong>{selectedMachine?.display_name ?? "Selecione uma VM"}</strong>
                  </span>
                  <span>
                    <small>Agente</small>
                    <strong>
                      {deployAnalysisText(selectedDeployAnalysis, "agent_name") ??
                        selectedAgentProfile?.name ??
                        "Selecione um agente"}
                    </strong>
                  </span>
                  <span>
                    <small>Plano</small>
                    <strong>{deployStrategyLabel(selectedDeployStrategy)}</strong>
                  </span>
                  <span>
                    <small>Análise</small>
                    <strong>
                      {deployAnalysisText(selectedDeployAnalysis, "status") ?? "sem análise"}
                      {deployAnalysisText(selectedDeployAnalysis, "confidence")
                        ? ` · ${deployAnalysisText(selectedDeployAnalysis, "confidence")}`
                        : ""}
                    </strong>
                  </span>
                  <span>
                    <small>Artefato</small>
                    <strong>{selectedVersion.artifact_path}</strong>
                  </span>
                </div>
              </section>

              {selectedDeployAnalysis ? (
                <details className="deploy-section-details" open>
                  <summary>
                    <span>Plano do agente</span>
                    <strong>
                      {deployAnalysisText(selectedDeployAnalysis, "confidence") ?? "confidence"}
                    </strong>
                    <ChevronDown aria-hidden="true" size={16} />
                  </summary>
                  <p className="empty-note">
                    {deployAnalysisText(selectedDeployAnalysis, "summary") ??
                      "Plano validado antes da geração do pacote."}
                  </p>
                </details>
              ) : null}

              <details
                className="deploy-section-details"
                open={readiness.nextStep === "environment" && Boolean(activeDeployEnvironment)}
              >
                <summary>
                  <span>Ambiente local</span>
                  <strong>{environmentSummary}</strong>
                  <ChevronDown aria-hidden="true" size={16} />
                </summary>
                {activeDeployEnvironment?.variables.length ? (
                  <div className="deploy-env-grid">
                    {activeDeployEnvironment.variables.map((variable) => (
                      <label key={variable.key}>
                        <span>
                          {variable.key}
                          {variable.required ? <small>obrigatória</small> : null}
                        </span>
                        <input
                          aria-label={`Valor de ${variable.key}`}
                          type={variable.secret ? "password" : "text"}
                          value={environmentDraft[variable.key] ?? ""}
                          placeholder={variable.placeholder}
                          onChange={(event) =>
                            setEnvironmentDraft((current) => ({
                              ...current,
                              [variable.key]: event.target.value,
                            }))
                          }
                        />
                      </label>
                    ))}
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => void saveEnvironment()}
                      disabled={environmentSaving || !selectedMachineId}
                    >
                      <Check aria-hidden="true" size={15} /> Salvar ambiente
                    </button>
                  </div>
                ) : (
                  <p className="empty-note">
                    Este pacote não declarou variáveis em `.env.example`.
                  </p>
                )}
              </details>

              <details className="deploy-section-details">
                <summary>
                  <span>Ações manuais</span>
                  <strong>{readiness.nextStep}</strong>
                  <ChevronDown aria-hidden="true" size={16} />
                </summary>
                <div className="deploy-primary-actions">
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => void approveVersion()}
                    disabled={busy || !canApproveVersion(selectedVersion)}
                  >
                    <Check aria-hidden="true" size={15} /> Aprovar pacote
                  </button>
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => void prepareTarget()}
                    disabled={busy || !prepareReady}
                  >
                    <Upload aria-hidden="true" size={15} /> Preparar target
                  </button>
                  <button
                    className="primary-button"
                    type="button"
                    onClick={() => void deploySelected()}
                    disabled={busy || !deployReady}
                  >
                    <Play aria-hidden="true" size={15} /> Deploy
                  </button>
                  <button
                    className="secondary-button danger"
                    type="button"
                    onClick={() => void stopSelected()}
                    disabled={busy || !selectedMachineId}
                  >
                    <Square aria-hidden="true" size={15} /> Stop
                  </button>
                </div>
              </details>

              <details className="deploy-technical-details">
                <summary>
                  <span>Detalhes técnicos</span>
                  <strong>manifest, versões, progresso e logs</strong>
                  <ChevronDown aria-hidden="true" size={16} />
                </summary>
                <div className="deploy-technical-grid">
                  <section className="deploy-artifact">
                    <div className="machines-list-head">
                      <strong>Review técnico</strong>
                      <select
                        value={artifactPath}
                        onChange={(event) => setArtifactPath(event.target.value)}
                        aria-label="Artefato para revisar"
                      >
                        <option value="manifest.json">manifest.json</option>
                        <option value="RUNBOOK.md">RUNBOOK.md</option>
                        <option value="analysis/deploy-plan.json">
                          analysis/deploy-plan.json
                        </option>
                        <option value="analysis/project-context.json">
                          analysis/project-context.json
                        </option>
                        <option value="analysis/validation-report.json">
                          analysis/validation-report.json
                        </option>
                        <option value="docker-compose.yml">docker-compose.yml</option>
                        <option value=".env.example">.env.example</option>
                        <option value="scripts/preflight.sh">scripts/preflight.sh</option>
                        <option value="scripts/deploy.sh">scripts/deploy.sh</option>
                        <option value="scripts/healthcheck.sh">scripts/healthcheck.sh</option>
                        <option value="scripts/logs.sh">scripts/logs.sh</option>
                        <option value="scripts/rollback.sh">scripts/rollback.sh</option>
                        <option value="scripts/stop.sh">scripts/stop.sh</option>
                        <option value="scripts/install-deploy.ps1">
                          scripts/install-deploy.ps1
                        </option>
                      </select>
                    </div>
                    <pre>{artifact || "Selecione um artefato."}</pre>
                  </section>

                  <section className="machine-progress">
                    <div className="machines-list-head">
                      <strong>Progress</strong>
                      <button className="text-button" type="button" onClick={() => setProgress([])}>
                        Clear
                      </button>
                    </div>
                    {visibleProgress.length ? (
                      <ol>
                        {visibleProgress.slice(-8).map((entry, index) => (
                          <li key={`${entry.runId}-${entry.stepKey}-${index}`}>
                            <span>{deployStepLabel(entry.stepKey)}</span>
                            <strong>{entry.message || entry.status}</strong>
                          </li>
                        ))}
                      </ol>
                    ) : (
                      <p className="empty-note">Sem progresso para este deploy.</p>
                    )}
                  </section>

                  <section className="deploy-versions">
                    <div className="machines-list-head">
                      <strong>Versões e rollback</strong>
                      <span>{versions.length}</span>
                    </div>
                    <div className="deploy-version-list">
                      {versions.map((version) => (
                        <div key={version.id} className="deploy-version-row">
                          <button
                            className="deploy-version-main"
                            type="button"
                            onClick={() => {
                              setArtifactPath("manifest.json");
                              void api
                                .readDeployArtifact(version.id, "manifest.json")
                                .then((result) => {
                                  if (result.ok) setArtifact(result.value);
                                });
                            }}
                          >
                            <span>
                              <strong>{version.label}</strong>
                              <small>{deployStatusLabel(version.status)}</small>
                            </span>
                            <span>{version.review_status}</span>
                          </button>
                          <button
                            className="text-button"
                            type="button"
                            onClick={(event) => {
                              event.stopPropagation();
                              void reactivateVersion(version);
                            }}
                            disabled={
                              busy || !selectedMachineId || version.review_status !== "approved"
                            }
                          >
                            Reactivar
                          </button>
                        </div>
                      ))}
                    </div>
                  </section>

                  {runs.length ? (
                    <section className="deploy-runs">
                      <div className="machines-list-head">
                        <strong>Runs</strong>
                      </div>
                      {runs.slice(0, 5).map((run) => (
                        <button
                          key={run.id}
                          className="deploy-run-row"
                          type="button"
                          onClick={() => void loadRunLogs(run)}
                        >
                          <span>{run.operation}</span>
                          <strong>
                            {run.status}
                            {run.agent_name ? ` · ${run.orchestration_status}` : ""}
                          </strong>
                          <small>{run.started_at}</small>
                        </button>
                      ))}
                    </section>
                  ) : null}

                  <section className="machine-logs">
                    <div className="machines-list-head">
                      <strong>Logs</strong>
                      {retryActionLabel(latestRun) ? (
                        <button
                          className="text-button"
                          type="button"
                          onClick={() =>
                            latestRepair.patchPending
                              ? void createRepairVersion()
                              : latestRun?.operation === "prepare"
                              ? void prepareTarget()
                              : latestRun?.operation === "stop"
                                ? void stopSelected()
                                : void deploySelected()
                          }
                        >
                          {retryActionLabel(latestRun)}
                        </button>
                      ) : null}
                    </div>
                    <pre>{runLogs || latestRun?.summary || "Sem logs carregados."}</pre>
                  </section>
                </div>
              </details>
            </>
          ) : (
            <div className="deploy-empty-start">
              {renderNextActionCard()}
              <div className="terminal-empty">Crie ou selecione uma stack para começar.</div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
