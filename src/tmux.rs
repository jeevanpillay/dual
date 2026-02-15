use std::path::Path;
use std::process::Command;

/// Session name prefix for dual-managed sessions.
const SESSION_PREFIX: &str = "dual-";

/// Check if tmux is available on the system.
pub fn is_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Create a new detached tmux session for a workspace.
///
/// - Session name follows the container naming convention: dual-{repo}-{branch}
/// - Working directory is set to the workspace clone dir
/// - Optionally sources shell RC for command interception
pub fn create_session(
    session_name: &str,
    cwd: &Path,
    shell_rc: Option<&str>,
) -> Result<(), TmuxError> {
    // Create detached session with CWD
    let output = Command::new("tmux")
        .args(build_new_session_args(session_name, cwd))
        .output()
        .map_err(|e| TmuxError::NotFound(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(TmuxError::Failed {
            operation: "new-session".to_string(),
            session: session_name.to_string(),
            stderr,
        });
    }

    // Source shell RC if provided
    if let Some(rc) = shell_rc {
        send_keys(session_name, rc)?;
    }

    Ok(())
}

/// Attach to an existing tmux session.
///
/// If already inside tmux (`$TMUX` is set), uses `switch-client` to avoid
/// nested sessions. Otherwise uses `attach-session`.
pub fn attach(session_name: &str) -> Result<(), TmuxError> {
    let (cmd, op) = if is_inside_tmux() {
        (["switch-client", "-t", session_name], "switch-client")
    } else {
        (["attach-session", "-t", session_name], "attach-session")
    };

    let status = Command::new("tmux")
        .args(cmd)
        .status()
        .map_err(|e| TmuxError::NotFound(e.to_string()))?;

    if !status.success() {
        return Err(TmuxError::Failed {
            operation: op.to_string(),
            session: session_name.to_string(),
            stderr: format!("exit code: {}", status.code().unwrap_or(-1)),
        });
    }

    Ok(())
}

/// Check if we're already inside a tmux session.
pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok_and(|v| !v.is_empty())
}

/// Detach the current client from a session.
pub fn detach(session_name: &str) -> Result<(), TmuxError> {
    tmux_simple(&["detach-client", "-s", session_name])
}

/// Destroy a tmux session (kill all panes and processes).
pub fn destroy(session_name: &str) -> Result<(), TmuxError> {
    tmux_simple(&["kill-session", "-t", session_name])
}

/// Check if a session exists and is alive.
pub fn is_alive(session_name: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// List all dual-managed tmux sessions.
/// Returns session names that start with the dual- prefix.
pub fn list_sessions() -> Vec<String> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter(|line| line.starts_with(SESSION_PREFIX))
                .map(|s| s.to_string())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Send keys (a command string) to a tmux session pane.
pub fn send_keys(session_name: &str, keys: &str) -> Result<(), TmuxError> {
    tmux_simple(&["send-keys", "-t", session_name, keys, "Enter"])
}

/// Build the arguments for tmux new-session (for testing).
pub fn build_new_session_args(session_name: &str, cwd: &Path) -> Vec<String> {
    vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        session_name.to_string(),
        "-c".to_string(),
        cwd.to_string_lossy().to_string(),
    ]
}

/// Generate the session name for a workspace.
pub fn session_name(repo: &str, branch: &str) -> String {
    use crate::config::encode_branch;
    format!("{SESSION_PREFIX}{repo}-{}", encode_branch(branch))
}

fn tmux_simple(args: &[&str]) -> Result<(), TmuxError> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| TmuxError::NotFound(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(TmuxError::Failed {
            operation: args.first().unwrap_or(&"unknown").to_string(),
            session: args
                .iter()
                .position(|&a| a == "-t" || a == "-s")
                .and_then(|i| args.get(i + 1))
                .unwrap_or(&"unknown")
                .to_string(),
            stderr,
        });
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum TmuxError {
    #[error("tmux not found: {0}")]
    NotFound(String),

    #[error("tmux {operation} failed for {session}: {stderr}")]
    Failed {
        operation: String,
        session: String,
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn new_session_args_correct() {
        let args = build_new_session_args(
            "dual-lightfast-main",
            Path::new("/home/user/dual-workspaces/lightfast/main"),
        );
        assert_eq!(
            args,
            vec![
                "new-session",
                "-d",
                "-s",
                "dual-lightfast-main",
                "-c",
                "/home/user/dual-workspaces/lightfast/main",
            ]
        );
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
        // Session names should match container names for consistency
        use crate::config;
        assert_eq!(
            session_name("lightfast", "main"),
            config::container_name("lightfast", "main")
        );
        assert_eq!(
            session_name("lightfast", "feat/auth"),
            config::container_name("lightfast", "feat/auth")
        );
    }

    #[test]
    fn session_prefix_is_dual() {
        assert_eq!(SESSION_PREFIX, "dual-");
    }

    #[test]
    fn tmux_availability_check_runs() {
        // This test just verifies the function doesn't panic
        // It may return true or false depending on the system
        let _available = is_available();
    }

    #[test]
    fn is_inside_tmux_detects_env() {
        // Save and restore TMUX env var
        let original = std::env::var("TMUX").ok();

        // SAFETY: This test runs single-threaded and restores the original value.
        unsafe {
            // When TMUX is set to a non-empty value, we're inside tmux
            std::env::set_var("TMUX", "/tmp/tmux-1000/default,12345,0");
            assert!(is_inside_tmux());

            // When TMUX is empty, we're not inside tmux
            std::env::set_var("TMUX", "");
            assert!(!is_inside_tmux());

            // When TMUX is unset, we're not inside tmux
            std::env::remove_var("TMUX");
            assert!(!is_inside_tmux());

            // Restore original value
            match original {
                Some(v) => std::env::set_var("TMUX", v),
                None => std::env::remove_var("TMUX"),
            }
        }
    }
}
