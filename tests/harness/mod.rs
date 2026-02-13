use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

/// RAII test fixture that manages Docker containers, tmux sessions, and temp directories.
///
/// Resources are cleaned up automatically on Drop — including on test assertion panics
/// (Rust default panic strategy is "unwind", which triggers Drop).
///
/// Naming convention: `dual-test-{uuid}` prevents cross-test contamination.
pub struct TestFixture {
    /// Unique identifier for this test run.
    pub id: String,
    /// Short ID for display (first 8 chars of UUID).
    pub short_id: String,
    /// Docker containers created by this fixture (cleaned up on Drop).
    containers: Vec<String>,
    /// Tmux sessions created by this fixture (cleaned up on Drop).
    tmux_sessions: Vec<String>,
    /// Temporary directories created by this fixture (cleaned up on Drop).
    temp_dirs: Vec<PathBuf>,
}

impl TestFixture {
    /// Create a new test fixture with a unique UUID-based identifier.
    pub fn new() -> Self {
        let id = Uuid::new_v4().to_string();
        let short_id = id[..8].to_string();
        Self {
            id,
            short_id,
            containers: Vec::new(),
            tmux_sessions: Vec::new(),
            temp_dirs: Vec::new(),
        }
    }

    /// Generate a unique container name for this test.
    /// Pattern: `dual-test-{uuid}` (46 chars, within Docker's 63-char limit).
    pub fn container_name(&self) -> String {
        format!("dual-test-{}", self.id)
    }

    /// Generate a unique tmux session name for this test.
    /// Pattern: `dual-test-{uuid}` (no length limit for tmux).
    pub fn session_name(&self) -> String {
        format!("dual-test-{}", self.id)
    }

    /// Create a temporary directory and register it for cleanup.
    pub fn temp_dir(&mut self) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dual-test-{}", self.id));
        std::fs::create_dir_all(&dir).expect("failed to create temp dir");
        self.temp_dirs.push(dir.clone());
        dir
    }

    /// Create a named subdirectory under a parent directory.
    /// Not registered separately — parent dir cleanup handles it.
    pub fn temp_subdir(parent: &std::path::Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        std::fs::create_dir_all(&dir).expect("failed to create temp subdir");
        dir
    }

    /// Register a container name for RAII cleanup.
    pub fn register_container(&mut self, name: String) {
        self.containers.push(name);
    }

    /// Register a tmux session for RAII cleanup.
    pub fn register_tmux_session(&mut self, name: String) {
        self.tmux_sessions.push(name);
    }

    /// Create a DualConfig with workspace_root set to a test directory.
    pub fn test_config(
        workspace_root: &std::path::Path,
        toml_extra: &str,
    ) -> dual::config::DualConfig {
        let toml_str = format!(
            "workspace_root = \"{}\"\n{}",
            workspace_root.display(),
            toml_extra
        );
        dual::config::parse(&toml_str).expect("failed to parse test config")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Cleanup containers (force-remove handles running + stopped)
        for name in &self.containers {
            let _ = Command::new("docker").args(["rm", "-f", name]).output();
        }

        // Cleanup tmux sessions
        for session in &self.tmux_sessions {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", session])
                .output();
        }

        // Cleanup temp directories
        for dir in &self.temp_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Defense-in-depth: remove ALL test resources matching the `dual-test-` prefix.
///
/// Run before/after test suites to clean up orphaned resources from
/// SIGKILL or other abnormal termination where Drop didn't fire.
pub fn cleanup_sweep() {
    // Remove all test containers
    if let Ok(output) = Command::new("docker")
        .args(["ps", "-aq", "--filter", "name=dual-test-"])
        .output()
    {
        let ids = String::from_utf8_lossy(&output.stdout);
        for id in ids.lines().filter(|l| !l.is_empty()) {
            let _ = Command::new("docker").args(["rm", "-f", id]).output();
        }
    }

    // Remove all test tmux sessions
    if let Ok(output) = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for session in stdout.lines().filter(|l| l.starts_with("dual-test-")) {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", session])
                .output();
        }
    }
}
