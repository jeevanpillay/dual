use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dual",
    version,
    about = "Terminal workspace orchestrator for parallel multi-repo development"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all workspaces and their status
    List,

    /// Destroy a workspace (stop container, remove clone)
    Destroy {
        /// Workspace to destroy (e.g. lightfast-feat-auth)
        workspace: Option<String>,
    },

    /// Open all services for a workspace in the browser
    Open {
        /// Workspace to open (defaults to current)
        workspace: Option<String>,
    },

    /// List running workspace URLs
    Urls {
        /// Workspace to show URLs for (defaults to all)
        workspace: Option<String>,
    },
}
