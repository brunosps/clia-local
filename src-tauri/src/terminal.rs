use anyhow::{anyhow, Context};
use chrono::Utc;
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const TERMINAL_LOG_PREFIX: &str = "terminal-";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TerminalStatus {
    #[allow(dead_code)]
    Idle,
    Running,
    Exited,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct TerminalSession {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub shell: String,
    pub status: TerminalStatus,
    pub log_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TerminalOutputEvent {
    pub session_id: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TerminalStatusEvent {
    pub session_id: String,
    pub status: TerminalStatus,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TerminalErrorEvent {
    pub session_id: String,
    pub message: String,
}

#[derive(Clone)]
pub struct TerminalManager {
    registry: Arc<Mutex<TerminalRegistry>>,
}

struct TerminalRegistry {
    next_id: u64,
    sessions: HashMap<String, TerminalRuntime>,
    log_dir: PathBuf,
}

struct TerminalRuntime {
    session: TerminalSession,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self::with_log_dir(default_log_dir())
    }

    fn with_log_dir(log_dir: PathBuf) -> Self {
        Self {
            registry: Arc::new(Mutex::new(TerminalRegistry {
                next_id: 1,
                sessions: HashMap::new(),
                log_dir,
            })),
        }
    }

    pub fn list_sessions(&self) -> anyhow::Result<Vec<TerminalSession>> {
        let registry = self.lock()?;
        let mut sessions = registry
            .sessions
            .values()
            .map(|runtime| runtime.session.clone())
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(sessions)
    }

    pub fn write_input(&self, session_id: &str, data: &str) -> anyhow::Result<()> {
        let mut registry = self.lock()?;
        let runtime = registry
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("terminal session not found: {session_id}"))?;
        ensure_running(&runtime.session)?;
        let writer = runtime
            .writer
            .as_mut()
            .ok_or_else(|| anyhow!("terminal session writer is unavailable"))?;
        writer
            .write_all(data.as_bytes())
            .with_context(|| format!("failed to write input to terminal session {session_id}"))?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> anyhow::Result<()> {
        if !valid_terminal_size(cols, rows) {
            return Err(anyhow!("terminal size must be greater than zero"));
        }

        let registry = self.lock()?;
        let runtime = registry
            .sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("terminal session not found: {session_id}"))?;
        ensure_running(&runtime.session)?;
        let master = runtime
            .master
            .as_ref()
            .ok_or_else(|| anyhow!("terminal session PTY is unavailable"))?;
        master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn stop(&self, session_id: &str) -> anyhow::Result<TerminalSession> {
        let mut killer = {
            let mut registry = self.lock()?;
            let runtime = registry
                .sessions
                .get_mut(session_id)
                .ok_or_else(|| anyhow!("terminal session not found: {session_id}"))?;

            if runtime.session.status != TerminalStatus::Running {
                return Ok(runtime.session.clone());
            }

            runtime.session.status = TerminalStatus::Stopped;
            runtime.session.updated_at = now();
            runtime
                .killer
                .as_ref()
                .map(|killer| killer.clone_killer())
                .ok_or_else(|| anyhow!("terminal session killer is unavailable"))?
        };

        killer
            .kill()
            .with_context(|| format!("failed to stop terminal session {session_id}"))?;
        let mut force_killer = killer.clone_killer();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(750));
            let _ = force_killer.kill();
        });

        self.get_session(session_id)
    }

    pub fn close(&self, session_id: &str) -> anyhow::Result<()> {
        let removed = {
            let mut registry = self.lock()?;
            let Some(mut runtime) = registry.sessions.remove(session_id) else {
                return Err(anyhow!("terminal session not found: {session_id}"));
            };
            if runtime.session.status == TerminalStatus::Running {
                if let Some(killer) = runtime.killer.as_mut() {
                    let _ = killer.kill();
                }
                runtime.session.status = TerminalStatus::Stopped;
            }
            runtime
        };

        let _ = std::fs::remove_file(removed.session.log_path);
        Ok(())
    }

    pub fn cleanup_temp_logs(&self) -> anyhow::Result<()> {
        let log_dir = self.lock()?.log_dir.clone();
        cleanup_temp_logs_in(&log_dir)
    }

    fn next_session(&self, cwd: &Path, shell: &str) -> anyhow::Result<TerminalSession> {
        let mut registry = self.lock()?;
        std::fs::create_dir_all(&registry.log_dir)?;
        let id = format!("terminal-{}", registry.next_id);
        registry.next_id += 1;
        let title = format!("Terminal {}", registry.next_id - 1);
        let log_path = registry
            .log_dir
            .join(format!("{TERMINAL_LOG_PREFIX}{id}.log"));
        File::create(&log_path)
            .with_context(|| format!("failed to create terminal log {}", log_path.display()))?;
        let timestamp = now();
        Ok(TerminalSession {
            id,
            title,
            cwd: cwd.display().to_string(),
            shell: shell.to_string(),
            status: TerminalStatus::Running,
            log_path: log_path.display().to_string(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
            exit_code: None,
        })
    }

    fn insert_runtime(
        &self,
        session: TerminalSession,
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        killer: Box<dyn ChildKiller + Send + Sync>,
    ) -> anyhow::Result<TerminalSession> {
        let mut registry = self.lock()?;
        registry.sessions.insert(
            session.id.clone(),
            TerminalRuntime {
                session: session.clone(),
                master: Some(master),
                writer: Some(writer),
                killer: Some(killer),
            },
        );
        Ok(session)
    }

    fn set_status(
        &self,
        session_id: &str,
        status: TerminalStatus,
        exit_code: Option<i32>,
    ) -> anyhow::Result<TerminalSession> {
        let mut registry = self.lock()?;
        let runtime = registry
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("terminal session not found: {session_id}"))?;
        runtime.session.status = status;
        runtime.session.exit_code = exit_code;
        runtime.session.updated_at = now();
        Ok(runtime.session.clone())
    }

    fn get_session(&self, session_id: &str) -> anyhow::Result<TerminalSession> {
        let registry = self.lock()?;
        registry
            .sessions
            .get(session_id)
            .map(|runtime| runtime.session.clone())
            .ok_or_else(|| anyhow!("terminal session not found: {session_id}"))
    }

    fn current_status(&self, session_id: &str) -> Option<TerminalStatus> {
        self.lock().ok().and_then(|registry| {
            registry
                .sessions
                .get(session_id)
                .map(|runtime| runtime.session.status.clone())
        })
    }

    fn lock(&self) -> anyhow::Result<std::sync::MutexGuard<'_, TerminalRegistry>> {
        self.registry
            .lock()
            .map_err(|_| anyhow!("terminal manager lock poisoned"))
    }

    #[cfg(test)]
    fn insert_session_metadata(&self, session: TerminalSession) -> anyhow::Result<TerminalSession> {
        let mut registry = self.lock()?;
        registry.sessions.insert(
            session.id.clone(),
            TerminalRuntime {
                session: session.clone(),
                master: None,
                writer: None,
                killer: None,
            },
        );
        Ok(session)
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TerminalRegistry {
    fn drop(&mut self) {
        let _ = cleanup_temp_logs_in(&self.log_dir);
    }
}

pub fn create_session(
    app: tauri::AppHandle,
    manager: TerminalManager,
    path: PathBuf,
    shell: Option<String>,
    initial_input: Option<String>,
) -> anyhow::Result<TerminalSession> {
    let cwd = canonical_terminal_cwd(&path)?;
    let shell = resolve_shell(shell.as_deref())?;
    let session = manager.next_session(&cwd, &shell)?;

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(default_pty_size())?;
    let command = shell_command(&shell, &cwd);
    let child = pair
        .slave
        .spawn_command(command)
        .with_context(|| format!("failed to spawn terminal shell {shell}"))?;
    let killer = child.clone_killer();
    let reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;
    let master = pair.master;

    let session = manager.insert_runtime(session, master, writer, killer)?;
    spawn_output_reader(
        app.clone(),
        session.id.clone(),
        session.log_path.clone(),
        reader,
    );
    spawn_waiter(app, manager.clone(), session.id.clone(), child);

    if let Some(input) = initial_input.filter(|input| !input.is_empty()) {
        // Let the shell initialize before injecting the agent launch command.
        thread::sleep(Duration::from_millis(150));
        let normalized_input = if input.ends_with('\n') {
            input
        } else {
            format!("{input}\n")
        };
        manager.write_input(&session.id, &normalized_input)?;
    }

    Ok(session)
}

pub fn default_shell() -> &'static str {
    if cfg!(target_os = "windows") {
        "pwsh"
    } else {
        "bash"
    }
}

pub fn resolve_shell(shell: Option<&str>) -> anyhow::Result<String> {
    let shell = match shell.map(str::trim).filter(|shell| !shell.is_empty()) {
        Some(shell) => shell.to_string(),
        None => default_shell().to_string(),
    };
    let allowed = if cfg!(target_os = "windows") {
        matches!(shell.as_str(), "pwsh" | "powershell")
    } else {
        matches!(shell.as_str(), "bash")
    };

    if allowed {
        Ok(shell)
    } else {
        Err(anyhow!("unsupported terminal shell: {shell}"))
    }
}

pub fn canonical_terminal_cwd(path: &Path) -> anyhow::Result<PathBuf> {
    let cwd = std::fs::canonicalize(path)
        .with_context(|| format!("failed to resolve terminal cwd {}", path.display()))?;
    if !cwd.is_dir() {
        return Err(anyhow!("terminal cwd is not a directory"));
    }
    Ok(cwd)
}

pub fn valid_terminal_size(cols: u16, rows: u16) -> bool {
    cols > 0 && rows > 0
}

fn ensure_running(session: &TerminalSession) -> anyhow::Result<()> {
    if session.status == TerminalStatus::Running {
        Ok(())
    } else {
        Err(anyhow!(
            "terminal session {} is not running: {:?}",
            session.id,
            session.status
        ))
    }
}

fn shell_command(shell: &str, cwd: &Path) -> CommandBuilder {
    let mut command = CommandBuilder::new(shell);
    if cfg!(target_os = "windows") && shell == "pwsh" {
        command.arg("-NoLogo");
    }
    command.cwd(cwd.as_os_str());
    command
}

fn default_pty_size() -> PtySize {
    PtySize {
        rows: DEFAULT_ROWS,
        cols: DEFAULT_COLS,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn spawn_output_reader(
    app: tauri::AppHandle,
    session_id: String,
    log_path: String,
    mut reader: Box<dyn Read + Send>,
) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                    let _ = append_log(&log_path, data.as_bytes());
                    let _ = app.emit(
                        "terminal://output",
                        TerminalOutputEvent {
                            session_id: session_id.clone(),
                            data,
                        },
                    );
                }
                Err(error) => {
                    let _ = app.emit(
                        "terminal://error",
                        TerminalErrorEvent {
                            session_id: session_id.clone(),
                            message: error.to_string(),
                        },
                    );
                    break;
                }
            }
        }
    });
}

fn spawn_waiter(
    app: tauri::AppHandle,
    manager: TerminalManager,
    session_id: String,
    mut child: Box<dyn Child + Send>,
) {
    thread::spawn(move || match child.wait() {
        Ok(exit_status) => {
            let exit_code = Some(exit_status.exit_code() as i32);
            let status = if manager.current_status(&session_id) == Some(TerminalStatus::Stopped) {
                TerminalStatus::Stopped
            } else if exit_status.success() {
                TerminalStatus::Exited
            } else {
                TerminalStatus::Failed
            };
            if let Ok(session) = manager.set_status(&session_id, status.clone(), exit_code) {
                let _ = app.emit(
                    "terminal://status",
                    TerminalStatusEvent {
                        session_id,
                        status,
                        exit_code: session.exit_code,
                        message: None,
                    },
                );
            }
        }
        Err(error) => {
            let _ = manager.set_status(&session_id, TerminalStatus::Failed, None);
            let _ = app.emit(
                "terminal://error",
                TerminalErrorEvent {
                    session_id,
                    message: error.to_string(),
                },
            );
        }
    });
}

fn append_log(log_path: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("failed to append terminal log {log_path}"))?;
    file.write_all(bytes)?;
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn default_log_dir() -> PathBuf {
    std::env::temp_dir().join("clia-app").join("terminal")
}

fn cleanup_temp_logs_in(log_dir: &Path) -> anyhow::Result<()> {
    if !log_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_file() && name.starts_with(TERMINAL_LOG_PREFIX) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-gui-terminal-test-{unique}"));
        std::fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    #[test]
    fn default_shell_matches_target_family() {
        if cfg!(target_os = "windows") {
            assert_eq!(default_shell(), "pwsh");
        } else {
            assert_eq!(default_shell(), "bash");
        }
    }

    #[test]
    fn resolve_shell_rejects_unsupported_shells() {
        assert!(resolve_shell(Some("python")).is_err());
    }

    #[test]
    fn canonical_terminal_cwd_requires_directory() {
        let root = fixture_root();
        let file_path = root.join("file.txt");
        std::fs::write(&file_path, "not a dir").expect("write file");

        assert!(canonical_terminal_cwd(&root)
            .expect("canonical dir")
            .is_dir());
        assert!(canonical_terminal_cwd(&file_path).is_err());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn terminal_size_rejects_zero_dimensions() {
        assert!(valid_terminal_size(80, 24));
        assert!(!valid_terminal_size(0, 24));
        assert!(!valid_terminal_size(80, 0));
    }

    #[test]
    fn manager_creates_lists_updates_and_removes_session_metadata() {
        let root = fixture_root();
        let log_dir = root.join("logs");
        let manager = TerminalManager::with_log_dir(log_dir);
        let session = manager
            .next_session(&root, "bash")
            .expect("create session metadata");
        manager
            .insert_session_metadata(session.clone())
            .expect("insert runtime");

        assert_eq!(manager.list_sessions().expect("list").len(), 1);
        let updated = manager
            .set_status(&session.id, TerminalStatus::Exited, Some(0))
            .expect("status update");
        assert_eq!(updated.status, TerminalStatus::Exited);
        assert_eq!(updated.exit_code, Some(0));
        manager.close(&session.id).expect("close session");
        assert!(manager
            .list_sessions()
            .expect("list after close")
            .is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn cleanup_temp_logs_removes_only_terminal_logs() {
        let root = fixture_root();
        let log_dir = root.join("logs");
        std::fs::create_dir_all(&log_dir).expect("create logs");
        std::fs::write(log_dir.join("terminal-terminal-1.log"), "terminal").expect("write log");
        std::fs::write(log_dir.join("keep.txt"), "keep").expect("write keep");

        cleanup_temp_logs_in(&log_dir).expect("cleanup logs");

        assert!(!log_dir.join("terminal-terminal-1.log").exists());
        assert!(log_dir.join("keep.txt").exists());

        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
