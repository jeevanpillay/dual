use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::RepoHints;

/// Subset of devcontainer.json fields that Dual consumes.
/// See: https://containers.dev/implementors/json_reference/
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevcontainerJson {
    /// Docker image to use (mutually exclusive with `build`)
    pub image: Option<String>,

    /// Build configuration for Dockerfile-based images
    pub build: Option<BuildConfig>,

    /// Ports to forward from the container
    pub forward_ports: Option<Vec<PortSpec>>,

    /// Environment variables for the container
    pub container_env: Option<HashMap<String, String>>,

    /// Command to run after container creation
    pub post_create_command: Option<CommandSpec>,

    /// Mount configurations
    pub mounts: Option<Vec<MountSpec>>,
}

/// Build configuration from devcontainer.json.
#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    /// Path to Dockerfile (relative to devcontainer.json location)
    pub dockerfile: Option<String>,

    /// Build context path (relative to devcontainer.json location)
    pub context: Option<String>,

    /// Docker build arguments
    pub args: Option<HashMap<String, String>>,

    /// Build target stage for multi-stage builds
    pub target: Option<String>,
}

/// Port specification — devcontainer allows integers or strings.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PortSpec {
    Number(u16),
    String(String),
}

impl PortSpec {
    /// Convert to u16, parsing strings as integers.
    pub fn to_port(&self) -> Option<u16> {
        match self {
            PortSpec::Number(n) => Some(*n),
            PortSpec::String(s) => s.parse::<u16>().ok(),
        }
    }
}

/// Command specification — devcontainer allows string, array, or object forms.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CommandSpec {
    /// Single shell command: "pnpm install"
    String(String),
    /// Exec form: ["pnpm", "install"]
    Array(Vec<String>),
    /// Parallel commands: {"install": "pnpm install", "build": "pnpm build"}
    Object(HashMap<String, StringOrArray>),
}

impl CommandSpec {
    /// Flatten to a single shell command string.
    /// - String: returned as-is
    /// - Array: joined with spaces
    /// - Object: values joined with " && "
    pub fn to_shell_command(&self) -> String {
        match self {
            CommandSpec::String(s) => s.clone(),
            CommandSpec::Array(arr) => arr.join(" "),
            CommandSpec::Object(map) => {
                let commands: Vec<String> = map
                    .values()
                    .map(|v| match v {
                        StringOrArray::String(s) => s.clone(),
                        StringOrArray::Array(arr) => arr.join(" "),
                    })
                    .collect();
                commands.join(" && ")
            }
        }
    }
}

/// Value in a command object — can be string or array.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrArray {
    String(String),
    Array(Vec<String>),
}

/// Mount specification — devcontainer allows string or object forms.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MountSpec {
    /// Docker mount syntax: "type=volume,target=/workspace/node_modules"
    String(String),
    /// Structured mount object
    Object(MountObject),
}

/// Structured mount configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountObject {
    #[serde(rename = "type")]
    pub mount_type: Option<String>,
    pub source: Option<String>,
    pub target: String,
}

const WORKSPACE_PREFIX: &str = "/workspace/";

impl MountSpec {
    /// Extract anonymous volume path if this is a volume mount targeting /workspace/*.
    /// Returns the relative path (e.g., "node_modules" from "/workspace/node_modules").
    pub fn as_anonymous_volume(&self) -> Option<String> {
        match self {
            MountSpec::String(s) => {
                // Parse "type=volume,target=/workspace/node_modules" format
                let mut mount_type = None;
                let mut target = None;
                let mut has_source = false;
                for part in s.split(',') {
                    let (key, value) = part.split_once('=')?;
                    match key.trim() {
                        "type" => mount_type = Some(value.trim()),
                        "target" | "dst" | "destination" => target = Some(value.trim()),
                        "source" | "src" => has_source = true,
                        _ => {}
                    }
                }
                if mount_type == Some("volume") && !has_source {
                    target
                        .filter(|t| t.starts_with(WORKSPACE_PREFIX))
                        .map(|t| t[WORKSPACE_PREFIX.len()..].to_string())
                } else {
                    None
                }
            }
            MountSpec::Object(obj) => {
                if obj.mount_type.as_deref() == Some("volume")
                    && obj.source.is_none()
                    && obj.target.starts_with(WORKSPACE_PREFIX)
                {
                    return Some(obj.target[WORKSPACE_PREFIX.len()..].to_string());
                }
                None
            }
        }
    }
}

/// Resolve the path to devcontainer.json in a workspace directory.
///
/// Checks in order:
/// 1. `.devcontainer/devcontainer.json`
/// 2. `.devcontainer.json` (root)
///
/// Returns None if neither exists.
pub fn find_devcontainer_json(workspace_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        workspace_dir
            .join(".devcontainer")
            .join("devcontainer.json"),
        workspace_dir.join(".devcontainer.json"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

/// Parse a devcontainer.json string into DevcontainerJson.
pub fn parse_devcontainer(json_str: &str) -> Result<DevcontainerJson, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Load and parse devcontainer.json from a workspace directory.
/// Returns None if no devcontainer.json exists.
pub fn load_devcontainer(workspace_dir: &Path) -> Option<DevcontainerJson> {
    let path = find_devcontainer_json(workspace_dir)?;
    let contents = std::fs::read_to_string(&path).ok()?;
    parse_devcontainer(&contents).ok()
}

/// Convert DevcontainerJson fields to RepoHints.
///
/// Maps:
/// - `image` → `hints.image`
/// - `build` → `hints.dockerfile`
/// - `forwardPorts` → `hints.ports`
/// - `containerEnv` → `hints.env`
/// - `postCreateCommand` → `hints.setup`
/// - `mounts` (volume type, /workspace/* target) → `hints.anonymous_volumes`
pub fn to_repo_hints(dc: &DevcontainerJson, devcontainer_dir: &Path) -> RepoHints {
    let mut hints = RepoHints::default();

    // Image (mutually exclusive with build)
    if let Some(ref image) = dc.image {
        hints.image = image.clone();
    }

    // Build → DockerfileBuild
    if let Some(ref build) = dc.build
        && build.dockerfile.is_some()
    {
        hints.dockerfile = Some(crate::config::DockerfileBuild {
            path: build
                .dockerfile
                .clone()
                .unwrap_or_else(|| "Dockerfile".to_string()),
            context: build.context.clone().unwrap_or_else(|| ".".to_string()),
            args: build.args.clone().unwrap_or_default(),
            target: build.target.clone(),
            base_dir: Some(devcontainer_dir.to_path_buf()),
        });
    }

    // Ports
    if let Some(ref ports) = dc.forward_ports {
        hints.ports = ports.iter().filter_map(|p| p.to_port()).collect();
    }

    // Environment variables
    if let Some(ref env) = dc.container_env {
        hints.env = env.clone();
    }

    // Setup command
    if let Some(ref cmd) = dc.post_create_command {
        let shell_cmd = cmd.to_shell_command();
        if !shell_cmd.is_empty() {
            hints.setup = Some(shell_cmd);
        }
    }

    // Anonymous volumes from mounts
    if let Some(ref mounts) = dc.mounts {
        let extra_volumes: Vec<String> = mounts
            .iter()
            .filter_map(|m| m.as_anonymous_volume())
            .collect();
        if !extra_volumes.is_empty() {
            for vol in extra_volumes {
                if !hints.anonymous_volumes.contains(&vol) {
                    hints.anonymous_volumes.push(vol);
                }
            }
        }
    }

    hints
}

/// Load devcontainer.json from a workspace and convert to RepoHints.
/// Returns None if no devcontainer.json exists or it fails to parse.
pub fn load_devcontainer_as_hints(workspace_dir: &Path) -> Option<RepoHints> {
    let path = find_devcontainer_json(workspace_dir)?;
    let devcontainer_dir = path.parent().unwrap_or(workspace_dir);
    let contents = std::fs::read_to_string(&path).ok()?;
    let dc = parse_devcontainer(&contents).ok()?;
    Some(to_repo_hints(&dc, devcontainer_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_image() {
        let json = r#"{"image": "node:20"}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.image.as_deref(), Some("node:20"));
        assert!(dc.build.is_none());
    }

    #[test]
    fn parse_with_build() {
        let json = r#"{
            "build": {
                "dockerfile": "Dockerfile",
                "context": "..",
                "args": {"NODE_VERSION": "20"},
                "target": "development"
            }
        }"#;
        let dc = parse_devcontainer(json).unwrap();
        assert!(dc.image.is_none());
        let build = dc.build.unwrap();
        assert_eq!(build.dockerfile.as_deref(), Some("Dockerfile"));
        assert_eq!(build.context.as_deref(), Some(".."));
        assert_eq!(build.args.unwrap().get("NODE_VERSION").unwrap(), "20");
        assert_eq!(build.target.as_deref(), Some("development"));
    }

    #[test]
    fn parse_forward_ports_integers() {
        let json = r#"{"forwardPorts": [3000, 8080]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc
            .forward_ports
            .unwrap()
            .iter()
            .filter_map(|p| p.to_port())
            .collect();
        assert_eq!(ports, vec![3000, 8080]);
    }

    #[test]
    fn parse_forward_ports_strings() {
        let json = r#"{"forwardPorts": ["3000", "8080"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc
            .forward_ports
            .unwrap()
            .iter()
            .filter_map(|p| p.to_port())
            .collect();
        assert_eq!(ports, vec![3000, 8080]);
    }

    #[test]
    fn parse_forward_ports_mixed() {
        let json = r#"{"forwardPorts": [3000, "8080"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc
            .forward_ports
            .unwrap()
            .iter()
            .filter_map(|p| p.to_port())
            .collect();
        assert_eq!(ports, vec![3000, 8080]);
    }

    #[test]
    fn parse_forward_ports_invalid_string_skipped() {
        let json = r#"{"forwardPorts": [3000, "not-a-port"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        let ports: Vec<u16> = dc
            .forward_ports
            .unwrap()
            .iter()
            .filter_map(|p| p.to_port())
            .collect();
        assert_eq!(ports, vec![3000]);
    }

    #[test]
    fn parse_container_env() {
        let json = r#"{"containerEnv": {"NODE_ENV": "development", "DEBUG": "true"}}"#;
        let dc = parse_devcontainer(json).unwrap();
        let env = dc.container_env.unwrap();
        assert_eq!(env.get("NODE_ENV").unwrap(), "development");
        assert_eq!(env.get("DEBUG").unwrap(), "true");
    }

    #[test]
    fn parse_post_create_command_string() {
        let json = r#"{"postCreateCommand": "pnpm install"}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(
            dc.post_create_command.unwrap().to_shell_command(),
            "pnpm install"
        );
    }

    #[test]
    fn parse_post_create_command_array() {
        let json = r#"{"postCreateCommand": ["pnpm", "install"]}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(
            dc.post_create_command.unwrap().to_shell_command(),
            "pnpm install"
        );
    }

    #[test]
    fn parse_post_create_command_object() {
        let json = r#"{"postCreateCommand": {"install": "pnpm install", "build": "pnpm build"}}"#;
        let dc = parse_devcontainer(json).unwrap();
        let cmd = dc.post_create_command.unwrap().to_shell_command();
        // Object order is non-deterministic, but both commands should be present
        assert!(cmd.contains("pnpm install"));
        assert!(cmd.contains("pnpm build"));
        assert!(cmd.contains(" && "));
    }

    #[test]
    fn parse_post_create_command_object_with_array_value() {
        let json = r#"{"postCreateCommand": {"install": ["pnpm", "install"]}}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(
            dc.post_create_command.unwrap().to_shell_command(),
            "pnpm install"
        );
    }

    #[test]
    fn parse_mount_object_volume() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("volume".to_string()),
            source: None,
            target: "/workspace/node_modules".to_string(),
        });
        assert_eq!(
            mount.as_anonymous_volume(),
            Some("node_modules".to_string())
        );
    }

    #[test]
    fn parse_mount_object_with_source_not_anonymous() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("volume".to_string()),
            source: Some("my-vol".to_string()),
            target: "/workspace/node_modules".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), None);
    }

    #[test]
    fn parse_mount_object_bind_not_anonymous() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("bind".to_string()),
            source: None,
            target: "/workspace/node_modules".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), None);
    }

    #[test]
    fn parse_mount_object_non_workspace_target() {
        let mount = MountSpec::Object(MountObject {
            mount_type: Some("volume".to_string()),
            source: None,
            target: "/data/cache".to_string(),
        });
        assert_eq!(mount.as_anonymous_volume(), None);
    }

    #[test]
    fn parse_unknown_fields_ignored() {
        let json = r#"{
            "image": "node:20",
            "customizations": {"vscode": {"extensions": ["ms-python.python"]}},
            "remoteUser": "vscode",
            "features": {"ghcr.io/devcontainers/features/node:1": {}}
        }"#;
        let dc = parse_devcontainer(json).unwrap();
        assert_eq!(dc.image.as_deref(), Some("node:20"));
    }

    #[test]
    fn parse_empty_object() {
        let json = r#"{}"#;
        let dc = parse_devcontainer(json).unwrap();
        assert!(dc.image.is_none());
        assert!(dc.build.is_none());
        assert!(dc.forward_ports.is_none());
    }

    #[test]
    fn to_repo_hints_image_only() {
        let dc = DevcontainerJson {
            image: Some("python:3.12".to_string()),
            build: None,
            forward_ports: None,
            container_env: None,
            post_create_command: None,
            mounts: None,
        };
        let hints = to_repo_hints(&dc, Path::new("."));
        assert_eq!(hints.image, "python:3.12");
    }

    #[test]
    fn to_repo_hints_full() {
        let dc = DevcontainerJson {
            image: Some("node:20".to_string()),
            build: None,
            forward_ports: Some(vec![
                PortSpec::Number(3000),
                PortSpec::String("8080".to_string()),
            ]),
            container_env: Some(HashMap::from([("NODE_ENV".to_string(), "dev".to_string())])),
            post_create_command: Some(CommandSpec::String("pnpm install".to_string())),
            mounts: None,
        };
        let hints = to_repo_hints(&dc, Path::new("."));
        assert_eq!(hints.image, "node:20");
        assert_eq!(hints.ports, vec![3000, 8080]);
        assert_eq!(hints.env.get("NODE_ENV").unwrap(), "dev");
        assert_eq!(hints.setup.as_deref(), Some("pnpm install"));
    }

    #[test]
    fn find_devcontainer_json_in_subdir() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-find");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "node:20"}"#,
        )
        .unwrap();

        let found = find_devcontainer_json(&dir);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".devcontainer/devcontainer.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_devcontainer_json_at_root() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-root");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".devcontainer.json"), r#"{"image": "node:20"}"#).unwrap();

        let found = find_devcontainer_json(&dir);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".devcontainer.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_devcontainer_json_missing() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert!(find_devcontainer_json(&dir).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_devcontainer_prefers_subdir_over_root() {
        let dir = std::env::temp_dir().join("dual-test-devcontainer-prefer");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "python:3.12"}"#,
        )
        .unwrap();
        std::fs::write(dir.join(".devcontainer.json"), r#"{"image": "node:20"}"#).unwrap();

        let found = find_devcontainer_json(&dir).unwrap();
        assert!(found.ends_with(".devcontainer/devcontainer.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
