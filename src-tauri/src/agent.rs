use crate::rtk;
use crate::store::{AgentMessage, AgentProfile, AgentSession, Database};
use anyhow::{anyhow, Context};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct AgentStreamEvent {
    pub session_id: i64,
    pub kind: String,
    pub content: String,
    pub raw_json: Option<String>,
    pub message: Option<AgentMessage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStatusEvent {
    pub session: AgentSession,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMetricEvent {
    pub session_id: i64,
    pub run_id: String,
    pub provider: String,
    pub phase: String,
    pub elapsed_ms: i64,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProviderHealth {
    pub provider: String,
    pub ok: bool,
    pub supported: bool,
    pub program: String,
    pub version: Option<String>,
    pub message: String,
    pub details: Value,
}

#[derive(Debug, Clone)]
pub struct AgentBlockingResult {
    pub session: AgentSession,
    pub assistant_output: String,
}

static RUNNING_AGENTS: OnceLock<Mutex<HashMap<i64, Arc<Mutex<Child>>>>> = OnceLock::new();
static CLAUDE_RUNTIMES: OnceLock<Mutex<HashMap<String, Arc<Mutex<ClaudeRuntime>>>>> =
    OnceLock::new();

struct ClaudeRuntime {
    key: String,
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    rx: Receiver<String>,
    stderr: Arc<Mutex<String>>,
    pid: u32,
}

#[derive(Clone)]
struct AgentContextPolicy {
    configured_mode: String,
    effective_mode: String,
    reason: &'static str,
    lean: bool,
}

pub fn spawn_agent_message(
    app: AppHandle,
    db: Database,
    session: AgentSession,
    prompt: String,
    metadata: Option<Value>,
) {
    thread::spawn(move || {
        if let Err(error) = run_agent_message(&app, &db, session.id, &prompt, metadata) {
            let _ = db.add_agent_message(session.id, "system", &error.to_string(), None);
            if let Ok(session) = db.update_agent_session_status(session.id, "failed", None) {
                emit_status(&app, &session);
            }
            let _ = app.emit(
                "agent://event",
                AgentStreamEvent {
                    session_id: session.id,
                    kind: "error".to_string(),
                    content: error.to_string(),
                    raw_json: None,
                    message: None,
                },
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub fn run_agent_prompt_blocking(
    app: &AppHandle,
    db: &Database,
    profile: &AgentProfile,
    project_id: Option<i64>,
    project_path: &str,
    title: &str,
    prompt: &str,
    metadata: Option<Value>,
    timeout: Duration,
) -> anyhow::Result<AgentBlockingResult> {
    let session =
        db.create_agent_session_scoped(profile, project_id, project_path, title, "chat", None)?;
    db.add_agent_message(session.id, "user", prompt, None)?;
    let app_for_thread = app.clone();
    let app_for_status = app.clone();
    let db_for_thread = db.clone();
    let session_id = session.id;
    let prompt = prompt.to_string();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = run_agent_message(
            &app_for_thread,
            &db_for_thread,
            session_id,
            &prompt,
            metadata,
        );
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            stop_agent_process(session_id);
            let failed = db.update_agent_session_status(session_id, "failed", None)?;
            emit_status(&app_for_status, &failed);
            anyhow::bail!(
                "deploy_agent_timeout: agent did not return a deploy plan within {} seconds",
                timeout.as_secs()
            );
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            anyhow::bail!("deploy_agent_failed: agent runner stopped before returning a plan");
        }
    }
    let session = db.get_agent_session(session_id)?;
    let assistant_output = db
        .list_agent_messages(session_id)?
        .into_iter()
        .rev()
        .find(|message| message.role == "assistant" && !message.content.trim().is_empty())
        .map(|message| message.content)
        .ok_or_else(|| anyhow!("deploy_agent_invalid_json: agent returned no assistant output"))?;
    Ok(AgentBlockingResult {
        session,
        assistant_output,
    })
}

pub fn stop_agent_process(session_id: i64) {
    if let Some(child) = running_agents()
        .lock()
        .ok()
        .and_then(|agents| agents.get(&session_id).cloned())
    {
        if let Ok(mut child) = child.lock() {
            terminate_child_process_tree(&mut child);
        }
    }
}

#[cfg(unix)]
fn configure_agent_process(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_agent_process(_command: &mut Command) {}

fn terminate_child_process_tree(child: &mut Child) {
    let pid = child.id();
    #[cfg(unix)]
    {
        let process_group = format!("-{pid}");
        let _ = Command::new("kill")
            .args(["-TERM", &process_group])
            .status();
        for _ in 0..10 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = Command::new("kill")
            .args(["-KILL", &process_group])
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
}

fn run_agent_message(
    app: &AppHandle,
    db: &Database,
    session_id: i64,
    prompt: &str,
    metadata: Option<Value>,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let run_id = new_run_id(session_id);
    let session = db.update_agent_session_status(session_id, "running", None)?;
    emit_status(app, &session);
    emit_metric(
        app,
        db,
        &session,
        &run_id,
        "started",
        &started,
        json!({
            "scope": session.scope,
            "project_path": session.project_path,
            "prompt_bytes": prompt.len(),
            "metadata": metadata
        }),
    );
    let context_policy = resolve_context_policy(&session, prompt, metadata.as_ref());
    let rtk_enabled = db
        .get_agent_profile(session.profile_id)
        .map(|profile| profile.rtk_enabled)
        .unwrap_or(false);
    emit_metric(
        app,
        db,
        &session,
        &run_id,
        "context_policy",
        &started,
        context_policy_details(&session.provider, &context_policy),
    );

    if session.provider == "claude" {
        return run_claude_runtime_message(
            app,
            db,
            session,
            prompt,
            &started,
            &run_id,
            &context_policy,
            rtk_enabled,
        );
    }

    let mut command = build_agent_command(&session, prompt, &run_id, &context_policy)?;
    let rtk_env = rtk::configure_agent_command(app, &mut command, rtk_enabled);
    if rtk_enabled {
        emit_metric(
            app,
            db,
            &session,
            &run_id,
            "rtk_env",
            &started,
            serde_json::to_value(&rtk_env).unwrap_or_else(|_| json!({})),
        );
    }
    emit_metric(
        app,
        db,
        &session,
        &run_id,
        "command_built",
        &started,
        command_details(&command),
    );
    configure_agent_process(&mut command);
    let mut child = match command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let message = missing_cli_message(&session.provider);
            emit_metric(
                app,
                db,
                &session,
                &run_id,
                "failed",
                &started,
                json!({
                    "message": message,
                    "spawn_error": error.to_string()
                }),
            );
            return Err(anyhow!(message).context(error));
        }
    };
    let child_pid = child.id();
    emit_metric(
        app,
        db,
        &session,
        &run_id,
        "process_spawned",
        &started,
        json!({ "pid": child_pid }),
    );

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("failed to capture {} stderr", session.provider))?;
    let stderr_handle = thread::spawn(move || read_to_string(stderr));
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to capture {} stdout", session.provider))?;

    let child = Arc::new(Mutex::new(child));
    running_agents()
        .lock()
        .map_err(|_| anyhow!("failed to track running agent"))?
        .insert(session_id, Arc::clone(&child));

    let mut provider_session_id = session
        .provider_session_id
        .clone()
        .or_else(|| session.codex_session_id.clone());
    let mut agent_error_hint: Option<String> = None;
    let mut saw_first_event = false;
    let mut captured_assistant = false;
    for line in BufReader::new(stdout).lines() {
        let line = line.with_context(|| format!("failed to read {} output", session.provider))?;
        if line.trim().is_empty() {
            continue;
        }
        if !saw_first_event {
            saw_first_event = true;
            emit_metric(
                app,
                db,
                &session,
                &run_id,
                "first_event",
                &started,
                json!({ "bytes": line.len() }),
            );
        }

        let parsed = serde_json::from_str::<Value>(&line).ok();
        if let Some(value) = parsed.as_ref() {
            if agent_error_hint.is_none() {
                agent_error_hint = auth_or_permission_hint(&session.provider, &line);
            }
            if provider_session_id.is_none() {
                provider_session_id = extract_session_id(value);
                if let Some(id) = provider_session_id.as_deref() {
                    let updated =
                        db.update_agent_session_status(session_id, "running", Some(id))?;
                    emit_status(app, &updated);
                }
            }
            if let Some((phase, details)) = provider_metric(&session.provider, value) {
                emit_metric(app, db, &session, &run_id, phase, &started, details);
            }
        }

        let raw_kind = parsed
            .as_ref()
            .and_then(|value| value.get("type").or_else(|| value.get("kind")))
            .and_then(Value::as_str)
            .unwrap_or("event")
            .to_string();
        let assistant_delta = parsed
            .as_ref()
            .and_then(|value| capture_agent_delta(&session.provider, value));
        let kind = assistant_delta
            .as_ref()
            .map(|_| "assistant_delta".to_string())
            .unwrap_or(raw_kind);
        let captured = parsed
            .as_ref()
            .and_then(|value| capture_agent_output(&session.provider, value));
        let event_content = assistant_delta
            .as_deref()
            .or_else(|| captured.as_ref().map(|capture| capture.content.as_str()))
            .unwrap_or_default()
            .to_string();
        let message = match captured {
            Some(capture) => {
                if capture.role == "assistant" {
                    captured_assistant = true;
                    emit_metric(
                        app,
                        db,
                        &session,
                        &run_id,
                        "assistant_output",
                        &started,
                        json!({ "bytes": capture.content.len() }),
                    );
                }
                db.add_agent_message(session_id, capture.role, &capture.content, Some(&line))
                    .ok()
            }
            None => db
                .add_agent_message(session_id, "event", "", Some(&line))
                .ok(),
        };

        let _ = app.emit(
            "agent://event",
            AgentStreamEvent {
                session_id,
                kind,
                content: event_content,
                raw_json: Some(line),
                message,
            },
        );
    }

    let status = wait_for_child(&child)?;
    running_agents()
        .lock()
        .map_err(|_| anyhow!("failed to untrack running agent"))?
        .remove(&session_id);
    let stderr_text = stderr_handle
        .join()
        .unwrap_or_else(|_| format!("failed to collect {} stderr", session.provider));

    if status.success() {
        if !captured_assistant {
            if let Some(content) = read_last_message_fallback(&session, &run_id) {
                if let Ok(message) = db.add_agent_message(session_id, "assistant", &content, None) {
                    emit_metric(
                        app,
                        db,
                        &session,
                        &run_id,
                        "assistant_output_fallback",
                        &started,
                        json!({ "bytes": content.len() }),
                    );
                    let _ = app.emit(
                        "agent://event",
                        AgentStreamEvent {
                            session_id,
                            kind: "assistant".to_string(),
                            content,
                            raw_json: None,
                            message: Some(message),
                        },
                    );
                }
            }
        }
        emit_metric(
            app,
            db,
            &session,
            &run_id,
            "finished",
            &started,
            json!({
                "exit_code": status.code(),
                "provider_session_id": provider_session_id.as_deref()
            }),
        );
        let session =
            db.update_agent_session_status(session_id, "done", provider_session_id.as_deref())?;
        emit_status(app, &session);
        return Ok(());
    }

    if let Ok(session) = db.get_agent_session(session_id) {
        if session.status == "stopped" {
            emit_status(app, &session);
            return Ok(());
        }
    }

    let message = if let Some(hint) =
        agent_error_hint.or_else(|| auth_or_permission_hint(&session.provider, &stderr_text))
    {
        hint
    } else if stderr_text.trim().is_empty() {
        format!("{} exited with status {status}", session.provider)
    } else {
        stderr_text
    };
    emit_metric(
        app,
        db,
        &session,
        &run_id,
        "failed",
        &started,
        json!({
            "exit_code": status.code(),
            "message": message
        }),
    );
    Err(anyhow!(message))
}

fn wait_for_child(child: &Arc<Mutex<Child>>) -> anyhow::Result<std::process::ExitStatus> {
    loop {
        {
            let mut child = child
                .lock()
                .map_err(|_| anyhow!("failed to lock running agent process"))?;
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[allow(clippy::too_many_arguments)]
fn run_claude_runtime_message(
    app: &AppHandle,
    db: &Database,
    session: AgentSession,
    prompt: &str,
    started: &Instant,
    run_id: &str,
    context_policy: &AgentContextPolicy,
    rtk_enabled: bool,
) -> anyhow::Result<()> {
    let runtime = ensure_claude_runtime(&session, context_policy, app, rtk_enabled)?;
    let (child, pid, key) = {
        let runtime_guard = runtime
            .lock()
            .map_err(|_| anyhow!("failed to lock Claude runtime"))?;
        (
            Arc::clone(&runtime_guard.child),
            runtime_guard.pid,
            runtime_guard.key.clone(),
        )
    };
    running_agents()
        .lock()
        .map_err(|_| anyhow!("failed to track running agent"))?
        .insert(session.id, Arc::clone(&child));
    emit_metric(
        app,
        db,
        &session,
        run_id,
        "runtime_ready",
        started,
        json!({ "pid": pid, "key": key }),
    );

    {
        let runtime_guard = runtime
            .lock()
            .map_err(|_| anyhow!("failed to lock Claude runtime"))?;
        let mut stdin = runtime_guard
            .stdin
            .lock()
            .map_err(|_| anyhow!("failed to lock Claude runtime stdin"))?;
        let line = claude_user_message_json(prompt)?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
    }
    emit_metric(
        app,
        db,
        &session,
        run_id,
        "message_sent",
        started,
        json!({ "bytes": prompt.len() }),
    );

    let mut provider_session_id = session.provider_session_id.clone();
    let mut agent_error_hint: Option<String> = None;
    let mut saw_first_event = false;
    let mut captured_assistant = false;
    let mut result_is_error = false;

    loop {
        let received = {
            let runtime_guard = runtime
                .lock()
                .map_err(|_| anyhow!("failed to lock Claude runtime"))?;
            runtime_guard.rx.recv_timeout(Duration::from_millis(100))
        };
        let line = match received {
            Ok(line) => line,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let status = child
                    .lock()
                    .map_err(|_| anyhow!("failed to lock Claude runtime process"))?
                    .try_wait()?;
                if let Some(status) = status {
                    remove_claude_runtime(&key);
                    running_agents()
                        .lock()
                        .map_err(|_| anyhow!("failed to untrack running agent"))?
                        .remove(&session.id);
                    let stderr = runtime
                        .lock()
                        .ok()
                        .and_then(|runtime| runtime.stderr.lock().ok().map(|text| text.clone()))
                        .unwrap_or_default();
                    let message = auth_or_permission_hint("claude", &stderr).unwrap_or_else(|| {
                        if stderr.trim().is_empty() {
                            format!("Claude exited with status {status}")
                        } else {
                            stderr
                        }
                    });
                    emit_metric(
                        app,
                        db,
                        &session,
                        run_id,
                        "failed",
                        started,
                        json!({ "exit_code": status.code(), "message": message }),
                    );
                    return Err(anyhow!(message));
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                remove_claude_runtime(&key);
                running_agents()
                    .lock()
                    .map_err(|_| anyhow!("failed to untrack running agent"))?
                    .remove(&session.id);
                return Err(anyhow!("Claude runtime stream closed"));
            }
        };

        if line.trim().is_empty() {
            continue;
        }
        if !saw_first_event {
            saw_first_event = true;
            emit_metric(
                app,
                db,
                &session,
                run_id,
                "first_event",
                started,
                json!({ "bytes": line.len(), "runtime": "persistent" }),
            );
        }

        let parsed = serde_json::from_str::<Value>(&line).ok();
        if let Some(value) = parsed.as_ref() {
            if agent_error_hint.is_none() {
                agent_error_hint = auth_or_permission_hint(&session.provider, &line);
            }
            if provider_session_id.is_none() {
                provider_session_id = extract_session_id(value);
                if let Some(id) = provider_session_id.as_deref() {
                    let updated =
                        db.update_agent_session_status(session.id, "running", Some(id))?;
                    emit_status(app, &updated);
                }
            }
            if claude_result_is_error(value) {
                result_is_error = true;
            }
            if let Some((phase, details)) = provider_metric(&session.provider, value) {
                emit_metric(app, db, &session, run_id, phase, started, details);
            }
        }

        let raw_kind = parsed
            .as_ref()
            .and_then(|value| value.get("type").or_else(|| value.get("kind")))
            .and_then(Value::as_str)
            .unwrap_or("event")
            .to_string();
        let assistant_delta = parsed
            .as_ref()
            .and_then(|value| capture_agent_delta(&session.provider, value));
        let kind = assistant_delta
            .as_ref()
            .map(|_| "assistant_delta".to_string())
            .unwrap_or(raw_kind);
        let captured = parsed
            .as_ref()
            .and_then(|value| capture_agent_output(&session.provider, value));
        let event_content = assistant_delta
            .as_deref()
            .or_else(|| captured.as_ref().map(|capture| capture.content.as_str()))
            .unwrap_or_default()
            .to_string();
        let message = match captured {
            Some(capture) => {
                if capture.role == "assistant" {
                    captured_assistant = true;
                    emit_metric(
                        app,
                        db,
                        &session,
                        run_id,
                        "assistant_output",
                        started,
                        json!({ "bytes": capture.content.len() }),
                    );
                }
                db.add_agent_message(session.id, capture.role, &capture.content, Some(&line))
                    .ok()
            }
            None => db
                .add_agent_message(session.id, "event", "", Some(&line))
                .ok(),
        };

        let _ = app.emit(
            "agent://event",
            AgentStreamEvent {
                session_id: session.id,
                kind: kind.clone(),
                content: event_content,
                raw_json: Some(line),
                message,
            },
        );

        if kind == "result" {
            break;
        }
    }

    running_agents()
        .lock()
        .map_err(|_| anyhow!("failed to untrack running agent"))?
        .remove(&session.id);
    if result_is_error {
        let message =
            agent_error_hint.unwrap_or_else(|| "Claude returned an error result".to_string());
        emit_metric(
            app,
            db,
            &session,
            run_id,
            "failed",
            started,
            json!({ "message": message }),
        );
        return Err(anyhow!(message));
    }
    emit_metric(
        app,
        db,
        &session,
        run_id,
        "finished",
        started,
        json!({
            "runtime": "persistent",
            "assistant_output": captured_assistant,
            "provider_session_id": provider_session_id.as_deref()
        }),
    );
    let session =
        db.update_agent_session_status(session.id, "done", provider_session_id.as_deref())?;
    emit_status(app, &session);
    Ok(())
}

fn running_agents() -> &'static Mutex<HashMap<i64, Arc<Mutex<Child>>>> {
    RUNNING_AGENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn claude_runtimes() -> &'static Mutex<HashMap<String, Arc<Mutex<ClaudeRuntime>>>> {
    CLAUDE_RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn warm_runtime(
    session: &AgentSession,
    app: &AppHandle,
    rtk_enabled: bool,
) -> anyhow::Result<()> {
    if session.provider == "claude" {
        let context_policy = resolve_context_policy(session, "", None);
        let _ = ensure_claude_runtime(session, &context_policy, app, rtk_enabled)?;
    }
    Ok(())
}

fn ensure_claude_runtime(
    session: &AgentSession,
    context_policy: &AgentContextPolicy,
    app: &AppHandle,
    rtk_enabled: bool,
) -> anyhow::Result<Arc<Mutex<ClaudeRuntime>>> {
    let key = claude_runtime_key(session, context_policy, rtk_enabled);
    {
        let mut runtimes = claude_runtimes()
            .lock()
            .map_err(|_| anyhow!("failed to lock Claude runtimes"))?;
        if let Some(runtime) = runtimes.get(&key).cloned() {
            if claude_runtime_alive(&runtime)? {
                return Ok(runtime);
            }
            runtimes.remove(&key);
        }
    }
    let runtime = Arc::new(Mutex::new(spawn_claude_runtime(
        session,
        context_policy,
        app,
        rtk_enabled,
        key.clone(),
    )?));
    claude_runtimes()
        .lock()
        .map_err(|_| anyhow!("failed to lock Claude runtimes"))?
        .insert(key, Arc::clone(&runtime));
    Ok(runtime)
}

fn remove_claude_runtime(key: &str) {
    if let Ok(mut runtimes) = claude_runtimes().lock() {
        runtimes.remove(key);
    }
}

fn claude_runtime_alive(runtime: &Arc<Mutex<ClaudeRuntime>>) -> anyhow::Result<bool> {
    let runtime = runtime
        .lock()
        .map_err(|_| anyhow!("failed to lock Claude runtime"))?;
    let mut child = runtime
        .child
        .lock()
        .map_err(|_| anyhow!("failed to lock Claude runtime process"))?;
    Ok(child.try_wait()?.is_none())
}

fn spawn_claude_runtime(
    session: &AgentSession,
    context_policy: &AgentContextPolicy,
    app: &AppHandle,
    rtk_enabled: bool,
    key: String,
) -> anyhow::Result<ClaudeRuntime> {
    let mut command = build_claude_runtime_command(session, context_policy)?;
    let _ = rtk::configure_agent_command(app, &mut command, rtk_enabled);
    configure_agent_process(&mut command);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| missing_cli_message("claude"))?;
    let pid = child.id();
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to capture Claude runtime stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to capture Claude runtime stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("failed to capture Claude runtime stderr"))?;
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let stderr_text = Arc::new(Mutex::new(String::new()));
    let stderr_target = Arc::clone(&stderr_text);
    thread::spawn(move || {
        let text = read_to_string(stderr);
        if let Ok(mut target) = stderr_target.lock() {
            *target = text;
        }
    });
    Ok(ClaudeRuntime {
        key,
        child: Arc::new(Mutex::new(child)),
        stdin: Arc::new(Mutex::new(stdin)),
        rx,
        stderr: stderr_text,
        pid,
    })
}

fn build_claude_runtime_command(
    session: &AgentSession,
    context_policy: &AgentContextPolicy,
) -> anyhow::Result<Command> {
    let mut command = Command::new("claude");
    command.current_dir(&session.project_path);
    command.args([
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--include-partial-messages",
        "--exclude-dynamic-system-prompt-sections",
        "--verbose",
    ]);
    append_claude_context_policy(&mut command, context_policy);
    if let Some(provider_session_id) = session.provider_session_id.as_deref() {
        command.args(["--resume", provider_session_id]);
    }
    append_claude_model_effort_permissions(&mut command, session);
    Ok(command)
}

fn claude_runtime_key(
    session: &AgentSession,
    context_policy: &AgentContextPolicy,
    rtk_enabled: bool,
) -> String {
    format!(
        "claude:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        session.workspace_id,
        session.project_id.unwrap_or_default(),
        session.profile_id,
        session.scope,
        session.project_path,
        session.model.as_deref().unwrap_or_default(),
        session.reasoning_effort.as_deref().unwrap_or_default(),
        session.sandbox,
        context_policy.effective_mode,
        rtk_enabled
    )
}

fn claude_user_message_json(prompt: &str) -> anyhow::Result<String> {
    Ok(serde_json::to_string(&json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [
                { "type": "text", "text": prompt }
            ]
        }
    }))?)
}

fn claude_result_is_error(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("result")
        && value
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn resolve_context_policy(
    session: &AgentSession,
    prompt: &str,
    metadata: Option<&Value>,
) -> AgentContextPolicy {
    let configured_mode = match session.context_mode.trim() {
        "full" => "full",
        _ => "auto_lean",
    };
    if configured_mode == "full" {
        return AgentContextPolicy {
            configured_mode: configured_mode.to_string(),
            effective_mode: "full".to_string(),
            reason: "profile_full",
            lean: false,
        };
    }

    if is_unknown_provider_slash(prompt, metadata) {
        return AgentContextPolicy {
            configured_mode: configured_mode.to_string(),
            effective_mode: "full".to_string(),
            reason: "unknown_provider_slash",
            lean: false,
        };
    }

    if prompt_requests_provider_context(prompt, metadata) {
        return AgentContextPolicy {
            configured_mode: configured_mode.to_string(),
            effective_mode: "full".to_string(),
            reason: "provider_context_requested",
            lean: false,
        };
    }

    AgentContextPolicy {
        configured_mode: configured_mode.to_string(),
        effective_mode: "lean".to_string(),
        reason: "auto_lean",
        lean: true,
    }
}

fn is_unknown_provider_slash(prompt: &str, metadata: Option<&Value>) -> bool {
    metadata.and_then(|value| value.get("skill")).is_none() && prompt.trim_start().starts_with('/')
}

fn prompt_requests_provider_context(prompt: &str, metadata: Option<&Value>) -> bool {
    let lower = prompt.to_ascii_lowercase();
    let explicit_prompt = [
        "mcp",
        "context7",
        "playwright",
        "github",
        "gmail",
        "google drive",
        "google calendar",
        "websearch",
        "web search",
        "webfetch",
        "pesquisa",
        "research",
        "documentação",
        "documentation",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if explicit_prompt {
        return true;
    }

    let Some(skill) = metadata.and_then(|value| value.get("skill")) else {
        return false;
    };
    let skill_name = skill
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    [
        "functional-doc",
        "qa",
        "source-grounding",
        "search-first",
        "install-aws",
        "install-azure",
    ]
    .iter()
    .any(|needle| skill_name.contains(needle))
}

fn context_policy_details(provider: &str, policy: &AgentContextPolicy) -> Value {
    let (mcp_enabled, native_skills_enabled, tools_profile) = match provider {
        "claude" if policy.lean => (false, false, "essential"),
        "copilot" if policy.lean => (false, true, "provider_default"),
        "codex" if policy.lean => (true, true, "provider_default_ignore_rules"),
        _ => (true, true, "provider_default"),
    };
    json!({
        "configured_mode": &policy.configured_mode,
        "effective_mode": &policy.effective_mode,
        "reason": policy.reason,
        "lean": policy.lean,
        "mcp_enabled": mcp_enabled,
        "native_skills_enabled": native_skills_enabled,
        "tools_profile": tools_profile
    })
}

fn new_run_id(session_id: i64) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{session_id}-{millis}")
}

fn emit_metric(
    app: &AppHandle,
    db: &Database,
    session: &AgentSession,
    run_id: &str,
    phase: &str,
    started: &Instant,
    details: Value,
) {
    let elapsed_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
    let details_json = serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_string());
    let _ = db.add_agent_run_event(
        session.id,
        run_id,
        &session.provider,
        phase,
        elapsed_ms,
        &details_json,
    );
    let _ = app.emit(
        "agent://metric",
        AgentMetricEvent {
            session_id: session.id,
            run_id: run_id.to_string(),
            provider: session.provider.clone(),
            phase: phase.to_string(),
            elapsed_ms,
            details,
        },
    );
}

fn command_details(command: &Command) -> Value {
    json!({
        "program": command.get_program().to_string_lossy(),
        "args": command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
        "cwd": command.get_current_dir().map(|path| path.to_string_lossy().to_string())
    })
}

fn provider_metric(provider: &str, value: &Value) -> Option<(&'static str, Value)> {
    match provider {
        "claude" => claude_metric(value),
        "codex" => codex_metric(value),
        "copilot" => copilot_metric(value),
        _ => None,
    }
}

fn claude_metric(value: &Value) -> Option<(&'static str, Value)> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if event_type == "system" || event_type == "system/init" {
        return Some((
            "provider_init",
            json!({
                "model": first_string_field(value, "model"),
                "cwd": first_string_field(value, "cwd"),
                "tools": first_array_len(value, "tools"),
                "skills": first_array_len(value, "skills"),
                "agents": first_array_len(value, "agents"),
                "slash_commands": first_array_len(value, "slash_commands"),
                "mcp_servers": first_array_len(value, "mcp_servers")
            }),
        ));
    }
    if event_type == "stream_event"
        && value
            .get("event")
            .and_then(|event| event.get("type"))
            .and_then(Value::as_str)
            == Some("message_start")
    {
        return Some(("model_start", json!({})));
    }
    if event_type == "result" {
        return Some((
            "result",
            json!({
                "duration_ms": first_i64_field(value, "duration_ms"),
                "duration_api_ms": first_i64_field(value, "duration_api_ms"),
                "ttft_ms": first_i64_field(value, "ttft_ms"),
                "input_tokens": first_i64_field(value, "input_tokens"),
                "output_tokens": first_i64_field(value, "output_tokens"),
                "cache_creation_input_tokens": first_i64_field(value, "cache_creation_input_tokens"),
                "cache_read_input_tokens": first_i64_field(value, "cache_read_input_tokens"),
                "total_cost_usd": first_f64_field(value, "total_cost_usd"),
                "is_error": value.get("is_error").and_then(Value::as_bool).unwrap_or(false)
            }),
        ));
    }
    None
}

fn codex_metric(value: &Value) -> Option<(&'static str, Value)> {
    let event_type = value.get("type").and_then(Value::as_str).unwrap_or("event");
    if event_type == "turn.completed" {
        let usage = value.get("usage").unwrap_or(value);
        return Some((
            "result",
            json!({
                "input_tokens": first_i64_field(usage, "input_tokens"),
                "cached_input_tokens": first_i64_field(usage, "cached_input_tokens"),
                "output_tokens": first_i64_field(usage, "output_tokens"),
                "reasoning_output_tokens": first_i64_field(usage, "reasoning_output_tokens")
            }),
        ));
    }
    if event_type.contains("thread") || event_type.contains("session") {
        return Some((
            "provider_session",
            json!({ "event_type": event_type, "session_id": extract_session_id(value) }),
        ));
    }
    if is_error_event(value) {
        return Some((
            "provider_error",
            json!({ "message": extract_error_text(value) }),
        ));
    }
    None
}

fn copilot_metric(value: &Value) -> Option<(&'static str, Value)> {
    if is_error_event(value) {
        return Some((
            "provider_error",
            json!({ "message": extract_error_text(value) }),
        ));
    }
    let event_type = value.get("type").and_then(Value::as_str).unwrap_or("event");
    if event_type.contains("started") || event_type.contains("created") {
        return Some(("provider_init", json!({ "event_type": event_type })));
    }
    if event_type.contains("completed") {
        return Some(("result", json!({ "event_type": event_type })));
    }
    None
}

fn first_string_field(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(text) = map.get(key).and_then(Value::as_str) {
                return Some(text.to_string());
            }
            map.values()
                .find_map(|value| first_string_field(value, key))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|value| first_string_field(value, key)),
        _ => None,
    }
}

fn first_i64_field(value: &Value, key: &str) -> Option<i64> {
    match value {
        Value::Object(map) => {
            if let Some(number) = map.get(key).and_then(Value::as_i64) {
                return Some(number);
            }
            map.values().find_map(|value| first_i64_field(value, key))
        }
        Value::Array(items) => items.iter().find_map(|value| first_i64_field(value, key)),
        _ => None,
    }
}

fn first_f64_field(value: &Value, key: &str) -> Option<f64> {
    match value {
        Value::Object(map) => {
            if let Some(number) = map.get(key).and_then(Value::as_f64) {
                return Some(number);
            }
            map.values().find_map(|value| first_f64_field(value, key))
        }
        Value::Array(items) => items.iter().find_map(|value| first_f64_field(value, key)),
        _ => None,
    }
}

fn first_array_len(value: &Value, key: &str) -> Option<usize> {
    match value {
        Value::Object(map) => {
            if let Some(items) = map.get(key).and_then(Value::as_array) {
                return Some(items.len());
            }
            map.values().find_map(|value| first_array_len(value, key))
        }
        Value::Array(items) => items.iter().find_map(|value| first_array_len(value, key)),
        _ => None,
    }
}

fn build_agent_command(
    session: &AgentSession,
    prompt: &str,
    run_id: &str,
    context_policy: &AgentContextPolicy,
) -> anyhow::Result<Command> {
    match session.provider.as_str() {
        "codex" => build_codex_command(session, prompt, run_id, context_policy),
        "claude" => build_claude_command(session, prompt, context_policy),
        "copilot" => build_copilot_command(session, prompt, context_policy),
        provider => Err(anyhow!("unsupported agent provider: {provider}")),
    }
}

fn build_codex_command(
    session: &AgentSession,
    prompt: &str,
    run_id: &str,
    context_policy: &AgentContextPolicy,
) -> anyhow::Result<Command> {
    let mut command = Command::new(resolve_codex_program()?);
    command.current_dir(&session.project_path);
    let last_message_path = codex_last_message_path(session, run_id);
    let provider_session_id = session
        .provider_session_id
        .as_deref()
        .or(session.codex_session_id.as_deref());
    if let Some(provider_session_id) = provider_session_id {
        command.args(["exec", "resume", "--json"]);
        append_codex_model_and_effort(&mut command, session);
        if is_yolo(session) {
            command.arg("--dangerously-bypass-approvals-and-sandbox");
        }
        append_codex_context_policy(&mut command, context_policy);
        command
            .arg("--output-last-message")
            .arg(last_message_path.as_os_str());
        command.arg(provider_session_id);
    } else {
        command.args([
            "exec",
            "--json",
            "--skip-git-repo-check",
            "-C",
            &session.project_path,
        ]);
        if is_yolo(session) {
            command.arg("--dangerously-bypass-approvals-and-sandbox");
        } else {
            command.args(["-s", &session.sandbox]);
        }
        append_codex_context_policy(&mut command, context_policy);
        append_codex_model_and_effort(&mut command, session);
        command
            .arg("--output-last-message")
            .arg(last_message_path.as_os_str());
    }
    command.arg(prompt);
    Ok(command)
}

fn codex_last_message_path(session: &AgentSession, run_id: &str) -> PathBuf {
    let safe_run_id = run_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    std::env::temp_dir().join(format!(
        "dev-workflow-codex-last-message-{}-{safe_run_id}.txt",
        session.id
    ))
}

fn read_last_message_fallback(session: &AgentSession, run_id: &str) -> Option<String> {
    if session.provider != "codex" {
        return None;
    }
    let path = codex_last_message_path(session, run_id);
    std::fs::read_to_string(path)
        .ok()
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

fn resolve_codex_program() -> anyhow::Result<String> {
    let path_env = std::env::var("PATH").ok();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    resolve_codex_program_from(
        path_env.as_deref(),
        home.as_deref(),
        is_wsl_environment(),
        cfg!(windows),
    )
}

fn resolve_codex_program_from(
    path_env: Option<&str>,
    home: Option<&Path>,
    is_wsl: bool,
    is_windows_host: bool,
) -> anyhow::Result<String> {
    let mut saw_windows_codex = false;
    let candidate_names = codex_candidate_names(is_windows_host);

    if let Some(path_env) = path_env {
        for entry in std::env::split_paths(path_env) {
            for name in candidate_names {
                let candidate = entry.join(name);
                if !candidate.is_file() {
                    continue;
                }
                if is_wsl && is_windows_mount_path(&candidate) {
                    saw_windows_codex = true;
                    continue;
                }
                return Ok(candidate.to_string_lossy().to_string());
            }
        }
    }

    if is_wsl {
        if let Some(candidate) = home.and_then(find_codex_linux_vendor_binary) {
            return Ok(candidate.to_string_lossy().to_string());
        }
        if saw_windows_codex {
            return Err(anyhow!(codex_windows_wsl_message()));
        }
    }

    Ok("codex".to_string())
}

fn codex_candidate_names(is_windows_host: bool) -> &'static [&'static str] {
    if is_windows_host {
        &["codex.exe", "codex.cmd", "codex.bat", "codex.ps1", "codex"]
    } else {
        &["codex"]
    }
}

fn find_codex_linux_vendor_binary(home: &Path) -> Option<PathBuf> {
    let root = home.join(".nvm/versions/node");
    let mut candidates = Vec::new();
    collect_codex_linux_vendor_binaries(&root, 0, &mut candidates);
    candidates.sort();
    candidates.pop()
}

fn collect_codex_linux_vendor_binaries(root: &Path, depth: u8, candidates: &mut Vec<PathBuf>) {
    if depth > 14 || !root.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_codex_linux_vendor_binaries(&path, depth + 1, candidates);
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) != Some("codex") {
            continue;
        }
        let text = path.to_string_lossy();
        if text.contains("codex-linux-") && text.contains("/vendor/") && text.contains("/bin/codex")
        {
            candidates.push(path);
        }
    }
}

fn is_wsl_environment() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|value| value.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

fn is_windows_mount_path(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text.starts_with("/mnt/c/") || text.contains("/mnt/c/Users/")
}

fn build_claude_command(
    session: &AgentSession,
    prompt: &str,
    context_policy: &AgentContextPolicy,
) -> anyhow::Result<Command> {
    let mut command = Command::new("claude");
    command.current_dir(&session.project_path);
    command.args([
        "-p",
        "--output-format",
        "stream-json",
        "--include-partial-messages",
        "--exclude-dynamic-system-prompt-sections",
        "--verbose",
    ]);
    append_claude_context_policy(&mut command, context_policy);
    if let Some(provider_session_id) = session.provider_session_id.as_deref() {
        command.args(["--resume", provider_session_id]);
    }
    append_claude_model_effort_permissions(&mut command, session);
    command.arg(prompt);
    Ok(command)
}

fn build_copilot_command(
    session: &AgentSession,
    prompt: &str,
    context_policy: &AgentContextPolicy,
) -> anyhow::Result<Command> {
    let mut command = Command::new("copilot");
    command.current_dir(&session.project_path);
    command.args([
        "-C",
        &session.project_path,
        "--prompt",
        prompt,
        "--output-format",
        "json",
        "--stream=on",
    ]);
    append_copilot_context_policy(&mut command, context_policy);
    if let Some(provider_session_id) = session.provider_session_id.as_deref() {
        command.arg(format!("--connect={provider_session_id}"));
    }
    if let Some(model) = trimmed(session.model.as_deref()) {
        command.arg(format!("--model={model}"));
    }
    if let Some(effort) = trimmed(session.reasoning_effort.as_deref()) {
        command.arg(format!("--effort={effort}"));
    }
    append_copilot_permissions(&mut command, session);
    Ok(command)
}

fn append_codex_model_and_effort(command: &mut Command, session: &AgentSession) {
    if let Some(model) = trimmed(session.model.as_deref()) {
        command.args(["-m", model]);
    }
    if let Some(effort) = trimmed(session.reasoning_effort.as_deref()) {
        command
            .arg("-c")
            .arg(format!("model_reasoning_effort=\"{effort}\""));
    }
}

fn append_codex_context_policy(command: &mut Command, context_policy: &AgentContextPolicy) {
    if context_policy.lean {
        command.arg("--ignore-rules");
    }
}

fn append_claude_context_policy(command: &mut Command, context_policy: &AgentContextPolicy) {
    if !context_policy.lean {
        return;
    }
    command.args([
        "--strict-mcp-config",
        "--mcp-config",
        r#"{"mcpServers":{}}"#,
        "--disable-slash-commands",
        "--tools",
        "AskUserQuestion,Bash,PowerShell,Read,Write,Edit,Glob,Grep,LSP",
    ]);
}

fn append_copilot_context_policy(command: &mut Command, context_policy: &AgentContextPolicy) {
    if context_policy.lean {
        command.arg("--disable-builtin-mcps");
    }
}

fn append_claude_model_effort_permissions(command: &mut Command, session: &AgentSession) {
    if let Some(model) = trimmed(session.model.as_deref()) {
        command.args(["--model", model]);
    }
    if let Some(effort) = trimmed(session.reasoning_effort.as_deref()) {
        command.args(["--effort", effort]);
    }
    if is_yolo(session) {
        command.arg("--dangerously-skip-permissions");
    } else {
        command.args([
            "--permission-mode",
            claude_permission_mode(&session.sandbox),
        ]);
    }
}

fn append_copilot_permissions(command: &mut Command, session: &AgentSession) {
    if is_yolo(session) {
        command.arg("--allow-all");
        return;
    }
    match session.sandbox.as_str() {
        "read-only" => {
            command.arg("--available-tools=view,grep,glob");
        }
        "workspace-write" => {
            command
                .arg("--allow-tool=write")
                .arg("--allow-tool=shell")
                .arg(format!("--add-dir={}", session.project_path));
        }
        _ => {}
    }
}

fn claude_permission_mode(sandbox: &str) -> &'static str {
    match sandbox {
        "workspace-write" => "acceptEdits",
        _ => "plan",
    }
}

fn is_yolo(session: &AgentSession) -> bool {
    session.sandbox == "danger-full-access"
}

fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn missing_cli_message(provider: &str) -> &'static str {
    match provider {
        "claude" => "Claude CLI not found. Install Claude Code and sign in.",
        "copilot" => {
            "Copilot CLI not found. Install GitHub Copilot CLI and ensure \"copilot\" is in PATH."
        }
        "codex" => "Codex CLI not found. Install Codex and sign in.",
        _ => "Agent CLI not found.",
    }
}

pub fn check_provider_health(
    provider: &str,
    project_path: &str,
) -> anyhow::Result<AgentProviderHealth> {
    match provider {
        "codex" => codex_health(project_path),
        "claude" => cli_health("claude", "claude", &["--version"], project_path),
        "copilot" => cli_health("copilot", "copilot", &["--version"], project_path),
        other => Ok(AgentProviderHealth {
            provider: other.to_string(),
            ok: false,
            supported: false,
            program: other.to_string(),
            version: None,
            message: format!("Unsupported agent provider: {other}"),
            details: json!({}),
        }),
    }
}

fn codex_health(project_path: &str) -> anyhow::Result<AgentProviderHealth> {
    let program = match resolve_codex_program() {
        Ok(program) => program,
        Err(error) => {
            return Ok(AgentProviderHealth {
                provider: "codex".to_string(),
                ok: false,
                supported: true,
                program: "codex".to_string(),
                version: None,
                message: error.to_string(),
                details: json!({ "error": error.to_string() }),
            });
        }
    };
    let version = run_health_command(&program, &["--version"], project_path)?;
    let doctor = run_health_command(&program, &["doctor", "--json"], project_path).ok();
    let ok = version.status_success
        && doctor
            .as_ref()
            .map(|item| item.status_success)
            .unwrap_or(true);
    let output = version.clean_output();
    Ok(AgentProviderHealth {
        provider: "codex".to_string(),
        ok,
        supported: true,
        program,
        version: first_line(&output),
        message: if ok {
            "Codex ready".to_string()
        } else {
            auth_or_permission_hint("codex", &output)
                .unwrap_or_else(|| "Codex health check returned a failure.".to_string())
        },
        details: json!({
            "version": version.clean_output(),
            "doctor": doctor.map(|item| item.clean_output())
        }),
    })
}

fn cli_health(
    provider: &str,
    program: &str,
    args: &[&str],
    project_path: &str,
) -> anyhow::Result<AgentProviderHealth> {
    let output = run_health_command(program, args, project_path)?;
    let clean = output.clean_output();
    Ok(AgentProviderHealth {
        provider: provider.to_string(),
        ok: output.status_success,
        supported: true,
        program: program.to_string(),
        version: first_line(&clean),
        message: if output.status_success {
            format!("{} ready", provider_label(provider))
        } else {
            auth_or_permission_hint(provider, &clean).unwrap_or(clean.clone())
        },
        details: json!({ "output": clean }),
    })
}

struct HealthOutput {
    status_success: bool,
    stdout: String,
    stderr: String,
}

impl HealthOutput {
    fn clean_output(&self) -> String {
        let joined = format!("{}\n{}", self.stdout.trim(), self.stderr.trim());
        joined.trim().to_string()
    }
}

fn run_health_command(
    program: &str,
    args: &[&str],
    project_path: &str,
) -> anyhow::Result<HealthOutput> {
    let mut command = Command::new(program);
    if !project_path.trim().is_empty() {
        command.current_dir(project_path);
    }
    let output = command.args(args).output()?;
    Ok(HealthOutput {
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

fn codex_windows_wsl_message() -> &'static str {
    "Codex CLI is installed from a Windows Node path but is running inside Linux/WSL without the Linux optional package. Install or reinstall Codex in the Linux environment, then retry AI Commit.\n\nSuggested command: npm install -g @openai/codex@latest"
}

fn auth_or_permission_hint(provider: &str, output: &str) -> Option<String> {
    let lower = output.to_ascii_lowercase();
    let has_auth_error = lower.contains("authentication_failed")
        || lower.contains("invalid authentication credentials")
        || lower.contains("failed to authenticate")
        || lower.contains("unauthorized")
        || lower.contains("api_error_status\":401")
        || lower.contains("error_status\":401")
        || lower.contains(" 401")
        || lower.contains(":401");
    let has_forbidden_error = lower.contains("forbidden")
        || lower.contains("permission denied")
        || lower.contains("api_error_status\":403")
        || lower.contains("error_status\":403")
        || lower.contains(" 403")
        || lower.contains(":403");

    if has_auth_error {
        return Some(format!(
            "{} authentication failed. Sign in again in your terminal, then clear this chat and retry.\n\n{}",
            provider_label(provider),
            login_guidance(provider)
        ));
    }

    if has_forbidden_error {
        return Some(format!(
            "{} returned a permission or access error. Check your account, organization policy, selected model, and login state, then clear this chat and retry.\n\n{}",
            provider_label(provider),
            login_guidance(provider)
        ));
    }

    if provider == "codex"
        && lower.contains("missing optional dependency")
        && lower.contains("@openai/codex-linux")
    {
        return Some(codex_windows_wsl_message().to_string());
    }

    None
}

fn provider_label(provider: &str) -> &'static str {
    match provider {
        "claude" => "Claude",
        "copilot" => "Copilot",
        "codex" => "Codex",
        _ => "Agent",
    }
}

fn login_guidance(provider: &str) -> &'static str {
    match provider {
        "claude" => "Run: claude auth\nIf you use a setup token, run: claude setup-token",
        "copilot" => "Run: copilot login",
        "codex" => "Run: codex login",
        _ => "Run the agent CLI login command again.",
    }
}

fn read_to_string(mut reader: impl Read) -> String {
    let mut value = String::new();
    let _ = reader.read_to_string(&mut value);
    value
}

fn emit_status(app: &AppHandle, session: &AgentSession) {
    let _ = app.emit(
        "agent://status",
        AgentStatusEvent {
            session: session.clone(),
        },
    );
}

struct CapturedOutput {
    role: &'static str,
    content: String,
}

fn capture_agent_output(provider: &str, value: &Value) -> Option<CapturedOutput> {
    match provider {
        "claude" => capture_claude_output(value),
        "codex" => capture_codex_output(value),
        "copilot" => capture_copilot_output(value),
        _ => extract_agent_text(value).map(|content| CapturedOutput {
            role: "assistant",
            content,
        }),
    }
}

fn capture_agent_delta(provider: &str, value: &Value) -> Option<String> {
    match provider {
        "claude" => capture_claude_delta(value),
        "codex" => capture_codex_delta(value),
        "copilot" => capture_copilot_delta(value),
        _ => None,
    }
}

fn capture_claude_delta(value: &Value) -> Option<String> {
    let event = value.get("event").unwrap_or(value);
    let event_type = event.get("type").and_then(Value::as_str);
    if event_type != Some("content_block_delta") {
        return None;
    }
    event
        .get("delta")
        .and_then(|delta| delta.get("text"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|text| !text.is_empty())
}

fn capture_codex_delta(value: &Value) -> Option<String> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !event_type.contains("delta") {
        return None;
    }
    value
        .get("delta")
        .or_else(|| value.get("text"))
        .or_else(|| value.get("item").and_then(|item| item.get("delta")))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|text| !text.is_empty())
}

fn capture_copilot_delta(value: &Value) -> Option<String> {
    let top_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !top_type.contains("delta") {
        return None;
    }
    value
        .get("delta")
        .or_else(|| value.get("text"))
        .or_else(|| value.get("data").and_then(|data| data.get("deltaContent")))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|text| !text.is_empty())
}

fn capture_claude_output(value: &Value) -> Option<CapturedOutput> {
    let top_type = value.get("type").and_then(Value::as_str);
    if top_type == Some("assistant") {
        return extract_claude_message_text(value.get("message")?).map(|content| CapturedOutput {
            role: "assistant",
            content,
        });
    }
    if top_type == Some("result") {
        let is_error = value
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_error {
            return value
                .get("result")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|content| !content.is_empty())
                .map(|content| CapturedOutput {
                    role: "system",
                    content: content.to_string(),
                });
        }
    }
    None
}

fn capture_codex_output(value: &Value) -> Option<CapturedOutput> {
    let item = value.get("item")?;
    let item_type = item.get("type").and_then(Value::as_str);
    if item_type == Some("agent_message") {
        return item
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(|content| CapturedOutput {
                role: "assistant",
                content: content.to_string(),
            });
    }
    if item_type == Some("error") {
        return item
            .get("message")
            .or_else(|| item.get("text"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .map(|content| CapturedOutput {
                role: "system",
                content: content.to_string(),
            });
    }
    None
}

fn capture_copilot_output(value: &Value) -> Option<CapturedOutput> {
    if is_error_event(value) {
        return extract_error_text(value).map(|content| CapturedOutput {
            role: "system",
            content,
        });
    }

    let top_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if top_type.contains("delta")
        || top_type.contains("created")
        || top_type.contains("started")
        || top_type.contains("tool")
    {
        return None;
    }

    extract_copilot_final_text(value).map(|content| CapturedOutput {
        role: "assistant",
        content,
    })
}

fn extract_claude_message_text(message: &Value) -> Option<String> {
    let content = message.get("content")?.as_array()?;
    let parts = content
        .iter()
        .filter_map(|item| {
            let item_type = item.get("type").and_then(Value::as_str)?;
            if item_type != "text" {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    let text = parts.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn extract_copilot_final_text(value: &Value) -> Option<String> {
    if value
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|event_type| event_type == "assistant.message")
    {
        if let Some(content) = value
            .get("data")
            .and_then(|data| data.get("content"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|content| !content.is_empty())
        {
            return Some(content.to_string());
        }
    }

    if let Some(content) = value
        .get("response")
        .and_then(|response| response.get("output_text"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|content| !content.is_empty())
    {
        return Some(content.to_string());
    }

    if let Some(content) = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(extract_content_text)
    {
        return Some(content);
    }

    if value
        .get("role")
        .and_then(Value::as_str)
        .is_some_and(|role| role == "assistant")
    {
        if let Some(content) = extract_content_text(value) {
            return Some(content);
        }
    }

    if let Some(message) = value.get("message") {
        if message
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role == "assistant")
        {
            if let Some(content) = extract_content_text(message) {
                return Some(content);
            }
        }
    }

    value
        .get("response")
        .and_then(|response| response.get("output"))
        .and_then(Value::as_array)
        .and_then(|items| {
            let parts = items
                .iter()
                .filter(|item| {
                    item.get("type")
                        .and_then(Value::as_str)
                        .is_none_or(|item_type| item_type == "message")
                })
                .filter_map(extract_content_text)
                .collect::<Vec<_>>();
            let text = parts.join("\n").trim().to_string();
            (!text.is_empty()).then_some(text)
        })
}

fn extract_content_text(value: &Value) -> Option<String> {
    let content = value.get("content")?;
    match content {
        Value::String(text) => {
            let text = text.trim().to_string();
            (!text.is_empty()).then_some(text)
        }
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(|item| {
                    item.get("text")
                        .or_else(|| item.get("content"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>();
            let text = parts.join("\n").trim().to_string();
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn is_error_event(value: &Value) -> bool {
    value
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || value.get("error").is_some()
        || value
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|event_type| event_type.contains("error"))
}

fn extract_error_text(value: &Value) -> Option<String> {
    value
        .get("result")
        .or_else(|| value.get("message"))
        .or_else(|| value.get("error"))
        .and_then(|content| match content {
            Value::String(text) => Some(text.as_str()),
            Value::Object(map) => map
                .get("message")
                .or_else(|| map.get("text"))
                .and_then(Value::as_str),
            _ => None,
        })
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_session_id(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in [
                "session_id",
                "sessionId",
                "conversation_id",
                "conversationId",
                "thread_id",
                "threadId",
            ] {
                if let Some(id) = map.get(key).and_then(Value::as_str) {
                    if !id.trim().is_empty() {
                        return Some(id.to_string());
                    }
                }
            }
            map.values().find_map(extract_session_id)
        }
        Value::Array(items) => items.iter().find_map(extract_session_id),
        _ => None,
    }
}

fn extract_agent_text(value: &Value) -> Option<String> {
    let mut parts = Vec::new();
    collect_text(value, &mut parts);
    let text = parts.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn collect_text(value: &Value, parts: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for key in ["text", "delta", "message", "content", "response"] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    if !text.trim().is_empty() {
                        parts.push(text.to_string());
                    }
                }
            }
            for value in map.values() {
                if value.is_object() || value.is_array() {
                    collect_text(value, parts);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_text(item, parts);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Agent usage (best-effort)
//
// Claude (`/usage`) and Codex (`/status`) expose quota only through interactive
// slash commands, so we drive them in a PTY, capture the rendered output, strip
// ANSI, and parse any "NN%" windows. Best-effort: if the CLI is missing, hangs,
// or renders nothing parseable, we return an empty window list and the UI shows
// "indisponível". Copilot has no known headless usage command → `supported:false`.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct UsageWindow {
    pub label: String,
    pub pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentUsage {
    pub provider: String,
    pub supported: bool,
    pub raw: String,
    pub windows: Vec<UsageWindow>,
}

pub fn agent_usage(provider: &str, project_path: &str) -> anyhow::Result<AgentUsage> {
    let (program, slash) = match provider {
        "claude" => ("claude", "/usage"),
        "codex" => ("codex", "/status"),
        _ => {
            return Ok(AgentUsage {
                provider: provider.to_string(),
                supported: false,
                raw: String::new(),
                windows: Vec::new(),
            });
        }
    };
    let raw = capture_slash_command(program, slash, project_path).unwrap_or_default();
    let clean = strip_ansi(&raw);
    let windows = parse_usage_windows(&clean);
    Ok(AgentUsage {
        provider: provider.to_string(),
        supported: true,
        raw: clean,
        windows,
    })
}

fn capture_slash_command(program: &str, slash: &str, cwd: &str) -> anyhow::Result<String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::Write;

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut command = CommandBuilder::new(program);
    if !cwd.is_empty() {
        command.cwd(cwd);
    }
    let mut child = pair.slave.spawn_command(command)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

    let buffer = Arc::new(Mutex::new(Vec::<u8>::new()));
    let reader_buffer = buffer.clone();
    let reader_handle = thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut buf) = reader_buffer.lock() {
                        buf.extend_from_slice(&chunk[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Let the TUI boot, run the slash command, give it time to render, then exit.
    thread::sleep(Duration::from_millis(900));
    let _ = writer.write_all(format!("{slash}\r").as_bytes());
    let _ = writer.flush();
    thread::sleep(Duration::from_millis(2500));
    let _ = writer.write_all(b"/quit\r");
    let _ = writer.flush();
    thread::sleep(Duration::from_millis(250));
    let _ = writer.write_all(&[0x03]); // Ctrl-C as a fallback
    let _ = writer.flush();
    thread::sleep(Duration::from_millis(200));
    let _ = child.kill();
    drop(writer);
    let _ = child.wait();
    let _ = reader_handle.join();

    let bytes = buffer.lock().map(|buf| buf.clone()).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Remove ANSI escape sequences (CSI / OSC) so the text is parseable.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    // OSC: consume until BEL or ESC\
                    while let Some(&c) = chars.peek() {
                        if c == '\u{7}' {
                            chars.next();
                            break;
                        }
                        chars.next();
                    }
                }
                _ => {}
            }
            continue;
        }
        out.push(ch);
    }
    out
}

/// Best-effort: collect lines that report a percentage as usage windows.
fn parse_usage_windows(text: &str) -> Vec<UsageWindow> {
    let mut windows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > 120 {
            continue;
        }
        if let Some(pct) = first_percentage(trimmed) {
            windows.push(UsageWindow {
                label: trimmed.to_string(),
                pct: Some(pct),
            });
        }
        if windows.len() >= 6 {
            break;
        }
    }
    windows
}

/// First `NN%` (or `NN.N%`) value found in a string, as 0..=100.
fn first_percentage(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'%' {
                if let Ok(value) = text[start..i].parse::<f64>() {
                    if (0.0..=100.0).contains(&value) {
                        return Some(value);
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_csi_sequences() {
        assert_eq!(strip_ansi("\u{1b}[31mhello\u{1b}[0m world"), "hello world");
        assert_eq!(strip_ansi("\u{1b}[2J\u{1b}[Hclean"), "clean");
    }

    #[test]
    fn first_percentage_finds_value_in_range() {
        assert_eq!(first_percentage("Sessão (5h): 62% usado"), Some(62.0));
        assert_eq!(first_percentage("limite 12.5% restante"), Some(12.5));
        assert_eq!(first_percentage("sem número"), None);
        assert_eq!(first_percentage("ano 200% ignora fora de faixa"), None);
    }

    #[test]
    fn parse_usage_windows_keeps_lines_with_percentages() {
        let text = "Uso atual\nSessão 5h: 62%\nSemana: 28%\nrodapé sem número";
        let windows = parse_usage_windows(text);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].pct, Some(62.0));
        assert_eq!(windows[1].pct, Some(28.0));
    }

    #[test]
    fn agent_usage_marks_copilot_unsupported() {
        let usage = agent_usage("copilot", "").unwrap();
        assert!(!usage.supported);
        assert!(usage.windows.is_empty());
    }

    #[test]
    fn codex_resolver_prefers_linux_vendor_binary_over_windows_path_in_wsl() {
        let root = temp_root("dw-gui-agent-codex");
        let home = root.join("home");
        let vendor = home.join(".nvm/versions/node/v24.15.0/lib/node_modules/@openai/.codex-test/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin");
        std::fs::create_dir_all(&vendor).expect("vendor dir");
        std::fs::write(vendor.join("codex"), "#!/bin/sh\n").expect("vendor codex");

        let resolved = resolve_codex_program_from(
            Some("/mnt/c/nvm4w/nodejs:/usr/local/bin"),
            Some(&home),
            true,
            false,
        )
        .expect("resolved");

        assert!(resolved.ends_with("/bin/codex"));
        assert!(!resolved.starts_with("/mnt/c/"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn codex_resolver_rejects_windows_only_codex_in_wsl() {
        let root = temp_root("dw-gui-agent-codex-win");
        let win = root.join("mnt/c/Users/bruno/AppData/Local/nvm/nodejs");
        std::fs::create_dir_all(&win).expect("win dir");
        std::fs::write(win.join("codex"), "windows").expect("win codex");
        let path_env = win.to_string_lossy().to_string();

        let error = resolve_codex_program_from(Some(&path_env), Some(&root), true, false)
            .expect_err("windows codex should be rejected");

        assert!(error.to_string().contains("Windows Node path"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn codex_resolver_falls_back_outside_wsl() {
        let resolved =
            resolve_codex_program_from(Some("/missing"), None, false, false).expect("resolved");
        assert_eq!(resolved, "codex");
    }

    #[test]
    fn codex_resolver_prefers_windows_command_shim_on_windows_host() {
        let root = temp_root("dw-gui-agent-codex-windows-host");
        let bin = root.join("nvm4w/nodejs");
        std::fs::create_dir_all(&bin).expect("bin dir");
        std::fs::write(bin.join("codex"), "#!/bin/sh\n").expect("shell shim");
        std::fs::write(bin.join("codex.cmd"), "@ECHO off\r\n").expect("cmd shim");
        let path_env = bin.to_string_lossy().to_string();

        let resolved =
            resolve_codex_program_from(Some(&path_env), None, false, true).expect("resolved");

        assert!(resolved.ends_with("codex.cmd"));

        let _ = std::fs::remove_dir_all(root);
    }

    fn session(provider: &str, sandbox: &str) -> AgentSession {
        AgentSession {
            id: 1,
            profile_id: 1,
            workspace_id: 1,
            project_id: Some(1),
            requirement_card_id: None,
            scope: "chat".to_string(),
            project_path: "/tmp/project".to_string(),
            provider: provider.to_string(),
            model: Some("model-a".to_string()),
            reasoning_effort: Some("high".to_string()),
            sandbox: sandbox.to_string(),
            context_mode: "auto_lean".to_string(),
            provider_session_id: None,
            codex_session_id: None,
            status: "idle".to_string(),
            title: "Task".to_string(),
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        }
    }

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect()
    }

    fn lean_policy() -> AgentContextPolicy {
        resolve_context_policy(&session("codex", "read-only"), "hi", None)
    }

    fn full_policy() -> AgentContextPolicy {
        let mut session = session("codex", "read-only");
        session.context_mode = "full".to_string();
        resolve_context_policy(&session, "hi", None)
    }

    fn temp_root(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{prefix}-{unique}"));
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn codex_yolo_uses_bypass_flag() {
        let command = build_codex_command(
            &session("codex", "danger-full-access"),
            "hi",
            "run-1",
            &lean_policy(),
        )
        .unwrap();
        let args = args(&command);
        assert!(args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
        assert!(!args.contains(&"-s".to_string()));
        assert!(args.contains(&"--output-last-message".to_string()));
    }

    #[test]
    fn codex_resume_uses_exec_resume_json() {
        let mut session = session("codex", "read-only");
        session.provider_session_id = Some("codex-session-1".to_string());
        let command = build_codex_command(&session, "continue", "run-2", &lean_policy()).unwrap();
        let args = args(&command);
        assert!(args.starts_with(&[
            "exec".to_string(),
            "resume".to_string(),
            "--json".to_string()
        ]));
        assert!(args.contains(&"codex-session-1".to_string()));
        assert!(args.contains(&"--output-last-message".to_string()));
    }

    #[test]
    fn claude_yolo_uses_bypass_flag() {
        let command = build_claude_command(
            &session("claude", "danger-full-access"),
            "hi",
            &lean_policy(),
        )
        .unwrap();
        let args = args(&command);
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(!args.contains(&"--permission-mode".to_string()));
    }

    #[test]
    fn claude_stream_json_uses_verbose() {
        let command =
            build_claude_command(&session("claude", "read-only"), "hi", &lean_policy()).unwrap();
        let args = args(&command);
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--exclude-dynamic-system-prompt-sections".to_string()));
        assert!(args.contains(&"--verbose".to_string()));
    }

    #[test]
    fn claude_runtime_uses_streaming_input() {
        let command =
            build_claude_runtime_command(&session("claude", "read-only"), &lean_policy()).unwrap();
        let args = args(&command);
        assert!(args.contains(&"--input-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"--exclude-dynamic-system-prompt-sections".to_string()));
    }

    #[test]
    fn claude_auto_lean_limits_context_surface() {
        let command =
            build_claude_command(&session("claude", "read-only"), "hi", &lean_policy()).unwrap();
        let args = args(&command);

        assert!(args.contains(&"--strict-mcp-config".to_string()));
        assert!(args.contains(&r#"{"mcpServers":{}}"#.to_string()));
        assert!(args.contains(&"--disable-slash-commands".to_string()));
        assert!(args.contains(&"--tools".to_string()));
        assert!(args.contains(
            &"AskUserQuestion,Bash,PowerShell,Read,Write,Edit,Glob,Grep,LSP".to_string()
        ));
    }

    #[test]
    fn claude_full_keeps_provider_defaults() {
        let command =
            build_claude_command(&session("claude", "read-only"), "hi", &full_policy()).unwrap();
        let args = args(&command);

        assert!(!args.contains(&"--strict-mcp-config".to_string()));
        assert!(!args.contains(&"--disable-slash-commands".to_string()));
        assert!(!args.contains(&"--tools".to_string()));
    }

    #[test]
    fn claude_user_message_json_matches_streaming_shape() {
        let value: Value =
            serde_json::from_str(&claude_user_message_json("hello").unwrap()).unwrap();
        assert_eq!(value.get("type").and_then(Value::as_str), Some("user"));
        assert_eq!(
            value
                .get("message")
                .and_then(|message| message.get("role"))
                .and_then(Value::as_str),
            Some("user")
        );
        assert_eq!(
            value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(Value::as_array)
                .and_then(|content| content.first())
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str),
            Some("hello")
        );
    }

    #[test]
    fn copilot_yolo_uses_allow_all() {
        let command = build_copilot_command(
            &session("copilot", "danger-full-access"),
            "hi",
            &lean_policy(),
        )
        .unwrap();
        let args = args(&command);
        assert!(args.contains(&"--allow-all".to_string()));
    }

    #[test]
    fn copilot_resume_uses_connect_flag() {
        let mut session = session("copilot", "read-only");
        session.provider_session_id = Some("copilot-session-1".to_string());
        let command = build_copilot_command(&session, "hi", &lean_policy()).unwrap();
        let args = args(&command);
        assert!(args.contains(&"--connect=copilot-session-1".to_string()));
        assert!(!args.iter().any(|arg| arg.starts_with("--resume")));
    }

    #[test]
    fn copilot_auto_lean_disables_builtin_mcps() {
        let command =
            build_copilot_command(&session("copilot", "read-only"), "hi", &lean_policy()).unwrap();
        let args = args(&command);

        assert!(args.contains(&"--disable-builtin-mcps".to_string()));
    }

    #[test]
    fn codex_auto_lean_ignores_execpolicy_rules() {
        let command = build_codex_command(
            &session("codex", "read-only"),
            "hi",
            "run-lean",
            &lean_policy(),
        )
        .unwrap();
        let args = args(&command);

        assert!(args.contains(&"--ignore-rules".to_string()));
    }

    #[test]
    fn unknown_provider_slash_uses_full_context() {
        let policy = resolve_context_policy(&session("claude", "read-only"), "/usage", None);

        assert!(!policy.lean);
        assert_eq!(policy.effective_mode, "full");
        assert_eq!(policy.reason, "unknown_provider_slash");
    }

    #[test]
    fn codex_metric_extracts_turn_usage() {
        let value = serde_json::json!({
            "type": "turn.completed",
            "usage": {
                "input_tokens": 14764,
                "cached_input_tokens": 7552,
                "output_tokens": 12,
                "reasoning_output_tokens": 0
            }
        });
        let (phase, details) = codex_metric(&value).expect("metric");

        assert_eq!(phase, "result");
        assert_eq!(
            details.get("input_tokens").and_then(Value::as_i64),
            Some(14764)
        );
        assert_eq!(
            details.get("cached_input_tokens").and_then(Value::as_i64),
            Some(7552)
        );
    }

    #[test]
    fn extracts_thread_id_as_provider_session_id() {
        let value = serde_json::json!({"type":"thread.started","thread_id":"abc"});
        assert_eq!(extract_session_id(&value).as_deref(), Some("abc"));
    }

    #[test]
    fn auth_hint_handles_claude_401_json() {
        let raw = r#"{"type":"result","api_error_status":401,"result":"Failed to authenticate. API Error: 401 Invalid authentication credentials"}"#;
        let hint = auth_or_permission_hint("claude", raw).expect("auth hint");
        assert!(hint.contains("Claude authentication failed"));
        assert!(hint.contains("claude auth"));
    }

    #[test]
    fn permission_hint_handles_403_json() {
        let raw = r#"{"type":"result","api_error_status":403,"result":"Forbidden"}"#;
        let hint = auth_or_permission_hint("copilot", raw).expect("permission hint");
        assert!(hint.contains("permission or access error"));
        assert!(hint.contains("copilot login"));
    }

    #[test]
    fn codex_optional_dependency_hint_points_to_linux_install() {
        let raw = "Error: Missing optional dependency @openai/codex-linux-x64. Reinstall Codex";
        let hint = auth_or_permission_hint("codex", raw).expect("optional dependency hint");
        assert!(hint.contains("Linux/WSL"));
        assert!(hint.contains("npm install -g @openai/codex@latest"));
    }

    #[test]
    fn claude_ignores_streaming_text_delta() {
        let value = serde_json::json!({
            "type": "stream_event",
            "event": {
                "type": "content_block_delta",
                "delta": {"type": "text_delta", "text": "partial"}
            }
        });
        assert!(capture_agent_output("claude", &value).is_none());
    }

    #[test]
    fn claude_captures_final_assistant_text_only() {
        let value = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "thinking", "thinking": "private"},
                    {"type": "text", "text": "Resposta final"}
                ]
            }
        });
        let captured = capture_agent_output("claude", &value).expect("captured text");
        assert_eq!(captured.role, "assistant");
        assert_eq!(captured.content, "Resposta final");
    }

    #[test]
    fn codex_captures_agent_message_only() {
        let value = serde_json::json!({
            "type": "item.completed",
            "item": {"type": "agent_message", "text": "Resumo final"}
        });
        let captured = capture_agent_output("codex", &value).expect("codex text");
        assert_eq!(captured.role, "assistant");
        assert_eq!(captured.content, "Resumo final");
    }

    #[test]
    fn codex_ignores_command_execution_output() {
        let value = serde_json::json!({
            "type": "item.completed",
            "item": {
                "type": "command_execution",
                "command": "cat README.md",
                "aggregated_output": "large output"
            }
        });
        assert!(capture_agent_output("codex", &value).is_none());
    }

    #[test]
    fn copilot_ignores_output_delta() {
        let value = serde_json::json!({
            "type": "response.output_text.delta",
            "delta": "partial"
        });
        assert!(capture_agent_output("copilot", &value).is_none());
    }

    #[test]
    fn copilot_captures_response_completed_output_text() {
        let value = serde_json::json!({
            "type": "response.completed",
            "response": {"output_text": "Resposta final"}
        });
        let captured = capture_agent_output("copilot", &value).expect("copilot text");
        assert_eq!(captured.role, "assistant");
        assert_eq!(captured.content, "Resposta final");
    }

    #[test]
    fn copilot_captures_chat_completion_message() {
        let value = serde_json::json!({
            "choices": [
                {"message": {"role": "assistant", "content": "Resposta final"}}
            ]
        });
        let captured = capture_agent_output("copilot", &value).expect("copilot text");
        assert_eq!(captured.role, "assistant");
        assert_eq!(captured.content, "Resposta final");
    }

    #[test]
    fn copilot_captures_assistant_message_event_content() {
        let value = serde_json::json!({
            "type": "assistant.message",
            "data": {
                "messageId": "msg_1",
                "model": "gpt-5.4",
                "content": "Olá! Como posso ajudar?",
                "phase": "final_answer"
            }
        });
        let captured = capture_agent_output("copilot", &value).expect("copilot text");
        assert_eq!(captured.role, "assistant");
        assert_eq!(captured.content, "Olá! Como posso ajudar?");
    }

    #[test]
    fn copilot_ignores_user_message_event_content() {
        let value = serde_json::json!({
            "type": "user.message",
            "data": {
                "content": "Quem é você e qual modelo voce está usando ?"
            }
        });
        assert!(capture_agent_output("copilot", &value).is_none());
    }
}
