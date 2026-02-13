use serde::Deserialize;
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = "dual.toml";
const DEFAULT_WORKSPACE_ROOT: &str = "dual-workspaces";

#[derive(Debug, Deserialize, PartialEq)]
pub struct DualConfig {
    /// Root directory for all workspaces (default: ~/dual-workspaces)
    pub workspace_root: Option<String>,

    /// Repository definitions
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RepoConfig {
    /// Short name for the repo (e.g. "lightfast-platform")
    pub name: String,

    /// Git URL or local path to clone from
    pub url: String,

    /// Branches to create workspaces for
    #[serde(default)]
    pub branches: Vec<String>,
}

impl DualConfig {
    /// Resolve the workspace root directory as an absolute path.
    /// Uses the configured value or defaults to ~/dual-workspaces.
    pub fn workspace_root(&self) -> PathBuf {
        if let Some(ref root) = self.workspace_root {
            let expanded = shellexpand(root);
            PathBuf::from(expanded)
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(DEFAULT_WORKSPACE_ROOT)
        }
    }

    /// Get the workspace directory for a repo + branch combination.
    /// Layout: {workspace_root}/{repo}/{encoded_branch}/
    pub fn workspace_dir(&self, repo: &str, branch: &str) -> PathBuf {
        self.workspace_root().join(repo).join(encode_branch(branch))
    }

    /// Get the container name for a repo + branch combination.
    /// Pattern: dual-{repo}-{encoded_branch}
    pub fn container_name(repo: &str, branch: &str) -> String {
        format!("dual-{}-{}", repo, encode_branch(branch))
    }
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

/// Discover and load the config file.
/// Search order: current directory, then ~/.config/dual/
pub fn load() -> Result<DualConfig, ConfigError> {
    let paths = discovery_paths();

    for path in &paths {
        if path.exists() {
            let contents = std::fs::read_to_string(path)
                .map_err(|e| ConfigError::ReadError(path.clone(), e))?;
            let config: DualConfig =
                toml::from_str(&contents).map_err(|e| ConfigError::ParseError(path.clone(), e))?;
            validate(&config)?;
            return Ok(config);
        }
    }

    Err(ConfigError::NotFound(paths))
}

/// Load config from a specific path.
pub fn load_from(path: &Path) -> Result<DualConfig, ConfigError> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError(path.to_path_buf(), e))?;
    let config: DualConfig =
        toml::from_str(&contents).map_err(|e| ConfigError::ParseError(path.to_path_buf(), e))?;
    validate(&config)?;
    Ok(config)
}

/// Parse config from a TOML string (useful for testing).
pub fn parse(toml_str: &str) -> Result<DualConfig, ConfigError> {
    let config: DualConfig = toml::from_str(toml_str)
        .map_err(|e| ConfigError::ParseError(PathBuf::from("<string>"), e))?;
    validate(&config)?;
    Ok(config)
}

fn validate(config: &DualConfig) -> Result<(), ConfigError> {
    for (i, repo) in config.repos.iter().enumerate() {
        if repo.name.is_empty() {
            return Err(ConfigError::Validation(format!(
                "repos[{i}]: 'name' cannot be empty"
            )));
        }
        if repo.url.is_empty() {
            return Err(ConfigError::Validation(format!(
                "repos[{i}] ({}): 'url' cannot be empty",
                repo.name
            )));
        }
    }
    Ok(())
}

fn discovery_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Current directory
    paths.push(PathBuf::from(CONFIG_FILENAME));

    // ~/.config/dual/dual.toml
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("dual").join(CONFIG_FILENAME));
    }

    paths
}

/// Minimal ~ expansion for workspace_root paths.
fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest).to_string_lossy().into_owned();
    }
    path.to_string()
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound(Vec<PathBuf>),
    ReadError(PathBuf, std::io::Error),
    ParseError(PathBuf, toml::de::Error),
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NotFound(paths) => {
                write!(f, "No dual.toml found. Searched:")?;
                for p in paths {
                    write!(f, "\n  - {}", p.display())?;
                }
                Ok(())
            }
            ConfigError::ReadError(path, err) => {
                write!(f, "Failed to read {}: {err}", path.display())
            }
            ConfigError::ParseError(path, err) => {
                write!(f, "Failed to parse {}: {err}", path.display())
            }
            ConfigError::Validation(msg) => write!(f, "Invalid config: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let config = parse("").unwrap();
        assert!(config.repos.is_empty());
        assert!(config.workspace_root.is_none());
    }

    #[test]
    fn parse_full_config() {
        let toml = r#"
workspace_root = "~/my-workspaces"

[[repos]]
name = "lightfast"
url = "git@github.com:org/lightfast.git"
branches = ["main", "feat/auth", "fix/memory-leak"]

[[repos]]
name = "agent-os"
url = "/local/path/to/agent-os"
branches = ["main", "v2-rewrite"]
"#;
        let config = parse(toml).unwrap();
        assert_eq!(config.workspace_root.as_deref(), Some("~/my-workspaces"));
        assert_eq!(config.repos.len(), 2);
        assert_eq!(config.repos[0].name, "lightfast");
        assert_eq!(config.repos[0].branches.len(), 3);
        assert_eq!(config.repos[1].name, "agent-os");
        assert_eq!(config.repos[1].url, "/local/path/to/agent-os");
    }

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
        assert_eq!(
            DualConfig::container_name("lightfast", "main"),
            "dual-lightfast-main"
        );
        assert_eq!(
            DualConfig::container_name("lightfast", "feat/auth"),
            "dual-lightfast-feat__auth"
        );
    }

    #[test]
    fn workspace_dir_format() {
        let config = parse("").unwrap();
        let dir = config.workspace_dir("lightfast", "feat/auth");
        // Should end with lightfast/feat__auth
        assert!(dir.ends_with("lightfast/feat__auth"));
    }

    #[test]
    fn validation_rejects_empty_name() {
        let toml = r#"
[[repos]]
name = ""
url = "https://example.com/repo.git"
"#;
        let err = parse(toml).unwrap_err();
        assert!(err.to_string().contains("'name' cannot be empty"));
    }

    #[test]
    fn validation_rejects_empty_url() {
        let toml = r#"
[[repos]]
name = "test"
url = ""
"#;
        let err = parse(toml).unwrap_err();
        assert!(err.to_string().contains("'url' cannot be empty"));
    }

    #[test]
    fn invalid_toml_produces_parse_error() {
        let err = parse("this is not valid toml [[[").unwrap_err();
        assert!(err.to_string().contains("Failed to parse"));
    }

    #[test]
    fn workspace_root_defaults_to_home() {
        let config = parse("").unwrap();
        let root = config.workspace_root();
        let home = dirs::home_dir().unwrap();
        assert_eq!(root, home.join("dual-workspaces"));
    }

    #[test]
    fn workspace_root_custom() {
        let config = parse("workspace_root = \"/tmp/my-workspaces\"").unwrap();
        assert_eq!(config.workspace_root(), PathBuf::from("/tmp/my-workspaces"));
    }
}
