import { describe, expect, it } from "vitest";
import {
  activeVersionForStack,
  appendDeployProgress,
  canApproveVersion,
  canDeployVersion,
  canPrepareVersion,
  deployErrorMessage,
  deployEnvironmentSummary,
  deployFindingPathLabel,
  deployRepairInfo,
  deployReadiness,
  deployRunsForContext,
  deployStatusLabel,
  isLegacyDeployPackage,
  hasPassedPrepareRun,
  latestDeployRunForContext,
  latestVersion,
  parseBlockingFindings,
  parseDeployFindings,
  progressForDeploy,
  retryActionLabel,
  sortDeployStacks,
} from "./deploy";
import type {
  DeployEnvironment,
  DeployRun,
  DeployStack,
  DeployVersion,
  WorkspaceMachine,
} from "./types";

const baseStack: DeployStack = {
  id: "stack-1",
  workspace_id: 1,
  name: "Stack",
  slug: "stack",
  status: "idle",
  active_version_id: null,
  active_machine_id: null,
  created_at: "2026-05-29T00:00:00Z",
  updated_at: "2026-05-29T00:00:00Z",
};

const baseVersion: DeployVersion = {
  id: "version-1",
  stack_id: "stack-1",
  workspace_id: 1,
  label: "deploy-001",
  status: "review_required",
  target_machine_id: null,
  artifact_path: "/tmp/package",
  manifest_path: "/tmp/package/manifest.json",
  manifest_json: "{}",
  review_status: "pending",
  reviewed_at: null,
  blocking_findings_json: "[]",
  created_at: "2026-05-29T00:00:00Z",
  updated_at: "2026-05-29T00:00:00Z",
};

const baseMachine: WorkspaceMachine = {
  id: "machine-1",
  workspace_id: 1,
  project_id: null,
  provider: "winbox",
  provider_runtime: "native",
  provider_profile: "dev",
  display_name: "Dev",
  preset_id: "ubuntu_desktop_deploy_vm",
  image_family: "linux_cloud",
  access_user: "bruno",
  status: "running",
  web_port: null,
  rdp_port: null,
  ssh_port: 2222,
  last_health_status: null,
  last_health_summary: null,
  last_error_code: null,
  last_error_message: null,
  created_at: "2026-05-29T00:00:00Z",
  updated_at: "2026-05-29T00:00:00Z",
};

const readyEnvironment: DeployEnvironment = {
  version_id: "version-1",
  stack_id: "stack-1",
  machine_id: "machine-1",
  file_path: "/tmp/.env",
  ready: true,
  required_count: 1,
  saved_count: 1,
  missing_keys: [],
  variables: [
    {
      key: "DATABASE_URL",
      value: "postgres://local",
      placeholder: "postgres://user:password@postgres:5432/app",
      required: true,
      secret: true,
      saved: true,
    },
  ],
};

describe("deploy helpers", () => {
  it("tracks progress and filters by selected deploy context", () => {
    const entries = appendDeployProgress([], {
      run_id: "run-1",
      stack_id: "stack-1",
      version_id: "version-1",
      machine_id: "machine-1",
      step_key: "transfer",
      status: "passed",
      message: "copied",
      percent: 50,
      timestamp: "2026-05-29T00:00:00Z",
    });

    expect(progressForDeploy(entries, { stackId: "stack-1" })).toHaveLength(1);
    expect(progressForDeploy(entries, { versionId: "other" })).toHaveLength(0);
  });

  it("filters deploy runs by selected version and machine", () => {
    const oldPrepare: DeployRun = {
      id: "run-old",
      stack_id: "stack-1",
      version_id: "version-old",
      machine_id: "machine-1",
      operation: "prepare",
      status: "passed",
      started_at: "2026-05-29T00:00:00Z",
      completed_at: "2026-05-29T00:00:01Z",
      summary: "prepared",
      agent_profile_id: 1,
      agent_name: "Codex Yolo",
      agent_provider: "codex",
      agent_model: "gpt-5.5",
      orchestration_status: "passed",
      orchestration_report_json: "{}",
    };
    const currentDeploy: DeployRun = {
      ...oldPrepare,
      id: "run-current",
      version_id: "version-1",
      operation: "deploy",
      status: "failed",
      started_at: "2026-05-29T00:01:00Z",
      completed_at: null,
      summary: "failed",
      orchestration_status: "blocked",
    };

    const runs = [currentDeploy, oldPrepare];
    expect(deployRunsForContext(runs, "version-1", "machine-1")).toEqual([currentDeploy]);
    expect(latestDeployRunForContext(runs, "version-1", "machine-1")).toBe(currentDeploy);
    expect(hasPassedPrepareRun(runs, "version-1", "machine-1")).toBe(false);
    expect(hasPassedPrepareRun(runs, "version-old", "machine-1")).toBe(true);
  });

  it("derives review and deploy readiness", () => {
    expect(canApproveVersion(baseVersion)).toBe(true);
    expect(
      canApproveVersion({
        ...baseVersion,
        blocking_findings_json:
          '[{"path":".env","reason":"excluded","severity":"warning","blocking":false}]',
      }),
    ).toBe(true);
    expect(canApproveVersion({ ...baseVersion, blocking_findings_json: '[{"path":".env"}]' })).toBe(
      false,
    );
    expect(
      canApproveVersion({
        ...baseVersion,
        blocking_findings_json:
          '[{"path":"config.txt","reason":"secret-like content marker `api_key=`","severity":"error","blocking":true}]',
      }),
    ).toBe(false);
    expect(canDeployVersion({ ...baseVersion, review_status: "approved" }, baseMachine)).toBe(true);
    expect(canDeployVersion({ ...baseVersion, review_status: "approved" }, baseMachine, null)).toBe(
      false,
    );
    expect(
      canDeployVersion(
        { ...baseVersion, review_status: "approved" },
        baseMachine,
        readyEnvironment,
      ),
    ).toBe(true);
    expect(canDeployVersion({ ...baseVersion, review_status: "approved" }, null)).toBe(false);
    expect(canPrepareVersion({ ...baseVersion, review_status: "approved" }, baseMachine)).toBe(
      true,
    );
    expect(canPrepareVersion(baseVersion, baseMachine)).toBe(false);
    expect(
      canDeployVersion(
        { ...baseVersion, review_status: "approved" },
        { ...baseMachine, preset_id: "windows_11", image_family: "windows", ssh_port: 2223 },
        readyEnvironment,
      ),
    ).toBe(true);
    expect(
      canDeployVersion(
        { ...baseVersion, review_status: "approved" },
        { ...baseMachine, preset_id: "xubuntu_lts", image_family: "linux_distro" },
      ),
    ).toBe(false);
  });

  it("derives guided deploy readiness from package, environment, approval, and target", () => {
    expect(deployReadiness(null, null, null).nextStep).toBe("package");
    expect(deployReadiness(baseVersion, baseMachine, null).nextStep).toBe("environment");
    expect(deployReadiness(baseVersion, baseMachine, readyEnvironment).nextStep).toBe("approval");
    const approved = { ...baseVersion, review_status: "approved" };
    expect(deployReadiness(approved, baseMachine, readyEnvironment)).toMatchObject({
      environmentReady: true,
      packageApproved: true,
      targetReady: true,
      deployReady: true,
      nextStep: "deploy",
    });
    expect(
      deployEnvironmentSummary({
        ...readyEnvironment,
        ready: false,
        saved_count: 0,
        missing_keys: ["DATABASE_URL"],
      }),
    ).toBe("1 variável pendente");
  });

  it("parses deploy package findings with warning and legacy blocking semantics", () => {
    const version = {
      ...baseVersion,
      artifact_path: "/tmp/package",
      blocking_findings_json: JSON.stringify([
        {
          path: "/tmp/package/projects/app/source/.env",
          reason: "environment file excluded from package",
          severity: "warning",
          blocking: false,
        },
        { path: "/tmp/package/projects/app/source/config.txt", reason: "legacy" },
      ]),
    };
    const findings = parseDeployFindings(version);
    expect(findings.map((finding) => finding.blocking)).toEqual([false, true]);
    expect(parseBlockingFindings(version)).toHaveLength(1);
    expect(isLegacyDeployPackage(version)).toBe(true);
    expect(deployFindingPathLabel(findings[0], version.artifact_path)).toBe(
      "projects/app/source/.env",
    );
  });

  it("finds latest and active versions", () => {
    const older = { ...baseVersion, id: "older", label: "deploy-001" };
    const newer = {
      ...baseVersion,
      id: "newer",
      label: "deploy-002",
      created_at: "2026-05-29T01:00:00Z",
    };
    expect(latestVersion([older, newer])?.id).toBe("newer");
    expect(
      activeVersionForStack({ ...baseStack, active_version_id: "older" }, [older, newer])?.id,
    ).toBe("older");
  });

  it("sorts active and failed stacks before idle stacks", () => {
    const sorted = sortDeployStacks([
      { ...baseStack, id: "idle", status: "idle" },
      { ...baseStack, id: "failed", status: "failed" },
      { ...baseStack, id: "healthy", status: "healthy" },
    ]);
    expect(sorted.map((stack) => stack.id)).toEqual(["healthy", "failed", "idle"]);
  });

  it("maps labels, retry actions, and actionable errors", () => {
    const run: DeployRun = {
      id: "run-1",
      stack_id: "stack-1",
      version_id: "version-1",
      machine_id: "machine-1",
      operation: "prepare",
      status: "failed",
      started_at: "2026-05-29T00:00:00Z",
      completed_at: null,
      summary: "shared_dir_missing",
      agent_profile_id: 1,
      agent_name: "Codex Yolo",
      agent_provider: "codex",
      agent_model: "gpt-5.5",
      orchestration_status: "blocked",
      orchestration_report_json: "{}",
    };
    expect(deployStatusLabel("review_required")).toBe("review required");
    expect(retryActionLabel(run)).toBe("Retry prepare");
    expect(deployErrorMessage("shared_dir_missing: missing")).toContain("pasta compartilhada");
    expect(
      deployErrorMessage(
        "linux target preflight failed: ssh failed with exit status 255: kex_exchange_identification: read: Connection reset by peer",
      ),
    ).toContain("não aceitou comando SSH");
    expect(deployErrorMessage("ssh_unavailable: timeout")).toContain("não aceitou");
    expect(deployErrorMessage("windows_ssh_bootstrap_required: refused")).toContain(
      "bootstrap como Administrador",
    );
    expect(deployErrorMessage("remote mkdir failed")).toContain("valide SSH");
    expect(deployErrorMessage("linux_base_missing: docker failed")).toContain("Docker/Compose");
    expect(deployErrorMessage("unsupported_deploy_target: nope")).toContain(
      "Ubuntu Server Deploy VM, Ubuntu Desktop Deploy VM ou Windows 11",
    );
    expect(deployErrorMessage("deploy_project_selection_stale: project 3")).toContain(
      "lista de projetos mudou",
    );
    expect(deployErrorMessage("deploy_project_selection_empty: none")).toContain(
      "Não há projeto selecionado",
    );
    expect(deployErrorMessage("deploy_agent_required: none")).toContain("Selecione o agente");
    expect(deployErrorMessage("deploy_agent_timeout: slow")).toContain("demorou demais");
    expect(deployErrorMessage("deploy_agent_invalid_json: bad")).toContain("JSON válido");
    expect(deployErrorMessage("deploy_agent_failed: cli")).toContain("agente falhou");
    expect(deployErrorMessage("deploy_plan_required: none")).toContain("Rode a análise");
    expect(deployErrorMessage("deploy_plan_validation_failed: blocked")).toContain(
      "plano do agente",
    );
    expect(deployErrorMessage("deploy_agent_target_scope: server")).toContain("Ubuntu Server");
    expect(deployErrorMessage("windows_runbook_manual_required: no ps1")).toContain(
      "runbook PowerShell",
    );
    expect(deployErrorMessage("deploy_runbook_incomplete: scripts")).toContain("runbook");
    expect(
      deployErrorMessage(
        "linux_ssh_bootstrap_required: ssh failed",
      ),
    ).toContain("fluxo novo tenta instalar a base");
    expect(deployErrorMessage("deploy_repair_pending: patch")).toContain("versão corrigida");
    expect(deployErrorMessage("deploy_repair_not_pending: none")).toContain("correção pendente");
  });

  it("summarizes deploy doctor repair state", () => {
    const run: DeployRun = {
      id: "run-1",
      stack_id: "stack-1",
      version_id: "version-1",
      machine_id: "machine-1",
      operation: "deploy",
      status: "failed",
      started_at: "2026-05-29T00:00:00Z",
      completed_at: null,
      summary: "deploy_repair_pending: patch",
      agent_profile_id: 1,
      agent_name: "Codex Yolo",
      agent_provider: "codex",
      agent_model: "gpt-5.5",
      orchestration_status: "repair_pending",
      orchestration_report_json: JSON.stringify({
        decision: "repair_pending",
        repair: {
          max_attempts: 3,
          attempts: [
            {
              attempt: 1,
              error: "test -x node_modules/.bin/tauri failed",
              diagnosis: {
                code: "tauri_missing",
                title: "Tauri CLI dependency is missing",
              },
            },
          ],
          agent_repair: {
            status: "patch_proposed",
            safe_to_apply: false,
            patch_summary: "install npm deps before tauri check",
            user_message: "Crie a versão corrigida.",
            patch_set: [{ path: "scripts/build-dev.sh", body: "npm install" }],
          },
          validation: {
            ade_safe_to_apply: true,
            validation_status: "passed",
            validation_errors: [],
            patch_paths: ["scripts/build-dev.sh"],
          },
          agent_safe_to_apply: false,
          ade_safe_to_apply: true,
        },
      }),
    };

    expect(retryActionLabel(run)).toBe("Criar versão corrigida");
    expect(deployRepairInfo(run)).toMatchObject({
      status: "repair_pending",
      attemptCount: 1,
      maxAttempts: 3,
      latestCode: "tauri_missing",
      agentSafeToApply: false,
      adeSafeToApply: true,
      validationStatus: "passed",
      patchPending: true,
      patchSummary: "install npm deps before tauri check",
      patches: ["scripts/build-dev.sh"],
    });
  });

  it("keeps agent patch blocked when ADE validation fails", () => {
    const run: DeployRun = {
      id: "run-1",
      stack_id: "stack-1",
      version_id: "version-1",
      machine_id: "machine-1",
      operation: "deploy",
      status: "failed",
      started_at: "2026-05-29T00:00:00Z",
      completed_at: null,
      summary: "deploy_agent_repair_blocked: invalid patch",
      agent_profile_id: 1,
      agent_name: "Codex Yolo",
      agent_provider: "codex",
      agent_model: "gpt-5.5",
      orchestration_status: "blocked",
      orchestration_report_json: JSON.stringify({
        decision: "blocked",
        repair: {
          max_attempts: 3,
          attempts: [
            {
              attempt: 1,
              error: "runbook-install failed: docker : O termo 'docker' nao e reconhecido",
              diagnosis: {
                code: "docker_missing",
                title: "Docker is missing on the target",
              },
            },
          ],
          agent_repair: {
            status: "patch_proposed",
            safe_to_apply: true,
            patch_summary: "unsafe source edit",
            user_message: "Patch inseguro.",
            patch_set: [{ path: "../src/main.rs", body: "fn main() {}" }],
          },
          validation: {
            ade_safe_to_apply: false,
            validation_status: "blocked",
            validation_errors: ["patch path escapes allowed package scope: ../src/main.rs"],
            patch_paths: ["../src/main.rs"],
          },
          agent_safe_to_apply: true,
          ade_safe_to_apply: false,
        },
      }),
    };

    expect(retryActionLabel(run)).toBe("Retry deploy");
    expect(deployRepairInfo(run)).toMatchObject({
      status: "blocked",
      latestCode: "docker_missing",
      agentSafeToApply: true,
      adeSafeToApply: false,
      validationStatus: "blocked",
      validationErrors: ["patch path escapes allowed package scope: ../src/main.rs"],
      patchPending: false,
    });
  });
});
