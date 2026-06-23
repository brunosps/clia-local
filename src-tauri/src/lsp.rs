//! Minimal Language Server Protocol transport.
//!
//! Spawns a language server as a child process, reads its `Content-Length`
//! framed stdout on a background thread (emitting each JSON message as a Tauri
//! event), and writes framed messages to its stdin. The frontend speaks LSP;
//! this layer only moves bytes and never interprets the protocol.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct LspMessageEvent {
    pub server_id: u32,
    pub message: String,
}

struct ServerRuntime {
    child: Child,
    stdin: ChildStdin,
}

#[derive(Clone)]
pub struct LspManager {
    inner: Arc<Mutex<LspState>>,
}

struct LspState {
    next_id: u32,
    servers: HashMap<u32, ServerRuntime>,
}

/// Resolve the server binary + args for a language (allowlist — the frontend
/// passes a language id, never an arbitrary program).
pub fn server_command(language: &str) -> Option<(&'static str, Vec<String>)> {
    match language {
        "rust" => Some(("rust-analyzer", Vec::new())),
        "typescript" | "javascript" => {
            Some(("typescript-language-server", vec!["--stdio".to_string()]))
        }
        _ => None,
    }
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LspState {
                next_id: 1,
                servers: HashMap::new(),
            })),
        }
    }

    pub fn start(&self, app: &tauri::AppHandle, language: &str, cwd: &str) -> anyhow::Result<u32> {
        let (program, args) = server_command(language)
            .ok_or_else(|| anyhow::anyhow!("no language server configured for {language}"))?;

        let mut child = match Command::new(program)
            .args(&args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(anyhow::anyhow!(
                    "language server `{program}` not found in PATH"
                ));
            }
            Err(error) => return Err(error.into()),
        };

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("language server has no stdout"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("language server has no stdin"))?;

        let id = {
            let mut state = self.lock()?;
            let id = state.next_id;
            state.next_id += 1;
            state.servers.insert(id, ServerRuntime { child, stdin });
            id
        };

        let app = app.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(message)) = read_message(&mut reader) {
                let _ = app.emit(
                    "lsp://message",
                    LspMessageEvent {
                        server_id: id,
                        message,
                    },
                );
            }
        });

        Ok(id)
    }

    pub fn send(&self, id: u32, message: &str) -> anyhow::Result<()> {
        let mut state = self.lock()?;
        let server = state
            .servers
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("unknown language server {id}"))?;
        server
            .stdin
            .write_all(format!("Content-Length: {}\r\n\r\n", message.len()).as_bytes())?;
        server.stdin.write_all(message.as_bytes())?;
        server.stdin.flush()?;
        Ok(())
    }

    pub fn stop(&self, id: u32) -> anyhow::Result<()> {
        let mut state = self.lock()?;
        if let Some(mut server) = state.servers.remove(&id) {
            let _ = server.child.kill();
        }
        Ok(())
    }

    fn lock(&self) -> anyhow::Result<std::sync::MutexGuard<'_, LspState>> {
        self.inner
            .lock()
            .map_err(|_| anyhow::anyhow!("lsp state poisoned"))
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Is a binary reachable on the process PATH?
pub fn binary_on_path(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
}

/// The prerequisite tool needed to auto-install a language's server.
pub fn install_prereq(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some("rustup"),
        "typescript" | "javascript" => Some("npm"),
        _ => None,
    }
}

fn install_plan(language: &str) -> Option<(&'static str, Vec<String>)> {
    match language {
        "rust" => Some((
            "rustup",
            vec![
                "component".to_string(),
                "add".to_string(),
                "rust-analyzer".to_string(),
            ],
        )),
        "typescript" | "javascript" => Some((
            "npm",
            vec![
                "install".to_string(),
                "-g".to_string(),
                "typescript-language-server".to_string(),
                "typescript".to_string(),
            ],
        )),
        _ => None,
    }
}

/// Auto-install the language server via its prerequisite tool (rustup / npm).
pub fn install_server(language: &str) -> anyhow::Result<String> {
    let (program, args) =
        install_plan(language).ok_or_else(|| anyhow::anyhow!("no installer for {language}"))?;
    let prereq = install_prereq(language).unwrap_or(program);
    if !binary_on_path(prereq) {
        return Err(anyhow::anyhow!(
            "`{prereq}` não encontrado no PATH — instale {prereq} e tente de novo"
        ));
    }
    let output = Command::new(program).args(&args).output()?;
    if output.status.success() {
        let server = server_command(language)
            .map(|(name, _)| name)
            .unwrap_or(program);
        Ok(format!("{server} instalado"))
    } else {
        Err(anyhow::anyhow!(
            "falha ao instalar: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Read one `Content-Length` framed message; Ok(None) on clean EOF.
fn read_message<R: BufRead>(reader: &mut R) -> anyhow::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let length = content_length.ok_or_else(|| anyhow::anyhow!("missing Content-Length header"))?;
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_command_allowlist() {
        assert_eq!(
            server_command("rust").map(|(p, _)| p),
            Some("rust-analyzer")
        );
        assert_eq!(
            server_command("typescript").map(|(p, _)| p),
            Some("typescript-language-server")
        );
        assert!(server_command("python").is_none());
    }

    #[test]
    fn read_message_parses_framed_payload() {
        let raw = "Content-Length: 17\r\n\r\n{\"jsonrpc\":\"2.0\"}";
        let mut reader = std::io::BufReader::new(raw.as_bytes());
        let message = read_message(&mut reader).expect("read").expect("some");
        assert_eq!(message, "{\"jsonrpc\":\"2.0\"}");
        // Clean EOF afterwards.
        assert!(read_message(&mut reader).expect("eof").is_none());
    }
}
