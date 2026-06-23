use anyhow::{anyhow, Context};
use std::path::Path;
use std::process::Command;

pub fn run_capture(cwd: Option<&Path>, program: &str, args: &[&str]) -> anyhow::Result<String> {
    let mut command = Command::new(program);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command
        .args(args)
        .output()
        .with_context(|| format!("failed to execute {program}"))?;
    normalize_output(program, args, output)
}

pub fn run_shell(cwd: &Path, command_text: &str) -> anyhow::Result<String> {
    #[cfg(target_os = "windows")]
    let output = Command::new("pwsh")
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(command_text)
        .current_dir(cwd)
        .output()
        .with_context(|| "failed to execute pwsh")?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("bash")
        .arg("-lc")
        .arg(command_text)
        .current_dir(cwd)
        .output()
        .with_context(|| "failed to execute bash")?;

    normalize_output("shell", &[command_text], output)
}

fn normalize_output(
    program: &str,
    args: &[&str],
    output: std::process::Output,
) -> anyhow::Result<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}").trim().to_string();

    if output.status.success() {
        Ok(combined)
    } else {
        Err(anyhow!(
            "{} {} failed with {}: {}",
            program,
            args.join(" "),
            output.status,
            combined
        ))
    }
}
