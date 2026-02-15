use std::path::Path;
use std::process::Command;

use crate::backend::{BackendError, MultiplexerBackend};

/// Session name prefix for dual-managed sessions.
const SESSION_PREFIX: &str = "dual-";

/// Terminal multiplexer backend using tmux.
pub struct TmuxBackend;

impl TmuxBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TmuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiplexerBackend for TmuxBackend {
    fn is_available(&self) -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn create_session(
        &self,
        session_name: &str,
        cwd: &Path,
        init_cmd: Option<&str>,
    ) -> Result<(), BackendError> {
        let output = Command::new("tmux")
            .args(build_new_session_args(session_name, cwd))
            .output()
            .map_err(|e| BackendError::NotFound {
                multiplexer: "tmux".to_string(),
                detail: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(BackendError::Failed {
                multiplexer: "tmux".to_string(),
                operation: "new-session".to_string(),
                session: session_name.to_string(),
                stderr,
            });
        }

        // Send initial command if provided
        if let Some(cmd) = init_cmd {
            self.send_keys(session_name, cmd)?;
        }

        Ok(())
    }

    fn attach(&self, session_name: &str) -> Result<(), BackendError> {
        let (cmd, op) = if self.is_inside() {
            (["switch-client", "-t", session_name], "switch-client")
        } else {
            (["attach-session", "-t", session_name], "attach-session")
        };

        let status =
            Command::new("tmux")
                .args(cmd)
                .status()
                .map_err(|e| BackendError::NotFound {
                    multiplexer: "tmux".to_string(),
                    detail: e.to_string(),
                })?;

        if !status.success() {
            return Err(BackendError::Failed {
                multiplexer: "tmux".to_string(),
                operation: op.to_string(),
                session: session_name.to_string(),
                stderr: format!("exit code: {}", status.code().unwrap_or(-1)),
            });
        }

        Ok(())
    }

    fn detach(&self, session_name: &str) -> Result<(), BackendError> {
        tmux_simple(&["detach-client", "-s", session_name])
    }

    fn destroy(&self, session_name: &str) -> Result<(), BackendError> {
        tmux_simple(&["kill-session", "-t", session_name])
    }

    fn is_alive(&self, session_name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn list_sessions(&self) -> Vec<String> {
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

    fn send_keys(&self, session_name: &str, keys: &str) -> Result<(), BackendError> {
        tmux_simple(&["send-keys", "-t", session_name, keys, "Enter"])
    }

    fn is_inside(&self) -> bool {
        std::env::var("TMUX").is_ok_and(|v| !v.is_empty())
    }
}

/// Build the arguments for `tmux new-session` (public for testing).
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

fn tmux_simple(args: &[&str]) -> Result<(), BackendError> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| BackendError::NotFound {
            multiplexer: "tmux".to_string(),
            detail: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(BackendError::Failed {
            multiplexer: "tmux".to_string(),
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
    fn session_prefix_is_dual() {
        assert_eq!(SESSION_PREFIX, "dual-");
    }

    #[test]
    fn tmux_availability_check_runs() {
        let backend = TmuxBackend::new();
        // This test just verifies the function doesn't panic
        let _available = backend.is_available();
    }

    #[test]
    fn is_inside_detects_env() {
        let backend = TmuxBackend::new();

        // Save and restore TMUX env var
        let original = std::env::var("TMUX").ok();

        // SAFETY: This test runs single-threaded and restores the original value.
        unsafe {
            // When TMUX is set to a non-empty value, we're inside tmux
            std::env::set_var("TMUX", "/tmp/tmux-1000/default,12345,0");
            assert!(backend.is_inside());

            // When TMUX is empty, we're not inside tmux
            std::env::set_var("TMUX", "");
            assert!(!backend.is_inside());

            // When TMUX is unset, we're not inside tmux
            std::env::remove_var("TMUX");
            assert!(!backend.is_inside());

            // Restore original value
            match original {
                Some(v) => std::env::set_var("TMUX", v),
                None => std::env::remove_var("TMUX"),
            }
        }
    }

    #[test]
    fn default_impl_works() {
        let backend = TmuxBackend::default();
        // Just verify it compiles and doesn't panic
        let _ = backend.is_inside();
    }
}
