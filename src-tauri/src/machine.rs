use crate::store;
use crate::winbox_provider::{
    check_status, discover, ensure_supported_distro, redacted_args, MachineError,
    MachineProviderStatus, WinboxDistro, WinboxProfile, WinboxProgress, WinboxProvider,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize)]
pub struct MachinePreset {
    pub id: String,
    pub label: String,
    pub image_family: String,
    pub boot: Option<String>,
    pub cloud_init_profile: Option<String>,
    pub version: Option<String>,
    pub default_ram: String,
    pub default_cpu: String,
    pub default_disk: String,
    pub deploy_capable: bool,
    pub supported: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkspaceMachineInput {
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub preset_id: String,
    pub display_name: String,
    pub provider_profile: Option<String>,
    pub ram: Option<String>,
    pub cpu: Option<String>,
    pub disk: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoveWorkspaceMachineInput {
    pub machine_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetWorkspaceMachinePasswordInput {
    pub machine_id: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineViewer {
    pub machine_id: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineSshProbe {
    pub machine_id: String,
    pub status: String,
    pub port: Option<i64>,
    pub user: String,
    pub command: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineProgressEvent {
    pub run_id: String,
    pub machine_id: Option<String>,
    pub provider_profile: String,
    pub operation: String,
    pub phase: String,
    pub status: String,
    pub message: String,
    pub percent: Option<u8>,
    pub timestamp: String,
}

pub fn provider_status() -> MachineProviderStatus {
    check_status()
}

pub fn presets() -> Vec<MachinePreset> {
    vec![
        MachinePreset {
            id: "ubuntu_deploy_vm".to_string(),
            label: "Ubuntu Server Deploy VM".to_string(),
            image_family: "linux_cloud".to_string(),
            boot: None,
            cloud_init_profile: Some("server".to_string()),
            version: Some("24.04".to_string()),
            default_ram: "4G".to_string(),
            default_cpu: "2".to_string(),
            default_disk: "64G".to_string(),
            deploy_capable: true,
            supported: true,
            disabled_reason: None,
        },
        MachinePreset {
            id: "ubuntu_desktop_deploy_vm".to_string(),
            label: "Ubuntu Desktop Deploy VM".to_string(),
            image_family: "linux_cloud".to_string(),
            boot: None,
            cloud_init_profile: Some("xubuntu-desktop".to_string()),
            version: Some("24.04".to_string()),
            default_ram: "6G".to_string(),
            default_cpu: "2".to_string(),
            default_disk: "80G".to_string(),
            deploy_capable: true,
            supported: true,
            disabled_reason: None,
        },
        MachinePreset {
            id: "windows_11".to_string(),
            label: "Windows 11".to_string(),
            image_family: "windows".to_string(),
            boot: None,
            cloud_init_profile: None,
            version: Some("11".to_string()),
            default_ram: "8G".to_string(),
            default_cpu: "4".to_string(),
            default_disk: "128G".to_string(),
            deploy_capable: false,
            supported: true,
            disabled_reason: None,
        },
    ]
}

pub fn list_machines(
    db: &store::Database,
    workspace_id: i64,
) -> anyhow::Result<Vec<store::WorkspaceMachine>> {
    let machines = db.list_workspace_machines(workspace_id)?;
    let Some(provider) = discover() else {
        return Ok(machines);
    };
    let profiles = match provider.list() {
        Ok(profiles) => profiles,
        Err(_) => return Ok(machines),
    };
    let reconciled = reconcile_with_provider_profiles(db, machines, &profiles)?;
    adopt_workspace_provider_profiles(db, workspace_id, provider.runtime(), reconciled, &profiles)
}

pub fn create_machine(
    app: &tauri::AppHandle,
    db: &store::Database,
    input: CreateWorkspaceMachineInput,
) -> anyhow::Result<store::WorkspaceMachine> {
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let distros = provider.distros().unwrap_or_default();
    let preset = resolve_preset(&input.preset_id, &distros).map_err(machine_error_to_anyhow)?;
    let access_user = validate_machine_credentials(&input, &preset)?;
    let profile_name = input
        .provider_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| machine_profile_name(input.workspace_id, &input.display_name));
    if let Some(existing) =
        db.find_workspace_machine_by_profile(input.workspace_id, "winbox", &profile_name)?
    {
        match provider.profile(&profile_name) {
            Ok(profile) => {
                let updated = update_machine_from_profile(db, &existing.id, &profile, None)?;
                materialize_windows_shared_scripts(&profile, existing.access_user.as_deref())?;
                return Ok(updated);
            }
            Err(error) if is_profile_not_found(&error) => {
                db.delete_workspace_machine(&existing.id)?;
            }
            Err(_) => return Ok(existing),
        }
    }

    match provider.profile(&profile_name) {
        Ok(profile) if profile_matches_preset(&profile, &preset) => {
            let machine = create_machine_record(
                db,
                provider.runtime(),
                &input,
                &profile_name,
                &preset,
                &access_user,
                normalize_status(&profile.status),
            )?;
            let updated = update_machine_from_profile(db, &machine.id, &profile, None)?;
            materialize_windows_shared_scripts(&profile, Some(&access_user))?;
            return Ok(updated);
        }
        Ok(_) => {
            return Err(anyhow!(
                "provider_profile_conflict: Winbox profile '{}' already exists with a different preset. Use another technical ID or remove the Winbox profile.",
                profile_name
            ));
        }
        Err(error) if is_profile_not_found(&error) => {}
        Err(error) => return Err(machine_error_to_anyhow(error)),
    }

    let machine = create_machine_record(
        db,
        provider.runtime(),
        &input,
        &profile_name,
        &preset,
        &access_user,
        "creating",
    )?;
    let args = build_install_args(&profile_name, &preset, &input, &access_user);
    let run_id = new_run_id();
    let machine_id = machine.id.clone();
    let app_handle = app.clone();
    let profile_for_event = profile_name.clone();
    if let Err(mut error) = provider.install(&args, |progress| {
        emit_progress(
            &app_handle,
            &run_id,
            Some(&machine_id),
            &profile_for_event,
            progress,
        );
    }) {
        let _ = update_machine_from_profile(
            db,
            &machine.id,
            &WinboxProfile {
                name: profile_name.clone(),
                status: "error".to_string(),
                web_port: None,
                rdp_port: None,
                ssh_port: None,
                ram: None,
                bundles: None,
                image_family: Some(preset.image_family.clone()),
                boot: preset.boot.clone(),
                cloud_init_profile: preset.cloud_init_profile.clone(),
                iso_path: None,
                shared_dir: None,
            },
            Some(&error),
        );
        error.provider_detail = Some(format!("winbox {}", redacted_args(&args).join(" ")));
        return Err(machine_error_to_anyhow(error));
    }
    let mut profile = provider
        .profile(&profile_name)
        .map_err(machine_error_to_anyhow)?;
    if preset.image_family == "windows" {
        match ensure_windows_ssh_forwarding(&provider, &profile) {
            Ok(updated_profile) => profile = updated_profile,
            Err(error) => {
                let mut error_profile = profile.clone();
                error_profile.status = "error".to_string();
                let _ = update_machine_from_profile(db, &machine.id, &error_profile, Some(&error));
                return Err(machine_error_to_anyhow(error));
            }
        }
    }
    if !profile_matches_preset(&profile, &preset) {
        let machine_error = provider_profile_mismatch_error(&profile, &preset);
        let mut error_profile = profile.clone();
        error_profile.status = "error".to_string();
        let _ = update_machine_from_profile(db, &machine.id, &error_profile, Some(&machine_error));
        return Err(machine_error_to_anyhow(machine_error));
    }
    let updated = update_machine_from_profile(db, &machine.id, &profile, None)?;
    materialize_windows_shared_scripts(&profile, Some(&access_user))?;
    if preset.id == "ubuntu_desktop_deploy_vm" {
        if let Some(password) = input
            .password
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let target = validate_password_change_fields(
                &preset.id,
                updated.ssh_port,
                Some(&access_user),
                password,
            )?;
            if let Err(error) = run_linux_desktop_setup(&target) {
                let machine_error = desktop_setup_failed_error(&error);
                let mut error_profile = profile.clone();
                error_profile.status = "error".to_string();
                let _ = update_machine_from_profile(
                    db,
                    &machine.id,
                    &error_profile,
                    Some(&machine_error),
                );
                return Err(machine_error_to_anyhow(machine_error));
            }
            let profile = provider
                .profile(&profile_name)
                .map_err(machine_error_to_anyhow)?;
            return update_machine_from_profile(db, &machine.id, &profile, None);
        }
    }
    Ok(updated)
}

pub fn refresh_machine(
    db: &store::Database,
    machine_id: &str,
) -> anyhow::Result<store::WorkspaceMachine> {
    let machine = db.get_workspace_machine(machine_id)?;
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let profile = provider
        .profile(&machine.provider_profile)
        .map_err(machine_error_to_anyhow)?;
    if should_preserve_desktop_setup_error(&machine) {
        let updated = update_machine_from_profile_preserving_error(db, &machine, &profile)?;
        materialize_windows_shared_scripts(&profile, machine.access_user.as_deref())?;
        return Ok(updated);
    }
    let updated = update_machine_from_profile(db, machine_id, &profile, None)?;
    materialize_windows_shared_scripts(&profile, machine.access_user.as_deref())?;
    Ok(updated)
}

pub fn start_machine(
    app: &tauri::AppHandle,
    db: &store::Database,
    machine_id: &str,
) -> anyhow::Result<store::WorkspaceMachine> {
    let machine = db.get_workspace_machine(machine_id)?;
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let run_id = new_run_id();
    let app_handle = app.clone();
    let machine_id_owned = machine.id.clone();
    let profile = machine.provider_profile.clone();
    provider
        .start(&profile, |progress| {
            emit_progress(
                &app_handle,
                &run_id,
                Some(&machine_id_owned),
                &profile,
                progress,
            );
        })
        .map_err(machine_error_to_anyhow)?;
    refresh_machine(db, machine_id)
}

pub fn stop_machine(
    db: &store::Database,
    machine_id: &str,
) -> anyhow::Result<store::WorkspaceMachine> {
    let machine = db.get_workspace_machine(machine_id)?;
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    provider
        .stop(&machine.provider_profile)
        .map_err(machine_error_to_anyhow)?;
    refresh_machine(db, machine_id)
}

pub fn set_machine_password(
    db: &store::Database,
    input: SetWorkspaceMachinePasswordInput,
) -> anyhow::Result<store::WorkspaceMachine> {
    let machine = db.get_workspace_machine(&input.machine_id)?;
    let target = validate_password_change_fields(
        &machine.preset_id,
        machine.ssh_port,
        machine.access_user.as_deref(),
        &input.password,
    )?;
    if let Err(error) = run_linux_desktop_setup(&target) {
        let machine_error = desktop_setup_failed_error(&error);
        let _ = update_machine_to_error(db, &machine, &machine_error);
        return Err(machine_error_to_anyhow(machine_error));
    }
    db.update_workspace_machine(store::WorkspaceMachineUpdate {
        id: &machine.id,
        status: if machine.status == "error" {
            "running"
        } else {
            machine.status.as_str()
        },
        web_port: machine.web_port,
        rdp_port: machine.rdp_port,
        ssh_port: machine.ssh_port,
        last_health_status: machine.last_health_status.as_deref(),
        last_health_summary: machine.last_health_summary.as_deref(),
        last_error_code: None,
        last_error_message: None,
    })
}

pub fn probe_machine_ssh(
    db: &store::Database,
    machine_id: &str,
) -> anyhow::Result<MachineSshProbe> {
    let machine = db.get_workspace_machine(machine_id)?;
    let probe = build_machine_ssh_probe(&machine);
    let health_status = match probe.status.as_str() {
        "ready" => "healthy",
        "missing_port" => "warning",
        _ => "failed",
    };
    db.update_workspace_machine(store::WorkspaceMachineUpdate {
        id: &machine.id,
        status: &machine.status,
        web_port: machine.web_port,
        rdp_port: machine.rdp_port,
        ssh_port: machine.ssh_port,
        last_health_status: Some(health_status),
        last_health_summary: Some(probe.message.as_str()),
        last_error_code: machine.last_error_code.as_deref(),
        last_error_message: machine.last_error_message.as_deref(),
    })?;
    Ok(probe)
}

pub fn open_machine(
    app: &tauri::AppHandle,
    db: &store::Database,
    machine_id: &str,
) -> anyhow::Result<MachineViewer> {
    let machine = db.get_workspace_machine(machine_id)?;
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let profile = provider
        .profile(&machine.provider_profile)
        .map_err(machine_error_to_anyhow)?;
    let mut machine = update_machine_from_profile(db, machine_id, &profile, None)?;
    let run_id = new_run_id();
    if machine.status != "running" {
        let app_handle = app.clone();
        let machine_id_owned = machine.id.clone();
        let profile_name = machine.provider_profile.clone();
        provider
            .start(&profile_name, |progress| {
                emit_progress(
                    &app_handle,
                    &run_id,
                    Some(&machine_id_owned),
                    &profile_name,
                    progress,
                );
            })
            .map_err(machine_error_to_anyhow)?;
        let profile = provider
            .profile(&machine.provider_profile)
            .map_err(machine_error_to_anyhow)?;
        machine = update_machine_from_profile(db, machine_id, &profile, None)?;
    }
    let url = viewer_url_for_machine(&provider, &machine)?;
    open_external_url(&url)?;
    Ok(MachineViewer {
        machine_id: machine.id,
        url,
    })
}

pub fn machine_logs(
    db: &store::Database,
    machine_id: &str,
    tail: Option<u32>,
) -> anyhow::Result<String> {
    let machine = db.get_workspace_machine(machine_id)?;
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let logs = provider
        .logs(&machine.provider_profile, tail.unwrap_or(200).min(2000))
        .map_err(machine_error_to_anyhow)?;
    Ok(logs.logs)
}

pub fn remove_machine(
    db: &store::Database,
    input: RemoveWorkspaceMachineInput,
) -> anyhow::Result<()> {
    let machine = db.get_workspace_machine(&input.machine_id)?;
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let remove_result = provider.remove(&machine.provider_profile).map(|_| ());
    remove_local_after_provider_remove(db, &input.machine_id, remove_result)
}

pub fn health_summary() -> anyhow::Result<String> {
    let provider = discover().ok_or_else(|| anyhow!("Winbox CLI was not found."))?;
    let health = provider.host_health().map_err(machine_error_to_anyhow)?;
    Ok(serde_json::to_string_pretty(&health)?)
}

fn resolve_preset(id: &str, distros: &[WinboxDistro]) -> Result<MachinePreset, MachineError> {
    let preset = presets()
        .into_iter()
        .find(|preset| preset.id == id)
        .ok_or_else(|| {
            MachineError::from_message("unsupported_preset", "Unsupported machine preset.")
        })?;
    if let Some(boot) = preset.boot.as_deref() {
        ensure_supported_distro(distros, boot)?;
    }
    Ok(preset)
}

fn validate_machine_credentials(
    input: &CreateWorkspaceMachineInput,
    preset: &MachinePreset,
) -> anyhow::Result<String> {
    let requested_user = input.user.as_deref().unwrap_or("").trim();
    let requested_password = input.password.as_deref().unwrap_or("").trim();

    if preset.image_family == "windows" {
        if requested_user.is_empty() {
            anyhow::bail!("credentials_required: Windows 11 requires a user.");
        }
        if requested_password.is_empty() {
            anyhow::bail!("credentials_required: Windows 11 requires a password.");
        }
        return Ok(requested_user.to_string());
    }

    if preset.id == "ubuntu_deploy_vm" {
        if !requested_password.is_empty() {
            anyhow::bail!(
                "password_not_supported: Ubuntu Server Deploy VM uses SSH keys; do not provide a password."
            );
        }
        return Ok(if requested_user.is_empty() {
            "bruno".to_string()
        } else {
            requested_user.to_string()
        });
    }

    if preset.id == "ubuntu_desktop_deploy_vm" {
        if requested_password.is_empty() {
            anyhow::bail!(
                "credentials_required: Ubuntu Desktop Deploy VM requires a password for graphical login and RDP."
            );
        }
        return Ok(if requested_user.is_empty() {
            "bruno".to_string()
        } else {
            requested_user.to_string()
        });
    }

    Ok(requested_user.to_string())
}

fn build_install_args(
    profile_name: &str,
    preset: &MachinePreset,
    input: &CreateWorkspaceMachineInput,
    access_user: &str,
) -> Vec<String> {
    let mut args = vec![
        "install".to_string(),
        "--yes".to_string(),
        "--family".to_string(),
        preset.image_family.clone(),
        "--ram".to_string(),
        input
            .ram
            .clone()
            .unwrap_or_else(|| preset.default_ram.clone()),
        "--cpu".to_string(),
        input
            .cpu
            .clone()
            .unwrap_or_else(|| preset.default_cpu.clone()),
        "--disk".to_string(),
        input
            .disk
            .clone()
            .unwrap_or_else(|| preset.default_disk.clone()),
        "--progress".to_string(),
        "jsonl".to_string(),
    ];
    if let Some(version) = &preset.version {
        args.extend(["--version".to_string(), version.clone()]);
    }
    if let Some(boot) = &preset.boot {
        args.extend(["--boot".to_string(), boot.clone()]);
    }
    if let Some(cloud_init_profile) = &preset.cloud_init_profile {
        args.extend(["--boot".to_string(), cloud_init_profile.clone()]);
    }
    args.extend(["--user".to_string(), access_user.to_string()]);
    if let Some(password) = input
        .password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.extend(["--pass".to_string(), password.to_string()]);
    }
    args.push(profile_name.to_string());
    args
}

fn windows_ssh_extra_ports(profile: &WinboxProfile) -> Option<String> {
    if profile.image_family.as_deref() != Some("windows") {
        return None;
    }
    let ssh_port = profile.ssh_port?;
    if ssh_port <= 0 || ssh_port > u16::MAX as i64 {
        return None;
    }
    Some(format!("{ssh_port}:22/tcp"))
}

fn ensure_windows_ssh_forwarding(
    provider: &WinboxProvider,
    profile: &WinboxProfile,
) -> Result<WinboxProfile, MachineError> {
    let Some(extra_ports) = windows_ssh_extra_ports(profile) else {
        return Ok(profile.clone());
    };
    provider.set_extra_ports(&profile.name, &extra_ports, true)?;
    provider.profile(&profile.name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PasswordChangeTarget {
    ssh_port: i64,
    access_user: String,
    password: String,
}

fn validate_password_change_fields(
    preset_id: &str,
    ssh_port: Option<i64>,
    access_user: Option<&str>,
    password: &str,
) -> anyhow::Result<PasswordChangeTarget> {
    if preset_id != "ubuntu_desktop_deploy_vm" {
        anyhow::bail!(
            "password_not_supported: password changes are only supported for Ubuntu Desktop Deploy VM."
        );
    }
    let ssh_port = ssh_port.ok_or_else(|| {
        anyhow!("ssh_port_missing: Ubuntu Desktop VM does not have an SSH port yet.")
    })?;
    let password = password.trim();
    if password.is_empty() {
        anyhow::bail!("credentials_required: graphical/RDP password is required.");
    }
    if password.contains(['\n', '\r', '\0']) {
        anyhow::bail!("invalid_password: password cannot contain line breaks or null bytes.");
    }
    let access_user = access_user
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("bruno")
        .to_string();
    Ok(PasswordChangeTarget {
        ssh_port,
        access_user,
        password: password.to_string(),
    })
}

fn run_linux_desktop_setup(target: &PasswordChangeTarget) -> anyhow::Result<()> {
    let host_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow!("host_clock_invalid: {error}"))?
        .as_secs();
    let script = linux_desktop_setup_script(host_epoch);
    run_ssh_password_script(target, &script)
}

fn linux_desktop_setup_script(host_epoch: u64) -> String {
    r#"set -eu
HOST_EPOCH=__HOST_EPOCH__
sync_guest_clock() {
  if command -v timedatectl >/dev/null 2>&1; then
    sudo timedatectl set-ntp true >/dev/null 2>&1 || true
  fi
  sudo systemctl restart systemd-timesyncd >/dev/null 2>&1 || true
  for _ in 1 2 3 4 5; do
    if command -v timedatectl >/dev/null 2>&1 && \
       timedatectl show -p NTPSynchronized --value 2>/dev/null | grep -qi '^yes$'; then
      break
    fi
    sleep 2
  done
  now_epoch="$(date -u +%s 2>/dev/null || echo 0)"
  skew=$((now_epoch - HOST_EPOCH))
  if [ "$skew" -lt -120 ] || [ "$skew" -gt 120 ]; then
    sudo date -u -s "@$HOST_EPOCH" >/dev/null 2>&1 || true
    sudo hwclock -w >/dev/null 2>&1 || true
    sudo systemctl restart systemd-timesyncd >/dev/null 2>&1 || true
  fi
}
apt_log_has_retryable_lock() {
  grep -Eiq 'could not get lock|unable to lock directory|is held by process|waiting for cache lock|dpkg frontend lock|dpkg lock' "$1"
}
apt_update_with_retry() {
  log="$(mktemp)"
  attempt=1
  while [ "$attempt" -le 90 ]; do
    if sudo apt-get update >"$log" 2>&1; then
      rm -f "$log"
      return 0
    fi
    if grep -qi "not valid yet" "$log"; then
      echo "[dw] apt repository metadata is newer than guest clock; syncing clock and retrying ($attempt/90)" >&2
      sync_guest_clock
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    if apt_log_has_retryable_lock "$log"; then
      echo "[dw] apt is busy; waiting for package manager lock ($attempt/90)" >&2
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    cat "$log" >&2
    rm -f "$log"
    return 1
  done
  cat "$log" >&2
  rm -f "$log"
  return 1
}
apt_install_with_retry() {
  log="$(mktemp)"
  attempt=1
  while [ "$attempt" -le 90 ]; do
    if sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "$@" >"$log" 2>&1; then
      rm -f "$log"
      return 0
    fi
    if apt_log_has_retryable_lock "$log"; then
      echo "[dw] apt is busy; waiting for package manager lock ($attempt/90)" >&2
      sleep 10
      attempt=$((attempt + 1))
      continue
    fi
    cat "$log" >&2
    rm -f "$log"
    return 1
  done
  cat "$log" >&2
  rm -f "$log"
  return 1
}
sudo chpasswd
sync_guest_clock
sudo passwd -u "$USER" >/dev/null 2>&1 || true
sudo usermod -c "$USER" "$USER" >/dev/null 2>&1 || true
if command -v systemctl >/dev/null 2>&1; then
  if ! command -v startxfce4 >/dev/null 2>&1 || \
     ! systemctl list-unit-files lightdm.service >/dev/null 2>&1 || \
	     ! systemctl list-unit-files xrdp.service >/dev/null 2>&1 || \
	     ! dpkg -s lightdm-gtk-greeter >/dev/null 2>&1 || \
	     ! dpkg -s accountsservice >/dev/null 2>&1 || \
	     ! dpkg -s xfce4-terminal >/dev/null 2>&1 || \
	     ! dpkg -s xterm >/dev/null 2>&1 || \
	     ! dpkg -s xserver-xorg-input-libinput >/dev/null 2>&1; then
	    export DEBIAN_FRONTEND=noninteractive
	    apt_update_with_retry
	    apt_install_with_retry xfce4 xfce4-terminal xterm lightdm lightdm-gtk-greeter accountsservice xrdp xorgxrdp xserver-xorg-input-libinput dbus-x11 policykit-1 x11-xserver-utils
	  fi
  session=xfce
  if [ -f /usr/share/xsessions/xubuntu.desktop ]; then
    session=xubuntu
  fi
  sudo install -d -m 0755 /etc/lightdm/lightdm.conf.d /var/lib/AccountsService/users
  cat <<EOF | sudo tee /etc/lightdm/lightdm.conf.d/50-dev-workflow-login.conf >/dev/null
[Seat:*]
greeter-session=lightdm-gtk-greeter
greeter-show-manual-login=true
greeter-hide-users=false
allow-guest=false
user-session=$session
EOF
  cat <<EOF | sudo tee "/var/lib/AccountsService/users/$USER" >/dev/null
[User]
XSession=$session
SystemAccount=false
EOF
  printf '%s\n' startxfce4 > "$HOME/.xsession"
  chmod 600 "$HOME/.xsession" >/dev/null 2>&1 || true
  if [ "$USER" != "ubuntu" ]; then
    cat <<'EOF' | sudo tee /var/lib/AccountsService/users/ubuntu >/dev/null
[User]
SystemAccount=true
EOF
    sudo sed -i 's/^hidden-users=.*/hidden-users=nobody nobody4 noaccess ubuntu/' /etc/lightdm/users.conf >/dev/null 2>&1 || true
  fi
  sudo systemctl restart accounts-daemon >/dev/null 2>&1 || true
  sudo systemctl set-default graphical.target >/dev/null 2>&1 || true
  if systemctl list-unit-files lightdm.service >/dev/null 2>&1; then
    printf '%s\n' /usr/sbin/lightdm | sudo tee /etc/X11/default-display-manager >/dev/null || true
    sudo systemctl enable lightdm >/dev/null 2>&1 || true
    sudo systemctl stop lightdm display-manager >/dev/null 2>&1 || true
    sudo systemctl restart systemd-logind >/dev/null 2>&1 || true
    sudo systemctl restart dbus >/dev/null 2>&1 || true
    sleep 2
    sudo systemctl reset-failed lightdm display-manager >/dev/null 2>&1 || true
    sudo systemctl restart lightdm >/dev/null 2>&1 || sudo systemctl restart display-manager >/dev/null 2>&1 || true
  fi
  sudo systemctl enable xrdp >/dev/null 2>&1 || true
  sudo systemctl restart xrdp >/dev/null 2>&1 || true
fi
systemctl --no-pager --plain is-active lightdm xrdp >/dev/null 2>&1 || true
"#
    .replace("__HOST_EPOCH__", &host_epoch.to_string())
}

fn run_ssh_password_script(
    target: &PasswordChangeTarget,
    remote_script: &str,
) -> anyhow::Result<()> {
    let mut last_detail = String::new();
    for attempt in 0..12 {
        let output = run_ssh_password_script_once(target, remote_script)?;
        if output.status.success() {
            return Ok(());
        }
        last_detail = ssh_output_detail(&output);
        if attempt < 11 && is_retryable_desktop_setup_failure(&last_detail) {
            thread::sleep(Duration::from_secs(5));
            continue;
        }
        break;
    }
    let detail = if last_detail.trim().is_empty() {
        "ssh desktop setup command failed".to_string()
    } else {
        last_detail.chars().take(2000).collect()
    };
    anyhow::bail!("desktop_setup_failed: {detail}");
}

fn run_ssh_password_script_once(
    target: &PasswordChangeTarget,
    remote_script: &str,
) -> anyhow::Result<std::process::Output> {
    let mut child = Command::new("ssh")
        .arg("-p")
        .arg(target.ssh_port.to_string())
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg(format!("{}@127.0.0.1", target.access_user))
        .arg(remote_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("ssh_stdin_unavailable: could not write password update."))?;
        stdin.write_all(format!("{}:{}\n", target.access_user, target.password).as_bytes())?;
    }
    drop(child.stdin.take());
    Ok(child.wait_with_output()?)
}

fn ssh_output_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    [stderr.trim(), stdout.trim()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_transient_ssh_failure(detail: &str) -> bool {
    let lower = detail.to_lowercase();
    lower.contains("connection refused")
        || lower.contains("connection timed out")
        || lower.contains("no route to host")
        || lower.contains("connection reset")
        || lower.contains("host is down")
}

fn is_retryable_desktop_setup_failure(detail: &str) -> bool {
    is_transient_ssh_failure(detail)
        || is_apt_release_not_valid_yet(detail)
        || is_apt_lock_unavailable(detail)
}

fn is_apt_release_not_valid_yet(detail: &str) -> bool {
    let lower = detail.to_lowercase();
    lower.contains("release file") && lower.contains("not valid yet")
}

fn is_apt_lock_unavailable(detail: &str) -> bool {
    let lower = detail.to_lowercase();
    lower.contains("could not get lock")
        || lower.contains("unable to lock directory")
        || lower.contains("is held by process")
        || lower.contains("waiting for cache lock")
        || lower.contains("dpkg frontend lock")
        || lower.contains("dpkg lock")
}

enum SshProbeCheck {
    Ready { banner: String },
    NotReady { detail: String },
    MissingPort,
}

fn build_machine_ssh_probe(machine: &store::WorkspaceMachine) -> MachineSshProbe {
    let user = machine_access_user(machine);
    let command = machine
        .ssh_port
        .map(|port| format!("ssh -p {port} {user}@127.0.0.1"));
    ssh_probe_from_check(
        &machine.id,
        &user,
        machine.ssh_port,
        command,
        check_ssh_port(machine.ssh_port),
    )
}

fn ssh_probe_from_check(
    machine_id: &str,
    user: &str,
    port: Option<i64>,
    command: Option<String>,
    check: SshProbeCheck,
) -> MachineSshProbe {
    match check {
        SshProbeCheck::Ready { banner } => MachineSshProbe {
            machine_id: machine_id.to_string(),
            status: "ready".to_string(),
            port,
            user: user.to_string(),
            command,
            message: format!(
                "SSH pronto em 127.0.0.1:{} ({})",
                port.map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                banner.trim()
            ),
        },
        SshProbeCheck::NotReady { detail } => MachineSshProbe {
            machine_id: machine_id.to_string(),
            status: "not_ready".to_string(),
            port,
            user: user.to_string(),
            command,
            message: format!(
                "SSH ainda não respondeu em 127.0.0.1:{}: {}",
                port.map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                detail
            ),
        },
        SshProbeCheck::MissingPort => MachineSshProbe {
            machine_id: machine_id.to_string(),
            status: "missing_port".to_string(),
            port: None,
            user: user.to_string(),
            command: None,
            message: "WinBox não expôs uma porta SSH para esta máquina.".to_string(),
        },
    }
}

fn check_ssh_port(port: Option<i64>) -> SshProbeCheck {
    let Some(port) = port else {
        return SshProbeCheck::MissingPort;
    };
    let Ok(port) = u16::try_from(port) else {
        return SshProbeCheck::NotReady {
            detail: "porta SSH inválida".to_string(),
        };
    };
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = match TcpStream::connect_timeout(&address, Duration::from_secs(3)) {
        Ok(stream) => stream,
        Err(error) => {
            return SshProbeCheck::NotReady {
                detail: error.to_string(),
            };
        }
    };
    if let Err(error) = stream.set_read_timeout(Some(Duration::from_secs(3))) {
        return SshProbeCheck::NotReady {
            detail: error.to_string(),
        };
    }
    let mut buffer = [0_u8; 256];
    match stream.read(&mut buffer) {
        Ok(0) => SshProbeCheck::NotReady {
            detail: "conexão fechada antes do banner SSH".to_string(),
        },
        Ok(size) => {
            let banner = String::from_utf8_lossy(&buffer[..size])
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            if ssh_banner_is_ready(&banner) {
                SshProbeCheck::Ready { banner }
            } else {
                SshProbeCheck::NotReady {
                    detail: format!("banner inesperado: {banner}"),
                }
            }
        }
        Err(error) => SshProbeCheck::NotReady {
            detail: error.to_string(),
        },
    }
}

fn ssh_banner_is_ready(banner: &str) -> bool {
    banner.trim_start().starts_with("SSH-")
}

fn viewer_url_for_machine(
    provider: &WinboxProvider,
    machine: &store::WorkspaceMachine,
) -> anyhow::Result<String> {
    let (provider_url, provider_error) = match provider.viewer_url(&machine.provider_profile) {
        Ok(viewer) => (non_empty_url(&viewer.url), None),
        Err(error) => (
            None,
            Some(format!("viewer-url failed: {}", error.message.trim())),
        ),
    };
    let urls = viewer_candidate_urls(
        machine.web_port,
        docker_novnc_port(&machine.provider_profile),
        provider_url.as_deref(),
    );
    for url in urls.iter() {
        if viewer_url_responds(url) {
            return Ok(url.clone());
        }
    }
    let candidates = describe_candidate_urls(&urls);
    let provider_error = provider_error
        .map(|error| format!(" {error}."))
        .unwrap_or_default();
    anyhow::bail!(
        "viewer_unavailable: noVNC is not responding for profile {}. Tried URLs: {}.{}",
        machine.provider_profile,
        candidates,
        provider_error
    )
}

/// Open a URL in the system browser robustly: tries several openers and checks
/// each child's exit status, so a failure surfaces as an error instead of being
/// swallowed by a fire-and-forget `spawn()`. Public so other modules (e.g.
/// `cloud`) share this single implementation.
pub fn open_external_url(url: &str) -> anyhow::Result<()> {
    // `xdg-open`/`gio` honor $BROWSER before the desktop default. A stale value
    // (e.g. a Windows/WSL path like `/mnt/c/.../chrome.exe` that doesn't exist
    // here) makes the open silently fail. Drop it for the child so the real
    // default browser is used.
    let drop_browser = browser_env_is_broken();
    let attempts = [
        ("xdg-open", vec![url.to_string()]),
        ("wslview", vec![url.to_string()]),
        ("gio", vec!["open".to_string(), url.to_string()]),
        (
            "cmd.exe",
            vec![
                "/C".to_string(),
                "start".to_string(),
                "".to_string(),
                url.to_string(),
            ],
        ),
    ];
    let mut failures = Vec::new();
    for (program, args) in attempts {
        let mut command = Command::new(program);
        command
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if drop_browser {
            command.env_remove("BROWSER");
        }
        let output = match command.output() {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                failures.push(format!("{program}: {error}"));
                continue;
            }
        };
        if output.status.success() {
            return Ok(());
        }
        failures.push(format!("{}: {}", program, ssh_output_detail(&output)));
    }
    anyhow::bail!(
        "open_url_failed: could not open {url}. {}",
        failures.join(" | ")
    );
}

/// True when `$BROWSER` is set to an absolute path that doesn't exist (a stale
/// Windows/WSL value is the usual culprit). Bare command names are left alone —
/// the opener resolves those via `PATH`.
fn browser_env_is_broken() -> bool {
    let Ok(value) = std::env::var("BROWSER") else {
        return false;
    };
    let first = value.split(':').next().unwrap_or("").trim();
    let program = first.split('%').next().unwrap_or(first).trim();
    !program.is_empty() && program.starts_with('/') && !std::path::Path::new(program).exists()
}

fn resolve_profile_web_port(profile: &WinboxProfile) -> Option<i64> {
    let ports = viewer_candidate_ports(profile.web_port, docker_novnc_port(&profile.name));
    for port in ports {
        if viewer_url_responds(&novnc_url_from_port(port)) {
            return Some(port);
        }
    }
    profile.web_port
}

fn viewer_candidate_ports(web_port: Option<i64>, docker_port: Option<i64>) -> Vec<i64> {
    let mut ports = Vec::new();
    if let Some(port) = web_port.filter(|port| *port > 0) {
        ports.push(port);
    }
    if let Some(port) = docker_port.filter(|port| *port > 0) {
        ports.push(port);
    }
    ports.dedup();
    ports
}

fn viewer_candidate_urls(
    web_port: Option<i64>,
    docker_port: Option<i64>,
    provider_url: Option<&str>,
) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(url) = provider_url.and_then(non_empty_url) {
        urls.push(url);
    }
    for port in viewer_candidate_ports(web_port, docker_port) {
        urls.push(novnc_url_from_port(port));
    }
    urls.dedup();
    urls
}

fn non_empty_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

fn describe_candidate_urls(urls: &[String]) -> String {
    if urls.is_empty() {
        return "none".to_string();
    }
    urls.join(", ")
}

fn viewer_url_responds(url: &str) -> bool {
    ureq::get(url)
        .timeout(Duration::from_secs(2))
        .call()
        .map(|response| (200..400).contains(&response.status()))
        .unwrap_or(false)
}

fn docker_novnc_port(profile_name: &str) -> Option<i64> {
    let container_name = format!("winbox-{profile_name}");
    let output = Command::new("docker")
        .arg("port")
        .arg(&container_name)
        .arg("8006/tcp")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let endpoint = stdout.lines().next()?.trim();
    port_from_docker_endpoint(endpoint)
}

fn port_from_docker_endpoint(endpoint: &str) -> Option<i64> {
    let (_, port) = endpoint.trim().rsplit_once(':')?;
    let port = port.trim();
    if port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    port.parse().ok()
}

fn novnc_url_from_port(port: i64) -> String {
    format!("http://127.0.0.1:{port}/vnc.html?autoconnect=true&resize=scale")
}

fn create_machine_record(
    db: &store::Database,
    provider_runtime: &str,
    input: &CreateWorkspaceMachineInput,
    profile_name: &str,
    preset: &MachinePreset,
    access_user: &str,
    status: &str,
) -> anyhow::Result<store::WorkspaceMachine> {
    db.create_workspace_machine(store::WorkspaceMachineCreate {
        workspace_id: input.workspace_id,
        project_id: input.project_id,
        provider: "winbox",
        provider_runtime,
        provider_profile: profile_name,
        display_name: &input.display_name,
        preset_id: &preset.id,
        image_family: &preset.image_family,
        access_user: Some(access_user),
        status,
    })
}

fn reconcile_with_provider_profiles(
    db: &store::Database,
    machines: Vec<store::WorkspaceMachine>,
    profiles: &[WinboxProfile],
) -> anyhow::Result<Vec<store::WorkspaceMachine>> {
    let profiles_by_name = profiles
        .iter()
        .map(|profile| (profile.name.as_str(), profile))
        .collect::<HashMap<_, _>>();
    let mut reconciled = Vec::with_capacity(machines.len());
    for machine in machines {
        if machine.provider != "winbox" {
            reconciled.push(machine);
            continue;
        }
        let Some(profile) = profiles_by_name.get(machine.provider_profile.as_str()) else {
            db.delete_workspace_machine(&machine.id)?;
            continue;
        };
        if should_preserve_desktop_setup_error(&machine) {
            reconciled.push(update_machine_from_profile_preserving_error(
                db, &machine, profile,
            )?);
            continue;
        }
        if machine_matches_profile(&machine, profile) {
            reconciled.push(machine);
        } else {
            reconciled.push(update_machine_from_profile(db, &machine.id, profile, None)?);
        }
    }
    Ok(reconciled)
}

fn adopt_workspace_provider_profiles(
    db: &store::Database,
    workspace_id: i64,
    provider_runtime: &str,
    mut machines: Vec<store::WorkspaceMachine>,
    profiles: &[WinboxProfile],
) -> anyhow::Result<Vec<store::WorkspaceMachine>> {
    let mut known_profiles = machines
        .iter()
        .map(|machine| machine.provider_profile.clone())
        .collect::<HashSet<_>>();
    let prefix = format!("dw-{workspace_id}-");
    for profile in profiles {
        if known_profiles.contains(&profile.name) || !profile.name.starts_with(&prefix) {
            continue;
        }
        let Some(preset) = preset_for_profile(profile) else {
            continue;
        };
        let display_name = display_name_from_profile(workspace_id, profile, &preset);
        let access_user = default_access_user_for_preset(&preset);
        let machine = db.create_workspace_machine(store::WorkspaceMachineCreate {
            workspace_id,
            project_id: None,
            provider: "winbox",
            provider_runtime,
            provider_profile: &profile.name,
            display_name: &display_name,
            preset_id: &preset.id,
            image_family: &preset.image_family,
            access_user: Some(&access_user),
            status: normalize_status(&profile.status),
        })?;
        let updated = update_machine_from_profile(db, &machine.id, profile, None)?;
        materialize_windows_shared_scripts(profile, Some(&access_user))?;
        known_profiles.insert(profile.name.clone());
        machines.push(updated);
    }
    Ok(machines)
}

fn preset_for_profile(profile: &WinboxProfile) -> Option<MachinePreset> {
    presets()
        .into_iter()
        .find(|preset| profile_matches_preset(profile, preset))
}

fn default_access_user_for_preset(preset: &MachinePreset) -> String {
    if preset.image_family == "windows" {
        "dev".to_string()
    } else {
        "bruno".to_string()
    }
}

fn machine_access_user(machine: &store::WorkspaceMachine) -> String {
    machine
        .access_user
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("bruno")
        .to_string()
}

fn display_name_from_profile(
    workspace_id: i64,
    profile: &WinboxProfile,
    preset: &MachinePreset,
) -> String {
    let prefix = format!("dw-{workspace_id}-");
    let suffix = profile.name.strip_prefix(&prefix).unwrap_or(&profile.name);
    let words = suffix
        .split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        preset.label.clone()
    } else {
        words.join(" ")
    }
}

fn machine_matches_profile(machine: &store::WorkspaceMachine, profile: &WinboxProfile) -> bool {
    let web_port = resolve_profile_web_port(profile);
    machine.status == normalize_status(&profile.status)
        && machine.web_port == web_port
        && machine.rdp_port == profile.rdp_port
        && machine.ssh_port == profile.ssh_port
}

fn profile_matches_preset(profile: &WinboxProfile, preset: &MachinePreset) -> bool {
    if profile.image_family.as_deref() != Some(preset.image_family.as_str()) {
        return false;
    }
    if preset.image_family == "linux_cloud" {
        let expected = preset.cloud_init_profile.as_deref().unwrap_or("server");
        let actual = profile
            .cloud_init_profile
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("server");
        return actual == expected;
    }
    match preset.boot.as_deref() {
        Some(boot) => profile.boot.as_deref() == Some(boot),
        None => true,
    }
}

fn remove_local_after_provider_remove(
    db: &store::Database,
    machine_id: &str,
    remove_result: Result<(), MachineError>,
) -> anyhow::Result<()> {
    match remove_result {
        Ok(()) => db.delete_workspace_machine(machine_id),
        Err(error) if is_profile_not_found(&error) => db.delete_workspace_machine(machine_id),
        Err(error) => Err(machine_error_to_anyhow(error)),
    }
}

fn is_profile_not_found(error: &MachineError) -> bool {
    error.code == "profile_not_found"
}

fn update_machine_from_profile(
    db: &store::Database,
    machine_id: &str,
    profile: &WinboxProfile,
    error: Option<&MachineError>,
) -> anyhow::Result<store::WorkspaceMachine> {
    db.update_workspace_machine(store::WorkspaceMachineUpdate {
        id: machine_id,
        status: normalize_status(&profile.status),
        web_port: resolve_profile_web_port(profile),
        rdp_port: profile.rdp_port,
        ssh_port: profile.ssh_port,
        last_health_status: None,
        last_health_summary: None,
        last_error_code: error.map(|value| value.code.as_str()),
        last_error_message: error.map(|value| value.message.as_str()),
    })
}

fn materialize_windows_shared_scripts(
    profile: &WinboxProfile,
    access_user: Option<&str>,
) -> anyhow::Result<()> {
    if profile.image_family.as_deref() != Some("windows") {
        return Ok(());
    }
    let Some(shared_dir) = profile
        .shared_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let root = Path::new(shared_dir).join("ade");
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("bootstrap-windows.ps1"),
        windows_machine_bootstrap_script(profile, access_user),
    )?;
    std::fs::write(
        root.join("README.txt"),
        windows_machine_shared_readme(profile),
    )?;
    Ok(())
}

fn windows_machine_bootstrap_script(profile: &WinboxProfile, access_user: Option<&str>) -> String {
    let user = access_user
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("bruno");
    let ssh_port = profile
        .ssh_port
        .map(|port| port.to_string())
        .unwrap_or_else(|| "<ssh-port>".to_string());
    format!(
        r#"# ADE Windows bootstrap for {profile_name}
# Run inside the Windows VM with PowerShell as Administrator.

$ErrorActionPreference = "Stop"

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {{
  throw "Run this script as Administrator."
}}

Write-Host "[dw] Creating base folders"
New-Item -ItemType Directory -Force -Path "C:\dw", "C:\dw\deploy", "C:\dw\logs" | Out-Null

Write-Host "[dw] Installing OpenSSH Server"
$capability = Get-WindowsCapability -Online -Name "OpenSSH.Server~~~~0.0.1.0"
if ($capability.State -ne "Installed") {{
  Add-WindowsCapability -Online -Name "OpenSSH.Server~~~~0.0.1.0"
}}

Write-Host "[dw] Enabling sshd"
Set-Service -Name sshd -StartupType Automatic
Start-Service sshd

if (-not (Get-NetFirewallRule -Name "OpenSSH-Server-In-TCP" -ErrorAction SilentlyContinue)) {{
  New-NetFirewallRule -Name "OpenSSH-Server-In-TCP" -DisplayName "OpenSSH Server (sshd)" -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 | Out-Null
}}

Write-Host "[dw] SSH binary"
where.exe ssh
ssh -V

Write-Host ""
Write-Host "[dw] Done. Test from WSL/host:"
Write-Host "ssh -p {ssh_port} {user}@127.0.0.1"
"#,
        profile_name = profile.name,
        ssh_port = ssh_port,
        user = user
    )
}

fn windows_machine_shared_readme(profile: &WinboxProfile) -> String {
    format!(
        "ADE Windows shared folder\n\
\n\
Profile: {profile_name}\n\
\n\
Run this inside the Windows VM with PowerShell as Administrator:\n\
\n\
  powershell -NoProfile -ExecutionPolicy Bypass -File .\\ade\\bootstrap-windows.ps1\n\
\n\
The bootstrap script installs/enables OpenSSH Server, opens the firewall rule,\n\
and creates C:\\dw\\deploy plus C:\\dw\\logs for later ADE deploy packages.\n\
",
        profile_name = profile.name
    )
}

fn desktop_setup_failed_error(error: &anyhow::Error) -> MachineError {
    let raw_message = error.to_string();
    let message = raw_message
        .strip_prefix("desktop_setup_failed: ")
        .unwrap_or(&raw_message)
        .to_string();
    let mut machine_error = MachineError::from_message("desktop_setup_failed", message);
    machine_error.hint = Some(
        "A VM foi criada, mas o setup gráfico não terminou. Aguarde o apt/cloud-init terminar ou salve a senha novamente para repetir o setup.".to_string(),
    );
    machine_error
}

fn provider_profile_mismatch_error(
    profile: &WinboxProfile,
    preset: &MachinePreset,
) -> MachineError {
    let mut machine_error = MachineError::from_message(
        "provider_profile_mismatch",
        format!(
            "WinBox created profile '{}' as image_family='{}' cloud_init_profile='{}', but ADE requested preset '{}' ({}/{}).",
            profile.name,
            profile.image_family.as_deref().unwrap_or("-"),
            profile.cloud_init_profile.as_deref().unwrap_or("-"),
            preset.id,
            preset.image_family,
            preset.cloud_init_profile.as_deref().unwrap_or("-")
        ),
    );
    machine_error.hint = Some(
        "Remove this machine and create it again after updating ADE/WinBox command arguments."
            .to_string(),
    );
    machine_error
}

fn should_preserve_desktop_setup_error(machine: &store::WorkspaceMachine) -> bool {
    machine.status == "error"
        && matches!(
            machine.last_error_code.as_deref(),
            Some("desktop_setup_failed" | "provider_profile_mismatch")
        )
}

fn update_machine_from_profile_preserving_error(
    db: &store::Database,
    machine: &store::WorkspaceMachine,
    profile: &WinboxProfile,
) -> anyhow::Result<store::WorkspaceMachine> {
    let mut error_profile = profile.clone();
    error_profile.status = "error".to_string();
    let error = stored_machine_error(machine);
    update_machine_from_profile(db, &machine.id, &error_profile, error.as_ref())
}

fn update_machine_to_error(
    db: &store::Database,
    machine: &store::WorkspaceMachine,
    error: &MachineError,
) -> anyhow::Result<store::WorkspaceMachine> {
    db.update_workspace_machine(store::WorkspaceMachineUpdate {
        id: &machine.id,
        status: "error",
        web_port: machine.web_port,
        rdp_port: machine.rdp_port,
        ssh_port: machine.ssh_port,
        last_health_status: machine.last_health_status.as_deref(),
        last_health_summary: machine.last_health_summary.as_deref(),
        last_error_code: Some(error.code.as_str()),
        last_error_message: Some(error.message.as_str()),
    })
}

fn stored_machine_error(machine: &store::WorkspaceMachine) -> Option<MachineError> {
    let code = machine.last_error_code.as_deref()?;
    Some(MachineError {
        code: code.to_string(),
        message: machine.last_error_message.clone().unwrap_or_default(),
        hint: None,
        provider_detail: None,
    })
}

fn normalize_status(status: &str) -> &str {
    match status {
        "running" => "running",
        "paused" => "paused",
        "absent" | "exited" | "created" | "dead" => "stopped",
        "error" => "error",
        "restarting" => "creating",
        _ => "unknown",
    }
}

fn machine_profile_name(workspace_id: i64, display_name: &str) -> String {
    let mut slug = display_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-');
    format!(
        "dw-{workspace_id}-{}",
        if slug.is_empty() { "machine" } else { slug }
    )
}

fn new_run_id() -> String {
    format!("machine-run-{}", chrono::Utc::now().timestamp_millis())
}

fn emit_progress(
    app: &tauri::AppHandle,
    run_id: &str,
    machine_id: Option<&str>,
    provider_profile: &str,
    progress: WinboxProgress,
) {
    let event = MachineProgressEvent {
        run_id: run_id.to_string(),
        machine_id: machine_id.map(ToOwned::to_owned),
        provider_profile: provider_profile.to_string(),
        operation: progress
            .operation
            .unwrap_or_else(|| "operation".to_string()),
        phase: progress.phase.unwrap_or_else(|| "running".to_string()),
        status: progress.status.unwrap_or_else(|| "running".to_string()),
        message: progress.message.unwrap_or_default(),
        percent: progress.percent,
        timestamp: progress
            .timestamp
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    let _ = app.emit("machine://progress", event);
}

fn machine_error_to_anyhow(error: MachineError) -> anyhow::Error {
    match error.hint.as_deref().filter(|hint| !hint.is_empty()) {
        Some(hint) => anyhow!("{}: {} ({hint})", error.code, error.message),
        None => anyhow!("{}: {}", error.code, error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db() -> (store::Database, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-gui-machine-test-{unique}"));
        std::fs::create_dir_all(&root).expect("create db dir");
        (store::Database::open(&root).expect("open db"), root)
    }

    fn create_workspace_machine(
        db: &store::Database,
        workspace_id: i64,
        provider_profile: &str,
        display_name: &str,
        status: &str,
    ) -> store::WorkspaceMachine {
        db.create_workspace_machine(store::WorkspaceMachineCreate {
            workspace_id,
            project_id: None,
            provider: "winbox",
            provider_runtime: "native",
            provider_profile,
            display_name,
            preset_id: "xubuntu_lts",
            image_family: "linux_distro",
            access_user: Some("bruno"),
            status,
        })
        .expect("create machine")
    }

    fn winbox_profile(name: &str, status: &str) -> WinboxProfile {
        WinboxProfile {
            name: name.to_string(),
            status: status.to_string(),
            web_port: Some(8007),
            rdp_port: Some(3392),
            ssh_port: None,
            ram: None,
            bundles: None,
            image_family: Some("linux_distro".to_string()),
            boot: Some("xubuntu".to_string()),
            cloud_init_profile: None,
            iso_path: None,
            shared_dir: None,
        }
    }

    #[test]
    fn profile_name_is_workspace_scoped_and_slugged() {
        assert_eq!(
            machine_profile_name(7, "Ubuntu Dev VM"),
            "dw-7-ubuntu-dev-vm"
        );
    }

    #[test]
    fn status_is_normalized_for_ui() {
        assert_eq!(normalize_status("running"), "running");
        assert_eq!(normalize_status("exited"), "stopped");
        assert_eq!(normalize_status("weird"), "unknown");
    }

    #[test]
    fn only_deploy_ubuntu_and_windows_presets_are_creatable() {
        let distros = vec![WinboxDistro {
            id: "ubuntu-server".to_string(),
            label: "Ubuntu Server".to_string(),
        }];

        let ubuntu = resolve_preset("ubuntu_deploy_vm", &[]).expect("ubuntu deploy preset");
        assert_eq!(ubuntu.label, "Ubuntu Server Deploy VM");
        assert_eq!(ubuntu.cloud_init_profile.as_deref(), Some("server"));
        assert_eq!(ubuntu.default_ram, "4G");
        let desktop =
            resolve_preset("ubuntu_desktop_deploy_vm", &[]).expect("ubuntu desktop preset");
        assert_eq!(desktop.label, "Ubuntu Desktop Deploy VM");
        assert_eq!(
            desktop.cloud_init_profile.as_deref(),
            Some("xubuntu-desktop")
        );
        assert_eq!(desktop.default_ram, "6G");
        assert!(resolve_preset("windows_11", &[]).is_ok());
        assert!(resolve_preset("ubuntu_server_lts", &distros).is_err());
        assert!(resolve_preset("xubuntu_lts", &distros).is_err());
    }

    #[test]
    fn credential_validation_keeps_passwords_out_of_ubuntu() {
        let preset = resolve_preset("ubuntu_deploy_vm", &[]).expect("preset");
        let input = CreateWorkspaceMachineInput {
            workspace_id: 1,
            project_id: None,
            preset_id: "ubuntu_deploy_vm".to_string(),
            display_name: "Ubuntu Deploy VM dev".to_string(),
            provider_profile: None,
            ram: None,
            cpu: None,
            disk: None,
            user: None,
            password: None,
        };

        assert_eq!(
            validate_machine_credentials(&input, &preset).expect("default user"),
            "bruno"
        );
        assert!(validate_machine_credentials(
            &CreateWorkspaceMachineInput {
                password: Some("secret".to_string()),
                ..input
            },
            &preset
        )
        .is_err());
    }

    #[test]
    fn credential_validation_requires_desktop_password() {
        let preset = resolve_preset("ubuntu_desktop_deploy_vm", &[]).expect("preset");
        let base = CreateWorkspaceMachineInput {
            workspace_id: 1,
            project_id: None,
            preset_id: "ubuntu_desktop_deploy_vm".to_string(),
            display_name: "Ubuntu Desktop Deploy VM dev".to_string(),
            provider_profile: None,
            ram: None,
            cpu: None,
            disk: None,
            user: Some("bruno".to_string()),
            password: Some("ChangeMe123!".to_string()),
        };

        assert_eq!(
            validate_machine_credentials(&base, &preset).expect("valid"),
            "bruno"
        );
        assert!(validate_machine_credentials(
            &CreateWorkspaceMachineInput {
                password: None,
                ..base
            },
            &preset
        )
        .is_err());
    }

    #[test]
    fn install_args_put_profile_after_options() {
        let preset = resolve_preset("ubuntu_desktop_deploy_vm", &[]).expect("preset");
        let input = CreateWorkspaceMachineInput {
            workspace_id: 1,
            project_id: None,
            preset_id: "ubuntu_desktop_deploy_vm".to_string(),
            display_name: "Ubuntu Desktop Deploy VM dev".to_string(),
            provider_profile: None,
            ram: Some("8G".to_string()),
            cpu: Some("4".to_string()),
            disk: Some("96G".to_string()),
            user: Some("bruno".to_string()),
            password: Some("ChangeMe123!".to_string()),
        };

        let args = build_install_args(
            "dw-1-ubuntu-desktop-deploy-vm-dev",
            &preset,
            &input,
            "bruno",
        );
        assert_eq!(args.first().map(String::as_str), Some("install"));
        assert_eq!(
            args.last().map(String::as_str),
            Some("dw-1-ubuntu-desktop-deploy-vm-dev")
        );
        assert_eq!(
            args.iter().position(|arg| arg == "--family"),
            Some(2),
            "WinBox install expects options before the profile argument"
        );
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--boot", "xubuntu-desktop"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--pass", "ChangeMe123!"]));
    }

    #[test]
    fn provider_profile_mismatch_error_describes_actual_and_expected_profile() {
        let preset = resolve_preset("ubuntu_desktop_deploy_vm", &[]).expect("preset");
        let profile = WinboxProfile {
            name: "dw-1-ubuntu-desktop-deploy-vm-dev".to_string(),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: Some(3389),
            ssh_port: Some(2222),
            ram: Some("8G".to_string()),
            bundles: None,
            image_family: Some("windows".to_string()),
            boot: None,
            cloud_init_profile: Some("server".to_string()),
            iso_path: None,
            shared_dir: None,
        };

        let error = provider_profile_mismatch_error(&profile, &preset);
        assert_eq!(error.code, "provider_profile_mismatch");
        assert!(error.message.contains("image_family='windows'"));
        assert!(error.message.contains("ubuntu_desktop_deploy_vm"));
    }

    #[test]
    fn password_change_validation_only_accepts_desktop_targets() {
        assert!(validate_password_change_fields(
            "ubuntu_deploy_vm",
            Some(2222),
            Some("bruno"),
            "ChangeMe123!"
        )
        .is_err());
        assert!(validate_password_change_fields(
            "ubuntu_desktop_deploy_vm",
            None,
            Some("bruno"),
            "ChangeMe123!"
        )
        .is_err());
        assert!(validate_password_change_fields(
            "ubuntu_desktop_deploy_vm",
            Some(2222),
            Some("bruno"),
            ""
        )
        .is_err());
        assert!(validate_password_change_fields(
            "ubuntu_desktop_deploy_vm",
            Some(2222),
            Some("bruno"),
            "bad\npassword"
        )
        .is_err());

        assert_eq!(
            validate_password_change_fields(
                "ubuntu_desktop_deploy_vm",
                Some(2222),
                Some("bruno"),
                "ChangeMe123!"
            )
            .expect("valid"),
            PasswordChangeTarget {
                ssh_port: 2222,
                access_user: "bruno".to_string(),
                password: "ChangeMe123!".to_string(),
            }
        );
    }

    #[test]
    fn desktop_setup_script_installs_graphical_and_rdp_runtime() {
        let script = linux_desktop_setup_script(1_700_000_000);
        assert!(script.contains("HOST_EPOCH=1700000000"));
        assert!(script.contains("sync_guest_clock()"));
        assert!(script.contains("sudo date -u -s \"@$HOST_EPOCH\""));
        assert!(script.contains("apt_update_with_retry"));
        assert!(script.contains("apt_install_with_retry"));
        assert!(script.contains("apt_log_has_retryable_lock"));
        assert!(script.contains("not valid yet"));
        assert!(script.contains("waiting for package manager lock"));
        assert!(script.contains(
            "xfce4 xfce4-terminal xterm lightdm lightdm-gtk-greeter accountsservice xrdp xorgxrdp xserver-xorg-input-libinput"
        ));
        assert!(script.contains("dpkg -s xfce4-terminal"));
        assert!(script.contains("dpkg -s xterm"));
        assert!(script.contains("dpkg -s xserver-xorg-input-libinput"));
        assert!(script.contains("greeter-session=lightdm-gtk-greeter"));
        assert!(script.contains("user-session=$session"));
        assert!(script.contains("systemd-logind"));
        assert!(script.contains("startxfce4"));
    }

    #[test]
    fn transient_ssh_failures_are_retryable() {
        assert!(is_transient_ssh_failure(
            "ssh: connect to host 127.0.0.1 port 2222: Connection refused"
        ));
        assert!(is_transient_ssh_failure("Connection timed out"));
        assert!(!is_transient_ssh_failure("Permission denied (publickey)"));
    }

    #[test]
    fn apt_release_clock_skew_is_retryable_for_desktop_setup() {
        assert!(is_retryable_desktop_setup_failure(
            "E: Release file for http://archive.ubuntu.com/ubuntu/dists/noble-updates/InRelease is not valid yet (invalid for another 26min 47s)."
        ));
        assert!(!is_retryable_desktop_setup_failure(
            "E: Unable to locate package definitely-missing"
        ));
    }

    #[test]
    fn apt_locks_are_retryable_for_desktop_setup() {
        assert!(is_retryable_desktop_setup_failure(
            "E: Could not get lock /var/lib/apt/lists/lock. It is held by process 1163 (apt-get)"
        ));
        assert!(is_retryable_desktop_setup_failure(
            "E: Unable to lock directory /var/lib/apt/lists/"
        ));
        assert!(is_retryable_desktop_setup_failure(
            "Waiting for cache lock: Could not get lock /var/lib/dpkg/lock-frontend"
        ));
    }

    #[test]
    fn desktop_setup_failed_error_uses_stable_code_and_message() {
        let error = anyhow!("desktop_setup_failed: E: Could not get lock /var/lib/apt/lists/lock");
        let machine_error = desktop_setup_failed_error(&error);
        assert_eq!(machine_error.code, "desktop_setup_failed");
        assert_eq!(
            machine_error.message,
            "E: Could not get lock /var/lib/apt/lists/lock"
        );
        assert!(machine_error
            .hint
            .unwrap_or_default()
            .contains("setup gráfico"));
    }

    #[test]
    fn docker_novnc_endpoint_builds_browser_url() {
        assert_eq!(port_from_docker_endpoint("127.0.0.1:8006"), Some(8006));
        assert_eq!(
            novnc_url_from_port(8006),
            "http://127.0.0.1:8006/vnc.html?autoconnect=true&resize=scale"
        );
        assert!(port_from_docker_endpoint("127.0.0.1:not-a-port").is_none());
    }

    #[test]
    fn viewer_candidates_prefer_provider_web_port_over_docker_endpoint() {
        assert_eq!(
            viewer_candidate_ports(Some(8016), Some(8006)),
            vec![8016, 8006]
        );
        assert_eq!(viewer_candidate_ports(Some(8007), Some(8007)), vec![8007]);
    }

    #[test]
    fn viewer_candidates_do_not_add_global_default_port() {
        assert!(viewer_candidate_ports(None, None).is_empty());
        assert_eq!(viewer_candidate_ports(None, Some(8016)), vec![8016]);
        assert_eq!(viewer_candidate_ports(Some(8007), None), vec![8007]);
        assert!(viewer_candidate_ports(Some(0), Some(-1)).is_empty());
    }

    #[test]
    fn viewer_urls_prefer_provider_viewer_url_for_windows_root_viewer() {
        assert_eq!(
            viewer_candidate_urls(
                Some(8006),
                None,
                Some("http://127.0.0.1:8006/?autoconnect=true&resize=scale")
            ),
            vec![
                "http://127.0.0.1:8006/?autoconnect=true&resize=scale".to_string(),
                "http://127.0.0.1:8006/vnc.html?autoconnect=true&resize=scale".to_string()
            ]
        );
    }

    #[test]
    fn viewer_urls_ignore_empty_provider_url_and_keep_legacy_fallback() {
        assert_eq!(
            viewer_candidate_urls(Some(8007), Some(8006), Some("   ")),
            vec![
                "http://127.0.0.1:8007/vnc.html?autoconnect=true&resize=scale".to_string(),
                "http://127.0.0.1:8006/vnc.html?autoconnect=true&resize=scale".to_string()
            ]
        );
    }

    #[test]
    fn windows_shared_scripts_include_openssh_bootstrap() {
        let root = std::env::temp_dir().join(format!(
            "dw-gui-windows-shared-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create shared root");
        let profile = WinboxProfile {
            name: "dw-3-windows-11-dev".to_string(),
            status: "running".to_string(),
            web_port: Some(8007),
            rdp_port: Some(3390),
            ssh_port: Some(2223),
            ram: None,
            bundles: None,
            image_family: Some("windows".to_string()),
            boot: None,
            cloud_init_profile: Some("server".to_string()),
            iso_path: None,
            shared_dir: Some(root.display().to_string()),
        };

        materialize_windows_shared_scripts(&profile, Some("bruno")).expect("write scripts");

        let script = std::fs::read_to_string(root.join("ade/bootstrap-windows.ps1"))
            .expect("read bootstrap script");
        let readme = std::fs::read_to_string(root.join("ade/README.txt")).expect("read readme");
        assert!(script.contains("OpenSSH.Server~~~~0.0.1.0"));
        assert!(script.contains("Set-Service -Name sshd -StartupType Automatic"));
        assert!(script.contains("ssh -p 2223 bruno@127.0.0.1"));
        assert!(readme.contains("bootstrap-windows.ps1"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn windows_ssh_forwarding_maps_host_ssh_to_guest_openssh() {
        let profile = WinboxProfile {
            name: "dw-3-windows-11-dev".to_string(),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: Some(3389),
            ssh_port: Some(2222),
            ram: None,
            bundles: None,
            image_family: Some("windows".to_string()),
            boot: None,
            cloud_init_profile: Some("server".to_string()),
            iso_path: None,
            shared_dir: None,
        };

        assert_eq!(
            windows_ssh_extra_ports(&profile).as_deref(),
            Some("2222:22/tcp")
        );
    }

    #[test]
    fn windows_ssh_forwarding_ignores_non_windows_profiles() {
        let mut profile = WinboxProfile {
            name: "dw-3-ubuntu-deploy-vm-dev".to_string(),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: Some(3389),
            ssh_port: Some(2222),
            ram: None,
            bundles: None,
            image_family: Some("linux_cloud".to_string()),
            boot: Some("/storage/boot.qcow2".to_string()),
            cloud_init_profile: Some("server".to_string()),
            iso_path: None,
            shared_dir: None,
        };

        assert_eq!(windows_ssh_extra_ports(&profile), None);

        profile.image_family = Some("windows".to_string());
        profile.ssh_port = None;
        assert_eq!(windows_ssh_extra_ports(&profile), None);
    }

    #[test]
    fn ssh_banner_detector_accepts_ssh_protocol_line() {
        assert!(ssh_banner_is_ready("SSH-2.0-OpenSSH_for_Windows_9.5"));
        assert!(ssh_banner_is_ready("SSH-2.0-OpenSSH_9.6p1 Ubuntu"));
        assert!(!ssh_banner_is_ready("HTTP/1.1 200 OK"));
        assert!(!ssh_banner_is_ready(""));
    }

    #[test]
    fn ssh_probe_reports_missing_port_without_network_check() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let machine = create_workspace_machine(
            &db,
            workspace.id,
            "dw-3-windows-11-dev",
            "Windows 11 dev",
            "running",
        );

        let probe = build_machine_ssh_probe(&machine);

        assert_eq!(probe.status, "missing_port");
        assert!(probe.command.is_none());
        assert!(probe.message.contains("não expôs"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn ssh_probe_reports_connection_failure_on_selected_port() {
        let probe = ssh_probe_from_check(
            "machine-1",
            "bruno",
            Some(2223),
            Some("ssh -p 2223 bruno@127.0.0.1".to_string()),
            SshProbeCheck::NotReady {
                detail: "connection refused".to_string(),
            },
        );

        assert_eq!(probe.status, "not_ready");
        assert_eq!(probe.port, Some(2223));
        assert_eq!(
            probe.command.as_deref(),
            Some("ssh -p 2223 bruno@127.0.0.1")
        );
        assert!(probe.message.contains("127.0.0.1:2223"));
    }

    #[test]
    fn ssh_probe_ready_message_includes_banner_and_command() {
        let probe = ssh_probe_from_check(
            "machine-1",
            "bruno",
            Some(2223),
            Some("ssh -p 2223 bruno@127.0.0.1".to_string()),
            SshProbeCheck::Ready {
                banner: "SSH-2.0-OpenSSH_for_Windows_9.5".to_string(),
            },
        );

        assert_eq!(probe.status, "ready");
        assert!(probe.message.contains("OpenSSH_for_Windows"));
        assert_eq!(
            probe.command.as_deref(),
            Some("ssh -p 2223 bruno@127.0.0.1")
        );
    }

    #[test]
    fn credential_validation_requires_windows_user_and_password() {
        let preset = resolve_preset("windows_11", &[]).expect("preset");
        let base = CreateWorkspaceMachineInput {
            workspace_id: 1,
            project_id: None,
            preset_id: "windows_11".to_string(),
            display_name: "Windows 11 dev".to_string(),
            provider_profile: None,
            ram: None,
            cpu: None,
            disk: None,
            user: Some("bruno".to_string()),
            password: Some("ChangeMe123!".to_string()),
        };

        assert_eq!(
            validate_machine_credentials(&base, &preset).expect("valid"),
            "bruno"
        );
        assert!(validate_machine_credentials(
            &CreateWorkspaceMachineInput {
                user: None,
                ..base.clone()
            },
            &preset
        )
        .is_err());
        assert!(validate_machine_credentials(
            &CreateWorkspaceMachineInput {
                password: None,
                ..base
            },
            &preset
        )
        .is_err());
    }

    #[test]
    fn existing_provider_profile_must_match_preset_before_adoption() {
        let preset = MachinePreset {
            id: "xubuntu_lts".to_string(),
            label: "Xubuntu LTS".to_string(),
            image_family: "linux_distro".to_string(),
            boot: Some("xubuntu".to_string()),
            cloud_init_profile: None,
            version: None,
            default_ram: "4G".to_string(),
            default_cpu: "2".to_string(),
            default_disk: "64G".to_string(),
            deploy_capable: false,
            supported: true,
            disabled_reason: None,
        };
        let matching = winbox_profile("dw-3-xubuntu-lts-dev", "running");
        let mut mismatched = matching.clone();
        mismatched.boot = Some("ubuntu-server".to_string());

        assert!(profile_matches_preset(&matching, &preset));
        assert!(!profile_matches_preset(&mismatched, &preset));
    }

    #[test]
    fn linux_cloud_profile_treats_empty_cloud_init_profile_as_server() {
        let preset = resolve_preset("ubuntu_deploy_vm", &[]).expect("preset");
        let mut matching = WinboxProfile {
            name: "dw-3-ubuntu-deploy-vm-dev".to_string(),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: Some(3389),
            ssh_port: Some(2222),
            ram: None,
            bundles: None,
            image_family: Some("linux_cloud".to_string()),
            boot: Some("/storage/boot.qcow2".to_string()),
            cloud_init_profile: None,
            iso_path: None,
            shared_dir: None,
        };
        assert!(profile_matches_preset(&matching, &preset));

        matching.cloud_init_profile = Some("xubuntu-desktop".to_string());
        assert!(!profile_matches_preset(&matching, &preset));
    }

    #[test]
    fn desktop_linux_cloud_profile_must_match_desktop_cloud_init_profile() {
        let preset = resolve_preset("ubuntu_desktop_deploy_vm", &[]).expect("preset");
        let mut matching = WinboxProfile {
            name: "dw-3-ubuntu-desktop-deploy-vm-dev".to_string(),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: Some(3389),
            ssh_port: Some(2222),
            ram: None,
            bundles: None,
            image_family: Some("linux_cloud".to_string()),
            boot: Some("/storage/boot.qcow2".to_string()),
            cloud_init_profile: Some("xubuntu-desktop".to_string()),
            iso_path: None,
            shared_dir: None,
        };
        assert!(profile_matches_preset(&matching, &preset));

        matching.cloud_init_profile = Some("server".to_string());
        assert!(!profile_matches_preset(&matching, &preset));
    }

    #[test]
    fn list_adopts_workspace_scoped_provider_profiles() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let profile = WinboxProfile {
            name: format!("dw-{}-ubuntu-desktop-deploy-vm-dev", workspace.id),
            status: "running".to_string(),
            web_port: Some(8006),
            rdp_port: Some(3389),
            ssh_port: Some(2222),
            ram: None,
            bundles: None,
            image_family: Some("linux_cloud".to_string()),
            boot: Some("/storage/boot.qcow2".to_string()),
            cloud_init_profile: Some("xubuntu-desktop".to_string()),
            iso_path: None,
            shared_dir: None,
        };

        let adopted =
            adopt_workspace_provider_profiles(&db, workspace.id, "native", vec![], &[profile])
                .expect("adopt provider profile");

        assert_eq!(adopted.len(), 1);
        assert_eq!(adopted[0].preset_id, "ubuntu_desktop_deploy_vm");
        assert_eq!(adopted[0].access_user.as_deref(), Some("bruno"));
        assert_eq!(adopted[0].status, "running");
        assert_eq!(adopted[0].ssh_port, Some(2222));
        assert_eq!(adopted[0].rdp_port, Some(3389));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn reconciliation_removes_local_machines_missing_from_winbox() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let orphan = create_workspace_machine(
            &db,
            workspace.id,
            "dw-3-lubuntu-lts-dev",
            "Lubuntu LTS dev",
            "running",
        );
        let kept = create_workspace_machine(
            &db,
            workspace.id,
            "dw-3-xubuntu-lts-dev",
            "Xubuntu LTS dev",
            "creating",
        );

        let rows = db
            .list_workspace_machines(workspace.id)
            .expect("list machines");
        let reconciled = reconcile_with_provider_profiles(
            &db,
            rows,
            &[winbox_profile("dw-3-xubuntu-lts-dev", "running")],
        )
        .expect("reconcile");

        assert_eq!(reconciled.len(), 1);
        assert_eq!(reconciled[0].id, kept.id);
        assert_eq!(reconciled[0].status, "running");
        assert_eq!(reconciled[0].web_port, Some(8007));
        assert!(db.get_workspace_machine(&orphan.id).is_err());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn remove_is_idempotent_when_winbox_profile_is_already_missing() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");
        let machine = create_workspace_machine(
            &db,
            workspace.id,
            "dw-3-lubuntu-lts-dev",
            "Lubuntu LTS dev",
            "running",
        );
        let missing = MachineError {
            code: "profile_not_found".to_string(),
            message: "Perfil não existe.".to_string(),
            hint: None,
            provider_detail: None,
        };

        remove_local_after_provider_remove(&db, &machine.id, Err(missing)).expect("remove local");

        assert!(db.get_workspace_machine(&machine.id).is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
