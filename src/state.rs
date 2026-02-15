use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::config;

const STATE_DIR: &str = ".dual";
const STATE_FILENAME: &str = "workspaces.toml";
const DEFAULT_WORKSPACE_ROOT: &str = ".dual/workspaces";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkspaceState {
    /// Root directory for all workspace clones (default: ~/.dual/workspaces)
    pub workspace_root: Option<String>,

    /// Active workspace entries
    #[serde(default)]
    pub workspaces: Vec<WorkspaceEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkspaceEntry {
    /// Short repo name (e.g. "lightfast")
    pub repo: String,

    /// Git URL or local path
    pub url: String,

    /// Branch name (e.g. "main", "feat/auth")
    pub branch: String,

    /// Explicit path to workspace directory (for `dual add` — user's existing clone).
    /// If None, workspace lives at {workspace_root}/{repo}/{encoded_branch}/
    /// and will be cloned on first launch.
    pub path: Option<String>,
}

impl WorkspaceState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            workspaces: Vec::new(),
        }
    }

    /// Resolve the workspace root directory as an absolute path.
    /// Uses the configured value or defaults to ~/.dual/workspaces.
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

    /// Get the workspace directory for an entry.
    /// Uses explicit path if set, otherwise computes from workspace_root.
    pub fn workspace_dir(&self, entry: &WorkspaceEntry) -> PathBuf {
        if let Some(ref path) = entry.path {
            PathBuf::from(shellexpand(path))
        } else {
            config::workspace_dir(&self.workspace_root(), &entry.repo, &entry.branch)
        }
    }

    /// Find a workspace entry by identifier (e.g. "lightfast-main").
    pub fn resolve_workspace(&self, identifier: &str) -> Option<&WorkspaceEntry> {
        self.workspaces.iter().find(|ws| {
            let id = config::workspace_id(&ws.repo, &ws.branch);
            id == identifier
        })
    }

    /// Get all workspace entries.
    pub fn all_workspaces(&self) -> &[WorkspaceEntry] {
        &self.workspaces
    }

    /// Add a workspace entry. Returns Err if duplicate exists.
    pub fn add_workspace(&mut self, entry: WorkspaceEntry) -> Result<(), StateError> {
        if self.has_workspace(&entry.repo, &entry.branch) {
            return Err(StateError::DuplicateWorkspace(
                entry.repo.clone(),
                entry.branch.clone(),
            ));
        }
        self.workspaces.push(entry);
        Ok(())
    }

    /// Remove a workspace entry by repo + branch. Returns true if found.
    pub fn remove_workspace(&mut self, repo: &str, branch: &str) -> bool {
        let before = self.workspaces.len();
        self.workspaces
            .retain(|ws| !(ws.repo == repo && ws.branch == branch));
        self.workspaces.len() < before
    }

    /// Find all workspaces for a given repo name.
    pub fn workspaces_for_repo(&self, repo: &str) -> Vec<&WorkspaceEntry> {
        self.workspaces
            .iter()
            .filter(|ws| ws.repo == repo)
            .collect()
    }

    /// Check if a workspace entry exists for repo + branch.
    pub fn has_workspace(&self, repo: &str, branch: &str) -> bool {
        self.workspaces
            .iter()
            .any(|ws| ws.repo == repo && ws.branch == branch)
    }
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the state file path: ~/.dual/workspaces.toml
pub fn state_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(STATE_DIR).join(STATE_FILENAME))
}

/// Load state from default location. Returns empty state if file doesn't exist.
pub fn load() -> Result<WorkspaceState, StateError> {
    let path = state_path().ok_or(StateError::NoHomeDir)?;

    if !path.exists() {
        return Ok(WorkspaceState::new());
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| StateError::ReadError(path.clone(), e))?;
    let state: WorkspaceState =
        toml::from_str(&contents).map_err(|e| StateError::ParseError(path.clone(), e))?;
    validate(&state)?;
    debug!(count = state.workspaces.len(), "loaded state");
    Ok(state)
}

/// Load state from a specific path.
pub fn load_from(path: &Path) -> Result<WorkspaceState, StateError> {
    if !path.exists() {
        return Ok(WorkspaceState::new());
    }

    let contents =
        std::fs::read_to_string(path).map_err(|e| StateError::ReadError(path.to_path_buf(), e))?;
    let state: WorkspaceState =
        toml::from_str(&contents).map_err(|e| StateError::ParseError(path.to_path_buf(), e))?;
    validate(&state)?;
    Ok(state)
}

/// Save state to default location. Uses atomic write with backup.
pub fn save(state: &WorkspaceState) -> Result<(), StateError> {
    let path = state_path().ok_or(StateError::NoHomeDir)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| StateError::WriteError(parent.to_path_buf(), e))?;
    }

    atomic_save(state, &path)
}

/// Save state to a specific path. Uses atomic write with backup.
pub fn save_to(state: &WorkspaceState, path: &Path) -> Result<(), StateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| StateError::WriteError(parent.to_path_buf(), e))?;
    }

    atomic_save(state, path)
}

/// Atomic save: serialize -> write to temp file -> backup existing -> rename.
///
/// Uses advisory file locking to prevent concurrent writes.
/// The lock is held on a lockfile for the duration of the write-backup-rename sequence.
fn atomic_save(state: &WorkspaceState, path: &Path) -> Result<(), StateError> {
    let contents = toml::to_string_pretty(state).map_err(StateError::SerializeError)?;

    let lock_path = path.with_extension("lock");

    // Acquire advisory lock
    let lock_file =
        File::create(&lock_path).map_err(|e| StateError::WriteError(lock_path.clone(), e))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| StateError::WriteError(lock_path.clone(), e))?;

    // Write to temp file in the same directory (same filesystem for rename)
    let tmp_path = path.with_extension("tmp");
    let mut tmp_file =
        File::create(&tmp_path).map_err(|e| StateError::WriteError(tmp_path.clone(), e))?;
    tmp_file
        .write_all(contents.as_bytes())
        .map_err(|e| StateError::WriteError(tmp_path.clone(), e))?;
    tmp_file
        .sync_all()
        .map_err(|e| StateError::WriteError(tmp_path.clone(), e))?;

    // Backup existing file (best-effort — don't fail if original doesn't exist)
    let bak_path = path.with_extension("toml.bak");
    if path.exists() {
        let _ = fs::copy(path, &bak_path);
    }

    // Atomic rename (POSIX guarantees this is atomic on same filesystem)
    fs::rename(&tmp_path, path).map_err(|e| StateError::WriteError(path.to_path_buf(), e))?;

    // Release lock (dropped with file, but explicit for clarity)
    let _ = lock_file.unlock();
    let _ = fs::remove_file(&lock_path);

    Ok(())
}

/// Parse state from TOML string (for testing).
pub fn parse(toml_str: &str) -> Result<WorkspaceState, StateError> {
    let state: WorkspaceState = toml::from_str(toml_str)
        .map_err(|e| StateError::ParseError(PathBuf::from("<string>"), e))?;
    validate(&state)?;
    Ok(state)
}

fn validate(state: &WorkspaceState) -> Result<(), StateError> {
    for (i, ws) in state.workspaces.iter().enumerate() {
        if ws.repo.is_empty() {
            return Err(StateError::Validation(format!(
                "workspaces[{i}]: 'repo' cannot be empty"
            )));
        }
        if ws.url.is_empty() {
            return Err(StateError::Validation(format!(
                "workspaces[{i}] ({}): 'url' cannot be empty",
                ws.repo
            )));
        }
        if ws.branch.is_empty() {
            return Err(StateError::Validation(format!(
                "workspaces[{i}] ({}): 'branch' cannot be empty",
                ws.repo
            )));
        }
    }
    Ok(())
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

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Could not determine home directory")]
    NoHomeDir,

    #[error("Failed to read {path}: {err}", path = .0.display(), err = .1)]
    ReadError(PathBuf, std::io::Error),

    #[error("Failed to write {path}: {err}", path = .0.display(), err = .1)]
    WriteError(PathBuf, std::io::Error),

    #[error("Failed to parse {path}: {err}", path = .0.display(), err = .1)]
    ParseError(PathBuf, toml::de::Error),

    #[error("Failed to serialize state: {0}")]
    SerializeError(toml::ser::Error),

    #[error("Invalid state: {0}")]
    Validation(String),

    #[error("Workspace {0}/{1} already exists")]
    DuplicateWorkspace(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let state = WorkspaceState::new();
        assert!(state.workspaces.is_empty());
        assert!(state.workspace_root.is_none());
    }

    #[test]
    fn parse_empty_state() {
        let state = parse("").unwrap();
        assert!(state.workspaces.is_empty());
        assert!(state.workspace_root.is_none());
    }

    #[test]
    fn parse_state_with_workspaces() {
        let toml = r#"
workspace_root = "~/my-workspaces"

[[workspaces]]
repo = "lightfast"
url = "git@github.com:org/lightfast.git"
branch = "main"
path = "/Users/dev/code/lightfast"

[[workspaces]]
repo = "lightfast"
url = "git@github.com:org/lightfast.git"
branch = "feat/auth"

[[workspaces]]
repo = "agent-os"
url = "/local/path/to/agent-os"
branch = "main"
path = "/local/path/to/agent-os"
"#;
        let state = parse(toml).unwrap();
        assert_eq!(state.workspace_root.as_deref(), Some("~/my-workspaces"));
        assert_eq!(state.workspaces.len(), 3);
        assert_eq!(state.workspaces[0].repo, "lightfast");
        assert_eq!(state.workspaces[0].branch, "main");
        assert_eq!(
            state.workspaces[0].path.as_deref(),
            Some("/Users/dev/code/lightfast")
        );
        assert_eq!(state.workspaces[1].repo, "lightfast");
        assert_eq!(state.workspaces[1].branch, "feat/auth");
        assert!(state.workspaces[1].path.is_none());
        assert_eq!(state.workspaces[2].repo, "agent-os");
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut state = WorkspaceState::new();
        state.workspace_root = Some("~/workspaces".to_string());
        state
            .add_workspace(WorkspaceEntry {
                repo: "test".to_string(),
                url: "https://example.com/test.git".to_string(),
                branch: "main".to_string(),
                path: Some("/home/user/test".to_string()),
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "test".to_string(),
                url: "https://example.com/test.git".to_string(),
                branch: "dev".to_string(),
                path: None,
            })
            .unwrap();

        let toml_str = toml::to_string_pretty(&state).unwrap();
        let parsed = parse(&toml_str).unwrap();
        assert_eq!(state, parsed);
    }

    #[test]
    fn add_workspace() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "https://example.com/lightfast.git".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();
        assert_eq!(state.workspaces.len(), 1);
    }

    #[test]
    fn add_duplicate_workspace_errors() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "https://example.com/lightfast.git".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();

        let result = state.add_workspace(WorkspaceEntry {
            repo: "lightfast".to_string(),
            url: "https://example.com/lightfast.git".to_string(),
            branch: "main".to_string(),
            path: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn remove_workspace() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "https://example.com/lightfast.git".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "https://example.com/lightfast.git".to_string(),
                branch: "feat/auth".to_string(),
                path: None,
            })
            .unwrap();

        assert!(state.remove_workspace("lightfast", "main"));
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].branch, "feat/auth");
    }

    #[test]
    fn remove_nonexistent_workspace() {
        let mut state = WorkspaceState::new();
        assert!(!state.remove_workspace("nope", "main"));
    }

    #[test]
    fn resolve_workspace_found() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "https://example.com/lightfast.git".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "https://example.com/lightfast.git".to_string(),
                branch: "feat/auth".to_string(),
                path: None,
            })
            .unwrap();

        let ws = state.resolve_workspace("lightfast-main").unwrap();
        assert_eq!(ws.repo, "lightfast");
        assert_eq!(ws.branch, "main");

        let ws = state.resolve_workspace("lightfast-feat__auth").unwrap();
        assert_eq!(ws.repo, "lightfast");
        assert_eq!(ws.branch, "feat/auth");
    }

    #[test]
    fn resolve_workspace_not_found() {
        let state = WorkspaceState::new();
        assert!(state.resolve_workspace("nonexistent").is_none());
    }

    #[test]
    fn all_workspaces() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "a".to_string(),
                url: "url".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "b".to_string(),
                url: "url".to_string(),
                branch: "dev".to_string(),
                path: None,
            })
            .unwrap();

        let all = state.all_workspaces();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn workspaces_for_repo() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "url".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "url".to_string(),
                branch: "feat/auth".to_string(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "other".to_string(),
                url: "url".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();

        assert_eq!(state.workspaces_for_repo("lightfast").len(), 2);
        assert_eq!(state.workspaces_for_repo("other").len(), 1);
        assert!(state.workspaces_for_repo("nonexistent").is_empty());
    }

    #[test]
    fn has_workspace() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "url".to_string(),
                branch: "main".to_string(),
                path: None,
            })
            .unwrap();

        assert!(state.has_workspace("lightfast", "main"));
        assert!(!state.has_workspace("lightfast", "dev"));
        assert!(!state.has_workspace("other", "main"));
    }

    #[test]
    fn workspace_root_default() {
        let state = WorkspaceState::new();
        let root = state.workspace_root();
        let home = dirs::home_dir().unwrap();
        assert_eq!(root, home.join(".dual/workspaces"));
    }

    #[test]
    fn workspace_root_custom() {
        let state = parse("workspace_root = \"/tmp/my-workspaces\"").unwrap();
        assert_eq!(state.workspace_root(), PathBuf::from("/tmp/my-workspaces"));
    }

    #[test]
    fn workspace_dir_with_explicit_path() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "url".to_string(),
                branch: "main".to_string(),
                path: Some("/Users/dev/code/lightfast".to_string()),
            })
            .unwrap();

        let dir = state.workspace_dir(&state.workspaces[0]);
        assert_eq!(dir, PathBuf::from("/Users/dev/code/lightfast"));
    }

    #[test]
    fn workspace_dir_computed() {
        let mut state = parse("workspace_root = \"/tmp/ws\"").unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".to_string(),
                url: "url".to_string(),
                branch: "feat/auth".to_string(),
                path: None,
            })
            .unwrap();

        let dir = state.workspace_dir(&state.workspaces[0]);
        assert_eq!(dir, PathBuf::from("/tmp/ws/lightfast/feat__auth"));
    }

    #[test]
    fn validation_rejects_empty_repo() {
        let toml = r#"
[[workspaces]]
repo = ""
url = "https://example.com/repo.git"
branch = "main"
"#;
        let err = parse(toml).unwrap_err();
        assert!(err.to_string().contains("'repo' cannot be empty"));
    }

    #[test]
    fn validation_rejects_empty_url() {
        let toml = r#"
[[workspaces]]
repo = "test"
url = ""
branch = "main"
"#;
        let err = parse(toml).unwrap_err();
        assert!(err.to_string().contains("'url' cannot be empty"));
    }

    #[test]
    fn validation_rejects_empty_branch() {
        let toml = r#"
[[workspaces]]
repo = "test"
url = "https://example.com/repo.git"
branch = ""
"#;
        let err = parse(toml).unwrap_err();
        assert!(err.to_string().contains("'branch' cannot be empty"));
    }

    #[test]
    fn state_path_exists() {
        let path = state_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.ends_with(".dual/workspaces.toml"));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("dual-test-state-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("workspaces.toml");

        let mut state = WorkspaceState::new();
        state.workspace_root = Some("/tmp/ws".to_string());
        state
            .add_workspace(WorkspaceEntry {
                repo: "test".to_string(),
                url: "https://example.com/test.git".to_string(),
                branch: "main".to_string(),
                path: Some("/home/user/test".to_string()),
            })
            .unwrap();

        save_to(&state, &path).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(state, loaded);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_missing_file_returns_empty() {
        let path = PathBuf::from("/tmp/dual-test-nonexistent/workspaces.toml");
        let state = load_from(&path).unwrap();
        assert!(state.workspaces.is_empty());
    }
}
