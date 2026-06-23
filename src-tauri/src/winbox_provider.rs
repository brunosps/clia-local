use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineProviderStatus {
    pub provider: String,
    pub runtime: String,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub status: String,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineError {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
    pub provider_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxVersion {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxDistro {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxProfile {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub web_port: Option<i64>,
    #[serde(default)]
    pub rdp_port: Option<i64>,
    #[serde(default)]
    pub ssh_port: Option<i64>,
    #[serde(default)]
    pub ram: Option<String>,
    #[serde(default)]
    pub bundles: Option<String>,
    #[serde(default)]
    pub image_family: Option<String>,
    #[serde(default)]
    pub boot: Option<String>,
    #[serde(default)]
    pub cloud_init_profile: Option<String>,
    #[serde(default)]
    pub iso_path: Option<String>,
    #[serde(default)]
    pub shared_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxOperation {
    pub profile: Option<WinboxProfile>,
    pub operation: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxLogs {
    pub profile: String,
    pub logs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxViewerUrl {
    pub profile: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinboxProgress {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub percent: Option<u8>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WinboxProvider {
    command: ProviderCommand,
}

#[derive(Debug, Clone)]
struct ProviderCommand {
    executable: String,
    runtime: String,
    prefix_args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JsonEnvelope<T> {
    ok: bool,
    value: Option<T>,
    error: Option<MachineError>,
}

pub fn discover() -> Option<WinboxProvider> {
    if let Ok(path) = env::var("WINBOX_BIN") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(WinboxProvider::new_native(trimmed.to_string()));
        }
    }
    if let Some(path) = find_on_path("winbox") {
        return Some(WinboxProvider::new_native(path.display().to_string()));
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(command) = env::var("WINBOX_WSL_COMMAND") {
            let trimmed = command.trim();
            if !trimmed.is_empty() {
                return Some(WinboxProvider {
                    command: ProviderCommand {
                        executable: "wsl.exe".to_string(),
                        runtime: "wsl".to_string(),
                        prefix_args: split_wsl_command(trimmed),
                    },
                });
            }
        }
    }
    None
}

pub fn check_status() -> MachineProviderStatus {
    let Some(provider) = discover() else {
        return MachineProviderStatus {
            provider: "winbox".to_string(),
            runtime: "unavailable".to_string(),
            executable: None,
            version: None,
            status: "unavailable".to_string(),
            message: "Winbox CLI was not found.".to_string(),
            hint: Some("Install Winbox or set WINBOX_BIN to the executable path.".to_string()),
        };
    };

    match provider.version() {
        Ok(version) => MachineProviderStatus {
            provider: "winbox".to_string(),
            runtime: provider.command.runtime.clone(),
            executable: Some(provider.command.executable.clone()),
            version: version.version,
            status: "ready".to_string(),
            message: "Winbox provider ready.".to_string(),
            hint: None,
        },
        Err(error) => MachineProviderStatus {
            provider: "winbox".to_string(),
            runtime: provider.command.runtime.clone(),
            executable: Some(provider.command.executable.clone()),
            version: None,
            status: "incompatible".to_string(),
            message: error.message,
            hint: error.hint,
        },
    }
}

impl WinboxProvider {
    fn new_native(executable: String) -> Self {
        Self {
            command: ProviderCommand {
                executable,
                runtime: "native".to_string(),
                prefix_args: Vec::new(),
            },
        }
    }

    pub fn runtime(&self) -> &str {
        &self.command.runtime
    }

    pub fn version(&self) -> Result<WinboxVersion, MachineError> {
        self.run_json(["version"])
    }

    pub fn distros(&self) -> Result<Vec<WinboxDistro>, MachineError> {
        self.run_json(["distros"])
    }

    pub fn list(&self) -> Result<Vec<WinboxProfile>, MachineError> {
        self.run_json(["list"])
    }

    pub fn host_health(&self) -> Result<Value, MachineError> {
        self.run_json(["host-health"])
    }

    pub fn profile(&self, name: &str) -> Result<WinboxProfile, MachineError> {
        self.run_json(["profile", name])
    }

    pub fn logs(&self, name: &str, tail: u32) -> Result<WinboxLogs, MachineError> {
        self.run_json(["logs", name, "--tail", &tail.to_string()])
    }

    pub fn viewer_url(&self, name: &str) -> Result<WinboxViewerUrl, MachineError> {
        self.run_json(["viewer-url", name])
    }

    pub fn set_extra_ports(
        &self,
        name: &str,
        extra_ports: &str,
        restart: bool,
    ) -> Result<Value, MachineError> {
        let mut args = vec!["set", "--extra-ports", extra_ports];
        if restart {
            args.push("--restart");
        }
        args.push(name);
        self.run_json(args)
    }

    pub fn stop(&self, name: &str) -> Result<Value, MachineError> {
        self.run_json(["stop", name])
    }

    pub fn remove(&self, name: &str) -> Result<Value, MachineError> {
        self.run_json(["remove", name, "--yes", "--delete-storage"])
    }

    pub fn start<F>(&self, name: &str, mut on_progress: F) -> Result<Value, MachineError>
    where
        F: FnMut(WinboxProgress),
    {
        self.run_json_streaming(["start", name, "--progress", "jsonl"], &mut on_progress)
    }

    pub fn install<F>(
        &self,
        args: &[String],
        mut on_progress: F,
    ) -> Result<WinboxOperation, MachineError>
    where
        F: FnMut(WinboxProgress),
    {
        self.run_json_streaming(args.iter().map(String::as_str), &mut on_progress)
    }

    fn run_json<T, I, S>(&self, args: I) -> Result<T, MachineError>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let output = self
            .command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| {
                MachineError::from_message("winbox_spawn_failed", error.to_string())
            })?;
        parse_envelope_output(&output.stdout, &output.stderr, output.status.success())
    }

    fn run_json_streaming<T, I, S, F>(
        &self,
        args: I,
        on_progress: &mut F,
    ) -> Result<T, MachineError>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
        F: FnMut(WinboxProgress),
    {
        let output = self
            .command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| {
                MachineError::from_message("winbox_spawn_failed", error.to_string())
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut envelope_line = None;
        for line in stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            if line.contains("\"ok\"") {
                envelope_line = Some(line.to_string());
                continue;
            }
            if let Ok(progress) = serde_json::from_str::<WinboxProgress>(line) {
                on_progress(progress);
                continue;
            }
            envelope_line = Some(line.to_string());
        }
        if let Some(line) = envelope_line {
            parse_envelope_line(
                &line,
                &String::from_utf8_lossy(&output.stderr),
                output.status.success(),
            )
        } else {
            parse_envelope_output(&output.stdout, &output.stderr, output.status.success())
        }
    }

    fn command<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut command = Command::new(&self.command.executable);
        command.args(&self.command.prefix_args);
        command.arg("--json");
        for arg in args {
            command.arg(arg.as_ref());
        }
        command
    }
}

impl MachineError {
    pub fn from_message(code: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: code.into(),
            hint: Some(default_hint_for_message(&message).to_string()),
            provider_detail: None,
            message,
        }
    }
}

pub fn redacted_args(args: &[String]) -> Vec<String> {
    let mut output = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            output.push("***".to_string());
            redact_next = false;
            continue;
        }
        let lower = arg.to_lowercase();
        if matches!(
            lower.as_str(),
            "--pass" | "--password" | "--token" | "--api-key"
        ) {
            output.push(arg.clone());
            redact_next = true;
        } else if lower.contains("password=")
            || lower.contains("token=")
            || lower.contains("api_key=")
        {
            output.push("***".to_string());
        } else {
            output.push(arg.clone());
        }
    }
    output
}

fn parse_envelope_output<T>(stdout: &[u8], stderr: &[u8], success: bool) -> Result<T, MachineError>
where
    T: DeserializeOwned,
{
    let stdout = String::from_utf8_lossy(stdout);
    let line = stdout
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| {
            MachineError::from_message(
                "winbox_empty_output",
                format!(
                    "Winbox returned no JSON output. stderr: {}",
                    String::from_utf8_lossy(stderr)
                ),
            )
        })?;
    parse_envelope_line(line, &String::from_utf8_lossy(stderr), success)
}

fn parse_envelope_line<T>(line: &str, stderr: &str, success: bool) -> Result<T, MachineError>
where
    T: DeserializeOwned,
{
    let envelope: JsonEnvelope<T> = serde_json::from_str(line).map_err(|error| {
        MachineError::from_message(
            "winbox_invalid_json",
            format!("Winbox returned invalid JSON: {error}. stderr: {stderr}"),
        )
    })?;
    if envelope.ok && success {
        return envelope.value.ok_or_else(|| {
            MachineError::from_message("winbox_missing_value", "Winbox response omitted value.")
        });
    }
    Err(envelope.error.unwrap_or_else(|| {
        MachineError::from_message("operation_failed", "Winbox command failed.")
    }))
}

fn default_hint_for_message(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains("not found") || lower.contains("no such file") {
        "Install Winbox or configure WINBOX_BIN."
    } else if lower.contains("json") {
        "Upgrade Winbox to a version that supports --json."
    } else {
        "Check Winbox and retry."
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(executable_name(name)))
        .find(|path| path.is_file())
}

fn executable_name(name: &str) -> OsString {
    #[cfg(target_os = "windows")]
    {
        OsString::from(format!("{name}.exe"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        OsString::from(name)
    }
}

#[cfg(target_os = "windows")]
fn split_wsl_command(command: &str) -> Vec<String> {
    command.split_whitespace().map(ToOwned::to_owned).collect()
}

pub fn ensure_supported_distro(distros: &[WinboxDistro], id: &str) -> Result<(), MachineError> {
    if distros.iter().any(|distro| distro.id == id) {
        return Ok(());
    }
    Err(MachineError::from_message(
        "unsupported_preset",
        format!("Winbox does not expose {id}."),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_envelope() {
        let value: WinboxVersion = parse_envelope_output(
            br#"{"ok":true,"value":{"name":"winbox","version":"1"}}"#,
            b"",
            true,
        )
        .expect("parse");
        assert_eq!(value.version.as_deref(), Some("1"));
    }

    #[test]
    fn parses_error_envelope() {
        let result: Result<WinboxVersion, MachineError> = parse_envelope_output(
            br#"{"ok":false,"error":{"code":"winbox_not_found","message":"missing","hint":"install"}}"#,
            b"",
            false,
        );
        let error = result.expect_err("error");
        assert_eq!(error.code, "winbox_not_found");
        assert_eq!(error.hint.as_deref(), Some("install"));
    }

    #[test]
    fn parses_viewer_url_envelope() {
        let value: WinboxViewerUrl = parse_envelope_output(
            br#"{"ok":true,"value":{"profile":"dw-3-windows-11-dev","url":"http://127.0.0.1:8006/?autoconnect=true&resize=scale"}}"#,
            b"",
            true,
        )
        .expect("parse");
        assert_eq!(value.profile, "dw-3-windows-11-dev");
        assert_eq!(
            value.url,
            "http://127.0.0.1:8006/?autoconnect=true&resize=scale"
        );
    }

    #[test]
    fn redacts_password_args() {
        let args = vec![
            "install".to_string(),
            "vm".to_string(),
            "--pass".to_string(),
            "secret".to_string(),
        ];
        assert_eq!(redacted_args(&args)[3], "***");
    }

    #[test]
    fn supported_distros_are_checked_individually() {
        let distros = vec![
            WinboxDistro {
                id: "ubuntu-server".to_string(),
                label: "Ubuntu Server".to_string(),
            },
            WinboxDistro {
                id: "xubuntu".to_string(),
                label: "Xubuntu".to_string(),
            },
        ];
        assert!(ensure_supported_distro(&distros, "ubuntu-server").is_ok());
        assert!(ensure_supported_distro(&distros, "xubuntu").is_ok());
        assert!(ensure_supported_distro(&distros, "lubuntu").is_err());
    }
}
