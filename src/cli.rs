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
    /// Register the current repo as a dual workspace
    Add {
        /// Short name for the repo (derived from directory name if omitted)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Create a new branch workspace for an existing repo
    Create {
        /// Repo name (must already be added)
        repo: String,

        /// Branch name
        branch: String,
    },

    /// Launch a workspace (clone, container, tmux session)
    Launch {
        /// Workspace to launch (e.g. lightfast-main)
        workspace: String,
    },

    /// List all workspaces and their status
    List,

    /// Destroy a workspace (stop container, remove clone)
    Destroy {
        /// Workspace to destroy (e.g. lightfast-feat__auth)
        workspace: String,
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

    /// Start the reverse proxy for browser access
    Proxy,

    /// Output shell RC for a container (used internally)
    #[command(name = "shell-rc", hide = true)]
    ShellRc {
        /// Container name
        container: String,
    },
}
