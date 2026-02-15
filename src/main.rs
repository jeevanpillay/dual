use std::path::PathBuf;

use clap::Parser;
use dual::cli::{Cli, Command};
use dual::clone;
use dual::config;
use dual::container;
use dual::proxy;
use dual::shell;
use dual::state;
use dual::tmux;

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        None => cmd_default(),
        Some(Command::Add { name }) => cmd_add(name.as_deref()),
        Some(Command::Create { repo, branch }) => cmd_create(&repo, &branch),
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
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("\nRun `dual add` inside a repo to get started.");
            return 1;
        }
    };

    let workspaces = st.all_workspaces();
    if workspaces.is_empty() {
        println!("No workspaces. Run `dual add` inside a repo to get started.");
        return 0;
    }

    println!("Workspaces:\n");
    print_workspace_status(&st);
    println!("\nUse `dual launch <workspace>` to start a workspace.");
    println!("Use `dual add` to register a new repo.");
    0
}

/// Register the current repo as a dual workspace.
fn cmd_add(name: Option<&str>) -> i32 {
    // Detect git repo info from current directory
    let (repo_root, url, branch) = match detect_git_repo() {
        Ok(info) => info,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // Derive repo name
    let repo_name = match name {
        Some(n) => n.to_string(),
        None => derive_repo_name(&repo_root),
    };

    // Load or create state
    let mut st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // Check for duplicates
    if st.has_workspace(&repo_name, &branch) {
        eprintln!("error: workspace {}/{} already exists", repo_name, branch);
        return 1;
    }

    // Check for .dual.toml — if missing, create a default one
    let hints_path = repo_root.join(".dual.toml");
    if !hints_path.exists() {
        let hints = config::RepoHints::default();
        if let Err(e) = config::write_hints(&repo_root, &hints) {
            eprintln!("warning: failed to write .dual.toml: {e}");
        } else {
            println!("Created .dual.toml with defaults (image: node:20)");
            println!("Edit it to customize ports, image, setup command, and env vars.");
        }
    }

    // Add workspace entry
    let entry = state::WorkspaceEntry {
        repo: repo_name.clone(),
        url,
        branch: branch.clone(),
        path: Some(repo_root.to_string_lossy().to_string()),
    };

    if let Err(e) = st.add_workspace(entry) {
        eprintln!("error: {e}");
        return 1;
    }

    // Save state
    if let Err(e) = state::save(&st) {
        eprintln!("error: failed to save state: {e}");
        return 1;
    }

    let ws_id = config::workspace_id(&repo_name, &branch);
    println!("Added workspace: {ws_id}");
    println!("Use `dual launch {ws_id}` to start.");
    0
}

/// Create a new branch workspace for an existing repo.
fn cmd_create(repo: &str, branch: &str) -> i32 {
    let mut st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // Find an existing workspace for this repo
    let existing = st.workspaces_for_repo(repo);
    if existing.is_empty() {
        eprintln!("error: repo '{repo}' not found. Run `dual add` inside the repo first.");
        return 1;
    }

    // Check if this branch already exists
    if st.has_workspace(repo, branch) {
        eprintln!("error: workspace {repo}/{branch} already exists");
        return 1;
    }

    // Get URL from existing entry
    let url = existing[0].url.clone();

    // Add new entry (no explicit path — will be cloned on launch)
    let entry = state::WorkspaceEntry {
        repo: repo.to_string(),
        url,
        branch: branch.to_string(),
        path: None,
    };

    if let Err(e) = st.add_workspace(entry) {
        eprintln!("error: {e}");
        return 1;
    }

    if let Err(e) = state::save(&st) {
        eprintln!("error: failed to save state: {e}");
        return 1;
    }

    let ws_id = config::workspace_id(repo, branch);
    println!("Created workspace: {ws_id}");
    println!("Use `dual launch {ws_id}` to start.");
    0
}

/// Launch a specific workspace: clone → container → shell RC → tmux → attach.
fn cmd_launch(workspace: &str) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let entry = match st.resolve_workspace(workspace) {
        Some(e) => e,
        None => {
            eprintln!("error: unknown workspace '{workspace}'");
            eprintln!("\nConfigured workspaces:");
            for ws in st.all_workspaces() {
                let id = config::workspace_id(&ws.repo, &ws.branch);
                eprintln!("  {id}");
            }
            return 1;
        }
    };

    let workspace_root = st.workspace_root();
    let container_name = config::container_name(&entry.repo, &entry.branch);
    let session_name = tmux::session_name(&entry.repo, &entry.branch);

    // Step 1: Resolve workspace directory
    let workspace_dir = if let Some(ref path) = entry.path {
        let dir = PathBuf::from(path);
        if !dir.join(".git").exists() {
            eprintln!(
                "error: workspace path {} does not contain a git repo",
                dir.display()
            );
            return 1;
        }
        dir
    } else {
        match clone::clone_workspace(&workspace_root, &entry.repo, &entry.url, &entry.branch) {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("error: clone failed: {e}");
                return 1;
            }
        }
    };

    // Step 2: Load hints for image
    let hints = config::load_hints(&workspace_dir).unwrap_or_default();

    // Step 3: Ensure container exists and is running
    match container::status(&container_name) {
        container::ContainerStatus::Missing => {
            println!("Creating container {container_name}...");
            if let Err(e) = container::create(&container_name, &workspace_dir, &hints.image) {
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

    // Step 4: Write shell RC file
    let rc_path = match shell::write_rc_file(&container_name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: failed to write shell RC: {e}");
            return 1;
        }
    };

    // Step 5: Create tmux session if not alive
    if !tmux::is_alive(&session_name) {
        let source_cmd = shell::source_file_command(&rc_path);
        if let Err(e) = tmux::create_session(&session_name, &workspace_dir, Some(&source_cmd)) {
            eprintln!("error: tmux session creation failed: {e}");
            return 1;
        }
    }

    // Step 6: Attach
    println!("Attaching to {session_name}...");
    if let Err(e) = tmux::attach(&session_name) {
        eprintln!("error: tmux attach failed: {e}");
        return 1;
    }

    0
}

/// List all configured workspaces with their live status.
fn cmd_list() -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let workspaces = st.all_workspaces();
    if workspaces.is_empty() {
        println!("No workspaces configured.");
        return 0;
    }

    print_workspace_status(&st);
    0
}

/// Destroy a workspace: tmux → container → clone.
fn cmd_destroy(workspace: &str) -> i32 {
    let mut st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let entry = match st.resolve_workspace(workspace) {
        Some(e) => e.clone(),
        None => {
            eprintln!("error: unknown workspace '{workspace}'");
            return 1;
        }
    };

    let workspace_root = st.workspace_root();
    let container_name = config::container_name(&entry.repo, &entry.branch);
    let session_name = tmux::session_name(&entry.repo, &entry.branch);

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

    // Remove clone (only for non-explicit-path workspaces)
    if entry.path.is_none() && clone::workspace_exists(&workspace_root, &entry.repo, &entry.branch)
    {
        println!("Removing clone...");
        if let Err(e) = clone::remove_workspace(&workspace_root, &entry.repo, &entry.branch) {
            eprintln!("error: failed to remove clone: {e}");
            return 1;
        }
    }

    // Remove from state
    st.remove_workspace(&entry.repo, &entry.branch);
    if let Err(e) = state::save(&st) {
        eprintln!("warning: failed to save state: {e}");
    }

    println!("Workspace '{workspace}' destroyed.");
    0
}

/// Open workspace services in the default browser.
fn cmd_open(workspace: Option<String>) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let url_groups = proxy::workspace_urls(&st);
    if url_groups.is_empty() {
        println!("No URLs configured. Add 'ports' to .dual.toml in your repo.");
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
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let url_groups = proxy::workspace_urls(&st);
    if url_groups.is_empty() {
        println!("No URLs configured. Add 'ports' to .dual.toml in your repo.");
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
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(proxy::start(&st)) {
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
fn print_workspace_status(st: &state::WorkspaceState) {
    let workspace_root = st.workspace_root();
    for ws in st.all_workspaces() {
        let workspace_id = config::workspace_id(&ws.repo, &ws.branch);
        let container_name = config::container_name(&ws.repo, &ws.branch);
        let session_name = tmux::session_name(&ws.repo, &ws.branch);

        let clone_exists = if ws.path.is_some() {
            // Explicit path — check if the path exists
            ws.path
                .as_ref()
                .map(|p| PathBuf::from(p).join(".git").exists())
                .unwrap_or(false)
        } else {
            clone::workspace_exists(&workspace_root, &ws.repo, &ws.branch)
        };
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

/// Detect git repo info from the current directory.
fn detect_git_repo() -> Result<(PathBuf, String, String), String> {
    // Get repo root
    let root_output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|_| "git not found".to_string())?;
    if !root_output.status.success() {
        return Err("not inside a git repository".to_string());
    }
    let root = PathBuf::from(String::from_utf8_lossy(&root_output.stdout).trim());

    // Get remote URL
    let url_output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|_| "git not found".to_string())?;
    let url = if url_output.status.success() {
        String::from_utf8_lossy(&url_output.stdout)
            .trim()
            .to_string()
    } else {
        root.to_string_lossy().to_string() // local-only repo, use path as URL
    };

    // Get current branch
    let branch_output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|_| "git not found".to_string())?;
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    Ok((root, url, branch))
}

/// Derive a short repo name from a directory path.
/// "/Users/jeevan/code/lightfast" → "lightfast"
fn derive_repo_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo")
        .to_string()
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
    fn add_subcommand() {
        let cli = Cli::parse_from(["dual", "add"]);
        if let Some(Command::Add { name }) = cli.command {
            assert!(name.is_none());
        } else {
            panic!("expected Add command");
        }
    }

    #[test]
    fn add_with_name() {
        let cli = Cli::parse_from(["dual", "add", "--name", "myrepo"]);
        if let Some(Command::Add { name }) = cli.command {
            assert_eq!(name.as_deref(), Some("myrepo"));
        } else {
            panic!("expected Add command");
        }
    }

    #[test]
    fn create_subcommand() {
        let cli = Cli::parse_from(["dual", "create", "lightfast", "feat/auth"]);
        if let Some(Command::Create { repo, branch }) = cli.command {
            assert_eq!(repo, "lightfast");
            assert_eq!(branch, "feat/auth");
        } else {
            panic!("expected Create command");
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

    #[test]
    fn derive_repo_name_from_path() {
        use std::path::Path;
        assert_eq!(
            super::derive_repo_name(Path::new("/Users/jeevan/code/lightfast")),
            "lightfast"
        );
        assert_eq!(
            super::derive_repo_name(Path::new("/home/user/projects/my-app")),
            "my-app"
        );
    }
}
