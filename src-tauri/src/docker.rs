// Docker management for the "Docker" tab — shells out to the `docker` CLI (mirrors
// the git.rs pattern). Listing uses a tab-delimited `--format` (robust against the
// commas inside compose label values). Container logs stream live via Tauri events.

use anyhow::{anyhow, Context};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use tauri::Emitter;

fn docker(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .with_context(|| format!("failed to execute `docker {}`", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(anyhow!("docker {} failed: {}", args.join(" "), detail))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: String,
    pub created: String,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerImage {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
    pub containers: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerNetwork {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub compose_project: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerVolume {
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub mountpoint: String,
}

#[derive(Clone, Serialize)]
struct DockerLogLine {
    container_id: String,
    line: String,
}

#[derive(Clone, Serialize)]
struct DockerLogEnd {
    container_id: String,
}

/// Helper: split a tab-delimited line and read a field by index (empty if missing).
fn field(parts: &[&str], index: usize) -> String {
    parts.get(index).copied().unwrap_or("").to_string()
}

fn opt_field(parts: &[&str], index: usize) -> Option<String> {
    let value = field(parts, index);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn list_containers() -> anyhow::Result<Vec<DockerContainer>> {
    let format = "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.State}}\t{{.Status}}\t{{.Ports}}\t{{.RunningFor}}\t{{.Label \"com.docker.compose.project\"}}\t{{.Label \"com.docker.compose.service\"}}";
    let out = docker(&["ps", "-a", "--no-trunc", "--format", format])?;
    Ok(out
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            DockerContainer {
                id: field(&parts, 0),
                name: field(&parts, 1),
                image: field(&parts, 2),
                state: field(&parts, 3),
                status: field(&parts, 4),
                ports: field(&parts, 5),
                created: field(&parts, 6),
                compose_project: opt_field(&parts, 7),
                compose_service: opt_field(&parts, 8),
            }
        })
        .collect())
}

pub fn list_images() -> anyhow::Result<Vec<DockerImage>> {
    let format =
        "{{.ID}}\t{{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}\t{{.Containers}}";
    let out = docker(&["images", "--format", format])?;
    Ok(out
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            DockerImage {
                id: field(&parts, 0),
                repository: field(&parts, 1),
                tag: field(&parts, 2),
                size: field(&parts, 3),
                created: field(&parts, 4),
                containers: field(&parts, 5),
            }
        })
        .collect())
}

pub fn list_networks() -> anyhow::Result<Vec<DockerNetwork>> {
    let format =
        "{{.ID}}\t{{.Name}}\t{{.Driver}}\t{{.Scope}}\t{{.Label \"com.docker.compose.project\"}}";
    let out = docker(&["network", "ls", "--format", format])?;
    Ok(out
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            DockerNetwork {
                id: field(&parts, 0),
                name: field(&parts, 1),
                driver: field(&parts, 2),
                scope: field(&parts, 3),
                compose_project: opt_field(&parts, 4),
            }
        })
        .collect())
}

pub fn list_volumes() -> anyhow::Result<Vec<DockerVolume>> {
    let format = "{{.Name}}\t{{.Driver}}\t{{.Scope}}\t{{.Mountpoint}}";
    let out = docker(&["volume", "ls", "--format", format])?;
    Ok(out
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            DockerVolume {
                name: field(&parts, 0),
                driver: field(&parts, 1),
                scope: field(&parts, 2),
                mountpoint: field(&parts, 3),
            }
        })
        .collect())
}

pub fn container_action(id: &str, action: &str) -> anyhow::Result<()> {
    let args: Vec<&str> = match action {
        "start" => vec!["start", id],
        "stop" => vec!["stop", id],
        "restart" => vec!["restart", id],
        "remove" => vec!["rm", "-f", id],
        other => return Err(anyhow!("unknown container action: {other}")),
    };
    docker(&args)?;
    Ok(())
}

/// Run one action across many containers in a single `docker` invocation (docker applies it
/// to all IDs concurrently). Backs the Docker tab's per-group "stop/remove all" actions.
pub fn containers_action(ids: &[String], action: &str) -> anyhow::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut args: Vec<&str> = match action {
        "start" => vec!["start"],
        "stop" => vec!["stop"],
        "restart" => vec!["restart"],
        "remove" => vec!["rm", "-f"],
        other => return Err(anyhow!("unknown container action: {other}")),
    };
    for id in ids {
        args.push(id.as_str());
    }
    docker(&args)?;
    Ok(())
}

pub fn remove_image(id: &str) -> anyhow::Result<()> {
    docker(&["rmi", "-f", id])?;
    Ok(())
}

pub fn remove_network(id: &str) -> anyhow::Result<()> {
    docker(&["network", "rm", id])?;
    Ok(())
}

pub fn remove_volume(name: &str) -> anyhow::Result<()> {
    docker(&["volume", "rm", name])?;
    Ok(())
}

// --- Live container logs (one streaming `docker logs -f` process per container) ---

static LOG_PROCS: OnceLock<Mutex<HashMap<String, Arc<Mutex<Child>>>>> = OnceLock::new();

fn log_procs() -> &'static Mutex<HashMap<String, Arc<Mutex<Child>>>> {
    LOG_PROCS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn emit_lines<R: Read>(app: &tauri::AppHandle, container_id: &str, reader: R) {
    let buffered = BufReader::new(reader);
    for line in buffered.lines() {
        match line {
            Ok(text) => {
                let _ = app.emit(
                    "docker://logs",
                    DockerLogLine {
                        container_id: container_id.to_string(),
                        line: text,
                    },
                );
            }
            Err(_) => break,
        }
    }
}

/// Start streaming a container's logs (`docker logs -f --tail 500`). Idempotent:
/// a second call for the same container while it's already streaming is a no-op.
pub fn logs_start(app: tauri::AppHandle, container_id: String) -> anyhow::Result<()> {
    if let Ok(map) = log_procs().lock() {
        if map.contains_key(&container_id) {
            return Ok(());
        }
    }

    let mut child = Command::new("docker")
        .args(["logs", "-f", "--tail", "500", &container_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| "failed to spawn `docker logs`")?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    if let Ok(mut map) = log_procs().lock() {
        map.insert(container_id.clone(), Arc::new(Mutex::new(child)));
    }

    if let Some(err) = stderr {
        let app = app.clone();
        let id = container_id.clone();
        thread::spawn(move || emit_lines(&app, &id, err));
    }
    if let Some(out) = stdout {
        let id = container_id.clone();
        thread::spawn(move || {
            emit_lines(&app, &id, out);
            // stdout EOF → the `docker logs -f` process ended (container gone/stopped).
            let _ = app.emit(
                "docker://logs-end",
                DockerLogEnd {
                    container_id: id.clone(),
                },
            );
            if let Ok(mut map) = log_procs().lock() {
                if let Some(handle) = map.remove(&id) {
                    if let Ok(mut process) = handle.lock() {
                        let _ = process.wait();
                    }
                }
            }
        });
    }
    Ok(())
}

pub fn logs_stop(container_id: &str) -> anyhow::Result<()> {
    let handle = log_procs()
        .lock()
        .ok()
        .and_then(|mut map| map.remove(container_id));
    if let Some(handle) = handle {
        if let Ok(mut process) = handle.lock() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
    Ok(())
}
