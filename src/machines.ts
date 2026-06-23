import type {
  MachineProgressEvent,
  MachineProviderStatus,
  MachineSshProbe,
  WorkspaceMachine,
} from "./types";

export type MachineProgressEntry = {
  runId: string;
  machineId: string | null;
  providerProfile: string;
  operation: string;
  phase: string;
  status: string;
  message: string;
  percent: number | null;
  timestamp: string;
};

export function providerBannerState(status: MachineProviderStatus | null) {
  if (!status) {
    return {
      tone: "loading" as const,
      label: "Checking Winbox",
      detail: "Detecting the local machine provider.",
    };
  }
  if (status.status === "ready") {
    return {
      tone: "ready" as const,
      label: "Winbox ready",
      detail: [status.version, status.runtime].filter(Boolean).join(" · "),
    };
  }
  if (status.status === "incompatible") {
    return {
      tone: "warning" as const,
      label: "Winbox incompatible",
      detail: status.hint || status.message,
    };
  }
  return {
    tone: "error" as const,
    label: "Winbox unavailable",
    detail: status.hint || status.message,
  };
}

export function machineStatusLabel(status: string) {
  switch (status) {
    case "creating":
      return "creating";
    case "running":
      return "running";
    case "stopped":
      return "stopped";
    case "paused":
      return "paused";
    case "removing":
      return "removing";
    case "error":
      return "error";
    default:
      return "unknown";
  }
}

export function sortMachines(machines: WorkspaceMachine[]) {
  const priority: Record<string, number> = {
    error: 0,
    creating: 1,
    removing: 1,
    running: 2,
    paused: 3,
    stopped: 4,
    unknown: 5,
  };
  return [...machines].sort((left, right) => {
    const leftPriority = priority[left.status] ?? priority.unknown;
    const rightPriority = priority[right.status] ?? priority.unknown;
    if (leftPriority !== rightPriority) return leftPriority - rightPriority;
    return left.display_name.localeCompare(right.display_name);
  });
}

export function appendMachineProgress(
  current: MachineProgressEntry[],
  event: MachineProgressEvent,
) {
  const entry: MachineProgressEntry = {
    runId: event.run_id,
    machineId: event.machine_id ?? null,
    providerProfile: event.provider_profile,
    operation: event.operation,
    phase: event.phase,
    status: event.status,
    message: event.message,
    percent: event.percent ?? null,
    timestamp: event.timestamp,
  };
  return [...current, entry].slice(-80);
}

export function progressForMachine(
  entries: MachineProgressEntry[],
  machine: WorkspaceMachine | null,
) {
  if (!machine) return entries.slice(-20);
  return entries.filter(
    (entry) => entry.machineId === machine.id || entry.providerProfile === machine.provider_profile,
  );
}

export function machineAccessUser(machine: WorkspaceMachine) {
  const saved = machine.access_user?.trim();
  if (saved) return saved;
  if (
    machine.preset_id === "ubuntu_deploy_vm" ||
    machine.preset_id === "ubuntu_desktop_deploy_vm" ||
    machine.image_family === "windows"
  ) {
    return "bruno";
  }
  return "";
}

export function machineSshCommand(machine: WorkspaceMachine) {
  const user = machineAccessUser(machine);
  const target = user ? `${user}@127.0.0.1` : "127.0.0.1";
  return `ssh -p ${machine.ssh_port ?? "-"} ${target}`;
}

export function windowsSharedHostPath(machine: WorkspaceMachine) {
  return `/home/bruno/Windows/${machine.provider_profile}`;
}

export function windowsBootstrapGuestPath() {
  return String.raw`\\host.lan\Data\ade\bootstrap-windows.ps1`;
}

export function windowsBootstrapCommand() {
  return `powershell -NoProfile -ExecutionPolicy Bypass -File ${windowsBootstrapGuestPath()}`;
}

export function windowsPostCreateMessage(machine: WorkspaceMachine) {
  return [
    "Bootstrap Windows criado.",
    `Execute ${windowsBootstrapGuestPath()} como administrador dentro da VM.`,
    machine.ssh_port ? `Depois valide SSH em :${machine.ssh_port}.` : "Depois valide SSH.",
  ].join(" ");
}

export function machineSshProbeMessage(probe: MachineSshProbe) {
  if (probe.status === "ready") return probe.message;
  if (probe.status === "missing_port") return probe.message;
  return `${probe.message}. Rode o bootstrap Windows e tente novamente.`;
}

const machineErrorCopy: Record<string, { message: string; hint: string }> = {
  winbox_not_found: {
    message: "Winbox CLI não encontrado.",
    hint: "Instale o Winbox ou configure WINBOX_BIN apontando para o executável.",
  },
  winbox_incompatible: {
    message: "Winbox incompatível com a integração.",
    hint: "Atualize o Winbox para uma versão com suporte a --json.",
  },
  docker_not_installed: {
    message: "Docker não está instalado.",
    hint: "Instale o Docker Engine e tente novamente.",
  },
  docker_daemon_unavailable: {
    message: "Docker daemon indisponível.",
    hint: "Inicie o Docker e confira as permissões do usuário.",
  },
  kvm_unavailable: {
    message: "KVM indisponível no host.",
    hint: "Habilite virtualização/KVM e tente novamente.",
  },
  storage_path_invalid: {
    message: "Caminho de storage inválido.",
    hint: "Escolha um caminho local gravável e com espaço suficiente.",
  },
  disk_full: {
    message: "Espaço em disco insuficiente.",
    hint: "Libere espaço ou escolha outro storage.",
  },
  port_unavailable: {
    message: "Porta indisponível.",
    hint: "Verifique conflitos nas portas do host.",
  },
  profile_not_found: {
    message: "Perfil Winbox não encontrado.",
    hint: "Atualize a lista ou escolha um perfil existente.",
  },
  unsupported_preset: {
    message: "Preset não suportado pelo Winbox atual.",
    hint: "Use Ubuntu Server LTS, Xubuntu LTS ou Windows 11 quando disponíveis.",
  },
  operation_failed: {
    message: "Operação da máquina falhou.",
    hint: "Abra os logs do Winbox e tente novamente.",
  },
  desktop_setup_failed: {
    message: "Setup desktop falhou.",
    hint: "Aguarde o apt/cloud-init terminar; se a VM ficou criada, salve a senha novamente para repetir o setup ou remova a máquina.",
  },
  provider_profile_mismatch: {
    message: "WinBox criou um perfil diferente do preset solicitado.",
    hint: "Remova essa máquina e crie novamente; a ADE agora envia as opções do WinBox antes do nome do perfil.",
  },
};

export function machineErrorMessage(error: string) {
  const code = error.match(/\b[a-z_]+(?:_[a-z_]+)+\b/)?.[0];
  if (!code) return error;
  const copy = machineErrorCopy[code];
  if (!copy) return error;
  return `${copy.message} ${copy.hint}`;
}
