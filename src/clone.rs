use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{DualConfig, RepoConfig};

/// Check if a URL looks like a local filesystem path (vs a remote git URL).
pub fn is_local_path(url: &str) -> bool {
    // Local paths: start with /, ./, ../, or ~/ and don't contain ://
    if url.contains("://") {
        return false;
    }
    url.starts_with('/') || url.starts_with("./") || url.starts_with("../") || url.starts_with("~/")
}

/// Check if a workspace clone already exists on disk.
pub fn workspace_exists(config: &DualConfig, repo: &str, branch: &str) -> bool {
    let dir = config.workspace_dir(repo, branch);
    dir.join(".git").exists()
}

/// List all existing workspace clones for a given repo.
/// Returns a vec of (branch_name, path) tuples for clones that exist on disk.
pub fn list_existing_workspaces(config: &DualConfig, repo: &RepoConfig) -> Vec<(String, PathBuf)> {
    let mut workspaces = Vec::new();
    for branch in &repo.branches {
        let dir = config.workspace_dir(&repo.name, branch);
        if dir.join(".git").exists() {
            workspaces.push((branch.clone(), dir));
        }
    }
    workspaces
}

/// Create a full git clone for a workspace.
///
/// - Local paths use `git clone --local` for hardlink-based fast clones
/// - Remote URLs use standard `git clone`
/// - Branch is checked out via `-b` flag
/// - Target directory is determined by config.workspace_dir()
pub fn clone_workspace(
    config: &DualConfig,
    repo: &str,
    url: &str,
    branch: &str,
) -> Result<PathBuf, CloneError> {
    let target_dir = config.workspace_dir(repo, branch);

    // Don't re-clone if it already exists
    if target_dir.join(".git").exists() {
        return Ok(target_dir);
    }

    // Ensure parent directory exists
    if let Some(parent) = target_dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CloneError::Filesystem(parent.to_path_buf(), e))?;
    }

    let mut cmd = Command::new("git");
    cmd.arg("clone");

    // Use --local for local paths (hardlinks, fast)
    if is_local_path(url) {
        cmd.arg("--local");
    }

    // Checkout the specified branch
    cmd.arg("-b").arg(branch);

    // Source and destination
    cmd.arg(url).arg(&target_dir);

    let output = cmd
        .output()
        .map_err(|e| CloneError::GitNotFound(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CloneError::GitFailed {
            repo: repo.to_string(),
            branch: branch.to_string(),
            stderr,
        });
    }

    Ok(target_dir)
}

/// Remove a workspace clone from disk.
pub fn remove_workspace(config: &DualConfig, repo: &str, branch: &str) -> Result<(), CloneError> {
    let dir = config.workspace_dir(repo, branch);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| CloneError::Filesystem(dir, e))?;
    }
    Ok(())
}

/// Build the git clone command arguments (for testing/debugging without executing).
pub fn build_clone_args(url: &str, branch: &str, target: &Path) -> Vec<String> {
    let mut args = vec!["clone".to_string()];

    if is_local_path(url) {
        args.push("--local".to_string());
    }

    args.push("-b".to_string());
    args.push(branch.to_string());
    args.push(url.to_string());
    args.push(target.to_string_lossy().to_string());

    args
}

#[derive(Debug)]
pub enum CloneError {
    GitNotFound(String),
    GitFailed {
        repo: String,
        branch: String,
        stderr: String,
    },
    Filesystem(PathBuf, std::io::Error),
}

impl std::fmt::Display for CloneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneError::GitNotFound(err) => write!(f, "git not found: {err}"),
            CloneError::GitFailed {
                repo,
                branch,
                stderr,
            } => {
                write!(f, "git clone failed for {repo}/{branch}: {stderr}")
            }
            CloneError::Filesystem(path, err) => {
                write!(f, "filesystem error at {}: {err}", path.display())
            }
        }
    }
}

impl std::error::Error for CloneError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn local_path_detection() {
        assert!(is_local_path("/home/user/repos/myrepo"));
        assert!(is_local_path("./relative/path"));
        assert!(is_local_path("../parent/path"));
        assert!(is_local_path("~/my-repos/project"));

        assert!(!is_local_path("git@github.com:org/repo.git"));
        assert!(!is_local_path("https://github.com/org/repo.git"));
        assert!(!is_local_path("ssh://git@github.com/org/repo.git"));
    }

    #[test]
    fn clone_args_remote() {
        let args = build_clone_args(
            "git@github.com:org/lightfast.git",
            "main",
            Path::new("/tmp/workspaces/lightfast/main"),
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "-b",
                "main",
                "git@github.com:org/lightfast.git",
                "/tmp/workspaces/lightfast/main",
            ]
        );
    }

    #[test]
    fn clone_args_local() {
        let args = build_clone_args(
            "/local/repos/lightfast",
            "feat/auth",
            Path::new("/tmp/workspaces/lightfast/feat__auth"),
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "--local",
                "-b",
                "feat/auth",
                "/local/repos/lightfast",
                "/tmp/workspaces/lightfast/feat__auth",
            ]
        );
    }

    #[test]
    fn workspace_exists_returns_false_for_missing() {
        let config = config::parse("workspace_root = \"/tmp/dual-test-nonexistent\"").unwrap();
        assert!(!workspace_exists(&config, "nonexistent", "main"));
    }

    #[test]
    fn list_existing_empty_when_no_clones() {
        let config = config::parse(
            r#"
workspace_root = "/tmp/dual-test-nonexistent"

[[repos]]
name = "test-repo"
url = "https://example.com/repo.git"
branches = ["main", "dev"]
"#,
        )
        .unwrap();
        let workspaces = list_existing_workspaces(&config, &config.repos[0]);
        assert!(workspaces.is_empty());
    }
}
