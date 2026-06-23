import { describe, expect, it } from "vitest";
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
} from "./machines";
import type { WorkspaceMachine } from "./types";

const baseMachine: WorkspaceMachine = {
  id: "machine-1",
  workspace_id: 1,
  project_id: null,
  provider: "winbox",
  provider_runtime: "native",
  provider_profile: "dev",
  display_name: "Dev",
  preset_id: "ubuntu_server_lts",
  image_family: "linux_distro",
  access_user: "bruno",
  status: "stopped",
  web_port: null,
  rdp_port: null,
  ssh_port: null,
  last_health_status: null,
  last_health_summary: null,
  last_error_code: null,
  last_error_message: null,
  created_at: "2026-05-29T00:00:00Z",
  updated_at: "2026-05-29T00:00:00Z",
};

describe("machine helpers", () => {
  it("maps provider banner states", () => {
    expect(providerBannerState(null).tone).toBe("loading");
    expect(
      providerBannerState({
        provider: "winbox",
        runtime: "native",
        executable: "winbox",
        version: "1",
        status: "ready",
        message: "ok",
        hint: null,
      }).tone,
    ).toBe("ready");
  });

  it("sorts busy and failed machines before idle machines", () => {
    const sorted = sortMachines([
      { ...baseMachine, id: "stopped", status: "stopped", display_name: "B" },
      { ...baseMachine, id: "error", status: "error", display_name: "A" },
      { ...baseMachine, id: "running", status: "running", display_name: "C" },
    ]);
    expect(sorted.map((machine) => machine.id)).toEqual(["error", "running", "stopped"]);
  });

  it("appends and filters progress by selected machine", () => {
    const entries = appendMachineProgress([], {
      run_id: "run-1",
      machine_id: "machine-1",
      provider_profile: "dev",
      operation: "install",
      phase: "pull",
      status: "running",
      message: "Pulling",
      percent: 20,
      timestamp: "2026-05-29T00:00:00Z",
    });
    expect(progressForMachine(entries, baseMachine)).toHaveLength(1);
    expect(
      progressForMachine(entries, { ...baseMachine, id: "other", provider_profile: "other" }),
    ).toHaveLength(0);
  });

  it("labels unknown statuses defensively", () => {
    expect(machineStatusLabel("running")).toBe("running");
    expect(machineStatusLabel("strange")).toBe("unknown");
  });

  it("maps known provider error codes to actionable copy", () => {
    expect(machineErrorMessage("docker_daemon_unavailable: Docker info failed")).toContain(
      "Inicie o Docker",
    );
    expect(machineErrorMessage("unexpected failure")).toBe("unexpected failure");
  });

  it("builds Windows bootstrap and SSH guidance from the selected machine", () => {
    const machine = {
      ...baseMachine,
      provider_profile: "dw-3-windows-11-dev",
      image_family: "windows",
      preset_id: "windows_11",
      access_user: "bruno",
      ssh_port: 2223,
    };

    expect(machineAccessUser(machine)).toBe("bruno");
    expect(machineSshCommand(machine)).toBe("ssh -p 2223 bruno@127.0.0.1");
    expect(windowsSharedHostPath(machine)).toBe("/home/bruno/Windows/dw-3-windows-11-dev");
    expect(windowsBootstrapGuestPath()).toBe(String.raw`\\host.lan\Data\ade\bootstrap-windows.ps1`);
    expect(windowsBootstrapCommand()).toContain("bootstrap-windows.ps1");
    expect(windowsPostCreateMessage(machine)).toContain(":2223");
  });

  it("adds retry guidance to SSH probe failures", () => {
    expect(
      machineSshProbeMessage({
        machine_id: "machine-1",
        status: "not_ready",
        port: 2223,
        user: "bruno",
        command: "ssh -p 2223 bruno@127.0.0.1",
        message: "SSH ainda não respondeu em 127.0.0.1:2223",
      }),
    ).toContain("Rode o bootstrap Windows");
  });
});
