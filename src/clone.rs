use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config;

/// Check if a URL looks like a local filesystem path (vs a remote git URL).
pub fn is_local_path(url: &str) -> bool {
    // Local paths: start with /, ./, ../, or ~/ and don't contain ://
    if url.contains("://") {
        return false;
    }
    url.starts_with('/') || url.starts_with("./") || url.starts_with("../") || url.starts_with("~/")
}

/// Check if a workspace clone already exists on disk.
pub fn workspace_exists(workspace_root: &Path, repo: &str, branch: &str) -> bool {
    let dir = config::workspace_dir(workspace_root, repo, branch);
    dir.join(".git").exists()
}

/// Create a full git clone for a workspace.
///
/// - Local paths use `git clone --local` for hardlink-based fast clones
/// - Remote URLs use standard `git clone`
/// - Branch is checked out via `-b` flag
/// - Target directory is determined by workspace_root + repo + branch
pub fn clone_workspace(
    workspace_root: &Path,
    repo: &str,
    url: &str,
    branch: &str,
) -> Result<PathBuf, CloneError> {
    let target_dir = config::workspace_dir(workspace_root, repo, branch);

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
pub fn remove_workspace(workspace_root: &Path, repo: &str, branch: &str) -> Result<(), CloneError> {
    let dir = config::workspace_dir(workspace_root, repo, branch);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| CloneError::Filesystem(dir, e))?;
    }
    Ok(())
}

/// Clone from a local main workspace using --local (hardlinks), then create a new branch.
///
/// This avoids the problem of `git clone -b <branch>` failing when the branch
/// doesn't exist at the remote. Instead, we clone from the local main workspace
/// and create the branch locally.
pub fn clone_from_local(
    main_workspace_path: &Path,
    target_dir: &Path,
    new_branch: &str,
) -> Result<PathBuf, CloneError> {
    // Don't re-clone if it already exists
    if target_dir.join(".git").exists() {
        return Ok(target_dir.to_path_buf());
    }

    // Ensure parent directory exists
    if let Some(parent) = target_dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CloneError::Filesystem(parent.to_path_buf(), e))?;
    }

    // Step 1: git clone --local <main_workspace_path> <target_dir>
    let clone_output = Command::new("git")
        .args(build_local_clone_args(main_workspace_path, target_dir))
        .output()
        .map_err(|e| CloneError::GitNotFound(e.to_string()))?;

    if !clone_output.status.success() {
        let stderr = String::from_utf8_lossy(&clone_output.stderr).to_string();
        return Err(CloneError::GitFailed {
            repo: main_workspace_path.to_string_lossy().to_string(),
            branch: new_branch.to_string(),
            stderr,
        });
    }

    // Step 2: git checkout -b <new_branch> in the cloned directory
    let checkout_output = Command::new("git")
        .args(["checkout", "-b", new_branch])
        .current_dir(target_dir)
        .output()
        .map_err(|e| CloneError::GitNotFound(e.to_string()))?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr).to_string();
        return Err(CloneError::GitFailed {
            repo: main_workspace_path.to_string_lossy().to_string(),
            branch: new_branch.to_string(),
            stderr,
        });
    }

    Ok(target_dir.to_path_buf())
}

/// Build the git clone --local arguments (for testing).
pub fn build_local_clone_args(main_workspace_path: &Path, target: &Path) -> Vec<String> {
    vec![
        "clone".to_string(),
        "--local".to_string(),
        main_workspace_path.to_string_lossy().to_string(),
        target.to_string_lossy().to_string(),
    ]
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

#[derive(Debug, thiserror::Error)]
pub enum CloneError {
    #[error("git not found: {0}")]
    GitNotFound(String),

    #[error("git clone failed for {repo}/{branch}: {stderr}")]
    GitFailed {
        repo: String,
        branch: String,
        stderr: String,
    },

    #[error("filesystem error at {path}: {err}", path = .0.display(), err = .1)]
    Filesystem(PathBuf, std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!workspace_exists(
            Path::new("/tmp/dual-test-nonexistent"),
            "nonexistent",
            "main"
        ));
    }

    #[test]
    fn local_clone_args() {
        let args = build_local_clone_args(
            Path::new("/home/user/code/lightfast"),
            Path::new("/tmp/workspaces/lightfast/feat__auth"),
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "--local",
                "/home/user/code/lightfast",
                "/tmp/workspaces/lightfast/feat__auth",
            ]
        );
    }
}
