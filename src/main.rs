use clap::Parser;
use dual::cli::{Cli, Command};
use dual::clone;
use dual::config;
use dual::container;
use dual::proxy;
use dual::shell;
use dual::tmux;

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        None => cmd_default(),
        Some(Command::Launch { workspace }) => cmd_launch(&workspace),
        Some(Command::List) => cmd_list(),
        Some(Command::Destroy { workspace }) => cmd_destroy(&workspace),
        Some(Command::Open { workspace }) => cmd_open(workspace),
        Some(Command::Urls { workspace }) => cmd_urls(workspace),
        Some(Command::Proxy) => cmd_proxy(),
        Some(Command::ShellRc { container }) => cmd_shell_rc(&container),
    };

    std::process::exit(exit_code);
}

/// Default (no subcommand): show workspace list with launch hint.
fn cmd_default() -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("\nCreate a dual.toml to get started.");
            return 1;
        }
    };

    let workspaces = cfg.all_workspaces();
    if workspaces.is_empty() {
        println!("No workspaces configured. Add repos to dual.toml.");
        return 0;
    }

    println!("Workspaces:\n");
    print_workspace_status(&cfg);
    println!("\nUse `dual launch <workspace>` to start a workspace.");
    println!("Use `dual list` for detailed status.");
    0
}

/// Launch a specific workspace: clone → container → shell RC → tmux → attach.
fn cmd_launch(workspace: &str) -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let (repo, branch) = match cfg.resolve_workspace(workspace) {
        Some(r) => r,
        None => {
            eprintln!("error: unknown workspace '{workspace}'");
            eprintln!("\nConfigured workspaces:");
            for (repo, branch) in cfg.all_workspaces() {
                let id = format!("{}-{}", repo.name, config::encode_branch(branch));
                eprintln!("  {id}");
            }
            return 1;
        }
    };

    let container_name = config::DualConfig::container_name(&repo.name, &branch);
    let session_name = tmux::session_name(&repo.name, &branch);

    // Step 1: Ensure clone exists
    let workspace_dir = match clone::clone_workspace(&cfg, &repo.name, &repo.url, &branch) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("error: clone failed: {e}");
            return 1;
        }
    };

    // Step 2: Ensure container exists and is running
    match container::status(&container_name) {
        container::ContainerStatus::Missing => {
            println!("Creating container {container_name}...");
            if let Err(e) = container::create(&cfg, &repo.name, &branch, None) {
                eprintln!("error: container create failed: {e}");
                return 1;
            }
            if let Err(e) = container::start(&container_name) {
                eprintln!("error: container start failed: {e}");
                return 1;
            }
        }
        container::ContainerStatus::Stopped => {
            println!("Starting container {container_name}...");
            if let Err(e) = container::start(&container_name) {
                eprintln!("error: container start failed: {e}");
                return 1;
            }
        }
        container::ContainerStatus::Running => {}
    }

    // Step 3: Write shell RC file
    let rc_path = match shell::write_rc_file(&container_name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: failed to write shell RC: {e}");
            return 1;
        }
    };

    // Step 4: Create tmux session if not alive
    if !tmux::is_alive(&session_name) {
        let source_cmd = shell::source_file_command(&rc_path);
        if let Err(e) = tmux::create_session(&session_name, &workspace_dir, Some(&source_cmd)) {
            eprintln!("error: tmux session creation failed: {e}");
            return 1;
        }
    }

    // Step 5: Attach
    println!("Attaching to {session_name}...");
    if let Err(e) = tmux::attach(&session_name) {
        eprintln!("error: tmux attach failed: {e}");
        return 1;
    }

    0
}

/// List all configured workspaces with their live status.
fn cmd_list() -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let workspaces = cfg.all_workspaces();
    if workspaces.is_empty() {
        println!("No workspaces configured.");
        return 0;
    }

    print_workspace_status(&cfg);
    0
}

/// Destroy a workspace: tmux → container → clone.
fn cmd_destroy(workspace: &str) -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let (repo, branch) = match cfg.resolve_workspace(workspace) {
        Some(r) => r,
        None => {
            eprintln!("error: unknown workspace '{workspace}'");
            return 1;
        }
    };

    let container_name = config::DualConfig::container_name(&repo.name, &branch);
    let session_name = tmux::session_name(&repo.name, &branch);

    // Destroy tmux session
    if tmux::is_alive(&session_name) {
        println!("Destroying tmux session {session_name}...");
        if let Err(e) = tmux::destroy(&session_name) {
            eprintln!("warning: tmux destroy failed: {e}");
        }
    }

    // Stop and remove container
    match container::status(&container_name) {
        container::ContainerStatus::Running => {
            println!("Stopping container {container_name}...");
            if let Err(e) = container::stop(&container_name) {
                eprintln!("warning: container stop failed: {e}");
            }
            println!("Removing container {container_name}...");
            if let Err(e) = container::destroy(&container_name) {
                eprintln!("warning: container remove failed: {e}");
            }
        }
        container::ContainerStatus::Stopped => {
            println!("Removing container {container_name}...");
            if let Err(e) = container::destroy(&container_name) {
                eprintln!("warning: container remove failed: {e}");
            }
        }
        container::ContainerStatus::Missing => {}
    }

    // Remove clone
    if clone::workspace_exists(&cfg, &repo.name, &branch) {
        println!("Removing clone...");
        if let Err(e) = clone::remove_workspace(&cfg, &repo.name, &branch) {
            eprintln!("error: failed to remove clone: {e}");
            return 1;
        }
    }

    println!("Workspace '{workspace}' destroyed.");
    0
}

/// Open workspace services in the default browser.
fn cmd_open(workspace: Option<String>) -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let url_groups = proxy::workspace_urls(&cfg);
    if url_groups.is_empty() {
        println!("No URLs configured. Add 'ports' to repo config in dual.toml.");
        return 0;
    }

    // Filter by workspace if specified
    let filtered: Vec<_> = match &workspace {
        Some(ws) => url_groups.into_iter().filter(|(id, _)| id == ws).collect(),
        None => url_groups,
    };

    if filtered.is_empty() {
        if let Some(ws) = &workspace {
            eprintln!("error: no URLs for workspace '{ws}'");
        }
        return 1;
    }

    for (_, urls) in &filtered {
        for url_line in urls {
            // Extract the URL part (after the status icon)
            let url = url_line
                .trim()
                .trim_start_matches('\u{25cf}')
                .trim_start_matches('\u{25cb}')
                .trim();
            let http_url = format!("http://{url}");
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&http_url).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open")
                .arg(&http_url)
                .spawn();
            println!("Opening {http_url}");
        }
    }

    0
}

/// Show workspace URLs.
fn cmd_urls(workspace: Option<String>) -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let url_groups = proxy::workspace_urls(&cfg);
    if url_groups.is_empty() {
        println!("No URLs configured. Add 'ports' to repo config in dual.toml.");
        return 0;
    }

    // Filter by workspace if specified
    let filtered: Vec<_> = match &workspace {
        Some(ws) => url_groups.into_iter().filter(|(id, _)| id == ws).collect(),
        None => url_groups,
    };

    for (workspace_id, urls) in &filtered {
        println!("{workspace_id}");
        for url in urls {
            println!("{url}");
        }
        println!();
    }

    0
}

/// Start the reverse proxy.
fn cmd_proxy() -> i32 {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(proxy::start(&cfg)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: proxy failed: {e}");
            1
        }
    }
}

/// Output shell RC for a container (used by `eval "$(dual shell-rc <name>)"`).
fn cmd_shell_rc(container_name: &str) -> i32 {
    print!("{}", shell::generate_rc(container_name));
    0
}

/// Print workspace status table.
fn print_workspace_status(cfg: &config::DualConfig) {
    for (repo, branch) in cfg.all_workspaces() {
        let workspace_id = format!("{}-{}", repo.name, config::encode_branch(branch));
        let container_name = config::DualConfig::container_name(&repo.name, branch);
        let session_name = tmux::session_name(&repo.name, branch);

        let clone_exists = clone::workspace_exists(cfg, &repo.name, branch);
        let container_st = container::status(&container_name);
        let tmux_alive = tmux::is_alive(&session_name);

        let status_icon = match (&container_st, tmux_alive) {
            (container::ContainerStatus::Running, true) => "\u{25cf} attached",
            (container::ContainerStatus::Running, false) => "\u{25cf} running",
            (container::ContainerStatus::Stopped, _) => "\u{25cb} stopped",
            (container::ContainerStatus::Missing, _) if clone_exists => "\u{25cb} stopped",
            (container::ContainerStatus::Missing, _) => "\u{25cc} lazy",
        };

        println!("  {workspace_id:<30} {status_icon}");
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use dual::cli::{Cli, Command};

    #[test]
    fn no_args_is_default() {
        let cli = Cli::parse_from(["dual"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn list_subcommand() {
        let cli = Cli::parse_from(["dual", "list"]);
        assert!(matches!(cli.command, Some(Command::List)));
    }

    #[test]
    fn launch_subcommand() {
        let cli = Cli::parse_from(["dual", "launch", "lightfast-main"]);
        if let Some(Command::Launch { workspace }) = cli.command {
            assert_eq!(workspace, "lightfast-main");
        } else {
            panic!("expected Launch command");
        }
    }

    #[test]
    fn destroy_subcommand() {
        let cli = Cli::parse_from(["dual", "destroy", "lightfast-main"]);
        if let Some(Command::Destroy { workspace }) = cli.command {
            assert_eq!(workspace, "lightfast-main");
        } else {
            panic!("expected Destroy command");
        }
    }

    #[test]
    fn open_without_workspace() {
        let cli = Cli::parse_from(["dual", "open"]);
        if let Some(Command::Open { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Open command");
        }
    }

    #[test]
    fn open_with_workspace() {
        let cli = Cli::parse_from(["dual", "open", "lightfast-feat__auth"]);
        if let Some(Command::Open { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("lightfast-feat__auth"));
        } else {
            panic!("expected Open command");
        }
    }

    #[test]
    fn urls_without_workspace() {
        let cli = Cli::parse_from(["dual", "urls"]);
        if let Some(Command::Urls { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Urls command");
        }
    }

    #[test]
    fn urls_with_workspace() {
        let cli = Cli::parse_from(["dual", "urls", "agent-os-main"]);
        if let Some(Command::Urls { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("agent-os-main"));
        } else {
            panic!("expected Urls command");
        }
    }

    #[test]
    fn proxy_subcommand() {
        let cli = Cli::parse_from(["dual", "proxy"]);
        assert!(matches!(cli.command, Some(Command::Proxy)));
    }

    #[test]
    fn shell_rc_subcommand() {
        let cli = Cli::parse_from(["dual", "shell-rc", "dual-lightfast-main"]);
        if let Some(Command::ShellRc { container }) = cli.command {
            assert_eq!(container, "dual-lightfast-main");
        } else {
            panic!("expected ShellRc command");
        }
    }
}
