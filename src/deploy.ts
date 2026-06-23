import type {
  DeployEnvironment,
  DeployProgressEvent,
  DeployRun,
  DeployStack,
  DeployVersion,
  WorkspaceMachine,
} from "./types";

export type DeployProgressEntry = {
  runId: string;
  stackId: string;
  versionId: string | null;
  machineId: string | null;
  stepKey: string;
  status: string;
  message: string;
  percent: number | null;
  timestamp: string;
};

export type DeployPackageFinding = {
  path: string;
  reason: string;
  severity: string;
  blocking: boolean;
};

export type DeployReadiness = {
  packageCreated: boolean;
  reviewClean: boolean;
  environmentReady: boolean;
  packageApproved: boolean;
  targetReady: boolean;
  prepareReady: boolean;
  deployReady: boolean;
  nextStep: "package" | "review" | "environment" | "approval" | "target" | "deploy";
};

export type DeployRepairInfo = {
  status: string;
  attemptCount: number;
  maxAttempts: number;
  latestCode: string | null;
  latestTitle: string | null;
  latestMessage: string | null;
  agentStatus: string | null;
  agentSafeToApply: boolean;
  adeSafeToApply: boolean;
  validationStatus: string | null;
  validationErrors: string[];
  patchPending: boolean;
  patchSummary: string | null;
  userMessage: string | null;
  patches: string[];
};

export function appendDeployProgress(current: DeployProgressEntry[], event: DeployProgressEvent) {
  const entry: DeployProgressEntry = {
    runId: event.run_id,
    stackId: event.stack_id,
    versionId: event.version_id ?? null,
    machineId: event.machine_id ?? null,
    stepKey: event.step_key,
    status: event.status,
    message: event.message,
    percent: event.percent ?? null,
    timestamp: event.timestamp,
  };
  return [...current, entry].slice(-120);
}

export function progressForDeploy(
  entries: DeployProgressEntry[],
  filters: {
    stackId?: string | null;
    versionId?: string | null;
    machineId?: string | null;
    runId?: string | null;
  },
) {
  return entries.filter((entry) => {
    if (filters.runId && entry.runId !== filters.runId) return false;
    if (filters.versionId && entry.versionId !== filters.versionId) return false;
    if (filters.stackId && entry.stackId !== filters.stackId) return false;
    if (filters.machineId && entry.machineId !== filters.machineId) return false;
    return true;
  });
}

export function deployRunsForContext(
  runs: DeployRun[],
  versionId?: string | null,
  machineId?: string | null,
) {
  return runs.filter((run) => {
    if (versionId && run.version_id !== versionId) return false;
    if (machineId && run.machine_id !== machineId) return false;
    return true;
  });
}

export function latestDeployRunForContext(
  runs: DeployRun[],
  versionId?: string | null,
  machineId?: string | null,
) {
  return deployRunsForContext(runs, versionId, machineId)[0] ?? null;
}

export function hasPassedPrepareRun(
  runs: DeployRun[],
  versionId?: string | null,
  machineId?: string | null,
) {
  return deployRunsForContext(runs, versionId, machineId).some(
    (run) => run.operation === "prepare" && run.status === "passed",
  );
}

export function deployStatusLabel(status: string) {
  switch (status) {
    case "review_required":
      return "review required";
    case "approved":
      return "ready";
    case "transferring":
      return "transferring";
    case "starting":
      return "starting";
    case "healthy":
      return "healthy";
    case "failed":
      return "failed";
    case "stopped":
      return "stopped";
    case "superseded":
      return "superseded";
    case "reactivating":
      return "reactivating";
    case "idle":
      return "idle";
    default:
      return status || "unknown";
  }
}

export function deployStatusTone(status: string) {
  if (status === "healthy" || status === "approved") return "running";
  if (status === "failed") return "error";
  if (
    status === "review_required" ||
    status === "transferring" ||
    status === "starting" ||
    status === "reactivating"
  ) {
    return "creating";
  }
  if (status === "stopped" || status === "superseded" || status === "idle") return "stopped";
  return "unknown";
}

export function deployStepLabel(stepKey: string) {
  switch (stepKey) {
    case "agent-precheck":
      return "Pré-checagem";
    case "agent-postcheck":
      return "Verificação final";
    case "deploy-doctor":
      return "Deploy Doctor";
    case "repair-recipe":
      return "Correção segura";
    case "agent-repair":
      return "Correção do agente";
    case "ssh-preflight":
      return "SSH e target";
    case "transfer":
      return "Enviar pacote";
    case "runbook-deploy":
      return "Executar runbook";
    case "runbook-healthcheck":
      return "Validar pacote";
    case "status":
      return "Status";
    case "compose-down":
      return "Parar anterior";
    default:
      return stepKey || "Etapa";
  }
}

export function parseDeployFindings(version: DeployVersion | null): DeployPackageFinding[] {
  if (!version?.blocking_findings_json) return [];
  try {
    const parsed = JSON.parse(version.blocking_findings_json);
    if (!Array.isArray(parsed)) return [];
    return parsed.map(normalizeDeployFinding);
  } catch {
    return [
      {
        path: "blocking_findings_json",
        reason: "invalid blocking findings payload",
        severity: "error",
        blocking: true,
      },
    ];
  }
}

export function parseBlockingFindings(version: DeployVersion | null): DeployPackageFinding[] {
  return parseDeployFindings(version).filter((finding) => finding.blocking);
}

export function canApproveVersion(version: DeployVersion | null) {
  return Boolean(
    version && version.review_status !== "approved" && !parseBlockingFindings(version).length,
  );
}

export function canDeployVersion(
  version: DeployVersion | null,
  machine: WorkspaceMachine | null,
  environment?: DeployEnvironment | null,
) {
  return Boolean(
    version &&
    machine &&
    machine.status === "running" &&
    isAutomaticDeployTarget(machine) &&
    version.review_status === "approved" &&
    !parseBlockingFindings(version).length &&
    (environment === undefined || environment?.ready),
  );
}

export function canPrepareVersion(
  version: DeployVersion | null,
  machine: WorkspaceMachine | null,
  environment?: DeployEnvironment | null,
) {
  return canDeployVersion(version, machine, environment);
}

export function deployReadiness(
  version: DeployVersion | null,
  machine: WorkspaceMachine | null,
  environment?: DeployEnvironment | null,
): DeployReadiness {
  const packageCreated = Boolean(version);
  const reviewClean = packageCreated && !parseBlockingFindings(version).length;
  const packageApproved = Boolean(version && version.review_status === "approved");
  const environmentReady = Boolean(environment?.ready);
  const targetReady = Boolean(machine && machine.status === "running" && isAutomaticDeployTarget(machine));
  const prepareReady = Boolean(reviewClean && packageApproved && environmentReady && targetReady);
  const deployReady = prepareReady;
  let nextStep: DeployReadiness["nextStep"] = "deploy";
  if (!packageCreated) nextStep = "package";
  else if (!reviewClean) nextStep = "review";
  else if (!environmentReady) nextStep = "environment";
  else if (!packageApproved) nextStep = "approval";
  else if (!targetReady) nextStep = "target";
  return {
    packageCreated,
    reviewClean,
    environmentReady,
    packageApproved,
    targetReady,
    prepareReady,
    deployReady,
    nextStep,
  };
}

export function isAutomaticDeployTarget(machine: WorkspaceMachine | null) {
  return Boolean(
    machine &&
      ["ubuntu_deploy_vm", "ubuntu_desktop_deploy_vm", "windows_11"].includes(machine.preset_id),
  );
}

export function deployEnvironmentValues(environment: DeployEnvironment | null) {
  return (
    environment?.variables.map((variable) => ({
      key: variable.key,
      value: variable.value,
    })) ?? []
  );
}

export function deployEnvironmentSummary(environment: DeployEnvironment | null) {
  if (!environment) return "Ambiente não carregado";
  if (!environment.variables.length) return "Sem variáveis obrigatórias";
  if (environment.ready) {
    return `${environment.saved_count}/${environment.required_count} variáveis salvas`;
  }
  if (environment.missing_keys.length === 1) return "1 variável pendente";
  return `${environment.missing_keys.length} variáveis pendentes`;
}

export function isLegacyDeployPackage(version: DeployVersion | null) {
  if (!version?.blocking_findings_json) return false;
  try {
    const parsed = JSON.parse(version.blocking_findings_json);
    return (
      Array.isArray(parsed) &&
      parsed.some(
        (finding) =>
          finding &&
          typeof finding === "object" &&
          !("blocking" in finding) &&
          !("severity" in finding),
      )
    );
  } catch {
    return true;
  }
}

export function latestVersion(versions: DeployVersion[]) {
  return (
    [...versions].sort((left, right) => right.created_at.localeCompare(left.created_at))[0] ?? null
  );
}

export function activeVersionForStack(stack: DeployStack | null, versions: DeployVersion[]) {
  if (!stack?.active_version_id) return null;
  return versions.find((version) => version.id === stack.active_version_id) ?? null;
}

export function activeVersionLabel(stack: DeployStack, versions: DeployVersion[]) {
  return activeVersionForStack(stack, versions)?.label ?? "-";
}

export function sortDeployStacks(stacks: DeployStack[]) {
  const priority: Record<string, number> = {
    healthy: 0,
    failed: 1,
    approved: 2,
    review_required: 3,
    stopped: 4,
    idle: 5,
  };
  return [...stacks].sort((left, right) => {
    const leftPriority = priority[left.status] ?? 9;
    const rightPriority = priority[right.status] ?? 9;
    if (leftPriority !== rightPriority) return leftPriority - rightPriority;
    return right.updated_at.localeCompare(left.updated_at);
  });
}

export function retryActionLabel(run: DeployRun | null) {
  if (!run || run.status !== "failed") return null;
  if (deployRepairInfo(run).patchPending) return "Criar versão corrigida";
  if (run.operation === "prepare") return "Retry prepare";
  if (run.operation === "deploy") return "Retry deploy";
  if (run.operation === "stop") return "Retry stop";
  return "Retry";
}

export function deployRepairInfo(run: DeployRun | null): DeployRepairInfo {
  const fallback: DeployRepairInfo = {
    status: run?.orchestration_status ?? "unknown",
    attemptCount: 0,
    maxAttempts: 3,
    latestCode: null,
    latestTitle: null,
    latestMessage: null,
    agentStatus: null,
    agentSafeToApply: false,
    adeSafeToApply: false,
    validationStatus: null,
    validationErrors: [],
    patchPending: false,
    patchSummary: null,
    userMessage: null,
    patches: [],
  };
  if (!run?.orchestration_report_json) return fallback;
  try {
    const parsed = JSON.parse(run.orchestration_report_json) as Record<string, unknown>;
    const repair = parsed.repair;
    if (!repair || typeof repair !== "object") return fallback;
    const repairRecord = repair as Record<string, unknown>;
    const attempts = Array.isArray(repairRecord.attempts)
      ? repairRecord.attempts.filter((item): item is Record<string, unknown> => Boolean(item && typeof item === "object"))
      : [];
    const latest = attempts[attempts.length - 1] ?? null;
    const diagnosis =
      latest?.diagnosis && typeof latest.diagnosis === "object"
        ? (latest.diagnosis as Record<string, unknown>)
        : null;
    const agentRepair =
      repairRecord.agent_repair && typeof repairRecord.agent_repair === "object"
        ? (repairRecord.agent_repair as Record<string, unknown>)
        : null;
    const validation =
      repairRecord.validation && typeof repairRecord.validation === "object"
        ? (repairRecord.validation as Record<string, unknown>)
        : null;
    const patchSet = Array.isArray(agentRepair?.patch_set) ? agentRepair.patch_set : [];
    const patches = patchSet
      .map((patch) =>
        patch && typeof patch === "object" && typeof (patch as Record<string, unknown>).path === "string"
          ? String((patch as Record<string, unknown>).path)
          : "",
      )
      .filter(Boolean);
    const agentSafeToApply = Boolean(agentRepair?.safe_to_apply);
    const adeSafeToApply =
      typeof repairRecord.ade_safe_to_apply === "boolean"
        ? repairRecord.ade_safe_to_apply
        : Boolean(validation?.ade_safe_to_apply);
    const validationErrors = Array.isArray(validation?.validation_errors)
      ? validation.validation_errors.map(String)
      : [];
    return {
      status: typeof parsed.decision === "string" ? parsed.decision : fallback.status,
      attemptCount: attempts.length,
      maxAttempts:
        typeof repairRecord.max_attempts === "number"
          ? repairRecord.max_attempts
          : fallback.maxAttempts,
      latestCode: typeof diagnosis?.code === "string" ? diagnosis.code : null,
      latestTitle: typeof diagnosis?.title === "string" ? diagnosis.title : null,
      latestMessage: typeof latest?.error === "string" ? latest.error : null,
      agentStatus: typeof agentRepair?.status === "string" ? agentRepair.status : null,
      agentSafeToApply,
      adeSafeToApply,
      validationStatus:
        typeof validation?.validation_status === "string" ? validation.validation_status : null,
      validationErrors,
      patchPending:
        run.orchestration_status === "repair_pending" &&
        adeSafeToApply &&
        patches.length > 0,
      patchSummary:
        typeof agentRepair?.patch_summary === "string" && agentRepair.patch_summary.trim()
          ? agentRepair.patch_summary
          : null,
      userMessage:
        typeof agentRepair?.user_message === "string" && agentRepair.user_message.trim()
          ? agentRepair.user_message
          : null,
      patches,
    };
  } catch {
    return fallback;
  }
}

export function deployErrorMessage(error: string) {
  if (error.startsWith("deploy_repair_pending:")) {
    return "O agente propôs uma correção no pacote. Crie a versão corrigida, revise/aprove e rode o deploy novamente.";
  }
  if (error.startsWith("deploy_agent_repair_blocked:")) {
    return "O agente analisou a falha, mas não encontrou uma correção segura de pacote. Abra os logs do Deploy Doctor.";
  }
  if (error.startsWith("linux_ssh_bootstrap_required:")) {
    return "A VM Linux falhou no preflight antigo de SSH/Docker. Atualize e rode Prepare Target de novo; o fluxo novo tenta instalar a base automaticamente antes de falhar.";
  }
  if (error.startsWith("windows_ssh_bootstrap_required:")) {
    return String.raw`O SSH do Windows ainda não está acessível. Dentro da VM, rode o bootstrap como Administrador: \\host.lan\Data\ade\bootstrap-windows.ps1. Depois valide SSH e tente Deploy novamente.`;
  }
  const lower = error.toLowerCase();
  if (lower.includes("remote mkdir failed")) {
    return "O deploy falhou antes da transferência ao acessar SSH/criar a pasta remota. No Windows, rode o bootstrap da pasta compartilhada como Administrador, valide SSH e tente Deploy novamente.";
  }
  if (
    lower.includes("ssh") &&
    (lower.includes("connection reset") ||
      lower.includes("connection refused") ||
      lower.includes("kex_exchange_identification"))
  ) {
    return "A porta SSH do WinBox está publicada, mas a VM não aceitou comando SSH agora. Aguarde a inicialização terminar, atualize o status e tente Prepare novamente.";
  }
  const code = error.match(/\b[a-z_]+(?:_[a-z_]+)+\b/)?.[0];
  if (!code) return error;
  const copy: Record<string, string> = {
    ssh_port_missing:
      "A VM ainda não expôs SSH. Rode o preparo do target ou confira o profile no WinBox.",
    ssh_unavailable:
      "A VM publicou porta SSH, mas não aceitou o comando automático. Aguarde a inicialização, atualize o status e tente Prepare novamente.",
    linux_base_missing:
      "Prepare tentou instalar Docker/Compose na VM Linux, mas o preflight ainda falhou. Abra os logs do run e confira o passo linux-base.",
    shared_dir_missing:
      "O WinBox não informou a pasta compartilhada. Atualize o provider ou confira o profile.",
    profile_not_found: "O profile WinBox da VM não foi encontrado.",
    winbox_not_found: "O CLI do WinBox não foi encontrado neste host.",
    docker_daemon_unavailable:
      "Docker indisponível no target. No Windows isso pode indicar runtime sem suporte.",
    unsupported_runtime:
      "Runtime do target não suporta este deploy. Confira Docker, Compose e virtualização.",
    unsupported_deploy_target:
      "Esta VM não é um alvo automático de deploy. Crie uma Ubuntu Server Deploy VM, Ubuntu Desktop Deploy VM ou Windows 11.",
    deploy_environment_incomplete:
      "Configure as variáveis do ambiente antes de preparar ou executar o deploy.",
    deploy_project_selection_stale:
      "A lista de projetos mudou. Atualize a tela e selecione novamente os projetos do pacote.",
    deploy_project_selection_empty:
      "Não há projeto selecionado para empacotar. Adicione ou selecione um projeto antes de criar o pacote.",
    deploy_agent_required:
      "Selecione o agente que vai analisar e planejar este deploy antes de continuar.",
    deploy_agent_workspace_mismatch:
      "O agente selecionado não pertence a este workspace. Escolha outro perfil de agente.",
    deploy_agent_timeout:
      "O agente demorou demais para devolver o plano. Tente novamente ou escolha outro agente.",
    deploy_agent_invalid_json:
      "O agente não devolveu um plano JSON válido. Gere novamente o plano antes de criar o pacote.",
    deploy_agent_failed:
      "O agente falhou antes de devolver o plano de deploy. Verifique a configuração do agente.",
    deploy_agent_target_scope:
      "O deploy assistido V2 exige uma Ubuntu Server Deploy VM, Ubuntu Desktop Deploy VM ou Windows 11.",
    deploy_target_not_running:
      "A VM alvo precisa estar running antes do deploy assistido.",
    windows_runbook_manual_required:
      "Este pacote Windows não tem runbook PowerShell automático. Rode o install-deploy.ps1 pela pasta compartilhada dentro da sessão gráfica do Windows.",
    deploy_runbook_incomplete:
      "O pacote não contém todos os scripts do runbook. Crie uma nova versão do pacote.",
    deploy_plan_required:
      "Rode a análise com o agente antes de criar o pacote. O deploy V2 sempre começa pelo plano.",
    deploy_plan_validation_failed:
      "O plano do agente não passou na validação. Revise os avisos ou ajuste a seleção antes de criar o pacote.",
    deploy_repair_not_pending:
      "Esse run não tem correção pendente do agente. Execute o deploy novamente ou abra os logs do Deploy Doctor.",
  };
  return copy[code] ?? error;
}

export function deployFindingPathLabel(
  finding: DeployPackageFinding,
  artifactPath?: string | null,
) {
  const path = finding.path || "unknown path";
  if (artifactPath && path.startsWith(`${artifactPath}/`)) {
    return path.slice(artifactPath.length + 1);
  }
  const deployIndex = path.indexOf("/.dw/deploy-packages/");
  if (deployIndex >= 0) {
    const parts = path.slice(deployIndex).split("/");
    const deployLabelIndex = parts.findIndex((part) => /^deploy-\d+$/.test(part));
    if (deployLabelIndex >= 0) {
      return parts.slice(deployLabelIndex + 1).join("/") || path;
    }
  }
  return path;
}

function normalizeDeployFinding(value: unknown): DeployPackageFinding {
  if (!value || typeof value !== "object") {
    return {
      path: "blocking_findings_json",
      reason: "invalid deploy package finding",
      severity: "error",
      blocking: true,
    };
  }
  const record = value as Record<string, unknown>;
  const blocking = typeof record.blocking === "boolean" ? record.blocking : true;
  const severity =
    typeof record.severity === "string" && record.severity.trim()
      ? record.severity
      : blocking
        ? "error"
        : "warning";
  return {
    path: typeof record.path === "string" && record.path.trim() ? record.path : "unknown path",
    reason:
      typeof record.reason === "string" && record.reason.trim()
        ? record.reason
        : "unknown deploy package finding",
    severity,
    blocking,
  };
}
