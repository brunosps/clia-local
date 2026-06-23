use anyhow::{anyhow, Context};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const RTK_ENV_BIN: &str = "CLIA_RTK_BIN";
const RTK_TELEMETRY_DISABLED: &str = "RTK_TELEMETRY_DISABLED";
const RTK_VERSION: &str = "0.42.3";
const RTK_BASE_URL: &str = "https://github.com/rtk-ai/rtk/releases/download/v0.42.3";

#[derive(Debug, Clone, Serialize)]
pub struct RtkStatus {
    pub enabled: bool,
    pub available: bool,
    pub supported: bool,
    pub telemetry_blocked: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub binary_source: Option<String>,
    pub setup_state: String,
    pub message: String,
    pub gain_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtkSetupCommand {
    pub provider: String,
    pub cwd: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtkSetupResult {
    pub applied: bool,
    pub commands: Vec<RtkSetupCommand>,
    pub stdout: String,
    pub stderr: String,
    pub status: RtkStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtkInstallResult {
    pub installed: bool,
    pub version: String,
    pub binary_path: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub status: RtkStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtkCommandEnv {
    pub enabled: bool,
    pub telemetry_blocked: bool,
    pub binary_path: Option<String>,
    pub binary_source: Option<String>,
    pub path_preprended: bool,
}

#[derive(Debug, Clone)]
struct RtkBinary {
    path: PathBuf,
    source: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct RtkReleaseAsset {
    file: &'static str,
    sha256: &'static str,
}

pub fn status(app: &AppHandle, enabled: bool, project_path: Option<&str>) -> RtkStatus {
    let Some(binary) = resolve_binary(app) else {
        return RtkStatus {
            enabled,
            available: false,
            supported: true,
            telemetry_blocked: true,
            version: None,
            binary_path: None,
            binary_source: None,
            setup_state: "missing".to_string(),
            message:
                "RTK binary not found. Install RTK from Workspace settings or set CLIA_RTK_BIN."
                    .to_string(),
            gain_summary: None,
        };
    };

    let version = run_rtk(&binary, &["--version"], project_path).ok();
    let gain = run_rtk(&binary, &["gain"], project_path).ok();
    let setup = run_rtk(&binary, &["init", "--show"], project_path).ok();
    let setup_state = if setup
        .as_ref()
        .map(|output| output.status_success)
        .unwrap_or(false)
    {
        "configured"
    } else if enabled {
        "needs_setup"
    } else {
        "available"
    };
    let available = version
        .as_ref()
        .map(|output| output.status_success)
        .unwrap_or(false);
    RtkStatus {
        enabled,
        available,
        supported: true,
        telemetry_blocked: true,
        version: version
            .as_ref()
            .and_then(|output| first_line(&output.stdout).or_else(|| first_line(&output.stderr))),
        binary_path: Some(binary.path.display().to_string()),
        binary_source: Some(binary.source.to_string()),
        setup_state: setup_state.to_string(),
        message: if available {
            "RTK ready. Telemetry is blocked by clia.dev.".to_string()
        } else {
            "RTK was found but did not respond to --version.".to_string()
        },
        gain_summary: gain
            .as_ref()
            .and_then(|output| first_line(&output.stdout).or_else(|| first_line(&output.stderr))),
    }
}

pub fn ensure_installed(
    app: &AppHandle,
    enabled: bool,
    project_path: Option<&str>,
) -> anyhow::Result<RtkInstallResult> {
    let current = status(app, enabled, project_path);
    if current.available {
        return Ok(RtkInstallResult {
            installed: false,
            version: RTK_VERSION.to_string(),
            binary_path: current.binary_path.clone(),
            stdout: current.message.clone(),
            stderr: String::new(),
            status: current,
        });
    }

    let asset = current_release_asset()?;
    let install_dir = managed_rtk_dir(app)?;
    let exe = rtk_exe_name();
    let install_path = install_dir.join(exe);
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("failed to create {}", install_dir.display()))?;

    let work_dir = env::temp_dir().join(format!(
        "clia-rtk-{RTK_VERSION}-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    let extract_dir = work_dir.join("extract");
    let archive_path = work_dir.join(asset.file);
    fs::create_dir_all(&extract_dir)
        .with_context(|| format!("failed to create {}", extract_dir.display()))?;

    let url = format!("{RTK_BASE_URL}/{}", asset.file);
    let mut stdout = format!("Downloading RTK {RTK_VERSION} from {url}\n");
    download_archive(&url, &archive_path)?;
    verify_sha256(&archive_path, asset.sha256)?;
    stdout.push_str("Checksum verified\n");

    extract_archive(&archive_path, &extract_dir)?;
    let extracted = find_binary_recursive(&extract_dir, exe)
        .ok_or_else(|| anyhow!("could not find {exe} in {}", asset.file))?;
    fs::copy(&extracted, &install_path).with_context(|| {
        format!(
            "failed to copy {} to {}",
            extracted.display(),
            install_path.display()
        )
    })?;
    make_executable(&install_path)?;
    let _ = fs::remove_dir_all(&work_dir);
    stdout.push_str(&format!("Installed RTK at {}\n", install_path.display()));

    let installed_status = status(app, enabled, project_path);
    if !installed_status.available {
        anyhow::bail!(
            "RTK was installed at {} but did not respond to --version",
            install_path.display()
        );
    }

    Ok(RtkInstallResult {
        installed: true,
        version: RTK_VERSION.to_string(),
        binary_path: Some(install_path.display().to_string()),
        stdout,
        stderr: String::new(),
        status: installed_status,
    })
}

pub fn configure_profile(
    app: &AppHandle,
    provider: &str,
    project_path: &str,
    enabled: bool,
    apply: bool,
) -> anyhow::Result<RtkSetupResult> {
    let commands = setup_commands(provider, project_path)?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    if apply {
        let binary = resolve_binary(app).ok_or_else(|| anyhow!("RTK binary not found"))?;
        for setup in &commands {
            let mut args = setup
                .command
                .split_whitespace()
                .skip(1)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            if args.is_empty() {
                anyhow::bail!("invalid RTK setup command: {}", setup.command);
            }
            let output = Command::new(&binary.path)
                .args(args.drain(..))
                .current_dir(&setup.cwd)
                .env(RTK_TELEMETRY_DISABLED, "1")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .with_context(|| format!("failed to run {}", setup.command))?;
            stdout.push_str(&String::from_utf8_lossy(&output.stdout));
            stderr.push_str(&String::from_utf8_lossy(&output.stderr));
            if !output.status.success() {
                anyhow::bail!("RTK setup failed: {}", setup.command);
            }
        }
    }

    Ok(RtkSetupResult {
        applied: apply,
        commands,
        stdout,
        stderr,
        status: status(app, enabled, Some(project_path)),
    })
}

pub fn configure_agent_command(
    app: &AppHandle,
    command: &mut Command,
    enabled: bool,
) -> RtkCommandEnv {
    command.env(RTK_TELEMETRY_DISABLED, "1");
    let Some(binary) = resolve_binary(app) else {
        return RtkCommandEnv {
            enabled,
            telemetry_blocked: true,
            binary_path: None,
            binary_source: None,
            path_preprended: false,
        };
    };
    command.env(RTK_ENV_BIN, &binary.path);
    if enabled {
        if let Some(parent) = binary.path.parent() {
            prepend_path(command, parent);
            return RtkCommandEnv {
                enabled,
                telemetry_blocked: true,
                binary_path: Some(binary.path.display().to_string()),
                binary_source: Some(binary.source.to_string()),
                path_preprended: true,
            };
        }
    }
    RtkCommandEnv {
        enabled,
        telemetry_blocked: true,
        binary_path: Some(binary.path.display().to_string()),
        binary_source: Some(binary.source.to_string()),
        path_preprended: false,
    }
}

fn setup_commands(provider: &str, project_path: &str) -> anyhow::Result<Vec<RtkSetupCommand>> {
    let cwd = if project_path.trim().is_empty() {
        env::current_dir()?.display().to_string()
    } else {
        project_path.trim().to_string()
    };
    let (command, description) = match provider.trim() {
        "codex" => (
            "rtk init --codex",
            "Add RTK awareness to the project AGENTS.md for Codex.",
        ),
        "claude" => (
            "rtk init --global --auto-patch",
            "Install the Claude Code PreToolUse hook with RTK's native backup flow.",
        ),
        "copilot" => (
            "rtk init --copilot",
            "Install project-scoped Copilot hooks and instructions.",
        ),
        other => anyhow::bail!("unsupported RTK provider setup: {other}"),
    };
    Ok(vec![RtkSetupCommand {
        provider: provider.trim().to_string(),
        cwd,
        command: command.to_string(),
        description: description.to_string(),
    }])
}

fn resolve_binary(app: &AppHandle) -> Option<RtkBinary> {
    if let Some(path) = env::var_os(RTK_ENV_BIN).map(PathBuf::from) {
        if is_executable_file(&path) {
            return Some(RtkBinary {
                path,
                source: "env",
            });
        }
    }
    for path in managed_candidates(app) {
        if is_executable_file(&path) {
            return Some(RtkBinary {
                path,
                source: "managed",
            });
        }
    }
    for path in bundled_candidates(app) {
        if is_executable_file(&path) {
            return Some(RtkBinary {
                path,
                source: "bundle",
            });
        }
    }
    find_on_path().map(|path| RtkBinary {
        path,
        source: "path",
    })
}

fn managed_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let Ok(base_dir) = managed_rtk_base_dir(app) else {
        return Vec::new();
    };
    let exe = rtk_exe_name();
    vec![
        base_dir.join(RTK_VERSION).join(exe),
        base_dir.join(exe),
        base_dir.join("bin").join(exe),
    ]
}

fn managed_rtk_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    Ok(managed_rtk_base_dir(app)?.join(RTK_VERSION))
}

fn managed_rtk_base_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let app_data_dir = if let Some(home) = env::var_os("DW_GUI_HOME") {
        PathBuf::from(home)
    } else if let Ok(path) = app.path().app_data_dir() {
        path
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(env::temp_dir)
            .join("clia-app")
    };
    Ok(app_data_dir.join("rtk"))
}

fn bundled_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        let exe = rtk_exe_name();
        candidates.push(resource_dir.join("binaries").join("rtk").join(exe));
        candidates.push(resource_dir.join("rtk").join(exe));
        candidates.push(resource_dir.join(exe));
    }
    candidates
}

fn find_on_path() -> Option<PathBuf> {
    let name = rtk_exe_name();
    let path_env = env::var_os("PATH")?;
    for entry in env::split_paths(&path_env) {
        let candidate = entry.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        if cfg!(windows) {
            for extension in ["cmd", "bat", "ps1"] {
                let candidate = entry.join(format!("rtk.{extension}"));
                if is_executable_file(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn rtk_exe_name() -> &'static str {
    if cfg!(windows) {
        "rtk.exe"
    } else {
        "rtk"
    }
}

fn prepend_path(command: &mut Command, path: &Path) {
    let mut entries = vec![path.to_path_buf()];
    if let Some(path_env) = env::var_os("PATH") {
        entries.extend(env::split_paths(&path_env));
    }
    if let Ok(joined) = env::join_paths(entries) {
        command.env("PATH", joined);
    }
}

struct RtkOutput {
    status_success: bool,
    stdout: String,
    stderr: String,
}

fn run_rtk(
    binary: &RtkBinary,
    args: &[&str],
    project_path: Option<&str>,
) -> anyhow::Result<RtkOutput> {
    let mut command = Command::new(&binary.path);
    command.args(args).env(RTK_TELEMETRY_DISABLED, "1");
    if let Some(project_path) = project_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.current_dir(project_path);
    }
    let output = command.output()?;
    Ok(RtkOutput {
        status_success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn first_line(value: &str) -> Option<String> {
    value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn current_release_asset() -> anyhow::Result<RtkReleaseAsset> {
    release_asset_for(env::consts::OS, env::consts::ARCH).ok_or_else(|| {
        anyhow!(
            "RTK download is not configured for {}-{}",
            env::consts::OS,
            env::consts::ARCH
        )
    })
}

fn release_asset_for(os: &str, arch: &str) -> Option<RtkReleaseAsset> {
    match (os, arch) {
        ("linux", "x86_64") => Some(RtkReleaseAsset {
            file: "rtk-x86_64-unknown-linux-musl.tar.gz",
            sha256: "5df764a633709cb85d248258d085d24ec95faa8bca0e6835a93cd57cadc4eb9e",
        }),
        ("linux", "aarch64") => Some(RtkReleaseAsset {
            file: "rtk-aarch64-unknown-linux-gnu.tar.gz",
            sha256: "2b7fa09d06f8dbf334c55482fad2e7ce4a1f8564bc9ed1f65d9f5992db8e5527",
        }),
        ("macos", "x86_64") => Some(RtkReleaseAsset {
            file: "rtk-x86_64-apple-darwin.tar.gz",
            sha256: "7c72d05cfc71b7e2f20755b3754b728acecc7c0b1fbbb08757828f9e7bedd81a",
        }),
        ("macos", "aarch64") => Some(RtkReleaseAsset {
            file: "rtk-aarch64-apple-darwin.tar.gz",
            sha256: "d47823afb25919e4e60838c5622e88ffa6536bc0b36a34a3f928bdccac40f614",
        }),
        ("windows", "x86_64") => Some(RtkReleaseAsset {
            file: "rtk-x86_64-pc-windows-msvc.zip",
            sha256: "334d05a6662576a84a78b771aee0749202eacabd87acbb9fd266e6a5466f700a",
        }),
        _ => None,
    }
}

fn download_archive(url: &str, archive_path: &Path) -> anyhow::Result<()> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(90))
        .call()
        .map_err(|err| anyhow!("RTK download failed: {err}"))?;
    let mut reader = response.into_reader();
    let mut file = File::create(archive_path)
        .with_context(|| format!("failed to create {}", archive_path.display()))?;
    std::io::copy(&mut reader, &mut file)
        .with_context(|| format!("failed to write {}", archive_path.display()))?;
    file.flush()?;
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> anyhow::Result<()> {
    let mut file =
        File::open(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        anyhow::bail!(
            "RTK checksum mismatch for {}: expected {}, got {}",
            path.display(),
            expected,
            actual
        );
    }
    Ok(())
}

fn extract_archive(archive_path: &Path, extract_dir: &Path) -> anyhow::Result<()> {
    let archive = archive_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if archive.ends_with(".zip") {
        extract_zip(archive_path, extract_dir)
    } else {
        extract_tar_gz(archive_path, extract_dir)
    }
}

fn extract_zip(archive_path: &Path, extract_dir: &Path) -> anyhow::Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("failed to open {}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read zip {}", archive_path.display()))?;
    archive
        .extract(extract_dir)
        .with_context(|| format!("failed to extract {}", archive_path.display()))?;
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, extract_dir: &Path) -> anyhow::Result<()> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(extract_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| "failed to execute tar for RTK archive")?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to extract {}: {}",
            archive_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn find_binary_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|value| value.to_str()) == Some(name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_binary_recursive(&path, name) {
                return Some(found);
            }
        }
    }
    None
}

fn make_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_commands_are_provider_specific() {
        let codex = setup_commands("codex", "/repo").expect("codex setup");
        assert_eq!(codex[0].command, "rtk init --codex");
        assert_eq!(codex[0].cwd, "/repo");

        let claude = setup_commands("claude", "/repo").expect("claude setup");
        assert_eq!(claude[0].command, "rtk init --global --auto-patch");

        let copilot = setup_commands("copilot", "/repo").expect("copilot setup");
        assert_eq!(copilot[0].command, "rtk init --copilot");
    }

    #[test]
    fn release_assets_cover_supported_platforms() {
        assert_eq!(
            release_asset_for("linux", "x86_64").map(|asset| asset.file),
            Some("rtk-x86_64-unknown-linux-musl.tar.gz")
        );
        assert_eq!(
            release_asset_for("windows", "x86_64").map(|asset| asset.file),
            Some("rtk-x86_64-pc-windows-msvc.zip")
        );
        assert!(release_asset_for("freebsd", "x86_64").is_none());
    }
}
