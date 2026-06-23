import { listen } from "@tauri-apps/api/event";
import {
  Copy,
  Eye,
  FileText,
  KeyRound,
  LoaderCircle,
  Play,
  Plus,
  RefreshCw,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import {
  appendMachineProgress,
  machineAccessUser,
  machineErrorMessage,
  machineSshCommand,
  machineSshProbeMessage,
  machineStatusLabel,
  progressForMachine,
  providerBannerState,
  sortMachines,
  windowsBootstrapCommand,
  windowsBootstrapGuestPath,
  windowsPostCreateMessage,
  windowsSharedHostPath,
  type MachineProgressEntry,
} from "./machines";
import { api } from "./tauri";
import type {
  MachinePreset,
  MachineProgressEvent,
  MachineProviderStatus,
  Project,
  Workspace,
  WorkspaceMachine,
} from "./types";

type MachineSection = "credentials" | "activity" | "logs";

function isUbuntuDeployPresetId(presetId: string) {
  return presetId === "ubuntu_deploy_vm" || presetId === "ubuntu_desktop_deploy_vm";
}

function isUbuntuDesktopDeployPresetId(presetId: string) {
  return presetId === "ubuntu_desktop_deploy_vm";
}

function presetSummary(preset: MachinePreset) {
  if (preset.id === "ubuntu_desktop_deploy_vm") {
    return "Desktop Ubuntu com SSH, RDP, navegador e base de deploy assistido.";
  }
  if (preset.id === "ubuntu_deploy_vm") {
    return "Ubuntu Server para deploy enxuto, terminal e serviços sem interface gráfica.";
  }
  if (preset.id === "windows_11") {
    return "Windows 11 para validar instaladores, RDP e fluxos específicos do Windows.";
  }
  return "Ambiente WinBox para desenvolvimento local.";
}

function presetHighlights(preset: MachinePreset) {
  if (preset.id === "ubuntu_desktop_deploy_vm") {
    return ["SSH", "RDP", "Desktop", "Deploy"];
  }
  if (preset.id === "ubuntu_deploy_vm") {
    return ["SSH", "Server", "Docker", "Deploy"];
  }
  if (preset.id === "windows_11") {
    return ["RDP", "Browser", "Windows"];
  }
  return preset.deploy_capable ? ["Deploy"] : ["Manual"];
}

function presetModeLabel(preset: MachinePreset) {
  return preset.deploy_capable ? "Deploy" : "Manual";
}

function defaultAccessUser(preset: MachinePreset | null | undefined) {
  if (!preset) return "";
  if (isUbuntuDeployPresetId(preset.id) || preset.image_family === "windows") return "bruno";
  return "";
}

function machineProfileName(workspaceId: number, displayName: string) {
  let slug = displayName
    .split("")
    .map((char) => (/^[a-z0-9]$/i.test(char) ? char.toLowerCase() : "-"))
    .join("");
  while (slug.includes("--")) slug = slug.replaceAll("--", "-");
  const trimmed = slug.replace(/^-+|-+$/g, "");
  return `dw-${workspaceId}-${trimmed || "machine"}`;
}

export default function MachinesPanel({
  activeProject,
  confirm,
  workspace,
}: {
  activeProject: Project | null;
  confirm: (options: {
    title: string;
    body?: string;
    confirmLabel?: string;
    danger?: boolean;
  }) => Promise<boolean>;
  workspace: Workspace;
}) {
  const [provider, setProvider] = useState<MachineProviderStatus | null>(null);
  const [presets, setPresets] = useState<MachinePreset[]>([]);
  const [machines, setMachines] = useState<WorkspaceMachine[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [progress, setProgress] = useState<MachineProgressEntry[]>([]);
  const [logs, setLogs] = useState("");
  const [health, setHealth] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const [createProgressTarget, setCreateProgressTarget] = useState<{
    profile: string;
    startedAt: number;
  } | null>(null);
  const [openSections, setOpenSections] = useState<Record<MachineSection, boolean>>({
    credentials: false,
    activity: false,
    logs: false,
  });
  const [passwordDrafts, setPasswordDrafts] = useState<Record<string, string>>({});
  const [draft, setDraft] = useState({
    presetId: "ubuntu_deploy_vm",
    displayName: "",
    profileName: "",
    ram: "",
    cpu: "",
    disk: "",
    user: "",
    password: "",
  });
  const providerState = providerBannerState(provider);
  const visiblePresets = useMemo(
    () =>
      presets.filter((preset) => isUbuntuDeployPresetId(preset.id) || preset.id === "windows_11"),
    [presets],
  );
  const sortedMachines = useMemo(() => sortMachines(machines), [machines]);
  const selectedMachine = useMemo(
    () => sortedMachines.find((machine) => machine.id === selectedId) ?? sortedMachines[0] ?? null,
    [selectedId, sortedMachines],
  );
  const selectedMachineError = selectedMachine
    ? [selectedMachine.last_error_code, selectedMachine.last_error_message]
        .filter(Boolean)
        .join(": ")
    : "";
  const recommendedPreset =
    visiblePresets.find((preset) => preset.id === "ubuntu_desktop_deploy_vm") ??
    visiblePresets[0] ??
    null;
  const selectedPreset =
    visiblePresets.find((preset) => preset.id === draft.presetId) ?? visiblePresets[0];
  const machineProgress = progressForMachine(progress, selectedMachine);
  const latestMachineProgress = machineProgress[machineProgress.length - 1] ?? null;
  const createProgress = createProgressTarget
    ? progress.filter((entry) => {
        const eventAt = Date.parse(entry.timestamp);
        return (
          entry.providerProfile === createProgressTarget.profile &&
          (Number.isNaN(eventAt) || eventAt >= createProgressTarget.startedAt - 1000)
        );
      })
    : [];
  const latestCreateProgress = createProgress[createProgress.length - 1] ?? null;
  const createProgressPercent = latestCreateProgress?.percent ?? null;

  function setMachineSection(section: MachineSection, open: boolean) {
    setOpenSections((current) => ({ ...current, [section]: open }));
  }

  const reload = useCallback(async () => {
    setBusy(true);
    setError("");
    const [providerResult, presetResult, machineResult] = await Promise.all([
      api.checkMachineProvider(),
      api.listMachinePresets(),
      api.listWorkspaceMachines(workspace.id),
    ]);
    if (providerResult.ok) setProvider(providerResult.value);
    else setError(providerResult.error);
    if (presetResult.ok) setPresets(presetResult.value);
    else setError(presetResult.error);
    if (machineResult.ok) setMachines(machineResult.value);
    else setError(machineResult.error);
    setBusy(false);
  }, [workspace.id]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      void reload();
    }, 0);
    return () => window.clearTimeout(timeout);
  }, [reload]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void listen<MachineProgressEvent>("machine://progress", (event) => {
      if (disposed) return;
      setProgress((current) => appendMachineProgress(current, event.payload));
    }).then((value) => {
      if (disposed) value();
      else unlisten = value;
    });
    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, []);

  function openCreate(presetId?: string) {
    const preset = visiblePresets.find((item) => item.id === presetId) ?? visiblePresets[0];
    setDraft({
      presetId: preset?.id ?? "ubuntu_deploy_vm",
      displayName: preset?.label ? `${preset.label} dev` : "",
      profileName: "",
      ram: preset?.default_ram ?? "",
      cpu: preset?.default_cpu ?? "",
      disk: preset?.default_disk ?? "",
      user: defaultAccessUser(preset),
      password: "",
    });
    setCreateProgressTarget(null);
    setCreateOpen(true);
  }

  async function createMachine(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedPreset || !selectedPreset.supported) return;
    const requestedProfileName = draft.profileName.trim();
    const requestedUser = draft.user.trim();
    const isWindows = selectedPreset.image_family === "windows";
    const isUbuntuDesktop = isUbuntuDesktopDeployPresetId(selectedPreset.id);
    const requiresPassword = isWindows || isUbuntuDesktop;
    if (isWindows && (!requestedUser || !draft.password.trim())) {
      setError("credentials_required: Windows 11 requer usuário e senha.");
      return;
    }
    if (isUbuntuDesktop && !draft.password.trim()) {
      setError("credentials_required: Ubuntu Desktop requer senha para login gráfico e RDP.");
      return;
    }
    const targetProfile =
      requestedProfileName && requestedProfileName.toLowerCase() !== "auto"
        ? requestedProfileName
        : machineProfileName(workspace.id, draft.displayName.trim());
    setCreateProgressTarget({ profile: targetProfile, startedAt: Date.now() });
    setBusy(true);
    setError("");
    const result = await api.createWorkspaceMachine({
      workspace_id: workspace.id,
      project_id: activeProject?.id ?? null,
      preset_id: draft.presetId,
      display_name: draft.displayName.trim(),
      provider_profile:
        requestedProfileName && requestedProfileName.toLowerCase() !== "auto"
          ? requestedProfileName
          : null,
      ram: draft.ram || selectedPreset.default_ram,
      cpu: draft.cpu || selectedPreset.default_cpu,
      disk: draft.disk || selectedPreset.default_disk,
      user: requestedUser || defaultAccessUser(selectedPreset) || null,
      password: requiresPassword ? draft.password || null : null,
    });
    if (result.ok) {
      setCreateOpen(false);
      setDraft((value) => ({ ...value, password: "" }));
      setSelectedId(result.value.id);
      if (result.value.image_family === "windows") {
        setMachineSection("credentials", true);
        setHealth(windowsPostCreateMessage(result.value));
      }
      await reload();
    } else {
      await reload();
      setError(result.error);
    }
    setCreateProgressTarget(null);
    setBusy(false);
  }

  async function refreshSelected(machine: WorkspaceMachine) {
    setBusy(true);
    const result = await api.refreshWorkspaceMachine(machine.id);
    if (result.ok) {
      setHealth(
        [
          `Status atualizado: ${machineStatusLabel(result.value.status)}`,
          result.value.web_port ? `Web ${result.value.web_port}` : null,
          result.value.ssh_port ? `SSH ${result.value.ssh_port}` : null,
          result.value.rdp_port ? `RDP ${result.value.rdp_port}` : null,
        ]
          .filter(Boolean)
          .join(" · "),
      );
      setMachineSection("logs", true);
      await reload();
    } else setError(result.error);
    setBusy(false);
  }

  async function startSelected(machine: WorkspaceMachine) {
    setBusy(true);
    setMachineSection("activity", true);
    const result = await api.startWorkspaceMachine(machine.id);
    if (result.ok) await reload();
    else setError(result.error);
    setBusy(false);
  }

  async function stopSelected(machine: WorkspaceMachine) {
    setBusy(true);
    setMachineSection("activity", true);
    const result = await api.stopWorkspaceMachine(machine.id);
    if (result.ok) await reload();
    else setError(result.error);
    setBusy(false);
  }

  async function openSelected(machine: WorkspaceMachine) {
    setBusy(true);
    const result = await api.openWorkspaceMachine(machine.id);
    if (result.ok) setHealth(`Abrindo ${result.value.url}`);
    else setError(result.error);
    setBusy(false);
  }

  async function loadLogs(machine: WorkspaceMachine) {
    setBusy(true);
    setMachineSection("logs", true);
    const result = await api.getWorkspaceMachineLogs(machine.id, 300);
    if (result.ok) setLogs(result.value);
    else setError(result.error);
    setBusy(false);
  }

  async function loadHealth() {
    setBusy(true);
    setMachineSection("logs", true);
    const result = await api.refreshWorkspaceMachineHealth();
    if (result.ok) setHealth(result.value);
    else setError(result.error);
    setBusy(false);
  }

  async function validateSshSelected(machine: WorkspaceMachine) {
    setBusy(true);
    setError("");
    setMachineSection("credentials", true);
    const result = await api.probeWorkspaceMachineSsh(machine.id);
    if (result.ok) {
      setHealth(machineSshProbeMessage(result.value));
      await reload();
    } else {
      setError(result.error);
    }
    setBusy(false);
  }

  async function saveMachinePassword(machine: WorkspaceMachine) {
    const password = (passwordDrafts[machine.id] ?? "").trim();
    if (!password) {
      setError("credentials_required: informe a senha gráfica/RDP.");
      return;
    }
    setBusy(true);
    setError("");
    const result = await api.setWorkspaceMachinePassword(machine.id, password);
    if (result.ok) {
      setMachines((current) =>
        current.map((item) => (item.id === result.value.id ? result.value : item)),
      );
      setPasswordDrafts((current) => ({ ...current, [machine.id]: "" }));
      setHealth("Senha gráfica/RDP atualizada. O LightDM foi iniciado e o XRDP reiniciado.");
    } else {
      setError(result.error);
    }
    setBusy(false);
  }

  async function copyText(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value);
      setError("");
    } catch {
      setError(`clipboard_unavailable: não consegui copiar ${label}; copie manualmente.`);
    }
  }

  async function removeSelected(machine: WorkspaceMachine) {
    const ok = await confirm({
      title: "Remover máquina?",
      body: `Isso remove o perfil ${machine.provider_profile} no Winbox e apaga o mapeamento do workspace.`,
      confirmLabel: "Remover",
      danger: true,
    });
    if (!ok) return;
    setBusy(true);
    const result = await api.removeWorkspaceMachine(machine.id);
    if (result.ok) {
      setSelectedId(null);
      await reload();
    } else {
      setError(result.error);
    }
    setBusy(false);
  }

  return (
    <div className="machines-panel">
      <header className="machines-header">
        <div className="machines-header-copy">
          <p className="eyebrow">Ambientes WinBox</p>
          <h1>Máquinas</h1>
          <p>Crie VMs para desenvolvimento, deploy assistido e validação manual.</p>
        </div>
        <div className="machines-actions">
          <button className="secondary-button" type="button" onClick={() => void reload()}>
            <RefreshCw aria-hidden="true" size={16} /> Atualizar
          </button>
          {sortedMachines.length ? (
            <button
              className="primary-button"
              type="button"
              onClick={() => openCreate()}
              disabled={provider?.status !== "ready" || busy}
            >
              <Plus aria-hidden="true" size={16} /> Nova máquina
            </button>
          ) : null}
        </div>
      </header>

      <section className={`machine-provider-banner ${providerState.tone}`}>
        <strong>{providerState.label}</strong>
        <span>{providerState.detail}</span>
      </section>

      {error ? <div className="git-error">{machineErrorMessage(error)}</div> : null}

      <div
        className={sortedMachines.length ? "machines-grid" : "machines-grid machines-grid-empty"}
      >
        <section
          className={sortedMachines.length ? "machines-list" : "machines-list machine-setup-panel"}
          aria-label="Máquinas"
        >
          <div className="machines-list-head">
            <strong>
              {sortedMachines.length ? `${machines.length} máquinas` : "Criar ambiente"}
            </strong>
            {busy ? <span className="status-pill pending">carregando</span> : null}
          </div>
          {!sortedMachines.length ? (
            <div className="machine-setup-empty">
              <div className="machine-setup-intro">
                <h2>Escolha a primeira VM</h2>
                <p>
                  Para testar deploy automático, comece pelo Ubuntu Desktop. Server e Windows
                  continuam disponíveis para cenários específicos.
                </p>
              </div>
              <div className="machine-preset-cards">
                {visiblePresets.map((preset) => (
                  <button
                    className="machine-preset-card"
                    type="button"
                    key={preset.id}
                    disabled={!preset.supported || provider?.status !== "ready"}
                    onClick={() => openCreate(preset.id)}
                    title={preset.disabled_reason ?? preset.label}
                  >
                    <span className="machine-preset-card-head">
                      <strong>{preset.label}</strong>
                      <span
                        className={
                          preset.deploy_capable
                            ? "machine-preset-badge deploy"
                            : "machine-preset-badge"
                        }
                      >
                        {presetModeLabel(preset)}
                      </span>
                    </span>
                    <span className="machine-preset-summary">{presetSummary(preset)}</span>
                    <span className="machine-preset-specs">
                      <span>{preset.default_ram} RAM</span>
                      <span>{preset.default_cpu} CPU</span>
                      <span>{preset.default_disk} disco</span>
                    </span>
                    <span className="machine-preset-highlights">
                      {presetHighlights(preset).map((highlight) => (
                        <span key={highlight}>{highlight}</span>
                      ))}
                    </span>
                    <span className="machine-preset-action">
                      <Plus aria-hidden="true" size={15} /> Criar essa VM
                    </span>
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <div className="machines-table">
              {sortedMachines.map((machine) => (
                <button
                  key={machine.id}
                  type="button"
                  className={
                    selectedMachine?.id === machine.id ? "machine-row active" : "machine-row"
                  }
                  onClick={() => setSelectedId(machine.id)}
                >
                  <span>
                    <strong>{machine.display_name}</strong>
                    <small>{machine.provider_profile}</small>
                  </span>
                  <span className={`machine-status ${machineStatusLabel(machine.status)}`}>
                    {machineStatusLabel(machine.status)}
                  </span>
                  <span>{machine.web_port ? `:${machine.web_port}` : "no web port"}</span>
                </button>
              ))}
            </div>
          )}
        </section>

        <section className="machine-detail" aria-label="Detalhe da máquina">
          {selectedMachine ? (
            <>
              <section className="machine-operation-panel">
                <div className="machine-detail-head">
                  <div>
                    <p className="eyebrow">Máquina selecionada</p>
                    <h2>{selectedMachine.display_name}</h2>
                    <p>
                      {selectedMachine.provider_profile} · {selectedMachine.provider_runtime}
                    </p>
                  </div>
                  <span className={`machine-status ${machineStatusLabel(selectedMachine.status)}`}>
                    {machineStatusLabel(selectedMachine.status)}
                  </span>
                </div>
                <div
                  className="machine-primary-actions"
                  role="toolbar"
                  aria-label="Ações da máquina"
                >
                  <div className="machine-action-group machine-action-main" role="group">
                    <button
                      className="primary-button machine-open-button"
                      type="button"
                      onClick={() => void openSelected(selectedMachine)}
                      disabled={busy}
                    >
                      <Eye aria-hidden="true" size={15} /> Abrir
                    </button>
                    {selectedMachine.ssh_port ? (
                      <button
                        className="secondary-button machine-ssh-button"
                        type="button"
                        onClick={() => void copyText(machineSshCommand(selectedMachine), "SSH")}
                        disabled={busy}
                        title="Copiar comando SSH"
                        aria-label="Copiar comando SSH"
                      >
                        <Copy aria-hidden="true" size={15} /> SSH
                      </button>
                    ) : null}
                  </div>
                  <div className="machine-action-group machine-action-tools" role="group">
                    <button
                      className="secondary-button machine-icon-button"
                      type="button"
                      onClick={() => void startSelected(selectedMachine)}
                      disabled={busy || selectedMachine.status === "running"}
                      title="Iniciar máquina"
                      aria-label="Iniciar máquina"
                    >
                      <Play aria-hidden="true" size={15} />
                    </button>
                    <button
                      className="secondary-button machine-icon-button"
                      type="button"
                      onClick={() => void stopSelected(selectedMachine)}
                      disabled={busy || selectedMachine.status !== "running"}
                      title="Parar máquina"
                      aria-label="Parar máquina"
                    >
                      <Square aria-hidden="true" size={15} />
                    </button>
                    <button
                      className="secondary-button machine-icon-button"
                      type="button"
                      onClick={() => void refreshSelected(selectedMachine)}
                      disabled={busy}
                      title="Atualizar status"
                      aria-label="Atualizar status"
                    >
                      <RefreshCw aria-hidden="true" size={15} />
                    </button>
                    <button
                      className="secondary-button machine-icon-button"
                      type="button"
                      onClick={() => void loadLogs(selectedMachine)}
                      disabled={busy}
                      title="Carregar logs"
                      aria-label="Carregar logs"
                    >
                      <FileText aria-hidden="true" size={15} />
                    </button>
                  </div>
                  <span className="machine-action-spacer" aria-hidden="true" />
                  <button
                    className="secondary-button danger machine-remove-button"
                    type="button"
                    onClick={() => void removeSelected(selectedMachine)}
                    disabled={busy}
                    title="Remover máquina"
                  >
                    <Trash2 aria-hidden="true" size={15} /> Remover
                  </button>
                </div>
              </section>

              <section className="machine-connection-panel" aria-label="Conexões da máquina">
                <article className="machine-connection-card">
                  <span>Web</span>
                  <strong>{selectedMachine.web_port ? `:${selectedMachine.web_port}` : "-"}</strong>
                  <small>Console no navegador</small>
                </article>
                <article className="machine-connection-card">
                  <span>RDP</span>
                  <strong>{selectedMachine.rdp_port ?? "-"}</strong>
                  <small>Acesso gráfico remoto</small>
                </article>
                <article className="machine-connection-card">
                  <span>SSH</span>
                  <strong>{selectedMachine.ssh_port ?? "-"}</strong>
                  <small>
                    {selectedMachine.ssh_port
                      ? machineAccessUser(selectedMachine) || "127.0.0.1"
                      : "Sem porta exposta"}
                  </small>
                </article>
                <article className="machine-connection-card">
                  <span>Atualizado</span>
                  <strong>{selectedMachine.updated_at.slice(0, 10)}</strong>
                  <small>
                    {selectedMachine.updated_at.slice(11, 19) || selectedMachine.status}
                  </small>
                </article>
              </section>

              {selectedMachine.status === "error" && selectedMachineError ? (
                <div className="git-error machine-error-note">
                  {machineErrorMessage(selectedMachineError)}
                </div>
              ) : null}

              <details
                className="machine-detail-section"
                open={openSections.credentials}
                onToggle={(event) =>
                  setMachineSection("credentials", event.currentTarget.open)
                }
              >
                <summary>
                  <span>
                    <strong>Credenciais e acesso</strong>
                    <small>
                      {selectedMachine.image_family === "windows"
                        ? "RDP, bootstrap e SSH"
                        : "SSH, login gráfico e senha RDP"}
                    </small>
                  </span>
                </summary>
                {selectedMachine.image_family === "windows" ? (
                  <div className="machine-access-grid">
                    <span>Usuário {machineAccessUser(selectedMachine) || "-"}</span>
                    <span>RDP {selectedMachine.rdp_port ?? "-"}</span>
                    <span>WEB {selectedMachine.web_port ?? "-"}</span>
                    <span>SSH {selectedMachine.ssh_port ?? "-"}</span>
                    <code>{windowsBootstrapCommand()}</code>
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => void copyText(windowsBootstrapCommand(), "bootstrap Windows")}
                      disabled={busy}
                    >
                      <Copy aria-hidden="true" size={15} /> Copiar bootstrap
                    </button>
                    {selectedMachine.ssh_port ? (
                      <>
                        <button
                          className="secondary-button"
                          type="button"
                          onClick={() => void copyText(machineSshCommand(selectedMachine), "SSH")}
                          disabled={busy}
                        >
                          <Copy aria-hidden="true" size={15} /> Copiar SSH
                        </button>
                        <button
                          className="secondary-button"
                          type="button"
                          onClick={() => void validateSshSelected(selectedMachine)}
                          disabled={busy}
                        >
                          <RefreshCw aria-hidden="true" size={15} /> Validar SSH
                        </button>
                      </>
                    ) : null}
                    <span>Shared {windowsBootstrapGuestPath()}</span>
                    <span>Host {windowsSharedHostPath(selectedMachine)}</span>
                    <small>
                      Execute o bootstrap como administrador na Windows. Depois valide o SSH na
                      porta {selectedMachine.ssh_port ?? "-"}.
                    </small>
                    {selectedMachine.ssh_port ? (
                      <code>{machineSshCommand(selectedMachine)}</code>
                    ) : null}
                  </div>
                ) : (
                  <div className="machine-access-grid">
                    <code>{machineSshCommand(selectedMachine)}</code>
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => void copyText(machineSshCommand(selectedMachine), "SSH")}
                    >
                      <Copy aria-hidden="true" size={15} /> Copiar SSH
                    </button>
                    <span>RDP {selectedMachine.rdp_port ?? "-"}</span>
                    <span>WEB {selectedMachine.web_port ?? "-"}</span>
                    {isUbuntuDesktopDeployPresetId(selectedMachine.preset_id) ? (
                      <span>Login gráfico {machineAccessUser(selectedMachine) || "bruno"}</span>
                    ) : null}
                    <small>
                      {isUbuntuDesktopDeployPresetId(selectedMachine.preset_id)
                        ? `SSH usa chave local. Login gráfico e RDP usam o usuário ${machineAccessUser(selectedMachine) || "bruno"} e a senha salva aqui.`
                        : "Autenticação por chave local."}
                    </small>
                    {isUbuntuDesktopDeployPresetId(selectedMachine.preset_id) ? (
                      <form
                        className="machine-password-form"
                        onSubmit={(event) => {
                          event.preventDefault();
                          void saveMachinePassword(selectedMachine);
                        }}
                      >
                        <label>
                          <span>Senha de {machineAccessUser(selectedMachine) || "bruno"}</span>
                          <input
                            type="password"
                            value={passwordDrafts[selectedMachine.id] ?? ""}
                            onChange={(event) =>
                              setPasswordDrafts((current) => ({
                                ...current,
                                [selectedMachine.id]: event.target.value,
                              }))
                            }
                            placeholder="Senha para LightDM e RDP"
                            autoComplete="new-password"
                          />
                        </label>
                        <button className="secondary-button" type="submit" disabled={busy}>
                          <KeyRound aria-hidden="true" size={15} /> Salvar senha
                        </button>
                      </form>
                    ) : null}
                  </div>
                )}
              </details>

              <details
                className="machine-detail-section machine-progress"
                open={openSections.activity}
                onToggle={(event) => setMachineSection("activity", event.currentTarget.open)}
              >
                <summary>
                  <span>
                    <strong>Atividade</strong>
                    <small>
                      {latestMachineProgress
                        ? latestMachineProgress.message || latestMachineProgress.status
                        : "Sem eventos para esta máquina"}
                    </small>
                  </span>
                </summary>
                <div className="machine-section-actions">
                  <button className="text-button" type="button" onClick={() => setProgress([])}>
                    Limpar
                  </button>
                </div>
                {machineProgress.length ? (
                  <ol>
                    {machineProgress.slice(-12).map((entry) => (
                      <li
                        key={`${entry.runId}-${entry.timestamp}-${entry.phase}-${entry.status}-${entry.message}`}
                      >
                        <span>{entry.phase}</span>
                        <strong>{entry.message || entry.status}</strong>
                      </li>
                    ))}
                  </ol>
                ) : (
                  <p className="empty-note">Sem eventos de progresso para esta máquina.</p>
                )}
              </details>

              <details
                className="machine-detail-section machine-logs"
                open={openSections.logs}
                onToggle={(event) => setMachineSection("logs", event.currentTarget.open)}
              >
                <summary>
                  <span>
                    <strong>Logs e saúde do host</strong>
                    <small>{logs || health ? "Dados carregados" : "Carregue sob demanda"}</small>
                  </span>
                </summary>
                <div className="machine-section-actions">
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => void loadLogs(selectedMachine)}
                    disabled={busy}
                  >
                    <FileText aria-hidden="true" size={15} /> Carregar logs
                  </button>
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => void loadHealth()}
                    disabled={busy}
                  >
                    Host health
                  </button>
                </div>
                <pre>{logs || health || "Nenhum log carregado."}</pre>
              </details>
            </>
          ) : (
            <div className="machine-start-guide">
              <section className="machine-start-hero">
                <p className="eyebrow">Fluxo recomendado</p>
                <h2>{recommendedPreset?.label ?? "Escolha uma VM"}</h2>
                <p>
                  Use a VM desktop quando quiser abrir navegador, validar Playwright, acessar por
                  RDP e rodar o deploy assistido na mesma máquina.
                </p>
              </section>
              <div className="machine-guide-grid">
                <section>
                  <strong>1. Criar</strong>
                  <span>
                    Escolha o preset, informe usuário/senha quando necessário e deixe o WinBox
                    provisionar.
                  </span>
                </section>
                <section>
                  <strong>2. Acessar</strong>
                  <span>Depois de running, use Open para web/RDP ou copie o comando SSH.</span>
                </section>
                <section>
                  <strong>3. Deploy</strong>
                  <span>Na aba Deploy, selecione a VM criada e rode a preparação do target.</span>
                </section>
              </div>
              <ol className="machine-setup-steps" aria-label="Recursos esperados">
                <li>Ubuntu Desktop: melhor caminho para deploy automático e testes com browser.</li>
                <li>Ubuntu Server: opção enxuta para serviços e terminal.</li>
                <li>Windows 11: validação manual de ambiente Windows por RDP.</li>
              </ol>
            </div>
          )}
        </section>
      </div>

      {createOpen ? (
        <div className="modal-backdrop elevated" role="presentation">
          <section className="modal-panel machine-create-modal" role="dialog" aria-modal="true">
            <header className="modal-header">
              <div>
                <h2>Nova máquina</h2>
                <p>Senha não é salva pela ADE.</p>
              </div>
              <button
                className="secondary-button icon-button"
                type="button"
                onClick={() => setCreateOpen(false)}
                disabled={Boolean(createProgressTarget)}
                aria-label="Fechar"
              >
                <X aria-hidden="true" size={16} />
              </button>
            </header>
            <form className="machine-create-form" onSubmit={(event) => void createMachine(event)}>
              <label>
                <span>Modelo</span>
                <select
                  value={draft.presetId}
                  onChange={(event) => {
                    const preset = visiblePresets.find((item) => item.id === event.target.value);
                    setDraft((value) => ({
                      ...value,
                      presetId: event.target.value,
                      displayName: preset?.label ? `${preset.label} dev` : value.displayName,
                      ram: preset?.default_ram ?? value.ram,
                      cpu: preset?.default_cpu ?? value.cpu,
                      disk: preset?.default_disk ?? value.disk,
                      user: defaultAccessUser(preset),
                      password: "",
                    }));
                  }}
                >
                  {visiblePresets.map((preset) => (
                    <option key={preset.id} value={preset.id} disabled={!preset.supported}>
                      {preset.label}
                      {preset.deploy_capable ? " · deploy automático" : " · manual"}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Nome</span>
                <input
                  value={draft.displayName}
                  onChange={(event) =>
                    setDraft((value) => ({ ...value, displayName: event.target.value }))
                  }
                  required
                />
              </label>
              <details className="machine-advanced-fields">
                <summary>Opções avançadas</summary>
                <label>
                  <span>ID técnico no WinBox</span>
                  <input
                    value={draft.profileName}
                    onChange={(event) =>
                      setDraft((value) => ({ ...value, profileName: event.target.value }))
                    }
                    placeholder={`dw-${workspace.id}-deploy-vm`}
                    aria-describedby="machine-provider-profile-hint"
                  />
                  <small id="machine-provider-profile-hint">
                    Deixe vazio para gerar automaticamente. Use só quando precisar casar com um
                    perfil local já conhecido.
                  </small>
                </label>
              </details>
              <div className="machine-resource-grid">
                <label>
                  <span>RAM</span>
                  <input
                    value={draft.ram}
                    onChange={(event) =>
                      setDraft((value) => ({ ...value, ram: event.target.value }))
                    }
                  />
                </label>
                <label>
                  <span>CPU</span>
                  <input
                    value={draft.cpu}
                    onChange={(event) =>
                      setDraft((value) => ({ ...value, cpu: event.target.value }))
                    }
                  />
                </label>
                <label>
                  <span>Disco</span>
                  <input
                    value={draft.disk}
                    onChange={(event) =>
                      setDraft((value) => ({ ...value, disk: event.target.value }))
                    }
                  />
                </label>
              </div>
              {selectedPreset && isUbuntuDeployPresetId(selectedPreset.id) ? (
                <label>
                  <span>
                    {isUbuntuDesktopDeployPresetId(selectedPreset.id)
                      ? "Usuário SSH e login gráfico"
                      : "Usuário SSH"}
                  </span>
                  <input
                    value={draft.user}
                    onChange={(event) =>
                      setDraft((value) => ({ ...value, user: event.target.value }))
                    }
                    placeholder="bruno"
                  />
                </label>
              ) : null}
              {selectedPreset?.image_family === "windows" ? (
                <>
                  <label>
                    <span>Usuário Windows/RDP</span>
                    <input
                      value={draft.user}
                      onChange={(event) =>
                        setDraft((value) => ({ ...value, user: event.target.value }))
                      }
                      required
                    />
                  </label>
                  <label>
                    <span>Senha Windows/RDP</span>
                    <input
                      type="password"
                      value={draft.password}
                      onChange={(event) =>
                        setDraft((value) => ({ ...value, password: event.target.value }))
                      }
                      required
                    />
                  </label>
                </>
              ) : null}
              {selectedPreset && isUbuntuDesktopDeployPresetId(selectedPreset.id) ? (
                <label>
                  <span>
                    Senha de {draft.user.trim() || defaultAccessUser(selectedPreset) || "bruno"}
                  </span>
                  <input
                    type="password"
                    value={draft.password}
                    onChange={(event) =>
                      setDraft((value) => ({ ...value, password: event.target.value }))
                    }
                    required
                  />
                </label>
              ) : null}
              {selectedPreset && !selectedPreset.supported ? (
                <div className="git-error">{selectedPreset.disabled_reason}</div>
              ) : null}
              {createProgressTarget ? (
                <section
                  className="machine-create-progress"
                  aria-live="polite"
                  aria-label="Progresso de criação da máquina"
                >
                  <div className="machine-create-progress-head">
                    <span>
                      <LoaderCircle aria-hidden="true" className="machine-create-spinner" size={16} />
                      Criando máquina
                    </span>
                    <strong>
                      {createProgressPercent !== null ? `${createProgressPercent}%` : "em andamento"}
                    </strong>
                  </div>
                  <div
                    className="machine-create-progress-track"
                    role="progressbar"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={createProgressPercent ?? undefined}
                  >
                    <span
                      className={
                        createProgressPercent !== null
                          ? "machine-create-progress-fill"
                          : "machine-create-progress-fill indeterminate"
                      }
                      style={
                        createProgressPercent !== null
                          ? { width: `${Math.max(4, Math.min(100, createProgressPercent))}%` }
                          : undefined
                      }
                    />
                  </div>
                  <ol className="machine-create-progress-events">
                    {createProgress.slice(-4).map((entry) => (
                      <li
                        key={`${entry.runId}-${entry.timestamp}-${entry.phase}-${entry.status}-${entry.message}`}
                      >
                        <span>{entry.phase}</span>
                        <strong>{entry.message || entry.status}</strong>
                      </li>
                    ))}
                    {!createProgress.length ? (
                      <li>
                        <span>aguardando</span>
                        <strong>Iniciando operação no WinBox</strong>
                      </li>
                    ) : null}
                  </ol>
                </section>
              ) : null}
              <footer className="modal-actions">
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setCreateOpen(false)}
                  disabled={Boolean(createProgressTarget)}
                >
                  Cancelar
                </button>
                <button
                  className="primary-button"
                  type="submit"
                  disabled={busy || !selectedPreset?.supported}
                >
                  {createProgressTarget ? (
                    <LoaderCircle aria-hidden="true" className="machine-create-spinner" size={16} />
                  ) : (
                    <Plus aria-hidden="true" size={16} />
                  )}
                  {createProgressTarget ? "Criando..." : "Criar"}
                </button>
              </footer>
            </form>
          </section>
        </div>
      ) : null}
    </div>
  );
}
