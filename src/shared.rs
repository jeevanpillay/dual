use std::path::{Path, PathBuf};

/// Ensure the shared directory exists for a repo.
pub fn ensure_shared_dir(repo: &str) -> Result<PathBuf, SharedError> {
    let dir = crate::config::shared_dir(repo).ok_or(SharedError::NoHomeDir)?;
    std::fs::create_dir_all(&dir).map_err(|e| SharedError::Filesystem(dir.clone(), e))?;
    Ok(dir)
}

/// Initialize shared directory from the main workspace.
///
/// For each file in `files`:
/// - If file exists in workspace and is NOT already a symlink to shared dir:
///   move it to shared dir, create symlink back (Unix) or copy back (Windows)
/// - If file is already a symlink to shared dir: skip
/// - If file doesn't exist in workspace: skip
///
/// Returns list of files that were moved.
pub fn init_from_main(
    workspace_dir: &Path,
    shared_dir: &Path,
    files: &[String],
) -> Result<Vec<String>, SharedError> {
    let mut moved = Vec::new();

    for file in files {
        let src = workspace_dir.join(file);
        let dst = shared_dir.join(file);

        if !src.exists() && src.symlink_metadata().is_err() {
            continue; // File not present yet, skip
        }

        // Check if already a symlink pointing to shared dir
        if src
            .symlink_metadata()
            .map(|m| m.is_symlink())
            .unwrap_or(false)
            && let Ok(target) = std::fs::read_link(&src)
            && target == dst
        {
            continue; // Already set up correctly
        }

        // Ensure parent dir exists in shared
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SharedError::Filesystem(parent.to_path_buf(), e))?;
        }

        // Move file/dir to shared dir
        // Use copy + remove for cross-device moves (rename fails across filesystems)
        copy_recursive(&src, &dst)?;
        remove_recursive(&src)?;

        // Create symlink back (Unix) or copy back (Windows)
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&dst, &src)
                .map_err(|e| SharedError::Filesystem(src.clone(), e))?;
        }
        #[cfg(windows)]
        {
            copy_recursive(&dst, &src)?;
        }

        moved.push(file.clone());
    }

    Ok(moved)
}

/// Copy shared files into a branch workspace.
///
/// For each file in `files`:
/// - If file exists in shared dir: copy to workspace (overwrite if exists)
/// - If file doesn't exist in shared dir: skip
///
/// Returns list of files that were copied.
pub fn copy_to_branch(
    workspace_dir: &Path,
    shared_dir: &Path,
    files: &[String],
) -> Result<Vec<String>, SharedError> {
    let mut copied = Vec::new();

    for file in files {
        let src = shared_dir.join(file);
        let dst = workspace_dir.join(file);

        if !src.exists() {
            continue; // Not in shared dir yet, skip
        }

        // Ensure parent dir exists in workspace
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SharedError::Filesystem(parent.to_path_buf(), e))?;
        }

        // Remove existing destination to handle overwrite cleanly
        if dst.exists() || dst.symlink_metadata().is_ok() {
            remove_recursive(&dst)?;
        }

        copy_recursive(&src, &dst)?;
        copied.push(file.clone());
    }

    Ok(copied)
}

/// Recursively copy a file or directory.
fn copy_recursive(src: &Path, dst: &Path) -> Result<(), SharedError> {
    let metadata = src
        .symlink_metadata()
        .map_err(|e| SharedError::Filesystem(src.to_path_buf(), e))?;

    if metadata.is_dir() {
        std::fs::create_dir_all(dst).map_err(|e| SharedError::Filesystem(dst.to_path_buf(), e))?;

        for entry in
            std::fs::read_dir(src).map_err(|e| SharedError::Filesystem(src.to_path_buf(), e))?
        {
            let entry = entry.map_err(|e| SharedError::Filesystem(src.to_path_buf(), e))?;
            let child_src = entry.path();
            let child_dst = dst.join(entry.file_name());
            copy_recursive(&child_src, &child_dst)?;
        }
    } else {
        std::fs::copy(src, dst).map_err(|e| SharedError::Filesystem(src.to_path_buf(), e))?;
    }

    Ok(())
}

/// Remove a file or directory.
fn remove_recursive(path: &Path) -> Result<(), SharedError> {
    let metadata = path
        .symlink_metadata()
        .map_err(|e| SharedError::Filesystem(path.to_path_buf(), e))?;

    if metadata.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| SharedError::Filesystem(path.to_path_buf(), e))?;
    } else {
        std::fs::remove_file(path).map_err(|e| SharedError::Filesystem(path.to_path_buf(), e))?;
    }

    Ok(())
}

#[derive(Debug)]
pub enum SharedError {
    NoHomeDir,
    Filesystem(PathBuf, std::io::Error),
}

impl std::fmt::Display for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedError::NoHomeDir => write!(f, "Could not determine home directory"),
            SharedError::Filesystem(path, err) => {
                write!(f, "Filesystem error at {}: {err}", path.display())
            }
        }
    }
}

impl std::error::Error for SharedError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_temp_dirs(test_name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("dual-test-shared-{test_name}"));
        let _ = fs::remove_dir_all(&base);
        let workspace = base.join("workspace");
        let shared = base.join("shared");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&shared).unwrap();
        (workspace, shared)
    }

    fn cleanup(test_name: &str) {
        let base = std::env::temp_dir().join(format!("dual-test-shared-{test_name}"));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn init_from_main_moves_file_and_creates_symlink() {
        let (workspace, shared) = setup_temp_dirs("init-file");

        // Create a file in workspace
        fs::write(workspace.join(".env.local"), "SECRET=abc123").unwrap();

        let result = init_from_main(&workspace, &shared, &[".env.local".to_string()]).unwrap();

        assert_eq!(result, vec![".env.local"]);
        // File exists in shared dir
        assert!(shared.join(".env.local").exists());
        assert_eq!(
            fs::read_to_string(shared.join(".env.local")).unwrap(),
            "SECRET=abc123"
        );

        #[cfg(unix)]
        {
            // Symlink exists in workspace pointing to shared dir
            let meta = workspace.join(".env.local").symlink_metadata().unwrap();
            assert!(meta.is_symlink());
            let target = fs::read_link(workspace.join(".env.local")).unwrap();
            assert_eq!(target, shared.join(".env.local"));
        }

        cleanup("init-file");
    }

    #[test]
    fn init_from_main_moves_directory() {
        let (workspace, shared) = setup_temp_dirs("init-dir");

        // Create a directory with files in workspace
        fs::create_dir_all(workspace.join(".vercel")).unwrap();
        fs::write(workspace.join(".vercel/project.json"), r#"{"orgId":"123"}"#).unwrap();
        fs::write(workspace.join(".vercel/README.txt"), "vercel config").unwrap();

        let result = init_from_main(&workspace, &shared, &[".vercel".to_string()]).unwrap();

        assert_eq!(result, vec![".vercel"]);
        // Directory exists in shared dir with all files
        assert!(shared.join(".vercel").is_dir());
        assert!(shared.join(".vercel/project.json").exists());
        assert!(shared.join(".vercel/README.txt").exists());

        #[cfg(unix)]
        {
            let meta = workspace.join(".vercel").symlink_metadata().unwrap();
            assert!(meta.is_symlink());
        }

        cleanup("init-dir");
    }

    #[test]
    fn init_from_main_skips_missing_files() {
        let (workspace, shared) = setup_temp_dirs("init-missing");

        let result = init_from_main(&workspace, &shared, &["nonexistent".to_string()]).unwrap();

        assert!(result.is_empty());

        cleanup("init-missing");
    }

    #[cfg(unix)]
    #[test]
    fn init_from_main_skips_existing_symlink() {
        let (workspace, shared) = setup_temp_dirs("init-symlink");

        // Create file in shared dir
        fs::write(shared.join(".env"), "KEY=val").unwrap();
        // Create symlink in workspace → shared dir
        std::os::unix::fs::symlink(shared.join(".env"), workspace.join(".env")).unwrap();

        let result = init_from_main(&workspace, &shared, &[".env".to_string()]).unwrap();

        assert!(result.is_empty()); // Nothing moved

        cleanup("init-symlink");
    }

    #[test]
    fn copy_to_branch_copies_file() {
        let (workspace, shared) = setup_temp_dirs("copy-file");

        // Create a file in shared dir
        fs::write(shared.join(".env.local"), "SECRET=xyz").unwrap();

        let result = copy_to_branch(&workspace, &shared, &[".env.local".to_string()]).unwrap();

        assert_eq!(result, vec![".env.local"]);
        assert!(workspace.join(".env.local").exists());
        assert_eq!(
            fs::read_to_string(workspace.join(".env.local")).unwrap(),
            "SECRET=xyz"
        );
        // It's a regular file, not a symlink
        let meta = workspace.join(".env.local").symlink_metadata().unwrap();
        assert!(!meta.is_symlink());

        cleanup("copy-file");
    }

    #[test]
    fn copy_to_branch_copies_directory() {
        let (workspace, shared) = setup_temp_dirs("copy-dir");

        // Create a directory with files in shared dir
        fs::create_dir_all(shared.join(".vercel")).unwrap();
        fs::write(shared.join(".vercel/project.json"), r#"{"orgId":"456"}"#).unwrap();

        let result = copy_to_branch(&workspace, &shared, &[".vercel".to_string()]).unwrap();

        assert_eq!(result, vec![".vercel"]);
        assert!(workspace.join(".vercel").is_dir());
        assert!(workspace.join(".vercel/project.json").exists());

        cleanup("copy-dir");
    }

    #[test]
    fn copy_to_branch_overwrites_existing() {
        let (workspace, shared) = setup_temp_dirs("copy-overwrite");

        // Create file with content "old" in workspace
        fs::write(workspace.join(".env"), "old").unwrap();
        // Create file with content "new" in shared dir
        fs::write(shared.join(".env"), "new").unwrap();

        let result = copy_to_branch(&workspace, &shared, &[".env".to_string()]).unwrap();

        assert_eq!(result, vec![".env"]);
        assert_eq!(fs::read_to_string(workspace.join(".env")).unwrap(), "new");

        cleanup("copy-overwrite");
    }

    #[test]
    fn copy_to_branch_skips_missing_shared_files() {
        let (workspace, shared) = setup_temp_dirs("copy-missing");

        let result = copy_to_branch(&workspace, &shared, &["nonexistent".to_string()]).unwrap();

        assert!(result.is_empty());

        cleanup("copy-missing");
    }

    #[test]
    fn shared_dir_path_format() {
        let dir = crate::config::shared_dir("lightfast").unwrap();
        assert!(dir.to_string_lossy().contains(".dual/shared/lightfast"));
    }

    #[cfg(unix)]
    #[test]
    fn init_from_main_creates_symlink_on_unix() {
        let (workspace, shared) = setup_temp_dirs("unix-symlink");

        fs::write(workspace.join(".env"), "VALUE=test").unwrap();

        init_from_main(&workspace, &shared, &[".env".to_string()]).unwrap();

        let target = fs::read_link(workspace.join(".env")).unwrap();
        assert_eq!(target, shared.join(".env"));

        cleanup("unix-symlink");
    }
}
