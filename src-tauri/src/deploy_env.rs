use crate::store;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct DeployEnvironmentInput {
    pub version_id: String,
    pub machine_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveDeployEnvironmentInput {
    pub version_id: String,
    pub machine_id: String,
    pub variables: Vec<DeployEnvironmentValueInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeployEnvironmentValueInput {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployEnvironment {
    pub version_id: String,
    pub stack_id: String,
    pub machine_id: String,
    pub file_path: String,
    pub ready: bool,
    pub required_count: usize,
    pub saved_count: usize,
    pub missing_keys: Vec<String>,
    pub variables: Vec<DeployEnvironmentVariable>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployEnvironmentVariable {
    pub key: String,
    pub value: String,
    pub placeholder: String,
    pub required: bool,
    pub secret: bool,
    pub saved: bool,
}

#[derive(Debug, Clone)]
struct EnvTemplateVariable {
    key: String,
    placeholder: String,
    required: bool,
    secret: bool,
}

pub fn load_environment(
    db: &store::Database,
    input: DeployEnvironmentInput,
) -> anyhow::Result<DeployEnvironment> {
    let version = db.get_deploy_version(&input.version_id)?;
    let stack = db.get_deploy_stack(&version.stack_id)?;
    build_environment(db, &version, &stack, &input.machine_id)
}

pub fn save_environment(
    db: &store::Database,
    input: SaveDeployEnvironmentInput,
) -> anyhow::Result<DeployEnvironment> {
    let version = db.get_deploy_version(&input.version_id)?;
    let stack = db.get_deploy_stack(&version.stack_id)?;
    let templates = read_template_variables(&version)?;
    let allowed = templates
        .iter()
        .map(|variable| variable.key.as_str())
        .collect::<Vec<_>>();
    let mut values = BTreeMap::new();
    for variable in input.variables {
        let key = variable.key.trim();
        if !is_valid_env_key(key) {
            anyhow::bail!("invalid deploy environment key: {key}");
        }
        if !allowed.is_empty() && !allowed.contains(&key) {
            anyhow::bail!("deploy environment key is not declared by .env.example: {key}");
        }
        if variable.value.contains('\0')
            || variable.value.contains('\n')
            || variable.value.contains('\r')
        {
            anyhow::bail!("deploy environment value for {key} must be a single line");
        }
        values.insert(key.to_string(), variable.value);
    }
    write_saved_env(db, &stack, &input.machine_id, &values)?;
    build_environment(db, &version, &stack, &input.machine_id)
}

pub fn require_environment_ready(
    db: &store::Database,
    version: &store::DeployVersion,
    stack: &store::DeployStack,
    machine_id: &str,
) -> anyhow::Result<DeployEnvironment> {
    let env = build_environment(db, version, stack, machine_id)?;
    if !env.ready {
        anyhow::bail!(
            "deploy_environment_incomplete: configure {} for this stack and VM",
            env.missing_keys.join(", ")
        );
    }
    Ok(env)
}

pub fn write_runtime_env(
    db: &store::Database,
    version: &store::DeployVersion,
    stack: &store::DeployStack,
    machine_id: &str,
    package_root: &Path,
) -> anyhow::Result<DeployEnvironment> {
    let env = require_environment_ready(db, version, stack, machine_id)?;
    let mut values = BTreeMap::new();
    for variable in &env.variables {
        if variable.saved {
            values.insert(variable.key.clone(), variable.value.clone());
        }
    }
    write_env_file(&package_root.join(".env"), &values)?;
    Ok(env)
}

fn build_environment(
    db: &store::Database,
    version: &store::DeployVersion,
    stack: &store::DeployStack,
    machine_id: &str,
) -> anyhow::Result<DeployEnvironment> {
    if machine_id.trim().is_empty() {
        anyhow::bail!("deploy target machine is required");
    }
    let templates = read_template_variables(version)?;
    let env_path = saved_env_path(db, stack, machine_id)?;
    let saved = read_env_values(&env_path)?;
    let mut variables = Vec::new();
    for template in templates {
        let value = saved.get(&template.key).cloned().unwrap_or_default();
        let saved_value = !value.trim().is_empty();
        variables.push(DeployEnvironmentVariable {
            key: template.key,
            value,
            placeholder: template.placeholder,
            required: template.required,
            secret: template.secret,
            saved: saved_value,
        });
    }
    let missing_keys = variables
        .iter()
        .filter(|variable| variable.required && !variable.saved)
        .map(|variable| variable.key.clone())
        .collect::<Vec<_>>();
    let saved_count = variables.iter().filter(|variable| variable.saved).count();
    let required_count = variables
        .iter()
        .filter(|variable| variable.required)
        .count();
    Ok(DeployEnvironment {
        version_id: version.id.clone(),
        stack_id: stack.id.clone(),
        machine_id: machine_id.to_string(),
        file_path: env_path.display().to_string(),
        ready: missing_keys.is_empty(),
        required_count,
        saved_count,
        missing_keys,
        variables,
    })
}

fn read_template_variables(
    version: &store::DeployVersion,
) -> anyhow::Result<Vec<EnvTemplateVariable>> {
    let path = Path::new(&version.artifact_path).join(".env.example");
    let values = read_env_values(&path)?;
    Ok(values
        .into_iter()
        .map(|(key, placeholder)| EnvTemplateVariable {
            required: true,
            secret: is_sensitive_env_key_or_value(&key, &placeholder),
            key,
            placeholder,
        })
        .collect())
}

fn saved_env_path(
    db: &store::Database,
    stack: &store::DeployStack,
    machine_id: &str,
) -> anyhow::Result<PathBuf> {
    let workspace = db.get_workspace(stack.workspace_id)?;
    Ok(Path::new(&workspace.root_path)
        .join(".dw")
        .join("deploy-secrets")
        .join(safe_segment(&stack.slug))
        .join(format!("{}.env", safe_segment(machine_id))))
}

fn write_saved_env(
    db: &store::Database,
    stack: &store::DeployStack,
    machine_id: &str,
    values: &BTreeMap<String, String>,
) -> anyhow::Result<()> {
    let path = saved_env_path(db, stack, machine_id)?;
    if let Some(root) = path.parent().and_then(|parent| parent.parent()) {
        std::fs::create_dir_all(root)
            .with_context(|| format!("failed to create {}", root.display()))?;
        std::fs::write(root.join(".gitignore"), "*\n!.gitignore\n")
            .with_context(|| format!("failed to write {}", root.join(".gitignore").display()))?;
    }
    write_env_file(&path, values)
}

fn write_env_file(path: &Path, values: &BTreeMap<String, String>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut content = String::new();
    for (key, value) in values {
        if !is_valid_env_key(key) {
            anyhow::bail!("invalid deploy environment key: {key}");
        }
        if value.contains('\0') || value.contains('\n') || value.contains('\r') {
            anyhow::bail!("deploy environment value for {key} must be a single line");
        }
        content.push_str(key);
        content.push('=');
        content.push_str(value);
        content.push('\n');
    }
    std::fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to chmod {}", path.display()))?;
    }
    Ok(())
}

fn read_env_values(path: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    if !path.exists() {
        return Ok(values);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if is_valid_env_key(key) {
            values.insert(key.to_string(), unquote_env_value(value.trim()).to_string());
        }
    }
    Ok(values)
}

fn unquote_env_value(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn is_valid_env_key(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_sensitive_env_key_or_value(key: &str, value: &str) -> bool {
    let lower_key = key.to_ascii_lowercase();
    let lower_value = value.to_ascii_lowercase();
    lower_key.contains("password")
        || lower_key.contains("secret")
        || lower_key.contains("token")
        || lower_key.contains("api_key")
        || lower_key.contains("apikey")
        || lower_key.ends_with("_key")
        || lower_value.contains("password")
        || lower_value.contains("secret")
        || lower_value.contains("token")
}

fn safe_segment(value: &str) -> String {
    let segment = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if segment.is_empty() {
        "target".to_string()
    } else {
        segment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_template_and_marks_sensitive_values() {
        let root = temp_root("env-template");
        let artifact = root.join("artifact");
        std::fs::create_dir_all(&artifact).expect("artifact");
        std::fs::write(
            artifact.join(".env.example"),
            "# comment\nDATABASE_URL=postgres://user:password@postgres:5432/app\nREDIS_URL=redis://redis:6379\n",
        )
        .expect("env example");
        let version = deploy_version_fixture(&artifact);
        let variables = read_template_variables(&version).expect("template");
        assert_eq!(variables.len(), 2);
        assert!(variables[0].secret);
        assert!(!variables[1].secret);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn env_file_round_trips_without_accepting_multiline_values() {
        let root = temp_root("env-file");
        let path = root.join("stack").join("vm.env");
        let mut values = BTreeMap::new();
        values.insert("DATABASE_URL".to_string(), "postgres://local".to_string());
        write_env_file(&path, &values).expect("write env");
        assert_eq!(
            read_env_values(&path).expect("read").get("DATABASE_URL"),
            Some(&"postgres://local".to_string())
        );
        values.insert("BAD".to_string(), "line\nbreak".to_string());
        assert!(write_env_file(&path, &values).is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn deploy_version_fixture(artifact: &Path) -> store::DeployVersion {
        store::DeployVersion {
            id: "version".to_string(),
            stack_id: "stack".to_string(),
            workspace_id: 1,
            label: "deploy-001".to_string(),
            status: "review_required".to_string(),
            target_machine_id: None,
            artifact_path: artifact.display().to_string(),
            manifest_path: artifact.join("manifest.json").display().to_string(),
            manifest_json: "{}".to_string(),
            review_status: "pending".to_string(),
            reviewed_at: None,
            blocking_findings_json: "[]".to_string(),
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "dw-deploy-env-{}-{}",
            label,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).expect("mkdir");
        root
    }
}
