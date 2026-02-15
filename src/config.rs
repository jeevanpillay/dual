use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const HINTS_FILENAME: &str = ".dual.toml";
const DEFAULT_IMAGE: &str = "node:20";

/// Shared configuration file propagation settings.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SharedConfig {
    /// Files and directories to share across workspaces.
    /// e.g. [".vercel", ".env.local", ".env"]
    #[serde(default)]
    pub files: Vec<String>,
}

/// Per-repo runtime hints, read from .dual.toml in a workspace directory.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<SharedConfig>,
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
            shared: None,
        }
    }
}

/// Get the shared config directory for a repo: ~/.dual/shared/{repo}/
pub fn shared_dir(repo: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".dual").join("shared").join(repo))
}

/// Load RepoHints from a workspace directory's .dual.toml.
/// Returns default hints if the file doesn't exist.
pub fn load_hints(workspace_dir: &Path) -> Result<RepoHints, HintsError> {
    let path = workspace_dir.join(HINTS_FILENAME);

    if !path.exists() {
        return Ok(RepoHints::default());
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| HintsError::ReadError(path.clone(), e))?;
    let hints: RepoHints =
        toml::from_str(&contents).map_err(|e| HintsError::ParseError(path, e))?;
    Ok(hints)
}

/// Write RepoHints to a workspace directory's .dual.toml.
pub fn write_hints(workspace_dir: &Path, hints: &RepoHints) -> Result<(), HintsError> {
    let path = workspace_dir.join(HINTS_FILENAME);
    let contents = toml::to_string_pretty(hints).map_err(HintsError::SerializeError)?;
    std::fs::write(&path, contents).map_err(|e| HintsError::WriteError(path, e))?;
    Ok(())
}

/// Write a default .dual.toml with helpful comments and examples.
/// Used by `dual add` when creating a new repo config.
pub fn write_default_hints(repo_root: &Path) -> Result<(), HintsError> {
    let template = r#"# Dual workspace configuration
# See: https://github.com/jeevanpillay/dual

# Docker image for the container runtime
image = "node:20"

# Ports your dev server uses (for reverse proxy routing)
# Example: ports = [3000, 3001]
# ports = []

# Shell command to run after container creation (e.g., dependency install)
# Example: setup = "pnpm install"
# setup = ""

# Environment variables passed to the container
# Example:
# [env]
# NODE_ENV = "development"

# Commands to route to the container (in addition to defaults)
# Default: npm, npx, pnpm, node, python, python3, pip, pip3, curl, make
# Example: extra_commands = ["cargo", "go", "ruby"]
# extra_commands = []

# Directories to isolate with anonymous Docker volumes
# These directories get their own volume so they don't sync between host/container
# Example: anonymous_volumes = ["node_modules", ".next", "target"]
# anonymous_volumes = ["node_modules"]

# Files to share across all workspaces of this repo
# These are gitignored files that should be available in every branch workspace
# [shared]
# files = [".env.local", ".vercel"]
"#;
    let hints_path = repo_root.join(HINTS_FILENAME);
    std::fs::write(&hints_path, template).map_err(|e| HintsError::WriteError(hints_path, e))?;
    Ok(())
}

/// Parse hints from TOML string (for testing).
pub fn parse_hints(toml_str: &str) -> Result<RepoHints, HintsError> {
    let hints: RepoHints = toml::from_str(toml_str)
        .map_err(|e| HintsError::ParseError(PathBuf::from("<string>"), e))?;
    Ok(hints)
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
    ParseError(PathBuf, toml::de::Error),

    #[error("Failed to serialize hints: {0}")]
    SerializeError(toml::ser::Error),
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
    fn parse_hints_minimal() {
        let hints = parse_hints("").unwrap();
        assert_eq!(hints.image, "node:20");
        assert!(hints.ports.is_empty());
    }

    #[test]
    fn parse_hints_full() {
        let toml = r#"
image = "python:3.12"
ports = [3000, 3001]
setup = "pnpm install"

[env]
NODE_ENV = "development"
"#;
        let hints = parse_hints(toml).unwrap();
        assert_eq!(hints.image, "python:3.12");
        assert_eq!(hints.ports, vec![3000, 3001]);
        assert_eq!(hints.setup.as_deref(), Some("pnpm install"));
        assert_eq!(hints.env.get("NODE_ENV").unwrap(), "development");
    }

    #[test]
    fn parse_hints_missing_fields_use_defaults() {
        let toml = r#"ports = [8080]"#;
        let hints = parse_hints(toml).unwrap();
        assert_eq!(hints.image, "node:20");
        assert_eq!(hints.ports, vec![8080]);
        assert!(hints.setup.is_none());
    }

    #[test]
    fn load_hints_from_missing_file() {
        let hints = load_hints(Path::new("/tmp/dual-test-nonexistent")).unwrap();
        assert_eq!(hints, RepoHints::default());
    }

    #[test]
    fn write_and_load_hints() {
        let dir = std::env::temp_dir().join("dual-test-hints-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let hints = RepoHints {
            image: "rust:latest".to_string(),
            ports: vec![8080, 9090],
            setup: Some("cargo build".to_string()),
            env: HashMap::from([("RUST_LOG".to_string(), "debug".to_string())]),
            extra_commands: vec!["cargo".to_string()],
            anonymous_volumes: vec!["node_modules".to_string(), "target".to_string()],
            shared: None,
        };

        write_hints(&dir, &hints).unwrap();
        let loaded = load_hints(&dir).unwrap();
        assert_eq!(hints, loaded);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_default_hints_has_comments() {
        let dir = std::env::temp_dir().join("dual-test-default-hints");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        write_default_hints(&dir).unwrap();

        let content = std::fs::read_to_string(dir.join(".dual.toml")).unwrap();
        assert!(content.contains("# Dual workspace configuration"));
        assert!(content.contains("image = \"node:20\""));
        assert!(content.contains("# ports = []"));
        assert!(content.contains("# setup = \"\""));
        assert!(content.contains("# [env]"));
        assert!(content.contains("# extra_commands = []"));
        assert!(content.contains("# anonymous_volumes = [\"node_modules\"]"));
        assert!(content.contains("# [shared]"));

        // Verify it's still parseable as valid TOML
        let hints = load_hints(&dir).unwrap();
        assert_eq!(hints.image, "node:20");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_hints_unknown_fields_ignored() {
        let toml = r#"
image = "node:20"
ports = [3000]
unknown_field = "should be ignored"
"#;
        // serde by default ignores unknown fields
        let hints = parse_hints(toml).unwrap();
        assert_eq!(hints.image, "node:20");
    }

    #[test]
    fn parse_hints_extra_commands() {
        let toml = r#"
extra_commands = ["cargo", "go", "ruby"]
"#;
        let hints = parse_hints(toml).unwrap();
        assert_eq!(hints.extra_commands, vec!["cargo", "go", "ruby"]);
    }

    #[test]
    fn parse_hints_anonymous_volumes() {
        let toml = r#"
anonymous_volumes = ["node_modules", ".next", "target"]
"#;
        let hints = parse_hints(toml).unwrap();
        assert_eq!(
            hints.anonymous_volumes,
            vec!["node_modules", ".next", "target"]
        );
    }

    #[test]
    fn parse_hints_anonymous_volumes_default() {
        let hints = parse_hints("").unwrap();
        assert_eq!(hints.anonymous_volumes, vec!["node_modules".to_string()]);
    }

    #[test]
    fn parse_hints_with_shared() {
        let toml = r#"
image = "node:20"

[shared]
files = [".vercel", ".env.local"]
"#;
        let hints = parse_hints(toml).unwrap();
        let shared = hints.shared.unwrap();
        assert_eq!(shared.files, vec![".vercel", ".env.local"]);
    }

    #[test]
    fn parse_hints_without_shared() {
        let hints = parse_hints("image = \"node:20\"").unwrap();
        assert!(hints.shared.is_none());
    }

    #[test]
    fn parse_hints_shared_empty_files() {
        let toml = r#"
[shared]
files = []
"#;
        let hints = parse_hints(toml).unwrap();
        let shared = hints.shared.unwrap();
        assert!(shared.files.is_empty());
    }

    #[test]
    fn write_hints_without_shared_omits_section() {
        let hints = RepoHints::default();
        let toml_str = toml::to_string_pretty(&hints).unwrap();
        assert!(!toml_str.contains("[shared]"));
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

    #[test]
    fn write_hints_with_shared_includes_section() {
        let hints = RepoHints {
            shared: Some(SharedConfig {
                files: vec![".env".to_string()],
            }),
            ..Default::default()
        };
        let toml_str = toml::to_string_pretty(&hints).unwrap();
        assert!(toml_str.contains("[shared]"));
        assert!(toml_str.contains(".env"));
    }
}
