use std::path::Path;

/// Abstraction over terminal multiplexers (tmux, zellij, etc.)
///
/// Each implementation wraps a specific multiplexer binary and provides
/// session lifecycle management. The trait is object-safe so command handlers
/// can accept `&dyn MultiplexerBackend`.
pub trait MultiplexerBackend {
    /// Check if the multiplexer binary is installed and available.
    fn is_available(&self) -> bool;

    /// Create a new detached session with the given name and working directory.
    /// Optionally send an initial command after creation.
    fn create_session(
        &self,
        session_name: &str,
        cwd: &Path,
        init_cmd: Option<&str>,
    ) -> Result<(), BackendError>;

    /// Attach the current terminal to an existing session.
    /// If already inside the multiplexer, use switch-client instead.
    fn attach(&self, session_name: &str) -> Result<(), BackendError>;

    /// Detach the current client from a session.
    fn detach(&self, session_name: &str) -> Result<(), BackendError>;

    /// Destroy a session and all its windows/panes.
    fn destroy(&self, session_name: &str) -> Result<(), BackendError>;

    /// Check if a session exists and has running processes.
    fn is_alive(&self, session_name: &str) -> bool;

    /// List all Dual-managed sessions (filtered by `dual-` prefix).
    fn list_sessions(&self) -> Vec<String>;

    /// Send keystrokes to a session's active pane.
    fn send_keys(&self, session_name: &str, keys: &str) -> Result<(), BackendError>;

    /// Check if we're currently inside this multiplexer.
    fn is_inside(&self) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("{multiplexer} not found: {detail}")]
    NotFound { multiplexer: String, detail: String },

    #[error("{multiplexer} {operation} failed for session '{session}': {stderr}")]
    Failed {
        multiplexer: String,
        operation: String,
        session: String,
        stderr: String,
    },
}
