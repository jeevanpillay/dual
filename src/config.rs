use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Dockerfile build configuration for building images from source.
/// Used when devcontainer.json specifies `build.dockerfile`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DockerfileBuild {
    /// Path to the Dockerfile (relative to base_dir or workspace root)
    pub path: String,

    /// Build context directory (relative to base_dir or workspace root)
    #[serde(default = "default_build_context")]
    pub context: String,

    /// Docker build arguments (--build-arg)
    #[serde(default)]
    pub args: HashMap<String, String>,

    /// Target build stage for multi-stage builds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Base directory for resolving relative paths (set by devcontainer loader).
    #[serde(skip)]
    pub base_dir: Option<PathBuf>,
}

fn default_build_context() -> String {
    ".".to_string()
}

const DUAL_DIR: &str = ".dual";
const SETTINGS_FILENAME: &str = "settings.json";
const DEFAULT_IMAGE: &str = "node:20";

/// Dual-specific orchestration config, read from .dual/settings.json.
///
/// Container configuration (image, ports, setup, env) lives in devcontainer.json.
/// This struct contains only Dual-specific fields that devcontainer can't express.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DualConfig {
    /// Path to devcontainer.json (required, set by dual init)
    pub devcontainer: String,

    /// Additional commands to route to the container (merged with defaults)
    #[serde(default)]
    pub extra_commands: Vec<String>,

    /// Directories to isolate with anonymous Docker volumes
    #[serde(default = "default_anonymous_volumes")]
    pub anonymous_volumes: Vec<String>,

    /// Shared files to propagate across workspaces
    #[serde(default)]
    pub shared: Vec<String>,
}

impl Default for DualConfig {
    fn default() -> Self {
        Self {
            devcontainer: ".devcontainer/devcontainer.json".to_string(),
            extra_commands: Vec::new(),
            anonymous_volumes: default_anonymous_volumes(),
            shared: Vec::new(),
        }
    }
}

/// Per-repo runtime hints — the merged internal representation used by all consumers.
///
/// Built from DualConfig (.dual/settings.json) + DevcontainerJson (devcontainer.json).
/// Consumer code (container, proxy, shell) uses this struct exclusively.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RepoHints {
    /// Docker image to use for containers (default: "node:20")
    #[serde(default = "default_image")]
    pub image: String,

    /// Ports that services bind to inside the container
    #[serde(default)]
    pub ports: Vec<u16>,

    /// Setup command to run after container creation (e.g. "pnpm install")
    pub setup: Option<String>,

    /// Environment variables for the container
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Additional commands to route to the container (merged with defaults)
    #[serde(default)]
    pub extra_commands: Vec<String>,

    /// Directories to isolate with anonymous Docker volumes
    #[serde(default = "default_anonymous_volumes")]
    pub anonymous_volumes: Vec<String>,

    /// Shared files to propagate across workspaces
    #[serde(default)]
    pub shared: Vec<String>,

    /// Dockerfile build config — if set, build image instead of pulling.
    /// Sourced from devcontainer.json build field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<DockerfileBuild>,
}

fn default_image() -> String {
    DEFAULT_IMAGE.to_string()
}

fn default_anonymous_volumes() -> Vec<String> {
    vec!["node_modules".to_string()]
}

impl Default for RepoHints {
    fn default() -> Self {
        Self {
            image: DEFAULT_IMAGE.to_string(),
            ports: Vec::new(),
            setup: None,
            env: HashMap::new(),
            extra_commands: Vec::new(),
            anonymous_volumes: default_anonymous_volumes(),
            shared: Vec::new(),
            dockerfile: None,
        }
    }
}

/// Get the shared config directory for a repo: ~/.dual/shared/{repo}/
pub fn shared_dir(repo: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".dual").join("shared").join(repo))
}

/// Load RepoHints from a workspace directory.
///
/// Loading flow:
/// 1. Read `.dual/settings.json` → `DualConfig` (error if missing)
/// 2. Use explicit devcontainer path from DualConfig
/// 3. Parse devcontainer.json → `RepoHints` for container fields
/// 4. Merge DualConfig fields + devcontainer RepoHints → final `RepoHints`
pub fn load_hints(workspace_dir: &Path) -> Result<RepoHints, HintsError> {
    let dual_config = load_dual_config(workspace_dir)?;
    let dc_path = workspace_dir.join(&dual_config.devcontainer);

    // Load container config from devcontainer.json
    let dc_hints = if dc_path.exists() {
        let devcontainer_dir = dc_path.parent().unwrap_or(workspace_dir);
        let contents = std::fs::read_to_string(&dc_path).ok();
        contents.and_then(|c| {
            let dc = crate::devcontainer::parse_devcontainer(&c).ok()?;
            Some(crate::devcontainer::to_repo_hints(&dc, devcontainer_dir))
        })
    } else {
        warn!(
            "devcontainer.json not found at '{}' (specified in .dual/settings.json)",
            dc_path.display()
        );
        None
    };

    Ok(merge_config(&dual_config, dc_hints.as_ref()))
}

/// Load DualConfig from .dual/settings.json. Returns error if file doesn't exist.
fn load_dual_config(workspace_dir: &Path) -> Result<DualConfig, HintsError> {
    let path = workspace_dir.join(DUAL_DIR).join(SETTINGS_FILENAME);

    if !path.exists() {
        return Err(HintsError::MissingConfig(workspace_dir.to_path_buf()));
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| HintsError::ReadError(path.clone(), e))?;

    let config: DualConfig =
        serde_json::from_str(&contents).map_err(|e| HintsError::JsonParseError(path, e))?;
    Ok(config)
}

/// Merge DualConfig + devcontainer RepoHints into final RepoHints.
///
/// devcontainer.json provides: image, ports, setup, env, dockerfile
/// DualConfig provides: extra_commands, anonymous_volumes, shared
fn merge_config(dual: &DualConfig, dc_hints: Option<&RepoHints>) -> RepoHints {
    let base = dc_hints.cloned().unwrap_or_default();

    RepoHints {
        // Container fields come from devcontainer.json (or defaults)
        image: base.image,
        ports: base.ports,
        setup: base.setup,
        env: base.env,
        dockerfile: base.dockerfile,

        // Dual-specific fields come from .dual/settings.json
        extra_commands: dual.extra_commands.clone(),
        anonymous_volumes: dual.anonymous_volumes.clone(),
        shared: dual.shared.clone(),
    }
}

/// Write DualConfig to a workspace directory's .dual/settings.json.
pub fn write_dual_config(workspace_dir: &Path, config: &DualConfig) -> Result<(), HintsError> {
    let dual_dir = workspace_dir.join(DUAL_DIR);
    std::fs::create_dir_all(&dual_dir).map_err(|e| HintsError::WriteError(dual_dir.clone(), e))?;

    let path = dual_dir.join(SETTINGS_FILENAME);
    let contents = serde_json::to_string_pretty(config).map_err(HintsError::JsonSerializeError)?;
    std::fs::write(&path, contents).map_err(|e| HintsError::WriteError(path, e))?;
    Ok(())
}

/// Parse DualConfig from JSON string (for testing).
pub fn parse_dual_config(json_str: &str) -> Result<DualConfig, HintsError> {
    let config: DualConfig = serde_json::from_str(json_str)
        .map_err(|e| HintsError::JsonParseError(PathBuf::from("<string>"), e))?;
    Ok(config)
}

/// Compute the workspace identifier from repo + branch.
/// e.g. ("lightfast", "feat/auth") → "lightfast-feat__auth"
pub fn workspace_id(repo: &str, branch: &str) -> String {
    format!("{}-{}", repo, encode_branch(branch))
}

/// Get the workspace directory for a repo + branch combination.
/// Layout: {workspace_root}/{repo}/{encoded_branch}/
pub fn workspace_dir(workspace_root: &Path, repo: &str, branch: &str) -> PathBuf {
    workspace_root.join(repo).join(encode_branch(branch))
}

/// Compute the container name for a repo + branch combination.
/// Pattern: dual-{repo}-{encoded_branch}
pub fn container_name(repo: &str, branch: &str) -> String {
    format!("dual-{}-{}", repo, encode_branch(branch))
}

/// Compute the tmux session name for a repo + branch combination.
/// Uses the same naming convention as container names for consistency.
pub fn session_name(repo: &str, branch: &str) -> String {
    container_name(repo, branch)
}

/// Encode a branch name for filesystem use.
/// Replaces `/` with `__` (double underscore).
/// e.g. "feat/auth" → "feat__auth"
pub fn encode_branch(branch: &str) -> String {
    branch.replace('/', "__")
}

/// Decode an encoded branch name back to the original.
/// Replaces `__` with `/`.
/// e.g. "feat__auth" → "feat/auth"
pub fn decode_branch(encoded: &str) -> String {
    encoded.replace("__", "/")
}

#[derive(Debug, thiserror::Error)]
pub enum HintsError {
    #[error("Failed to read {path}: {err}", path = .0.display(), err = .1)]
    ReadError(PathBuf, std::io::Error),

    #[error("Failed to write {path}: {err}", path = .0.display(), err = .1)]
    WriteError(PathBuf, std::io::Error),

    #[error("Failed to parse {path}: {err}", path = .0.display(), err = .1)]
    JsonParseError(PathBuf, serde_json::Error),

    #[error("Failed to serialize config: {0}")]
    JsonSerializeError(serde_json::Error),

    #[error("No .dual/settings.json found in {path}. Run `dual init` first.", path = .0.display())]
    MissingConfig(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_branch_with_slash() {
        assert_eq!(encode_branch("feat/auth"), "feat__auth");
        assert_eq!(encode_branch("fix/memory-leak"), "fix__memory-leak");
        assert_eq!(encode_branch("main"), "main");
    }

    #[test]
    fn decode_branch_roundtrip() {
        let original = "feat/auth";
        assert_eq!(decode_branch(&encode_branch(original)), original);
    }

    #[test]
    fn container_name_format() {
        assert_eq!(container_name("lightfast", "main"), "dual-lightfast-main");
        assert_eq!(
            container_name("lightfast", "feat/auth"),
            "dual-lightfast-feat__auth"
        );
    }

    #[test]
    fn workspace_dir_format() {
        let dir = workspace_dir(Path::new("/tmp/ws"), "lightfast", "feat/auth");
        assert_eq!(dir, PathBuf::from("/tmp/ws/lightfast/feat__auth"));
    }

    #[test]
    fn workspace_id_format() {
        assert_eq!(workspace_id("lightfast", "main"), "lightfast-main");
        assert_eq!(
            workspace_id("lightfast", "feat/auth"),
            "lightfast-feat__auth"
        );
    }

    #[test]
    fn default_hints() {
        let hints = RepoHints::default();
        assert_eq!(hints.image, "node:20");
        assert!(hints.ports.is_empty());
        assert!(hints.setup.is_none());
        assert!(hints.env.is_empty());
        assert!(hints.extra_commands.is_empty());
        assert_eq!(hints.anonymous_volumes, vec!["node_modules".to_string()]);
    }

    #[test]
    fn default_dual_config() {
        let config = DualConfig::default();
        assert_eq!(config.devcontainer, ".devcontainer/devcontainer.json");
        assert!(config.extra_commands.is_empty());
        assert_eq!(config.anonymous_volumes, vec!["node_modules".to_string()]);
        assert!(config.shared.is_empty());
    }

    #[test]
    fn parse_dual_config_minimal() {
        let json = r#"{"devcontainer": ".devcontainer/devcontainer.json"}"#;
        let config = parse_dual_config(json).unwrap();
        assert_eq!(config.devcontainer, ".devcontainer/devcontainer.json");
        assert!(config.extra_commands.is_empty());
        assert_eq!(config.anonymous_volumes, vec!["node_modules".to_string()]);
        assert!(config.shared.is_empty());
    }

    #[test]
    fn parse_dual_config_missing_devcontainer_errors() {
        let json = r#"{"extra_commands": ["cargo"]}"#;
        assert!(parse_dual_config(json).is_err());
    }

    #[test]
    fn parse_dual_config_full() {
        let json = r#"{
            "devcontainer": ".devcontainer/devcontainer.json",
            "extra_commands": ["cargo", "go"],
            "anonymous_volumes": ["node_modules", ".next", "target"],
            "shared": [".env.local", ".vercel"]
        }"#;
        let config = parse_dual_config(json).unwrap();
        assert_eq!(config.devcontainer, ".devcontainer/devcontainer.json");
        assert_eq!(config.extra_commands, vec!["cargo", "go"]);
        assert_eq!(
            config.anonymous_volumes,
            vec!["node_modules", ".next", "target"]
        );
        assert_eq!(config.shared, vec![".env.local", ".vercel"]);
    }

    #[test]
    fn parse_dual_config_extra_commands_only() {
        let json = r#"{"devcontainer": "dc.json", "extra_commands": ["cargo", "go", "ruby"]}"#;
        let config = parse_dual_config(json).unwrap();
        assert_eq!(config.extra_commands, vec!["cargo", "go", "ruby"]);
    }

    #[test]
    fn merge_config_devcontainer_only() {
        let dual = DualConfig::default();
        let dc_hints = RepoHints {
            image: "python:3.12".to_string(),
            ports: vec![3000, 8080],
            setup: Some("pnpm install".to_string()),
            env: HashMap::from([("NODE_ENV".to_string(), "development".to_string())]),
            ..Default::default()
        };

        let merged = merge_config(&dual, Some(&dc_hints));
        assert_eq!(merged.image, "python:3.12");
        assert_eq!(merged.ports, vec![3000, 8080]);
        assert_eq!(merged.setup.as_deref(), Some("pnpm install"));
        assert_eq!(merged.env.get("NODE_ENV").unwrap(), "development");
        // Dual defaults
        assert!(merged.extra_commands.is_empty());
        assert_eq!(merged.anonymous_volumes, vec!["node_modules".to_string()]);
    }

    #[test]
    fn merge_config_dual_only() {
        let dual = DualConfig {
            devcontainer: ".devcontainer/devcontainer.json".to_string(),
            extra_commands: vec!["cargo".to_string()],
            anonymous_volumes: vec!["node_modules".to_string(), "target".to_string()],
            shared: vec![".env.local".to_string()],
        };

        let merged = merge_config(&dual, None);
        // Container defaults
        assert_eq!(merged.image, "node:20");
        assert!(merged.ports.is_empty());
        assert!(merged.setup.is_none());
        // Dual fields
        assert_eq!(merged.extra_commands, vec!["cargo"]);
        assert_eq!(merged.anonymous_volumes, vec!["node_modules", "target"]);
        assert_eq!(merged.shared, vec![".env.local"]);
    }

    #[test]
    fn merge_config_both_sources() {
        let dual = DualConfig {
            devcontainer: ".devcontainer/devcontainer.json".to_string(),
            extra_commands: vec!["cargo".to_string()],
            anonymous_volumes: vec!["node_modules".to_string(), "target".to_string()],
            shared: vec![".env".to_string()],
        };
        let dc_hints = RepoHints {
            image: "rust:latest".to_string(),
            ports: vec![8080],
            setup: Some("cargo build".to_string()),
            env: HashMap::from([("RUST_LOG".to_string(), "debug".to_string())]),
            ..Default::default()
        };

        let merged = merge_config(&dual, Some(&dc_hints));
        // Container fields from devcontainer
        assert_eq!(merged.image, "rust:latest");
        assert_eq!(merged.ports, vec![8080]);
        assert_eq!(merged.setup.as_deref(), Some("cargo build"));
        assert_eq!(merged.env.get("RUST_LOG").unwrap(), "debug");
        // Dual fields from .dual/settings.json
        assert_eq!(merged.extra_commands, vec!["cargo"]);
        assert_eq!(merged.anonymous_volumes, vec!["node_modules", "target"]);
        assert_eq!(merged.shared, vec![".env"]);
    }

    #[test]
    fn merge_config_neither_source() {
        let dual = DualConfig::default();
        let merged = merge_config(&dual, None);
        assert_eq!(merged, RepoHints::default());
    }

    #[test]
    fn load_hints_from_missing_dir_errors() {
        let result = load_hints(Path::new("/tmp/dual-test-nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn load_hints_with_settings_and_devcontainer() {
        let dir = std::env::temp_dir().join("dual-test-dc-settings");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "python:3.12", "forwardPorts": [5000]}"#,
        )
        .unwrap();

        // Must also have .dual/settings.json
        let config = DualConfig {
            devcontainer: ".devcontainer/devcontainer.json".to_string(),
            ..Default::default()
        };
        write_dual_config(&dir, &config).unwrap();

        let hints = load_hints(&dir).unwrap();
        assert_eq!(hints.image, "python:3.12");
        assert_eq!(hints.ports, vec![5000]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_hints_dual_config_plus_devcontainer() {
        let dir = std::env::temp_dir().join("dual-test-merged");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();

        // Write devcontainer.json with container config
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "node:20", "forwardPorts": [3000], "postCreateCommand": "pnpm install"}"#,
        )
        .unwrap();

        // Write .dual/settings.json with orchestration config
        let dual_config = DualConfig {
            devcontainer: ".devcontainer/devcontainer.json".to_string(),
            extra_commands: vec!["cargo".to_string()],
            anonymous_volumes: vec!["node_modules".to_string(), ".next".to_string()],
            shared: vec![".env.local".to_string()],
        };
        write_dual_config(&dir, &dual_config).unwrap();

        let hints = load_hints(&dir).unwrap();
        // From devcontainer.json
        assert_eq!(hints.image, "node:20");
        assert_eq!(hints.ports, vec![3000]);
        assert_eq!(hints.setup.as_deref(), Some("pnpm install"));
        // From .dual/settings.json
        assert_eq!(hints.extra_commands, vec!["cargo"]);
        assert_eq!(hints.anonymous_volumes, vec!["node_modules", ".next"]);
        assert_eq!(hints.shared, vec![".env.local"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_hints_explicit_devcontainer_path() {
        let dir = std::env::temp_dir().join("dual-test-explicit-dc");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("custom")).unwrap();

        std::fs::write(
            dir.join("custom").join("devcontainer.json"),
            r#"{"image": "alpine:latest"}"#,
        )
        .unwrap();

        let dual_config = DualConfig {
            devcontainer: "custom/devcontainer.json".to_string(),
            ..Default::default()
        };
        write_dual_config(&dir, &dual_config).unwrap();

        let hints = load_hints(&dir).unwrap();
        assert_eq!(hints.image, "alpine:latest");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_and_load_dual_config_roundtrip() {
        let dir = std::env::temp_dir().join("dual-test-dual-config-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".devcontainer")).unwrap();

        // Write devcontainer.json
        std::fs::write(
            dir.join(".devcontainer").join("devcontainer.json"),
            r#"{"image": "rust:latest", "forwardPorts": [8080], "postCreateCommand": "cargo build", "containerEnv": {"RUST_LOG": "debug"}}"#,
        )
        .unwrap();

        // Write DualConfig
        let config = DualConfig {
            devcontainer: ".devcontainer/devcontainer.json".to_string(),
            extra_commands: vec!["cargo".to_string()],
            anonymous_volumes: vec!["node_modules".to_string(), "target".to_string()],
            shared: Vec::new(),
        };
        write_dual_config(&dir, &config).unwrap();

        // Load and verify merged result
        let hints = load_hints(&dir).unwrap();
        assert_eq!(hints.image, "rust:latest");
        assert_eq!(hints.ports, vec![8080]);
        assert_eq!(hints.setup.as_deref(), Some("cargo build"));
        assert_eq!(hints.env.get("RUST_LOG").unwrap(), "debug");
        assert_eq!(hints.extra_commands, vec!["cargo"]);
        assert_eq!(hints.anonymous_volumes, vec!["node_modules", "target"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_dual_config_without_shared_has_empty_array() {
        let config = DualConfig::default();
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        assert!(json_str.contains("\"shared\": []"));
    }

    #[test]
    fn write_dual_config_with_shared_includes_files() {
        let config = DualConfig {
            shared: vec![".env".to_string()],
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        assert!(json_str.contains("\"shared\""));
        assert!(json_str.contains(".env"));
    }

    #[test]
    fn session_name_format() {
        assert_eq!(session_name("lightfast", "main"), "dual-lightfast-main");
        assert_eq!(
            session_name("lightfast", "feat/auth"),
            "dual-lightfast-feat__auth"
        );
        assert_eq!(
            session_name("agent-os", "v2-rewrite"),
            "dual-agent-os-v2-rewrite"
        );
    }

    #[test]
    fn session_name_matches_container_name() {
        assert_eq!(
            session_name("lightfast", "main"),
            container_name("lightfast", "main")
        );
        assert_eq!(
            session_name("lightfast", "feat/auth"),
            container_name("lightfast", "feat/auth")
        );
    }
}
