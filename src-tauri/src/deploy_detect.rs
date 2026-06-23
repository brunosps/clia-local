use crate::store;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployServiceSuggestion {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPortSuggestion {
    pub container: i64,
    pub host: i64,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployProjectDetection {
    pub project_id: i64,
    pub name: String,
    pub path: String,
    pub language: String,
    pub framework: Option<String>,
    pub package_manager: Option<String>,
    pub has_dockerfile: bool,
    pub has_compose: bool,
    pub services: Vec<DeployServiceSuggestion>,
    pub ports: Vec<DeployPortSuggestion>,
    pub healthcheck: Option<String>,
    pub deploy_strategy: String,
    pub strategy_reason: String,
    pub runtime_commands: Vec<String>,
    pub requires_desktop_session: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployDetectionReport {
    pub workspace_id: i64,
    pub projects: Vec<DeployProjectDetection>,
    pub services: Vec<DeployServiceSuggestion>,
    pub ports: Vec<DeployPortSuggestion>,
    pub warnings: Vec<String>,
}

pub fn detect_projects(
    db: &store::Database,
    workspace_id: i64,
    project_ids: &[i64],
) -> anyhow::Result<DeployDetectionReport> {
    let available_projects = db.list_projects(workspace_id)?;
    let ids = if project_ids.is_empty() {
        available_projects
            .iter()
            .map(|project| project.id)
            .collect()
    } else {
        project_ids.to_vec()
    };
    let mut projects = Vec::new();
    let mut warnings = Vec::new();
    for id in ids {
        let project = available_projects
            .iter()
            .find(|project| project.id == id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "deploy_project_selection_stale: project {id} is not available in workspace {workspace_id}; refresh projects and select an existing project"
                )
            })?;
        let detection = detect_project(project);
        warnings.extend(detection.warnings.iter().cloned());
        projects.push(detection);
    }
    if projects.is_empty() {
        anyhow::bail!(
            "deploy_project_selection_empty: add or select at least one project before creating a deploy package"
        );
    }
    let mut services = Vec::<DeployServiceSuggestion>::new();
    let mut ports = Vec::<DeployPortSuggestion>::new();
    for project in &projects {
        for service in &project.services {
            if !services.iter().any(|item| item.name == service.name) {
                services.push(service.clone());
            }
        }
        for port in &project.ports {
            if !ports.iter().any(|item| item.host == port.host) {
                ports.push(port.clone());
            }
        }
    }
    Ok(DeployDetectionReport {
        workspace_id,
        projects,
        services,
        ports,
        warnings,
    })
}

fn detect_project(project: &store::Project) -> DeployProjectDetection {
    let root = Path::new(&project.path);
    let package_json = read_to_string(root.join("package.json"));
    let pyproject = read_to_string(root.join("pyproject.toml"));
    let requirements = read_to_string(root.join("requirements.txt"));
    let cargo = read_to_string(root.join("Cargo.toml"));
    let csproj = first_file_with_extension(root, "csproj").and_then(read_to_string);
    let mut haystack = String::new();
    for content in [&package_json, &pyproject, &requirements, &cargo, &csproj]
        .into_iter()
        .flatten()
    {
        haystack.push_str(content);
        haystack.push('\n');
    }
    let lower = haystack.to_lowercase();
    let has_dockerfile = root.join("Dockerfile").exists() || root.join("Dockerfile.dev").exists();
    let has_compose = root.join("docker-compose.yml").exists()
        || root.join("compose.yml").exists()
        || root.join("docker-compose.dev.yml").exists()
        || root.join("docker-compose.prod.yml").exists();
    let (language, mut framework, package_manager, default_port) =
        if let Some(package_json) = package_json.as_deref() {
            (
                "typescript".to_string(),
                detect_node_framework(package_json),
                detect_node_package_manager(root),
                3000,
            )
        } else if pyproject.is_some() || requirements.is_some() {
            (
                "python".to_string(),
                detect_python_framework(&lower),
                detect_python_package_manager(root),
                8000,
            )
        } else if let Some(cargo) = cargo.as_deref() {
            (
                "rust".to_string(),
                detect_rust_framework(cargo),
                Some("cargo".to_string()),
                8080,
            )
        } else if csproj.is_some() {
            (
                "dotnet".to_string(),
                Some("aspnetcore".to_string()),
                Some("dotnet".to_string()),
                8080,
            )
        } else {
            ("unknown".to_string(), None, None, 8080)
        };
    let mut warnings = Vec::new();
    let is_tauri = package_json
        .as_deref()
        .map(|content| detect_tauri_project(root, content))
        .unwrap_or(false);
    if is_tauri {
        framework = Some("tauri".to_string());
    }
    let (
        deploy_strategy,
        strategy_reason,
        runtime_commands,
        requires_desktop_session,
        ports,
        healthcheck,
    ) = if has_compose || has_dockerfile {
        (
            "custom_compose".to_string(),
            "project provides Dockerfile or Compose; ADE will not replace its runtime contract"
                .to_string(),
            Vec::new(),
            false,
            vec![DeployPortSuggestion {
                container: default_port,
                host: default_port,
                confidence: "suggested".to_string(),
            }],
            Some(format!("http://localhost:{default_port}/health")),
        )
    } else if is_tauri {
        (
            "desktop_dev".to_string(),
            "Tauri desktop project detected; package is prepared directly on the Ubuntu Desktop VM"
                .to_string(),
            node_runtime_commands(package_json.as_deref(), true),
            true,
            Vec::new(),
            None,
        )
    } else if language == "typescript" && framework.is_some() {
        (
            "web_service".to_string(),
            "Node web framework detected; ADE can generate a Docker web service".to_string(),
            node_runtime_commands(package_json.as_deref(), false),
            false,
            vec![DeployPortSuggestion {
                container: default_port,
                host: default_port,
                confidence: "suggested".to_string(),
            }],
            Some(format!("http://localhost:{default_port}/health")),
        )
    } else if matches!(language.as_str(), "python" | "rust" | "dotnet") {
        (
            "web_service".to_string(),
            format!("{language} project detected; ADE can generate a basic service container"),
            Vec::new(),
            false,
            vec![DeployPortSuggestion {
                container: default_port,
                host: default_port,
                confidence: "suggested".to_string(),
            }],
            Some(format!("http://localhost:{default_port}/health")),
        )
    } else {
        (
            "unsupported".to_string(),
            "no web service, desktop runtime, Dockerfile, or Compose contract was detected"
                .to_string(),
            Vec::new(),
            false,
            Vec::new(),
            None,
        )
    };
    if language == "unknown" || deploy_strategy == "unsupported" {
        warnings.push(format!("{}: {}", project.name, strategy_reason));
    }
    let services = detect_services(&lower);
    DeployProjectDetection {
        project_id: project.id,
        name: project.name.clone(),
        path: project.path.clone(),
        language,
        framework,
        package_manager,
        has_dockerfile,
        has_compose,
        services,
        ports,
        healthcheck,
        deploy_strategy,
        strategy_reason,
        runtime_commands,
        requires_desktop_session,
        warnings,
    }
}

fn detect_node_framework(package_json: &str) -> Option<String> {
    let lower = package_json.to_lowercase();
    for (marker, label) in [
        ("next", "next"),
        ("vite", "vite"),
        ("nestjs", "nestjs"),
        ("fastify", "fastify"),
        ("express", "express"),
    ] {
        if lower.contains(marker) {
            return Some(label.to_string());
        }
    }
    None
}

fn detect_tauri_project(root: &Path, package_json: &str) -> bool {
    if root.join("src-tauri").join("Cargo.toml").exists() {
        return true;
    }
    if package_json
        .to_ascii_lowercase()
        .contains("@tauri-apps/cli")
    {
        return true;
    }
    node_script(package_json, "dev")
        .or_else(|| node_script(package_json, "build"))
        .map(|script| script.to_ascii_lowercase().contains("tauri"))
        .unwrap_or(false)
}

fn node_script(package_json: &str, script_name: &str) -> Option<String> {
    serde_json::from_str::<Value>(package_json)
        .ok()?
        .get("scripts")?
        .get(script_name)?
        .as_str()
        .map(ToOwned::to_owned)
}

fn node_runtime_commands(package_json: Option<&str>, tauri: bool) -> Vec<String> {
    let Some(package_json) = package_json else {
        return Vec::new();
    };
    let mut commands = vec!["npm install".to_string()];
    if tauri {
        commands.push("cargo metadata --manifest-path src-tauri/Cargo.toml".to_string());
    }
    if node_script(package_json, "check:js").is_some() {
        commands.push("npm run check:js".to_string());
    }
    if tauri {
        commands.push("npm run dev".to_string());
    } else if node_script(package_json, "dev").is_some() {
        commands.push("npm run dev -- --host 0.0.0.0".to_string());
    } else if node_script(package_json, "start").is_some() {
        commands.push("npm start".to_string());
    }
    commands
}

fn detect_node_package_manager(root: &Path) -> Option<String> {
    if root.join("pnpm-lock.yaml").exists() {
        Some("pnpm".to_string())
    } else if root.join("yarn.lock").exists() {
        Some("yarn".to_string())
    } else {
        Some("npm".to_string())
    }
}

fn detect_python_framework(lower: &str) -> Option<String> {
    for (marker, label) in [
        ("fastapi", "fastapi"),
        ("django", "django"),
        ("flask", "flask"),
        ("starlette", "starlette"),
    ] {
        if lower.contains(marker) {
            return Some(label.to_string());
        }
    }
    None
}

fn detect_python_package_manager(root: &Path) -> Option<String> {
    if root.join("uv.lock").exists() {
        Some("uv".to_string())
    } else if root.join("poetry.lock").exists() {
        Some("poetry".to_string())
    } else {
        Some("pip".to_string())
    }
}

fn detect_rust_framework(cargo: &str) -> Option<String> {
    let lower = cargo.to_lowercase();
    for (marker, label) in [
        ("axum", "axum"),
        ("actix-web", "actix-web"),
        ("rocket", "rocket"),
        ("warp", "warp"),
        ("tonic", "tonic"),
    ] {
        if lower.contains(marker) {
            return Some(label.to_string());
        }
    }
    None
}

fn detect_services(lower: &str) -> Vec<DeployServiceSuggestion> {
    let checks: &[(&[&str], &str)] = &[
        (
            &[
                "postgres",
                "postgresql",
                "\"pg\"",
                "prisma",
                "typeorm",
                "kysely",
                "drizzle-orm",
            ],
            "postgres",
        ),
        (&["mysql2", "mysqlclient", "pymysql", "mysql"], "mysql"),
        (&["ioredis", "redis", "bullmq"], "redis"),
        (&["amqplib", "rabbitmq", "masstransit", "lapin"], "rabbitmq"),
        (&["nodemailer", "sendgrid", "mailkit", "lettre"], "smtp"),
        (&["client-s3", "boto3", "awssdk.s3", "aws-sdk-s3"], "s3"),
        (&["meilisearch", "typesense", "elasticsearch"], "search"),
        (&["opentelemetry", "otel"], "otel"),
    ];
    let mut out = Vec::new();
    for (markers, name) in checks {
        if markers.iter().any(|marker| lower.contains(marker)) {
            out.push(DeployServiceSuggestion {
                name: (*name).to_string(),
                reason: "detected dependency marker".to_string(),
            });
        }
    }
    out
}

fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn first_file_with_extension(root: &Path, extension: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_node_services_and_ports() {
        let root = temp_root();
        std::fs::write(
            root.join("package.json"),
            r#"{"dependencies":{"vite":"latest","pg":"latest","redis":"latest"}}"#,
        )
        .expect("write package");
        std::fs::write(root.join("pnpm-lock.yaml"), "").expect("write lock");
        let project = store::Project {
            id: 1,
            workspace_id: 1,
            name: "web".to_string(),
            path: root.display().to_string(),
            remote_url: None,
            created_at: "now".to_string(),
        };
        let detected = detect_project(&project);
        assert_eq!(detected.language, "typescript");
        assert_eq!(detected.framework.as_deref(), Some("vite"));
        assert_eq!(detected.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(detected.deploy_strategy, "web_service");
        assert!(!detected.requires_desktop_session);
        assert!(detected
            .services
            .iter()
            .any(|service| service.name == "postgres"));
        assert!(detected
            .services
            .iter()
            .any(|service| service.name == "redis"));
        assert_eq!(detected.ports[0].host, 3000);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn detects_tauri_as_desktop_dev_package() {
        let root = temp_root();
        std::fs::create_dir_all(root.join("src-tauri")).expect("src-tauri");
        std::fs::write(
            root.join("src-tauri").join("Cargo.toml"),
            "[package]\nname='app'\n",
        )
        .expect("cargo");
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"dev":"tauri dev","check:js":"node --check src/main.js"},"devDependencies":{"@tauri-apps/cli":"^2.0.0"}}"#,
        )
        .expect("package");
        let project = store::Project {
            id: 1,
            workspace_id: 1,
            name: "desktop".to_string(),
            path: root.display().to_string(),
            remote_url: None,
            created_at: "now".to_string(),
        };
        let detected = detect_project(&project);

        assert_eq!(detected.language, "typescript");
        assert_eq!(detected.framework.as_deref(), Some("tauri"));
        assert_eq!(detected.deploy_strategy, "desktop_dev");
        assert!(detected.requires_desktop_session);
        assert!(detected.ports.is_empty());
        assert!(detected
            .runtime_commands
            .iter()
            .any(|command| command.contains("cargo metadata")));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stale_project_selection_returns_actionable_error() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");

        let error = detect_projects(&db, workspace.id, &[3])
            .expect_err("stale project should fail")
            .to_string();

        assert!(error.contains("deploy_project_selection_stale"));
        assert!(error.contains("project 3"));
        assert!(!error.contains("project not found"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn empty_workspace_selection_returns_actionable_error() {
        let (db, root) = temp_db();
        let workspace_root = root.join("workspace");
        let workspace = db
            .create_workspace("Workspace", &workspace_root.display().to_string())
            .expect("create workspace");

        let error = detect_projects(&db, workspace.id, &[])
            .expect_err("empty workspace should fail")
            .to_string();

        assert!(error.contains("deploy_project_selection_empty"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn temp_root() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-deploy-detect-{unique}"));
        std::fs::create_dir_all(&root).expect("mkdir");
        root
    }

    fn temp_db() -> (store::Database, std::path::PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dw-deploy-detect-db-{unique}"));
        std::fs::create_dir_all(&root).expect("mkdir");
        (store::Database::open(&root).expect("open db"), root)
    }
}
