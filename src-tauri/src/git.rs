use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct ChangedFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub area: String,
    pub additions: u32,
    pub deletions: u32,
    pub can_stage_hunks: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilePatch {
    pub path: String,
    pub area: String,
    pub patch: String,
    pub hunks: Vec<PatchHunk>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchHunk {
    pub id: String,
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchCheckResult {
    pub ok: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorktreeCounts {
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicts: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitWorktreeFingerprint {
    pub counts: WorktreeCounts,
    pub fingerprint: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitWorktreeSnapshot {
    pub files: Vec<ChangedFile>,
    pub counts: WorktreeCounts,
    pub untracked_truncated: bool,
    pub fingerprint: String,
    pub generated_at: String,
}

pub fn status(repo: &Path) -> anyhow::Result<String> {
    git(repo, &["status", "--short", "--branch"])
}

pub fn diff(repo: &Path) -> anyhow::Result<String> {
    git(repo, &["diff", "--stat"]).and_then(|stat| {
        let patch = git(repo, &["diff"])?;
        Ok(format!("{stat}\n\n{patch}"))
    })
}

pub fn staged_diff(repo: &Path) -> anyhow::Result<String> {
    let stat = git(repo, &["diff", "--cached", "--stat"])?;
    let patch = git(repo, &["diff", "--cached", "--patch"])?;
    if stat.trim().is_empty() && patch.trim().is_empty() {
        return Ok(String::new());
    }
    Ok(format!("{stat}\n\n{patch}"))
}

pub fn log_graph(repo: &Path) -> anyhow::Result<String> {
    git(
        repo,
        &[
            "log",
            "--graph",
            "--decorate",
            "--oneline",
            "--all",
            "--date-order",
            "-n",
            "120",
        ],
    )
}

pub fn blame(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    git(repo, &["blame", "--date=short", "--", file_path])
}

#[derive(Debug, Clone, Serialize)]
pub struct BlameLine {
    pub line: u32,
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub author_email: String,
    pub date: String,
    pub summary: String,
}

const UNCOMMITTED_SHA: &str = "0000000000000000000000000000000000000000";

fn unix_to_iso(seconds: &str) -> String {
    seconds
        .trim()
        .parse::<i64>()
        .ok()
        .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

/// Parse `git blame --porcelain` into per-line records. Commit header fields
/// (author/summary/time) appear once per sha; cache them so repeated lines for
/// the same commit reuse the metadata.
pub fn blame_porcelain(repo: &Path, file_path: &str) -> anyhow::Result<Vec<BlameLine>> {
    validate_git_path(file_path)?;
    let output = git(repo, &["blame", "--porcelain", "--", file_path])?;
    Ok(parse_blame_porcelain(&output))
}

pub fn blame_porcelain_for_contents(
    repo: &Path,
    file_path: &str,
    content: &str,
) -> anyhow::Result<Vec<BlameLine>> {
    validate_git_path(file_path)?;
    match git_stdout_with_input(
        repo,
        &["blame", "--porcelain", "--contents", "-", "--", file_path],
        content,
    ) {
        Ok(output) => Ok(parse_blame_porcelain(&output)),
        Err(_) if !path_exists_in_head(repo, file_path) => Ok(synthetic_uncommitted_blame(content)),
        Err(err) => Err(err),
    }
}

fn path_exists_in_head(repo: &Path, file_path: &str) -> bool {
    git(repo, &["cat-file", "-e", &format!("HEAD:{file_path}")]).is_ok()
}

fn synthetic_uncommitted_blame(content: &str) -> Vec<BlameLine> {
    let date = chrono::Utc::now().to_rfc3339();
    content
        .lines()
        .enumerate()
        .map(|(index, _)| BlameLine {
            line: (index + 1) as u32,
            sha: UNCOMMITTED_SHA.to_string(),
            short_sha: UNCOMMITTED_SHA.chars().take(8).collect(),
            author: "Not Committed Yet".to_string(),
            author_email: String::new(),
            date: date.clone(),
            summary: "Uncommitted changes".to_string(),
        })
        .collect()
}

fn parse_blame_porcelain(output: &str) -> Vec<BlameLine> {
    // sha -> (author, email, author-time, summary)
    let mut headers: HashMap<String, (String, String, String, String)> = HashMap::new();
    let mut result = Vec::new();
    let mut cur_sha = String::new();
    let mut cur_line: u32 = 0;
    let (mut author, mut email, mut time, mut summary) =
        (String::new(), String::new(), String::new(), String::new());

    for line in output.lines() {
        if let Some(code) = line.strip_prefix('\t') {
            let _ = code; // content line — emit the blame entry for this line
            let meta = headers.get(&cur_sha).cloned().unwrap_or_default();
            result.push(BlameLine {
                line: cur_line,
                short_sha: cur_sha.chars().take(8).collect(),
                sha: cur_sha.clone(),
                author: meta.0,
                author_email: meta.1,
                date: unix_to_iso(&meta.2),
                summary: meta.3,
            });
            continue;
        }
        let parts: Vec<&str> = line.split(' ').collect();
        if parts.len() >= 3
            && parts[0].len() == 40
            && parts[0].chars().all(|c| c.is_ascii_hexdigit())
        {
            cur_sha = parts[0].to_string();
            cur_line = parts[2].parse().unwrap_or(0);
            continue;
        }
        if let Some(rest) = line.strip_prefix("author ") {
            author = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("author-mail ") {
            email = rest.trim_matches(['<', '>']).to_string();
        } else if let Some(rest) = line.strip_prefix("author-time ") {
            time = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("summary ") {
            summary = rest.to_string();
        } else if line.starts_with("filename ") {
            // End of this sha's header block — cache it (first occurrence wins).
            headers.entry(cur_sha.clone()).or_insert((
                author.clone(),
                email.clone(),
                time.clone(),
                summary.clone(),
            ));
        }
    }
    result
}

pub fn changed_files(repo: &Path) -> anyhow::Result<Vec<ChangedFile>> {
    let mut files = Vec::new();
    files.extend(diff_files(repo, "unstaged", false)?);
    files.extend(diff_files(repo, "staged", true)?);
    files.extend(untracked_files(repo, None)?.0);
    files.sort_by(|left, right| {
        left.area
            .cmp(&right.area)
            .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
    });
    Ok(files)
}

pub fn worktree_fingerprint(repo: &Path) -> anyhow::Result<GitWorktreeFingerprint> {
    let tracked_status = tracked_status(repo)?;
    let untracked = untracked_paths(repo)?;
    let counts = parse_worktree_counts(&tracked_status, untracked.len());

    Ok(GitWorktreeFingerprint {
        counts,
        fingerprint: fingerprint_worktree(repo, &tracked_status, &untracked),
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn worktree_snapshot(repo: &Path, untracked_limit: u32) -> anyhow::Result<GitWorktreeSnapshot> {
    let mut files = Vec::new();
    let unstaged = diff_files(repo, "unstaged", false)?;
    let staged = diff_files(repo, "staged", true)?;
    let untracked_paths = untracked_paths(repo)?;
    let untracked_total = untracked_paths.len();
    let untracked = changed_files_from_untracked_paths(
        repo,
        &untracked_paths,
        if untracked_limit == 0 {
            None
        } else {
            Some(untracked_limit as usize)
        },
    );
    let tracked_status = tracked_status(repo)?;
    let conflicts = conflict_files_from_status(&tracked_status);
    let fingerprint = fingerprint_worktree(repo, &tracked_status, &untracked_paths);

    files.extend(unstaged.iter().cloned());
    files.extend(staged.iter().cloned());
    files.extend(untracked);
    files.sort_by(|left, right| {
        left.area
            .cmp(&right.area)
            .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
    });

    Ok(GitWorktreeSnapshot {
        files,
        counts: WorktreeCounts {
            staged: staged.len() as u32,
            unstaged: unstaged.len() as u32,
            untracked: untracked_total as u32,
            conflicts: conflicts.len() as u32,
            total: staged.len() as u32 + unstaged.len() as u32 + untracked_total as u32,
        },
        untracked_truncated: untracked_limit > 0 && untracked_total > untracked_limit as usize,
        fingerprint,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn file_patch(repo: &Path, file_path: &str, area: &str) -> anyhow::Result<FilePatch> {
    validate_git_path(file_path)?;
    let cached = match area {
        "staged" => true,
        "unstaged" => false,
        _ => return Err(anyhow!("unknown patch area: {area}")),
    };
    let mut patch = diff_for_path(repo, file_path, cached)?;
    if patch.trim().is_empty() && !cached {
        // Untracked files have no `git diff`; synthesize one as all-additions.
        patch = diff_untracked(repo, file_path).unwrap_or_default();
    }
    Ok(FilePatch {
        path: file_path.to_string(),
        area: area.to_string(),
        hunks: parse_hunks(&patch),
        patch,
    })
}

pub fn file_patch_text(repo: &Path, file_path: &str, area: &str) -> anyhow::Result<String> {
    Ok(file_patch(repo, file_path, area)?.patch)
}

/// Diff an untracked file against /dev/null so its content shows as additions.
/// `git diff --no-index` exits 1 when the files differ, so we read stdout
/// regardless of the exit status instead of going through `git()`.
fn diff_untracked(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    let null_device = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "diff",
            "--no-index",
            "--patch",
            "--",
            null_device,
            file_path,
        ])
        .output()
        .with_context(|| format!("failed to diff untracked file in {}", repo.display()))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn stage_file(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    git(repo, &["add", "--", file_path])
}

pub fn unstage_file(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    git(repo, &["restore", "--staged", "--", file_path])
}

pub fn stage_all(repo: &Path) -> anyhow::Result<String> {
    git(repo, &["add", "-A"])
}

pub fn unstage_all(repo: &Path) -> anyhow::Result<String> {
    // Reset the index to HEAD (unstage everything) without touching the worktree.
    git(repo, &["reset", "-q"])
}

pub fn stage_hunk(repo: &Path, hunk_patch: &str) -> anyhow::Result<PatchCheckResult> {
    let check = git_with_input(repo, &["apply", "--cached", "--check"], hunk_patch)?;
    if !check.ok {
        return Ok(check);
    }
    git_with_input(repo, &["apply", "--cached"], hunk_patch)
}

pub fn unstage_hunk(repo: &Path, hunk_patch: &str) -> anyhow::Result<PatchCheckResult> {
    let check = git_with_input(
        repo,
        &["apply", "--cached", "--reverse", "--check"],
        hunk_patch,
    )?;
    if !check.ok {
        return Ok(check);
    }
    git_with_input(repo, &["apply", "--cached", "--reverse"], hunk_patch)
}

pub fn check_patch(repo: &Path, patch: &str) -> anyhow::Result<PatchCheckResult> {
    git_with_input(repo, &["apply", "--check"], patch)
}

pub fn apply_patch(repo: &Path, patch: &str) -> anyhow::Result<PatchCheckResult> {
    let check = check_patch(repo, patch)?;
    if !check.ok {
        return Ok(check);
    }
    git_with_input(repo, &["apply"], patch)
}

fn diff_files(repo: &Path, area: &str, cached: bool) -> anyhow::Result<Vec<ChangedFile>> {
    let name_status = if cached {
        git(repo, &["diff", "--cached", "--name-status", "--"])?
    } else {
        git(repo, &["diff", "--name-status", "--"])?
    };
    let stats = diff_stats(repo, cached)?;

    Ok(name_status
        .lines()
        .filter_map(|line| changed_file_from_name_status(line, area, &stats))
        .collect())
}

fn diff_stats(repo: &Path, cached: bool) -> anyhow::Result<HashMap<String, (u32, u32)>> {
    let output = if cached {
        git(repo, &["diff", "--cached", "--numstat", "--"])?
    } else {
        git(repo, &["diff", "--numstat", "--"])?
    };

    let mut stats = HashMap::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let path = parts.last().unwrap_or(&"").to_string();
        stats.insert(path, (parse_numstat(parts[0]), parse_numstat(parts[1])));
    }
    Ok(stats)
}

fn changed_file_from_name_status(
    line: &str,
    area: &str,
    stats: &HashMap<String, (u32, u32)>,
) -> Option<ChangedFile> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 2 {
        return None;
    }

    let status = parts[0].to_string();
    let (old_path, path) = if status.starts_with('R') || status.starts_with('C') {
        if parts.len() < 3 {
            return None;
        }
        (Some(parts[1].to_string()), parts[2].to_string())
    } else {
        (None, parts[1].to_string())
    };
    let (additions, deletions) = stats.get(&path).copied().unwrap_or((0, 0));

    Some(ChangedFile {
        can_stage_hunks: status == "M",
        path,
        old_path,
        status,
        area: area.to_string(),
        additions,
        deletions,
    })
}

fn untracked_files(repo: &Path, limit: Option<usize>) -> anyhow::Result<(Vec<ChangedFile>, usize)> {
    let paths = untracked_paths(repo)?;
    let total = paths.len();
    let files = changed_files_from_untracked_paths(repo, &paths, limit);
    Ok((files, total))
}

fn changed_files_from_untracked_paths(
    repo: &Path,
    paths: &[String],
    limit: Option<usize>,
) -> Vec<ChangedFile> {
    paths
        .iter()
        .take(limit.unwrap_or(usize::MAX))
        .map(|path| ChangedFile {
            // An untracked file is entirely new: every line counts as an addition
            // (git's `--numstat` omits untracked files, so we count them ourselves).
            additions: untracked_additions(repo, path),
            path: path.to_string(),
            old_path: None,
            status: "??".to_string(),
            area: "unstaged".to_string(),
            deletions: 0,
            can_stage_hunks: false,
        })
        .collect()
}

/// Count the lines of an untracked file as additions. Empty, binary, or
/// unreadable files report 0 (matching git, which shows binary diffs as `-`).
fn untracked_additions(repo: &Path, path: &str) -> u32 {
    let Ok(bytes) = std::fs::read(repo.join(path)) else {
        return 0;
    };
    if bytes.is_empty() || bytes.contains(&0) {
        return 0;
    }
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count() as u32;
    // A final line without a trailing newline still counts.
    if bytes.ends_with(b"\n") {
        newlines
    } else {
        newlines + 1
    }
}

fn untracked_paths(repo: &Path) -> anyhow::Result<Vec<String>> {
    let output = git(repo, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn tracked_status(repo: &Path) -> anyhow::Result<String> {
    git(repo, &["status", "--porcelain", "--untracked-files=no"])
}

fn parse_worktree_counts(tracked_status: &str, untracked_count: usize) -> WorktreeCounts {
    let mut staged = 0;
    let mut unstaged = 0;
    let mut conflicts = 0;
    for line in tracked_status.lines().filter(|line| line.len() >= 2) {
        let bytes = line.as_bytes();
        let index = bytes[0] as char;
        let worktree = bytes[1] as char;
        if index != ' ' && index != '?' {
            staged += 1;
        }
        if worktree != ' ' && worktree != '?' {
            unstaged += 1;
        }
        if is_conflict_status(index, worktree) {
            conflicts += 1;
        }
    }
    WorktreeCounts {
        staged,
        unstaged,
        untracked: untracked_count as u32,
        conflicts,
        total: staged + unstaged + untracked_count as u32,
    }
}

fn conflict_files_from_status(status: &str) -> Vec<String> {
    status
        .lines()
        .filter(|line| line.len() >= 3)
        .filter_map(|line| {
            let bytes = line.as_bytes();
            let index = bytes[0] as char;
            let worktree = bytes[1] as char;
            if !is_conflict_status(index, worktree) {
                return None;
            }
            Some(line[3..].to_string())
        })
        .collect()
}

fn is_conflict_status(index: char, worktree: char) -> bool {
    matches!(
        (index, worktree),
        ('D', 'D') | ('A', 'U') | ('U', 'D') | ('U', 'A') | ('D', 'U') | ('A', 'A') | ('U', 'U')
    )
}

fn fingerprint_parts(parts: &[String]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        hasher.write(part.as_bytes());
        hasher.write_u8(0);
    }
    format!("{:016x}", hasher.finish())
}

fn fingerprint_worktree(repo: &Path, tracked_status: &str, untracked: &[String]) -> String {
    fingerprint_parts(&[
        git(repo, &["rev-parse", "--verify", "HEAD"]).unwrap_or_default(),
        tracked_status.to_string(),
        untracked.join("\n"),
    ])
}

fn diff_for_path(repo: &Path, file_path: &str, cached: bool) -> anyhow::Result<String> {
    if cached {
        git(repo, &["diff", "--cached", "--patch", "--", file_path])
    } else {
        git(repo, &["diff", "--patch", "--", file_path])
    }
}

fn parse_hunks(patch: &str) -> Vec<PatchHunk> {
    let lines: Vec<&str> = patch.lines().collect();
    let header_end = lines
        .iter()
        .position(|line| line.starts_with("@@ "))
        .unwrap_or(lines.len());
    let file_header = lines[..header_end].join("\n");
    let mut hunks = Vec::new();
    let mut current = Vec::new();

    for line in lines.iter().skip(header_end) {
        if line.starts_with("@@ ") && !current.is_empty() {
            push_hunk(&mut hunks, &file_header, &current);
            current.clear();
        }
        current.push((*line).to_string());
    }

    if !current.is_empty() {
        push_hunk(&mut hunks, &file_header, &current);
    }

    hunks
}

fn push_hunk(hunks: &mut Vec<PatchHunk>, file_header: &str, hunk_lines: &[String]) {
    let Some(header) = hunk_lines.first() else {
        return;
    };
    let Some((old_start, old_lines, new_start, new_lines)) = parse_hunk_header(header) else {
        return;
    };
    let hunk_body = hunk_lines.join("\n");
    let patch = if file_header.is_empty() {
        format!("{hunk_body}\n")
    } else {
        format!("{file_header}\n{hunk_body}\n")
    };
    hunks.push(PatchHunk {
        id: format!("{}-{}-{}", hunks.len() + 1, old_start, new_start),
        header: header.to_string(),
        old_start,
        old_lines,
        new_start,
        new_lines,
        patch,
    });
}

fn parse_hunk_header(header: &str) -> Option<(u32, u32, u32, u32)> {
    let mut parts = header.split_whitespace();
    parts.next()?;
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    Some((
        parse_range_start(old)?,
        parse_range_len(old),
        parse_range_start(new)?,
        parse_range_len(new),
    ))
}

fn parse_range_start(range: &str) -> Option<u32> {
    range.split(',').next()?.parse().ok()
}

fn parse_range_len(range: &str) -> u32 {
    range
        .split(',')
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

fn parse_numstat(value: &str) -> u32 {
    value.parse().unwrap_or(0)
}

fn validate_git_path(path: &str) -> anyhow::Result<()> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(anyhow!("git path escapes project root"));
    }
    Ok(())
}

fn git(repo: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git in {}", repo.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            if stderr.is_empty() { stdout } else { stderr }
        ))
    }
}

/// Like `git`, but with extra environment variables (e.g. GIT_EDITOR=true so
/// `--continue` never blocks on an editor, or GIT_SEQUENCE_EDITOR for rebase).
fn git_env(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> anyhow::Result<String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(repo).args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command
        .output()
        .with_context(|| format!("failed to execute git in {}", repo.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            if stderr.is_empty() { stdout } else { stderr }
        ))
    }
}

fn git_with_input(repo: &Path, args: &[&str], input: &str) -> anyhow::Result<PatchCheckResult> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute git in {}", repo.display()))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stdout}\n{stderr}")
    };

    Ok(PatchCheckResult {
        ok: output.status.success(),
        output: combined,
    })
}

fn git_stdout_with_input(repo: &Path, args: &[&str], input: &str) -> anyhow::Result<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute git in {}", repo.display()))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            if stderr.is_empty() { stdout } else { stderr }
        ))
    }
}

// ---------------------------------------------------------------------------
// Structured history / refs (Fork-style git client)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CommitRef {
    pub name: String,
    pub kind: String, // "head" | "branch" | "remote" | "tag"
}

#[derive(Debug, Clone, Serialize)]
pub struct Commit {
    pub sha: String,
    pub short_sha: String,
    pub parents: Vec<String>,
    pub refs: Vec<CommitRef>,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitDetail {
    pub sha: String,
    pub short_sha: String,
    pub parents: Vec<String>,
    pub refs: Vec<CommitRef>,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub subject: String,
    pub body: String,
    pub files: Vec<CommitFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Branch {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteBranch {
    pub remote: String,
    pub name: String,
    pub full: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagEntry {
    pub name: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StashEntry {
    pub index: u32,
    pub label: String,
    pub message: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoState {
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub operation: Option<String>, // "merge" | "rebase" | "cherry-pick" | "revert"
    pub conflicts: Vec<String>,
    pub dirty: bool, // uncommitted changes (tracked or untracked) present
}

#[derive(Debug, Clone, Serialize)]
pub struct GitRepoSnapshotOptions {
    pub include_remotes: bool,
    pub include_tags: bool,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitRepoSnapshot {
    pub commits: Vec<Commit>,
    pub branches: Vec<Branch>,
    pub remote_branches: Vec<RemoteBranch>,
    pub tags: Vec<TagEntry>,
    pub stashes: Vec<StashEntry>,
    pub submodules: Vec<Submodule>,
    pub repo_state: RepoState,
    pub generated_at: String,
    pub options: GitRepoSnapshotOptions,
    pub warnings: Vec<String>,
}

const GRAPH_FORMAT: &str = "%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ae\x1f%aI\x1f%s";

pub fn commit_graph(
    repo: &Path,
    include_remotes: bool,
    include_tags: bool,
    limit: u32,
    skip: u32,
) -> anyhow::Result<Vec<Commit>> {
    let mut args: Vec<String> = vec![
        "log".into(),
        "--date-order".into(),
        format!("--pretty=format:{GRAPH_FORMAT}\x1e"),
        "--branches".into(),
    ];
    if include_remotes {
        args.push("--remotes".into());
    }
    if include_tags {
        args.push("--tags".into());
    }
    args.push(format!("--max-count={}", limit.max(1)));
    if skip > 0 {
        args.push(format!("--skip={skip}"));
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git(repo, &arg_refs)?;
    Ok(output
        .split('\x1e')
        .filter_map(parse_commit_record)
        .collect())
}

/// Commits that touched a file (Git-History panel), following renames.
pub fn log_file(repo: &Path, file_path: &str, limit: u32) -> anyhow::Result<Vec<Commit>> {
    validate_git_path(file_path)?;
    let args: Vec<String> = vec![
        "log".into(),
        "--follow".into(),
        "--date-order".into(),
        format!("--pretty=format:{GRAPH_FORMAT}\x1e"),
        format!("--max-count={}", limit.max(1)),
        "--".into(),
        file_path.into(),
    ];
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git(repo, &arg_refs)?;
    Ok(output
        .split('\x1e')
        .filter_map(parse_commit_record)
        .collect())
}

/// File contents at a specific commit (time-travel / compare).
pub fn show_file(repo: &Path, sha: &str, file_path: &str) -> anyhow::Result<String> {
    validate_rev(sha)?;
    validate_git_path(file_path)?;
    git(repo, &["show", &format!("{sha}:{file_path}")])
}

fn parse_commit_record(record: &str) -> Option<Commit> {
    let record = record.trim_matches(['\n', '\r']);
    if record.is_empty() {
        return None;
    }
    let fields: Vec<&str> = record.split('\x1f').collect();
    if fields.len() < 8 {
        return None;
    }
    Some(Commit {
        sha: fields[0].to_string(),
        short_sha: fields[1].to_string(),
        parents: fields[2]
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        refs: parse_refs(fields[3]),
        author_name: fields[4].to_string(),
        author_email: fields[5].to_string(),
        date: fields[6].to_string(),
        subject: fields[7].to_string(),
    })
}

fn parse_refs(decorate: &str) -> Vec<CommitRef> {
    let mut refs = Vec::new();
    for raw in decorate.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(rest) = token.strip_prefix("HEAD -> ") {
            refs.push(CommitRef {
                name: rest.trim().to_string(),
                kind: "head".to_string(),
            });
        } else if token == "HEAD" {
            refs.push(CommitRef {
                name: "HEAD".to_string(),
                kind: "head".to_string(),
            });
        } else if let Some(rest) = token.strip_prefix("tag: ") {
            refs.push(CommitRef {
                name: rest.trim().to_string(),
                kind: "tag".to_string(),
            });
        } else if token.contains('/') {
            refs.push(CommitRef {
                name: token.to_string(),
                kind: "remote".to_string(),
            });
        } else {
            refs.push(CommitRef {
                name: token.to_string(),
                kind: "branch".to_string(),
            });
        }
    }
    refs
}

pub fn commit_detail(repo: &Path, sha: &str) -> anyhow::Result<CommitDetail> {
    validate_rev(sha)?;
    let meta = git(
        repo,
        &[
            "show",
            "-s",
            &format!("--pretty=format:{GRAPH_FORMAT}\x1f%b"),
            sha,
        ],
    )?;
    let fields: Vec<&str> = meta.split('\x1f').collect();
    if fields.len() < 9 {
        return Err(anyhow!("unexpected commit metadata for {sha}"));
    }
    Ok(CommitDetail {
        sha: fields[0].to_string(),
        short_sha: fields[1].to_string(),
        parents: fields[2]
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        refs: parse_refs(fields[3]),
        author_name: fields[4].to_string(),
        author_email: fields[5].to_string(),
        date: fields[6].to_string(),
        subject: fields[7].to_string(),
        body: fields[8].trim().to_string(),
        files: commit_files(repo, sha)?,
    })
}

fn commit_files(repo: &Path, sha: &str) -> anyhow::Result<Vec<CommitFile>> {
    let numstat = git(repo, &["show", sha, "--numstat", "--format="])?;
    let mut stats: HashMap<String, (u32, u32)> = HashMap::new();
    for line in numstat.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        stats.insert(
            parts.last().unwrap_or(&"").to_string(),
            (parse_numstat(parts[0]), parse_numstat(parts[1])),
        );
    }

    let name_status = git(repo, &["show", sha, "--name-status", "--format="])?;
    let mut files = Vec::new();
    for line in name_status.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let status = parts[0].to_string();
        let (old_path, path) =
            if (status.starts_with('R') || status.starts_with('C')) && parts.len() >= 3 {
                (Some(parts[1].to_string()), parts[2].to_string())
            } else {
                (None, parts[1].to_string())
            };
        let (additions, deletions) = stats.get(&path).copied().unwrap_or((0, 0));
        files.push(CommitFile {
            path,
            old_path,
            status,
            additions,
            deletions,
        });
    }
    Ok(files)
}

pub fn commit_file_diff(repo: &Path, sha: &str, file_path: &str) -> anyhow::Result<FilePatch> {
    validate_rev(sha)?;
    validate_git_path(file_path)?;
    let patch = git(repo, &["show", sha, "--format=", "--", file_path])?;
    Ok(FilePatch {
        path: file_path.to_string(),
        area: "commit".to_string(),
        hunks: parse_hunks(&patch),
        patch,
    })
}

pub fn repo_state(repo: &Path) -> anyhow::Result<RepoState> {
    let branch = git(repo, &["symbolic-ref", "--short", "-q", "HEAD"])
        .ok()
        .filter(|value| !value.is_empty());
    let detached = branch.is_none();
    let upstream = git(
        repo,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .ok()
    .filter(|value| !value.is_empty() && value != "@{u}");

    let (ahead, behind) = if upstream.is_some() {
        git(
            repo,
            &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
        )
        .ok()
        .and_then(|value| {
            let parts: Vec<&str> = value.split_whitespace().collect();
            if parts.len() == 2 {
                Some((parts[1].parse().unwrap_or(0), parts[0].parse().unwrap_or(0)))
            } else {
                None
            }
        })
        .unwrap_or((0, 0))
    } else {
        (0, 0)
    };

    let git_dir = git(repo, &["rev-parse", "--git-dir"])
        .map(|value| repo.join(value.trim()))
        .unwrap_or_else(|_| repo.join(".git"));
    let operation = if git_dir.join("MERGE_HEAD").exists() {
        Some("merge".to_string())
    } else if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        Some("rebase".to_string())
    } else if git_dir.join("CHERRY_PICK_HEAD").exists() {
        Some("cherry-pick".to_string())
    } else if git_dir.join("REVERT_HEAD").exists() {
        Some("revert".to_string())
    } else {
        None
    };

    let tracked_status = tracked_status(repo).unwrap_or_default();
    let conflicts = conflict_files_from_status(&tracked_status);
    let dirty = !tracked_status.trim().is_empty() || repo_has_untracked_fast(repo);

    Ok(RepoState {
        branch,
        detached,
        upstream,
        ahead,
        behind,
        operation,
        conflicts,
        dirty,
    })
}

fn repo_has_untracked_fast(repo: &Path) -> bool {
    !git(
        repo,
        &["ls-files", "--others", "--exclude-standard", "--directory"],
    )
    .unwrap_or_default()
    .trim()
    .is_empty()
}

pub fn repo_snapshot(
    repo: &Path,
    include_remotes: bool,
    include_tags: bool,
    limit: u32,
) -> anyhow::Result<GitRepoSnapshot> {
    let repo_state = repo_state(repo)?;
    let mut warnings = Vec::new();

    Ok(GitRepoSnapshot {
        commits: snapshot_vec(
            &mut warnings,
            "commits",
            commit_graph(repo, include_remotes, include_tags, limit, 0),
        ),
        branches: snapshot_vec(&mut warnings, "branches", list_branches(repo)),
        remote_branches: snapshot_vec(&mut warnings, "remote branches", list_remote_branches(repo)),
        tags: snapshot_vec(&mut warnings, "tags", list_tags(repo)),
        stashes: snapshot_vec(&mut warnings, "stashes", list_stashes(repo)),
        submodules: snapshot_vec(&mut warnings, "submodules", list_submodules(repo)),
        repo_state,
        generated_at: chrono::Utc::now().to_rfc3339(),
        options: GitRepoSnapshotOptions {
            include_remotes,
            include_tags,
            limit: limit.max(1),
        },
        warnings,
    })
}

fn snapshot_vec<T>(
    warnings: &mut Vec<String>,
    label: &str,
    result: anyhow::Result<Vec<T>>,
) -> Vec<T> {
    match result {
        Ok(value) => value,
        Err(error) => {
            warnings.push(format!("{label}: {error}"));
            Vec::new()
        }
    }
}

pub fn list_branches(repo: &Path) -> anyhow::Result<Vec<Branch>> {
    let output = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)\x1f%(HEAD)\x1f%(upstream:short)\x1f%(upstream:track)",
            "refs/heads",
        ],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\x1f').collect();
            if fields.is_empty() || fields[0].is_empty() {
                return None;
            }
            let track = fields.get(3).copied().unwrap_or("");
            let (ahead, behind) = parse_track(track);
            Some(Branch {
                name: fields[0].to_string(),
                is_head: fields.get(1).copied().unwrap_or("") == "*",
                upstream: fields
                    .get(2)
                    .copied()
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
                ahead,
                behind,
            })
        })
        .collect())
}

pub fn list_remote_branches(repo: &Path) -> anyhow::Result<Vec<RemoteBranch>> {
    let output = git(
        repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/remotes"],
    )?;
    Ok(output
        .lines()
        .filter(|line| !line.is_empty() && !line.ends_with("/HEAD"))
        .filter_map(|full| {
            let (remote, name) = full.split_once('/')?;
            Some(RemoteBranch {
                remote: remote.to_string(),
                name: name.to_string(),
                full: full.to_string(),
            })
        })
        .collect())
}

pub fn list_tags(repo: &Path) -> anyhow::Result<Vec<TagEntry>> {
    let output = git(
        repo,
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)\x1f%(objectname:short)",
            "refs/tags",
        ],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let (name, sha) = line.split_once('\x1f')?;
            if name.is_empty() {
                return None;
            }
            Some(TagEntry {
                name: name.to_string(),
                sha: sha.to_string(),
            })
        })
        .collect())
}

pub fn list_stashes(repo: &Path) -> anyhow::Result<Vec<StashEntry>> {
    let output = git(repo, &["stash", "list", "--format=%gd\x1f%H\x1f%s"])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\x1f').collect();
            if fields.len() < 3 {
                return None;
            }
            let label = fields[0].to_string();
            let index = label
                .strip_prefix("stash@{")
                .and_then(|rest| rest.strip_suffix('}'))
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            Some(StashEntry {
                index,
                label,
                sha: fields[1].to_string(),
                message: fields[2].to_string(),
            })
        })
        .collect())
}

pub fn stash_detail(repo: &Path, index: u32) -> anyhow::Result<CommitDetail> {
    let reference = stash_ref(index);
    let meta = git(
        repo,
        &[
            "show",
            "-s",
            &format!("--pretty=format:{GRAPH_FORMAT}\x1f%b"),
            &reference,
        ],
    )?;
    let fields: Vec<&str> = meta.split('\x1f').collect();
    if fields.len() < 9 {
        return Err(anyhow!("unexpected stash metadata for {reference}"));
    }
    Ok(CommitDetail {
        sha: fields[0].to_string(),
        short_sha: fields[1].to_string(),
        parents: fields[2]
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        refs: vec![CommitRef {
            name: reference,
            kind: "stash".to_string(),
        }],
        author_name: fields[4].to_string(),
        author_email: fields[5].to_string(),
        date: fields[6].to_string(),
        subject: fields[7].to_string(),
        body: fields[8].trim().to_string(),
        files: stash_files(repo, index)?,
    })
}

fn stash_files(repo: &Path, index: u32) -> anyhow::Result<Vec<CommitFile>> {
    let reference = stash_ref(index);
    let numstat = git(
        repo,
        &[
            "stash",
            "show",
            "--include-untracked",
            "--numstat",
            &reference,
        ],
    )?;
    let mut stats: HashMap<String, (u32, u32)> = HashMap::new();
    for line in numstat.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        stats.insert(
            parts.last().unwrap_or(&"").to_string(),
            (parse_numstat(parts[0]), parse_numstat(parts[1])),
        );
    }

    let name_status = git(
        repo,
        &[
            "stash",
            "show",
            "--include-untracked",
            "--name-status",
            &reference,
        ],
    )?;
    let mut files = Vec::new();
    for line in name_status.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let status = parts[0].to_string();
        let (old_path, path) =
            if (status.starts_with('R') || status.starts_with('C')) && parts.len() >= 3 {
                (Some(parts[1].to_string()), parts[2].to_string())
            } else {
                (None, parts[1].to_string())
            };
        let (additions, deletions) = stats.get(&path).copied().unwrap_or((0, 0));
        files.push(CommitFile {
            path,
            old_path,
            status,
            additions,
            deletions,
        });
    }
    Ok(files)
}

pub fn stash_file_diff(repo: &Path, index: u32, file_path: &str) -> anyhow::Result<FilePatch> {
    validate_git_path(file_path)?;
    let reference = stash_ref(index);
    let base = format!("{reference}^1");
    let mut patch = git(repo, &["diff", &base, &reference, "--", file_path])?;
    if patch.trim().is_empty() {
        let untracked = format!("{reference}^3");
        patch = git(repo, &["diff", &base, &untracked, "--", file_path]).unwrap_or_default();
    }
    Ok(FilePatch {
        path: file_path.to_string(),
        area: "stash".to_string(),
        hunks: parse_hunks(&patch),
        patch,
    })
}

fn stash_ref(index: u32) -> String {
    format!("stash@{{{index}}}")
}

#[derive(Debug, Clone, Serialize)]
pub struct Submodule {
    pub path: String,
    pub sha: String,
    pub status: String,
    pub describe: Option<String>,
}

pub fn list_submodules(repo: &Path) -> anyhow::Result<Vec<Submodule>> {
    // `git submodule status` lines: "<flag><sha> <path> (<describe>)"; flag is
    // ' '=ok, '+'=needs update, '-'=uninitialized, 'U'=conflicts.
    let output = git(repo, &["submodule", "status"])?;
    Ok(output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let mut chars = line.chars();
            let flag = chars.next()?;
            let status = match flag {
                '+' => "out-of-date",
                '-' => "uninitialized",
                'U' => "conflict",
                _ => "ok",
            };
            let rest = &line[flag.len_utf8()..];
            let mut parts = rest.splitn(2, ' ');
            let sha = parts.next().unwrap_or("").to_string();
            let remainder = parts.next().unwrap_or("").trim();
            let (path, describe) = match remainder.rfind(" (") {
                Some(open) => (
                    remainder[..open].to_string(),
                    Some(remainder[open + 2..].trim_end_matches(')').to_string()),
                ),
                None => (remainder.to_string(), None),
            };
            if path.is_empty() {
                return None;
            }
            Some(Submodule {
                path,
                sha,
                status: status.to_string(),
                describe,
            })
        })
        .collect())
}

pub fn update_submodule(repo: &Path, path: &str, init: bool) -> anyhow::Result<String> {
    validate_git_path(path)?;
    let mut args: Vec<&str> = vec!["submodule", "update"];
    if init {
        args.push("--init");
    }
    args.push("--");
    args.push(path);
    git(repo, &args)
}

pub fn stash_save(
    repo: &Path,
    message: Option<&str>,
    include_untracked: bool,
) -> anyhow::Result<String> {
    let mut args: Vec<String> = vec!["stash".into(), "push".into()];
    if include_untracked {
        args.push("--include-untracked".into());
    }
    if let Some(message) = message.map(str::trim).filter(|m| !m.is_empty()) {
        args.push("-m".into());
        args.push(message.into());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    git(repo, &refs)
}

pub fn stash_file(repo: &Path, file_path: &str, message: Option<&str>) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    let mut args: Vec<String> = vec!["stash".into(), "push".into(), "--include-untracked".into()];
    if let Some(message) = message.map(str::trim).filter(|m| !m.is_empty()) {
        args.push("-m".into());
        args.push(message.into());
    }
    args.push("--".into());
    args.push(file_path.into());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    git(repo, &refs)
}

pub fn ignore_file(repo: &Path, file_path: &str, target: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    let target_path = match target {
        "info_exclude" => {
            let path = git(
                repo,
                &[
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-path",
                    "info/exclude",
                ],
            )?;
            PathBuf::from(path)
        }
        "gitignore" => repo.join(".gitignore"),
        _ => return Err(anyhow!("unknown ignore target: {target}")),
    };
    let pattern = ignore_pattern(file_path);
    append_unique_line(&target_path, &pattern)?;
    Ok(format!("Added {pattern} to {}", target_path.display()))
}

pub fn external_diff(repo: &Path, file_path: &str, area: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    if !matches!(area, "staged" | "unstaged") {
        return Err(anyhow!("unknown patch area: {area}"));
    }
    if git(repo, &["config", "--get", "diff.tool"])
        .unwrap_or_default()
        .is_empty()
    {
        return Err(anyhow!(
            "Configure um difftool externo antes de usar External Diff (git config diff.tool <tool>)."
        ));
    }
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .arg("difftool")
        .arg("--no-prompt");
    if area == "staged" {
        command.arg("--cached");
    }
    command.arg("--").arg(file_path);
    command
        .spawn()
        .with_context(|| format!("failed to start external diff for {file_path}"))?;
    Ok(format!("External diff started for {file_path}"))
}

fn ignore_pattern(file_path: &str) -> String {
    format!("/{}", file_path.trim_start_matches('/'))
}

fn append_unique_line(path: &Path, line: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let current = std::fs::read_to_string(path).unwrap_or_default();
    if current.lines().any(|existing| existing.trim() == line) {
        return Ok(());
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if !current.is_empty() && !current.ends_with('\n') {
        writeln!(file)?;
    }
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn stash_pop(repo: &Path, index: u32) -> anyhow::Result<String> {
    git(repo, &["stash", "pop", &format!("stash@{{{index}}}")])
}

pub fn stash_apply(repo: &Path, index: u32) -> anyhow::Result<String> {
    git(repo, &["stash", "apply", &format!("stash@{{{index}}}")])
}

pub fn stash_drop(repo: &Path, index: u32) -> anyhow::Result<String> {
    git(repo, &["stash", "drop", &format!("stash@{{{index}}}")])
}

fn parse_track(track: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;
    for part in track.trim_matches(['[', ']']).split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("ahead ") {
            ahead = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = part.strip_prefix("behind ") {
            behind = rest.trim().parse().unwrap_or(0);
        }
    }
    (ahead, behind)
}

fn validate_rev(rev: &str) -> anyhow::Result<()> {
    if rev.is_empty() || rev.starts_with('-') || rev.chars().any(char::is_whitespace) {
        return Err(anyhow!("invalid revision: {rev}"));
    }
    Ok(())
}

fn validate_ref_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty()
        || name.starts_with('-')
        || name.contains("..")
        || name
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
    {
        return Err(anyhow!("invalid ref name: {name}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

pub fn commit(repo: &Path, message: &str, amend: bool) -> anyhow::Result<String> {
    let trimmed = message.trim();
    if amend {
        if trimmed.is_empty() {
            return git(repo, &["commit", "--amend", "--no-edit"]);
        }
        return git(repo, &["commit", "--amend", "-m", trimmed]);
    }
    if trimmed.is_empty() {
        return Err(anyhow!("commit message is required"));
    }
    git(repo, &["commit", "-m", trimmed])
}

pub fn fetch(repo: &Path, remote: Option<&str>) -> anyhow::Result<String> {
    match remote {
        Some(remote) => {
            validate_ref_name(remote)?;
            git(repo, &["fetch", "--prune", remote])
        }
        None => git(repo, &["fetch", "--all", "--prune"]),
    }
}

pub fn pull(repo: &Path, rebase: bool) -> anyhow::Result<String> {
    let flag = if rebase { "--rebase" } else { "--no-rebase" };
    // With an upstream configured, a plain pull just works.
    if git(repo, &["rev-parse", "--abbrev-ref", "@{u}"]).is_ok() {
        return git(repo, &["pull", flag]);
    }
    // No upstream: pull from origin/<branch> if it exists and set tracking,
    // otherwise give a clear message instead of git's raw error.
    let branch = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch.trim();
    if branch.is_empty() || branch == "HEAD" {
        return Err(anyhow!(
            "HEAD destacado: faça checkout de uma branch antes do pull."
        ));
    }
    let remote_ref = format!("refs/remotes/origin/{branch}");
    if git(repo, &["rev-parse", "--verify", "--quiet", &remote_ref]).is_err() {
        return Err(anyhow!(
            "A branch '{branch}' não tem upstream e não existe em origin. Faça push primeiro (o botão Push cria o upstream)."
        ));
    }
    let result = git(repo, &["pull", flag, "origin", branch])?;
    let _ = git(
        repo,
        &[
            "branch",
            &format!("--set-upstream-to=origin/{branch}"),
            branch,
        ],
    );
    Ok(result)
}

pub fn push(repo: &Path, set_upstream: bool, force_with_lease: bool) -> anyhow::Result<String> {
    let mut args: Vec<&str> = vec!["push"];
    if force_with_lease {
        args.push("--force-with-lease");
    }
    if set_upstream {
        args.extend_from_slice(&["-u", "origin", "HEAD"]);
    }
    git(repo, &args)
}

// Checkout with an explicit policy for uncommitted changes (chosen by the user):
//   "plain"        — plain `git checkout` (git refuses if changes would be lost).
//   "discard"      — `git checkout --force` + clean untracked: drop local changes.
//   "stash"        — stash everything (incl. untracked), switch, leave it stashed.
//   "stash_apply"  — stash, switch, then apply the stash onto the target branch
//                    (may conflict; the user opted into carrying changes over).
pub fn checkout_branch(repo: &Path, name: &str, mode: &str) -> anyhow::Result<String> {
    validate_ref_name(name)?;
    match mode {
        "plain" => git(repo, &["checkout", name]),
        "discard" => {
            git(repo, &["checkout", "--force", name])?;
            // drop untracked/ignored leftovers that --force does not touch
            let _ = git(repo, &["clean", "-fd"]);
            Ok(format!("Switched to {name} (local changes discarded)"))
        }
        "stash" => {
            git(
                repo,
                &[
                    "stash",
                    "push",
                    "--include-untracked",
                    "-m",
                    "dwgui-checkout",
                ],
            )?;
            git(repo, &["checkout", name])?;
            Ok(format!(
                "Switched to {name}. Local changes stashed (dwgui-checkout) — pop them when you return."
            ))
        }
        "stash_apply" => {
            git(
                repo,
                &[
                    "stash",
                    "push",
                    "--include-untracked",
                    "-m",
                    "dwgui-checkout",
                ],
            )?;
            git(repo, &["checkout", name])?;
            let applied = git(repo, &["stash", "apply"]);
            match applied {
                Ok(_) => {
                    let _ = git(repo, &["stash", "drop"]);
                    Ok(format!("Switched to {name} and re-applied local changes."))
                }
                Err(error) => Ok(format!(
                    "Switched to {name}, but applying the stash hit conflicts ({error}). Your changes are safe in stash 'dwgui-checkout' — resolve in Local Changes."
                )),
            }
        }
        _ => Err(anyhow!("invalid checkout mode: {mode}")),
    }
}

pub fn create_branch(
    repo: &Path,
    name: &str,
    start_point: Option<&str>,
    checkout: bool,
) -> anyhow::Result<String> {
    validate_ref_name(name)?;
    if let Some(start) = start_point {
        validate_rev(start)?;
    }
    let verb = if checkout { "checkout" } else { "branch" };
    let mut args: Vec<&str> = if checkout {
        vec![verb, "-b", name]
    } else {
        vec![verb, name]
    };
    if let Some(start) = start_point {
        args.push(start);
    }
    git(repo, &args)
}

pub fn rename_branch(repo: &Path, old_name: &str, new_name: &str) -> anyhow::Result<String> {
    validate_ref_name(old_name)?;
    validate_ref_name(new_name)?;
    git(repo, &["branch", "-m", old_name, new_name])
}

pub fn delete_branch(repo: &Path, name: &str, force: bool) -> anyhow::Result<String> {
    validate_ref_name(name)?;
    git(repo, &["branch", if force { "-D" } else { "-d" }, name])
}

pub fn merge_branch(repo: &Path, name: &str) -> anyhow::Result<String> {
    validate_ref_name(name)?;
    git(repo, &["merge", name])
}

pub fn rebase_branch(repo: &Path, onto: &str) -> anyhow::Result<String> {
    validate_ref_name(onto)?;
    git(repo, &["rebase", onto])
}

pub fn cherry_pick(repo: &Path, sha: &str) -> anyhow::Result<String> {
    validate_rev(sha)?;
    git(repo, &["cherry-pick", sha])
}

pub fn revert(repo: &Path, sha: &str) -> anyhow::Result<String> {
    validate_rev(sha)?;
    git(repo, &["revert", "--no-edit", sha])
}

pub fn reset(repo: &Path, sha: &str, mode: &str) -> anyhow::Result<String> {
    validate_rev(sha)?;
    let flag = match mode {
        "soft" => "--soft",
        "mixed" => "--mixed",
        "hard" => "--hard",
        _ => return Err(anyhow!("invalid reset mode: {mode}")),
    };
    git(repo, &["reset", flag, sha])
}

pub fn create_tag(
    repo: &Path,
    name: &str,
    sha: Option<&str>,
    message: Option<&str>,
) -> anyhow::Result<String> {
    validate_ref_name(name)?;
    if let Some(sha) = sha {
        validate_rev(sha)?;
    }
    let mut args: Vec<&str> = vec!["tag"];
    if let Some(message) = message.map(str::trim).filter(|m| !m.is_empty()) {
        args.extend_from_slice(&["-a", name, "-m", message]);
    } else {
        args.push(name);
    }
    if let Some(sha) = sha {
        args.push(sha);
    }
    git(repo, &args)
}

pub fn abort_operation(repo: &Path, operation: &str) -> anyhow::Result<String> {
    let verb = match operation {
        "merge" => "merge",
        "rebase" => "rebase",
        "cherry-pick" => "cherry-pick",
        "revert" => "revert",
        _ => return Err(anyhow!("invalid operation: {operation}")),
    };
    git(repo, &[verb, "--abort"])
}

pub fn discard_file(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    if !path_exists_in_head(repo, file_path) {
        let _ = git(repo, &["rm", "--cached", "--force", "--", file_path]);
        remove_worktree_path(repo, file_path)?;
        return Ok(format!("Removed {file_path}"));
    }
    git(
        repo,
        &[
            "restore",
            "--staged",
            "--worktree",
            "--source=HEAD",
            "--",
            file_path,
        ],
    )
}

fn remove_worktree_path(repo: &Path, file_path: &str) -> anyhow::Result<()> {
    let target = repo.join(file_path);
    if !target.exists() {
        return Ok(());
    }
    let root = std::fs::canonicalize(repo)
        .with_context(|| format!("failed to resolve repo root {}", repo.display()))?;
    let canonical = std::fs::canonicalize(&target)
        .with_context(|| format!("failed to resolve worktree file {file_path}"))?;
    if !canonical.starts_with(root) {
        return Err(anyhow!("git path escapes project root"));
    }
    if canonical.is_dir() {
        std::fs::remove_dir_all(&canonical)?;
    } else {
        std::fs::remove_file(&canonical)?;
    }
    Ok(())
}

pub fn discard_hunk(repo: &Path, hunk_patch: &str) -> anyhow::Result<PatchCheckResult> {
    let check = git_with_input(repo, &["apply", "--reverse", "--check"], hunk_patch)?;
    if !check.ok {
        return Ok(check);
    }
    git_with_input(repo, &["apply", "--reverse"], hunk_patch)
}

pub fn delete_tag(repo: &Path, name: &str) -> anyhow::Result<String> {
    validate_ref_name(name)?;
    git(repo, &["tag", "-d", name])
}

/// Detach HEAD onto a specific commit (context-menu "checkout this commit").
pub fn checkout_commit(repo: &Path, sha: &str) -> anyhow::Result<String> {
    validate_rev(sha)?;
    git(repo, &["checkout", sha])
}

pub fn use_ours(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    git(repo, &["checkout", "--ours", "--", file_path])?;
    git(repo, &["add", "--", file_path])
}

pub fn use_theirs(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    git(repo, &["checkout", "--theirs", "--", file_path])?;
    git(repo, &["add", "--", file_path])
}

pub fn mark_resolved(repo: &Path, file_path: &str) -> anyhow::Result<String> {
    validate_git_path(file_path)?;
    git(repo, &["add", "--", file_path])
}

#[derive(Debug, Deserialize)]
pub struct RebaseStep {
    pub action: String,
    pub sha: String,
}

/// Drive `git rebase -i <base>` non-interactively: `editor_cmd` is the app's own
/// executable acting as the sequence editor (it overwrites the todo with the
/// `DWGUI_REBASE_TODO` text we pass via env); GIT_EDITOR=true keeps default
/// messages for squash/fixup. Mid-rebase conflicts surface via repo_state and
/// are resolved through the conflict UI + continue_operation("rebase").
pub fn start_interactive_rebase(
    repo: &Path,
    base: &str,
    steps: &[RebaseStep],
    editor_cmd: &str,
) -> anyhow::Result<String> {
    validate_rev(base)?;
    const ALLOWED: [&str; 6] = ["pick", "reword", "edit", "squash", "fixup", "drop"];
    let mut todo = String::new();
    for step in steps {
        validate_rev(&step.sha)?;
        if !ALLOWED.contains(&step.action.as_str()) {
            return Err(anyhow!("invalid rebase action: {}", step.action));
        }
        // Dropped commits are simply omitted from the todo.
        if step.action == "drop" {
            continue;
        }
        todo.push_str(&format!("{} {}\n", step.action, step.sha));
    }
    if todo.trim().is_empty() {
        return Err(anyhow!("rebase todo is empty (all commits dropped?)"));
    }
    git_env(
        repo,
        &["rebase", "-i", base],
        &[
            ("GIT_SEQUENCE_EDITOR", editor_cmd),
            ("GIT_EDITOR", "true"),
            ("DWGUI_REBASE_TODO", &todo),
        ],
    )
}

/// Continue an in-progress operation (merge/rebase/cherry-pick/revert) without
/// opening an editor.
pub fn continue_operation(repo: &Path, operation: &str) -> anyhow::Result<String> {
    let verb = match operation {
        "merge" => "merge",
        "rebase" => "rebase",
        "cherry-pick" => "cherry-pick",
        "revert" => "revert",
        _ => return Err(anyhow!("invalid operation: {operation}")),
    };
    git_env(repo, &[verb, "--continue"], &[("GIT_EDITOR", "true")])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_a_commit_record_with_parents_and_refs() {
        let record = "abc123\x1fabc12\x1fp1 p2\x1fHEAD -> main, origin/main, tag: v1\x1fBruno\x1fb@x.com\x1f2026-05-25T10:00:00-03:00\x1ffeat: do it";
        let commit = parse_commit_record(record).expect("commit");
        assert_eq!(commit.sha, "abc123");
        assert_eq!(commit.parents, vec!["p1", "p2"]);
        assert_eq!(commit.subject, "feat: do it");
        let kinds: Vec<&str> = commit.refs.iter().map(|r| r.kind.as_str()).collect();
        assert_eq!(kinds, vec!["head", "remote", "tag"]);
        assert_eq!(commit.refs[0].name, "main");
        assert_eq!(commit.refs[2].name, "v1");
    }

    #[test]
    fn parses_blame_porcelain_with_cached_headers() {
        let sha = "a".repeat(40);
        let out = format!(
            "{sha} 1 1 2\nauthor Bruno\nauthor-mail <b@x.com>\nauthor-time 1700000000\nsummary feat: do it\nfilename src/a.ts\n\tline one\n{sha} 2 2\n\tline two\n"
        );
        let lines = parse_blame_porcelain(&out);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line, 1);
        assert_eq!(lines[0].author, "Bruno");
        assert_eq!(lines[0].summary, "feat: do it");
        assert_eq!(lines[0].short_sha, "aaaaaaaa");
        // Second line reuses the cached header (no repeated block).
        assert_eq!(lines[1].line, 2);
        assert_eq!(lines[1].author, "Bruno");
    }

    #[test]
    fn blame_porcelain_for_contents_marks_inserted_lines_uncommitted() {
        let root = init_repo();
        let lines = blame_porcelain_for_contents(&root, "file.txt", "one\ninserted\ntwo\nthree\n")
            .expect("blame current buffer");

        assert_eq!(
            lines.iter().map(|line| line.line).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_ne!(lines[0].sha, UNCOMMITTED_SHA);
        assert_eq!(lines[1].sha, UNCOMMITTED_SHA);
        assert_ne!(lines[2].sha, UNCOMMITTED_SHA);
        assert_ne!(lines[3].sha, UNCOMMITTED_SHA);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn blame_porcelain_for_contents_synthesizes_untracked_lines() {
        let root = init_repo();
        let lines = blame_porcelain_for_contents(&root, "new.txt", "alpha\nbeta\n")
            .expect("blame new file");

        assert_eq!(
            lines.iter().map(|line| line.line).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(lines.iter().all(|line| line.sha == UNCOMMITTED_SHA));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn parses_upstream_track_counts() {
        assert_eq!(parse_track("[ahead 2, behind 3]"), (2, 3));
        assert_eq!(parse_track("[ahead 1]"), (1, 0));
        assert_eq!(parse_track("[behind 4]"), (0, 4));
        assert_eq!(parse_track(""), (0, 0));
        assert_eq!(parse_track("[gone]"), (0, 0));
    }

    #[test]
    fn parses_worktree_counts_from_porcelain_status() {
        let counts = parse_worktree_counts(
            "M  staged.txt\n M unstaged.txt\nMM both.txt\nUU conflict.txt\n",
            2,
        );

        assert_eq!(counts.staged, 3);
        assert_eq!(counts.unstaged, 3);
        assert_eq!(counts.untracked, 2);
        assert_eq!(counts.conflicts, 1);
        assert_eq!(counts.total, 8);
        assert_eq!(
            conflict_files_from_status("UU conflict.txt\n M clean.txt\n"),
            vec!["conflict.txt".to_string()]
        );
    }

    #[test]
    fn validate_rev_rejects_flags_and_spaces() {
        assert!(validate_rev("HEAD").is_ok());
        assert!(validate_rev("abc123").is_ok());
        assert!(validate_rev("--all").is_err());
        assert!(validate_rev("a b").is_err());
        assert!(validate_rev("").is_err());
    }

    #[test]
    fn repo_snapshot_collects_current_repo_state() {
        let root = init_repo();
        let snapshot = repo_snapshot(&root, false, false, 50).expect("snapshot");

        assert!(!snapshot.generated_at.is_empty());
        assert_eq!(snapshot.options.limit, 50);
        assert!(!snapshot.commits.is_empty());
        assert!(snapshot.repo_state.branch.is_some());
        assert!(snapshot.branches.iter().any(|branch| branch.is_head));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn worktree_snapshot_limits_untracked_files_but_reports_total() {
        let root = init_repo();
        std::fs::write(root.join("tracked.txt"), "tracked\n").expect("write tracked file");
        run_git(&root, &["add", "tracked.txt"]);
        std::fs::write(root.join("file.txt"), "one\nmodified\nthree\n").expect("modify file");
        std::fs::write(root.join("a.tmp"), "a\n").expect("write untracked a");
        std::fs::write(root.join("b.tmp"), "b\n").expect("write untracked b");

        let snapshot = worktree_snapshot(&root, 1).expect("worktree snapshot");

        assert_eq!(snapshot.counts.staged, 1);
        assert_eq!(snapshot.counts.unstaged, 1);
        assert_eq!(snapshot.counts.untracked, 2);
        assert!(snapshot.untracked_truncated);
        assert_eq!(
            snapshot
                .files
                .iter()
                .filter(|file| file.status == "??")
                .count(),
            1
        );
        assert!(!snapshot.fingerprint.is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn worktree_fingerprint_changes_when_untracked_files_change() {
        let root = init_repo();
        let before = worktree_fingerprint(&root).expect("fingerprint before");
        std::fs::write(root.join("new.txt"), "new\n").expect("write untracked");
        let after = worktree_fingerprint(&root).expect("fingerprint after");

        assert_ne!(before.fingerprint, after.fingerprint);
        assert_eq!(after.counts.untracked, 1);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn repo_state_marks_untracked_worktree_as_dirty() {
        let root = init_repo();
        std::fs::write(root.join("new.txt"), "new\n").expect("write untracked");
        let state = repo_state(&root).expect("repo state");

        assert!(state.dirty);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn staged_diff_contains_only_cached_changes() {
        let root = init_repo();
        std::fs::write(root.join("staged.txt"), "before staged\n").expect("write staged file");
        std::fs::write(root.join("unstaged.txt"), "before unstaged\n")
            .expect("write unstaged file");
        run_git(&root, &["add", "staged.txt", "unstaged.txt"]);
        run_git(&root, &["commit", "-m", "add tracked files"]);

        std::fs::write(root.join("staged.txt"), "after staged\n").expect("modify staged file");
        run_git(&root, &["add", "staged.txt"]);
        std::fs::write(root.join("unstaged.txt"), "after unstaged\n")
            .expect("modify unstaged file");

        let diff = staged_diff(&root).expect("staged diff");
        assert!(diff.contains("staged.txt"));
        assert!(diff.contains("after staged"));
        assert!(!diff.contains("unstaged.txt"));
        assert!(!diff.contains("after unstaged"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn staged_diff_is_empty_without_cached_changes() {
        let root = init_repo();
        std::fs::write(root.join("file.txt"), "one\nunstaged only\nthree\n").expect("modify file");

        assert_eq!(staged_diff(&root).expect("staged diff"), "");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn commit_creates_commit_with_subject_and_body() {
        let root = init_repo();
        std::fs::write(root.join("file.txt"), "one\ncommitted\nthree\n").expect("modify file");
        run_git(&root, &["add", "file.txt"]);

        commit(
            &root,
            "fix(git): keep app open after commit\n\nRefresh Git state without losing the commit form on failure.",
            false,
        )
        .expect("commit");

        let message = git(&root, &["log", "-1", "--pretty=%B"]).expect("log message");
        assert!(message.contains("fix(git): keep app open after commit"));
        assert!(message.contains("Refresh Git state without losing the commit form on failure."));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn commit_rejects_empty_message_without_mutating_history() {
        let root = init_repo();
        let before = git(&root, &["rev-parse", "HEAD"]).expect("head before");
        std::fs::write(root.join("file.txt"), "one\nchanged\nthree\n").expect("modify file");
        run_git(&root, &["add", "file.txt"]);

        assert!(commit(&root, "   ", false).is_err());

        let after = git(&root, &["rev-parse", "HEAD"]).expect("head after");
        assert_eq!(after, before);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn file_patch_text_synthesizes_untracked_patch() {
        let root = init_repo();
        std::fs::write(root.join("new.txt"), "new\n").expect("write untracked");

        let patch = file_patch_text(&root, "new.txt", "unstaged").expect("patch");

        assert!(patch.contains("new.txt"));
        assert!(patch.contains("+new"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn ignore_file_adds_info_exclude_once() {
        let root = init_repo();

        ignore_file(&root, "tmp/cache.log", "info_exclude").expect("ignore once");
        ignore_file(&root, "tmp/cache.log", "info_exclude").expect("ignore twice");

        let exclude = std::fs::read_to_string(root.join(".git/info/exclude")).expect("exclude");
        assert_eq!(
            exclude
                .lines()
                .filter(|line| line.trim() == "/tmp/cache.log")
                .count(),
            1
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stash_file_only_stashes_selected_path() {
        let root = init_repo();
        std::fs::write(root.join("file.txt"), "changed\n").expect("modify tracked");
        std::fs::write(root.join("other.txt"), "other\n").expect("write untracked");

        stash_file(&root, "file.txt", Some("single file")).expect("stash file");

        assert_eq!(
            std::fs::read_to_string(root.join("file.txt")).expect("read file"),
            "one\ntwo\nthree\n"
        );
        let status = git(&root, &["status", "--porcelain"]).expect("status");
        assert!(status.contains("?? other.txt"));
        assert!(!status.contains("file.txt"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stash_detail_lists_tracked_and_untracked_files() {
        let root = init_repo();
        std::fs::write(root.join("file.txt"), "one\nchanged\nthree\n").expect("modify tracked");
        std::fs::write(root.join("new.txt"), "new\n").expect("write untracked");
        run_git(
            &root,
            &["stash", "push", "--include-untracked", "-m", "stash detail"],
        );

        let detail = stash_detail(&root, 0).expect("stash detail");

        assert!(detail.subject.contains("stash detail"));
        assert!(detail
            .files
            .iter()
            .any(|file| file.path == "file.txt" && file.status == "M"));
        assert!(detail
            .files
            .iter()
            .any(|file| file.path == "new.txt" && file.status == "A"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stash_file_diff_reads_untracked_parent() {
        let root = init_repo();
        std::fs::write(root.join("new.txt"), "new\n").expect("write untracked");
        run_git(
            &root,
            &[
                "stash",
                "push",
                "--include-untracked",
                "-m",
                "stash untracked",
            ],
        );

        let patch = stash_file_diff(&root, 0, "new.txt").expect("stash file diff");

        assert!(patch.patch.contains("new file mode"));
        assert!(patch.patch.contains("+new"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn discard_file_removes_untracked_file() {
        let root = init_repo();
        std::fs::write(root.join("new.txt"), "new\n").expect("write untracked");

        discard_file(&root, "new.txt").expect("discard untracked");

        assert!(!root.join("new.txt").exists());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn discard_file_removes_staged_new_file() {
        let root = init_repo();
        std::fs::write(root.join("new.txt"), "new\n").expect("write new file");
        run_git(&root, &["add", "new.txt"]);

        discard_file(&root, "new.txt").expect("discard staged new file");

        assert!(!root.join("new.txt").exists());
        let status = git(&root, &["status", "--porcelain"]).expect("status");
        assert!(!status.contains("new.txt"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn fixture_root() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-gui-git-test-{unique}"));
        std::fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_repo() -> std::path::PathBuf {
        let root = fixture_root();
        run_git(&root, &["init"]);
        run_git(&root, &["config", "user.email", "test@example.com"]);
        run_git(&root, &["config", "user.name", "Test User"]);
        std::fs::write(root.join("file.txt"), "one\ntwo\nthree\n").expect("write file");
        run_git(&root, &["add", "file.txt"]);
        run_git(&root, &["commit", "-m", "initial"]);
        root
    }

    #[test]
    fn parses_hunks_from_unified_patch() {
        let patch = "diff --git a/file.txt b/file.txt\nindex 111..222 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,2 @@\n-old\n+new\n keep\n@@ -8 +8 @@\n-a\n+b\n";
        let hunks = parse_hunks(patch);

        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].old_start, 1);
        assert_eq!(hunks[0].old_lines, 2);
        assert_eq!(hunks[1].new_start, 8);
        assert!(hunks[0].patch.contains("diff --git a/file.txt b/file.txt"));
    }

    #[test]
    fn rejects_paths_that_escape_repo() {
        assert!(validate_git_path("../outside.txt").is_err());
        assert!(validate_git_path("/tmp/outside.txt").is_err());
        assert!(validate_git_path("src/App.tsx").is_ok());
    }

    #[test]
    fn stages_and_unstages_whole_file() {
        let root = init_repo();
        std::fs::write(root.join("file.txt"), "one\nchanged\nthree\n").expect("modify file");

        stage_file(&root, "file.txt").expect("stage file");
        assert_eq!(changed_files(&root).expect("files").len(), 1);
        assert!(git(&root, &["diff"]).expect("unstaged diff").is_empty());

        unstage_file(&root, "file.txt").expect("unstage file");
        assert!(!git(&root, &["diff"]).expect("unstaged diff").is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stages_and_unstages_single_hunk() {
        let root = fixture_root();
        run_git(&root, &["init"]);
        run_git(&root, &["config", "user.email", "test@example.com"]);
        run_git(&root, &["config", "user.name", "Test User"]);
        let original = (1..=24)
            .map(|line| format!("line {line}"))
            .collect::<Vec<String>>()
            .join("\n")
            + "\n";
        std::fs::write(root.join("file.txt"), original).expect("write file");
        run_git(&root, &["add", "file.txt"]);
        run_git(&root, &["commit", "-m", "initial"]);

        let mut lines = (1..=24)
            .map(|line| format!("line {line}"))
            .collect::<Vec<String>>();
        lines[1] = "line 2 changed".to_string();
        lines[20] = "line 21 changed".to_string();
        std::fs::write(root.join("file.txt"), lines.join("\n") + "\n").expect("modify file");

        let patch = file_patch(&root, "file.txt", "unstaged").expect("unstaged patch");
        assert!(patch.hunks.len() >= 2);

        let stage_result = stage_hunk(&root, &patch.hunks[0].patch).expect("stage hunk");
        assert!(stage_result.ok, "{}", stage_result.output);
        assert!(!git(&root, &["diff", "--cached"])
            .expect("cached diff")
            .is_empty());
        assert!(!git(&root, &["diff"]).expect("unstaged diff").is_empty());

        let staged_patch = file_patch(&root, "file.txt", "staged").expect("staged patch");
        let unstage_result =
            unstage_hunk(&root, &staged_patch.hunks[0].patch).expect("unstage hunk");
        assert!(unstage_result.ok, "{}", unstage_result.output);
        assert!(git(&root, &["diff", "--cached"])
            .expect("cached diff")
            .is_empty());

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn invalid_patch_check_does_not_mutate_worktree() {
        let root = init_repo();
        let before = std::fs::read_to_string(root.join("file.txt")).expect("before");
        let result = check_patch(&root, "not a patch").expect("check patch");
        let after = std::fs::read_to_string(root.join("file.txt")).expect("after");

        assert!(!result.ok);
        assert_eq!(before, after);

        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
