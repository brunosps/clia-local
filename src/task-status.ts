export type WiredTaskStatus =
  | "pending"
  | "awaiting_worker"
  | "dispatched"
  | "running"
  | "done"
  | "failed";

export type WiredTaskStatusView = {
  status: WiredTaskStatus;
  label: string;
  action: string | null;
  tone: "neutral" | "waiting" | "running" | "success" | "danger";
};

const statusViews: Record<WiredTaskStatus, WiredTaskStatusView> = {
  pending: {
    status: "pending",
    label: "pendente",
    action: null,
    tone: "neutral",
  },
  awaiting_worker: {
    status: "awaiting_worker",
    label: "aguardando worker",
    action: null,
    tone: "waiting",
  },
  dispatched: {
    status: "dispatched",
    label: "despachada",
    action: null,
    tone: "waiting",
  },
  running: {
    status: "running",
    label: "executando",
    action: null,
    tone: "running",
  },
  done: {
    status: "done",
    label: "concluida",
    action: null,
    tone: "success",
  },
  failed: {
    status: "failed",
    label: "falhou",
    action: "re-disparar",
    tone: "danger",
  },
};

export function normalizeWiredTaskStatus(status: string | null | undefined): WiredTaskStatus | null {
  const normalized = status?.trim();
  if (!normalized) return null;
  if (normalized === "waiting_approval" || normalized === "requested") return "pending";
  if (normalized === "approved") return "dispatched";
  if (normalized === "cancelled" || normalized === "rejected" || normalized === "expired") {
    return "failed";
  }
  return Object.prototype.hasOwnProperty.call(statusViews, normalized)
    ? (normalized as WiredTaskStatus)
    : null;
}

export function wiredTaskStatusView(status: string | null | undefined): WiredTaskStatusView | null {
  const normalized = normalizeWiredTaskStatus(status);
  return normalized ? statusViews[normalized] : null;
}

export function allWiredTaskStatusViews() {
  return Object.values(statusViews);
}
