mod cli;
mod clone;
mod config;
mod container;
mod shell;
mod tmux;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => cmd_launch(),
        Some(Command::List) => cmd_list(),
        Some(Command::Destroy { workspace }) => cmd_destroy(workspace),
        Some(Command::Open { workspace }) => cmd_open(workspace),
        Some(Command::Urls { workspace }) => cmd_urls(workspace),
    }
}

fn cmd_launch() {
    println!("dual: launching workspace picker... (not yet implemented)");
}

fn cmd_list() {
    println!("dual: listing workspaces... (not yet implemented)");
}

fn cmd_destroy(workspace: Option<String>) {
    match workspace {
        Some(ws) => println!("dual: destroying workspace '{ws}'... (not yet implemented)"),
        None => println!("dual: destroying current workspace... (not yet implemented)"),
    }
}

fn cmd_open(workspace: Option<String>) {
    match workspace {
        Some(ws) => println!("dual: opening workspace '{ws}' in browser... (not yet implemented)"),
        None => println!("dual: opening current workspace in browser... (not yet implemented)"),
    }
}

fn cmd_urls(workspace: Option<String>) {
    match workspace {
        Some(ws) => println!("dual: showing URLs for workspace '{ws}'... (not yet implemented)"),
        None => println!("dual: showing all workspace URLs... (not yet implemented)"),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::Cli;
    use clap::Parser;

    #[test]
    fn no_args_is_launch() {
        let cli = Cli::parse_from(["dual"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn list_subcommand() {
        let cli = Cli::parse_from(["dual", "list"]);
        assert!(matches!(cli.command, Some(crate::cli::Command::List)));
    }

    #[test]
    fn destroy_without_workspace() {
        let cli = Cli::parse_from(["dual", "destroy"]);
        if let Some(crate::cli::Command::Destroy { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Destroy command");
        }
    }

    #[test]
    fn destroy_with_workspace() {
        let cli = Cli::parse_from(["dual", "destroy", "lightfast-main"]);
        if let Some(crate::cli::Command::Destroy { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("lightfast-main"));
        } else {
            panic!("expected Destroy command");
        }
    }

    #[test]
    fn open_without_workspace() {
        let cli = Cli::parse_from(["dual", "open"]);
        if let Some(crate::cli::Command::Open { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Open command");
        }
    }

    #[test]
    fn open_with_workspace() {
        let cli = Cli::parse_from(["dual", "open", "lightfast-feat-auth"]);
        if let Some(crate::cli::Command::Open { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("lightfast-feat-auth"));
        } else {
            panic!("expected Open command");
        }
    }

    #[test]
    fn urls_without_workspace() {
        let cli = Cli::parse_from(["dual", "urls"]);
        if let Some(crate::cli::Command::Urls { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Urls command");
        }
    }

    #[test]
    fn urls_with_workspace() {
        let cli = Cli::parse_from(["dual", "urls", "agent-os-main"]);
        if let Some(crate::cli::Command::Urls { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("agent-os-main"));
        } else {
            panic!("expected Urls command");
        }
    }
}
