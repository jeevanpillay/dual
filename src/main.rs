use std::path::PathBuf;

use clap::Parser;
use dual::backend::MultiplexerBackend;
use dual::cli::{Cli, Command};
use dual::clone;
use dual::config;
use dual::container;
use dual::proxy;
use dual::shared;
use dual::shell;
use dual::state;
use dual::tmux_backend::TmuxBackend;
use dual::tui;
use tracing::{debug, error, info, warn};

fn main() {
    // Initialize tracing with DUAL_LOG env var (default: info)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DUAL_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .without_time()
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let backend = TmuxBackend::new();

    let exit_code = match cli.command {
        None => cmd_default(&backend),
        Some(Command::Add { name }) => cmd_add(name.as_deref()),
        Some(Command::Create { branch, repo }) => cmd_create(repo.as_deref(), &branch),
        Some(Command::Launch { workspace }) => cmd_launch(workspace.as_deref(), &backend),
        Some(Command::List) => cmd_list(&backend),
        Some(Command::Destroy { workspace }) => cmd_destroy(workspace.as_deref(), &backend),
        Some(Command::Open { workspace }) => cmd_open(workspace),
        Some(Command::Urls { workspace }) => cmd_urls(workspace),
        Some(Command::Sync { workspace }) => cmd_sync(workspace),
        Some(Command::Proxy) => cmd_proxy(),
        Some(Command::ShellRc { container }) => cmd_shell_rc(&container),
    };

    std::process::exit(exit_code);
}

/// Default (no subcommand): launch TUI workspace browser.
fn cmd_default(backend: &dyn MultiplexerBackend) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            info!("Run `dual add` inside a repo to get started.");
            return 1;
        }
    };

    if st.all_workspaces().is_empty() {
        info!("No workspaces. Run `dual add` inside a repo to get started.");
        return 0;
    }

    match tui::run(&st, backend) {
        Ok(Some(workspace_id)) => {
            // User selected a workspace — launch it
            cmd_launch(Some(&workspace_id), backend)
        }
        Ok(None) => 0, // User quit
        Err(e) => {
            error!("TUI error: {e}");
            1
        }
    }
}

/// Register the current repo as a dual workspace.
fn cmd_add(name: Option<&str>) -> i32 {
    // Detect git repo info from current directory
    let (repo_root, url, branch) = match detect_git_repo() {
        Ok(info) => info,
        Err(e) => {
            error!("{e}");
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
            error!("{e}");
            return 1;
        }
    };

    // Check for duplicates
    if st.has_workspace(&repo_name, &branch) {
        error!("workspace {}/{} already exists", repo_name, branch);
        return 1;
    }

    // Check for .dual.toml — if missing, create a default one with helpful comments
    let hints_path = repo_root.join(".dual.toml");
    if !hints_path.exists() {
        if let Err(e) = config::write_default_hints(&repo_root) {
            warn!("failed to write .dual.toml: {e}");
        } else {
            info!("Created .dual.toml with defaults (image: node:20)");
            info!("Edit it to customize ports, image, setup command, and env vars.");
        }
    }

    // Initialize shared directory if [shared] is configured
    let hints = config::load_hints(&repo_root).unwrap_or_default();
    if let Some(ref shared_config) = hints.shared
        && !shared_config.files.is_empty()
    {
        match shared::ensure_shared_dir(&repo_name) {
            Ok(shared_dir) => {
                match shared::init_from_main(&repo_root, &shared_dir, &shared_config.files) {
                    Ok(moved) => {
                        for f in &moved {
                            info!("  shared: {f} → ~/.dual/shared/{repo_name}/");
                        }
                    }
                    Err(e) => warn!("shared init failed: {e}"),
                }
            }
            Err(e) => warn!("could not create shared directory: {e}"),
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
        error!("{e}");
        return 1;
    }

    // Save state
    if let Err(e) = state::save(&st) {
        error!("failed to save state: {e}");
        return 1;
    }

    let ws_id = config::workspace_id(&repo_name, &branch);
    info!("Added workspace: {ws_id}");
    info!("Use `dual launch {ws_id}` to start.");
    0
}

/// Create a new branch workspace for an existing repo.
fn cmd_create(repo_arg: Option<&str>, branch: &str) -> i32 {
    let mut st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    // Resolve repo name: use --repo if provided, otherwise auto-detect from cwd
    let repo = match repo_arg {
        Some(r) => r.to_string(),
        None => match detect_repo_from_cwd(&st) {
            Some(r) => {
                info!("Auto-detected repo: {r}");
                r
            }
            None => {
                error!("could not detect repo from current directory");
                info!("Usage: dual create <branch> --repo <name>");
                info!("Or run from inside a repo that was added with `dual add`.");
                return 1;
            }
        },
    };

    // Find an existing workspace for this repo
    let existing = st.workspaces_for_repo(&repo);
    if existing.is_empty() {
        error!("repo '{repo}' not found. Run `dual add` inside the repo first.");
        return 1;
    }

    // Check if this branch already exists
    if st.has_workspace(&repo, branch) {
        error!("workspace {repo}/{branch} already exists");
        return 1;
    }

    // Get URL from existing entry
    let url = existing[0].url.clone();

    // Add new entry (no explicit path — will be cloned on launch)
    let entry = state::WorkspaceEntry {
        repo: repo.clone(),
        url,
        branch: branch.to_string(),
        path: None,
    };

    if let Err(e) = st.add_workspace(entry) {
        error!("{e}");
        return 1;
    }

    if let Err(e) = state::save(&st) {
        error!("failed to save state: {e}");
        return 1;
    }

    let ws_id = config::workspace_id(&repo, branch);
    info!("Created workspace: {ws_id}");
    info!("Use `dual launch {ws_id}` to start.");
    0
}

/// Launch a specific workspace: clone → container → shell RC → tmux → attach.
fn cmd_launch(workspace_arg: Option<&str>, backend: &dyn MultiplexerBackend) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    // Resolve workspace: use arg if provided, otherwise auto-detect from cwd
    let entry = if let Some(ws) = workspace_arg {
        match st.resolve_workspace(ws) {
            Some(e) => e,
            None => {
                error!("unknown workspace '{ws}'");
                info!("Configured workspaces:");
                for w in st.all_workspaces() {
                    let id = config::workspace_id(&w.repo, &w.branch);
                    info!("  {id}");
                }
                return 1;
            }
        }
    } else {
        match detect_workspace(&st) {
            Some(e) => {
                let ws_id = config::workspace_id(&e.repo, &e.branch);
                info!("Auto-detected workspace: {ws_id}");
                st.resolve_workspace(&ws_id).unwrap()
            }
            None => {
                error!("could not detect workspace from current directory");
                info!("Usage: dual launch [workspace]");
                return 1;
            }
        }
    };

    let workspace_root = st.workspace_root();
    let container_name = config::container_name(&entry.repo, &entry.branch);
    let session_name = config::session_name(&entry.repo, &entry.branch);
    debug!(
        repo = %entry.repo,
        branch = %entry.branch,
        "resolved workspace"
    );

    // Step 1: Resolve workspace directory
    let workspace_dir = if let Some(ref path) = entry.path {
        let dir = PathBuf::from(path);
        if !dir.join(".git").exists() {
            error!(
                "workspace path {} does not contain a git repo",
                dir.display()
            );
            return 1;
        }
        dir
    } else {
        // Try to clone from local main workspace first (fast, hardlinks)
        let target_dir = config::workspace_dir(&workspace_root, &entry.repo, &entry.branch);
        let main_workspace_path = st
            .workspaces_for_repo(&entry.repo)
            .into_iter()
            .find(|ws| ws.path.is_some())
            .and_then(|ws| ws.path.as_ref().map(PathBuf::from));

        match main_workspace_path {
            Some(main_path) if main_path.join(".git").exists() => {
                info!("Cloning from local main workspace...");
                match clone::clone_from_local(&main_path, &target_dir, &entry.branch) {
                    Ok(dir) => dir,
                    Err(e) => {
                        error!("local clone failed: {e}");
                        return 1;
                    }
                }
            }
            _ => {
                // Fallback: clone from remote URL
                match clone::clone_workspace(
                    &workspace_root,
                    &entry.repo,
                    &entry.url,
                    &entry.branch,
                ) {
                    Ok(dir) => dir,
                    Err(e) => {
                        error!("clone failed: {e}");
                        return 1;
                    }
                }
            }
        }
    };

    // Step 2: Handle shared files
    let hints = config::load_hints(&workspace_dir).unwrap_or_default();
    if let Some(ref shared_config) = hints.shared
        && !shared_config.files.is_empty()
        && let Ok(shared_dir) = shared::ensure_shared_dir(&entry.repo)
    {
        if entry.path.is_some() {
            // Main workspace: ensure shared files are initialized
            match shared::init_from_main(&workspace_dir, &shared_dir, &shared_config.files) {
                Ok(moved) => {
                    for f in &moved {
                        info!("  shared: {f} → ~/.dual/shared/{}/", entry.repo);
                    }
                }
                Err(e) => warn!("shared init failed: {e}"),
            }
        } else {
            // Branch workspace: copy shared files
            match shared::copy_to_branch(&workspace_dir, &shared_dir, &shared_config.files) {
                Ok(copied) => {
                    for f in &copied {
                        info!("  shared: copied {f}");
                    }
                }
                Err(e) => warn!("shared copy failed: {e}"),
            }
        }
    }

    // Step 3: Ensure container exists and is running
    let is_new_container = matches!(
        container::status(&container_name),
        container::ContainerStatus::Missing
    );
    match container::status(&container_name) {
        container::ContainerStatus::Missing => {
            info!("Creating container {container_name}...");
            if let Err(e) = container::create(
                &container_name,
                &workspace_dir,
                &hints.image,
                &hints.env,
                &hints.anonymous_volumes,
            ) {
                error!("container create failed: {e}");
                return 1;
            }
            if let Err(e) = container::start(&container_name) {
                error!("container start failed: {e}");
                return 1;
            }
        }
        container::ContainerStatus::Stopped => {
            info!("Starting container {container_name}...");
            if let Err(e) = container::start(&container_name) {
                error!("container start failed: {e}");
                return 1;
            }
        }
        container::ContainerStatus::Running => {}
    }

    // Step 3.5: Run setup command on new containers
    if is_new_container && let Some(ref setup) = hints.setup {
        info!("Running setup: {setup}");
        if let Err(e) = container::exec_setup(&container_name, setup) {
            error!("setup failed: {e}");
            return 1;
        }
    }

    // Step 4: Write shell RC file
    let rc_path = match shell::write_rc_file(&container_name, &hints.extra_commands) {
        Ok(p) => p,
        Err(e) => {
            error!("failed to write shell RC: {e}");
            return 1;
        }
    };

    // Step 5: Create tmux session if not alive
    if !backend.is_alive(&session_name) {
        let source_cmd = shell::source_file_command(&rc_path);
        if let Err(e) = backend.create_session(&session_name, &workspace_dir, Some(&source_cmd)) {
            error!("session creation failed: {e}");
            return 1;
        }
    }

    // Step 6: Attach
    info!("Attaching to {session_name}...");
    if let Err(e) = backend.attach(&session_name) {
        error!("attach failed: {e}");
        return 1;
    }

    0
}

/// List all configured workspaces with their live status.
fn cmd_list(backend: &dyn MultiplexerBackend) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    let workspaces = st.all_workspaces();
    if workspaces.is_empty() {
        info!("No workspaces configured.");
        return 0;
    }

    print_workspace_status(&st, backend);
    0
}

/// Destroy a workspace: tmux → container → clone.
fn cmd_destroy(workspace_arg: Option<&str>, backend: &dyn MultiplexerBackend) -> i32 {
    let mut st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    // Resolve workspace: use arg if provided, otherwise auto-detect from cwd
    let workspace;
    let entry = if let Some(ws) = workspace_arg {
        workspace = ws.to_string();
        match st.resolve_workspace(ws) {
            Some(e) => e.clone(),
            None => {
                error!("unknown workspace '{ws}'");
                return 1;
            }
        }
    } else {
        match detect_workspace(&st) {
            Some(e) => {
                workspace = config::workspace_id(&e.repo, &e.branch);
                info!("Auto-detected workspace: {workspace}");
                e
            }
            None => {
                error!("could not detect workspace from current directory");
                info!("Usage: dual destroy [workspace]");
                return 1;
            }
        }
    };

    let workspace_root = st.workspace_root();
    let container_name = config::container_name(&entry.repo, &entry.branch);
    let session_name = config::session_name(&entry.repo, &entry.branch);

    // Destroy tmux session
    if backend.is_alive(&session_name) {
        info!("Destroying session {session_name}...");
        if let Err(e) = backend.destroy(&session_name) {
            warn!("session destroy failed: {e}");
        }
    }

    // Stop and remove container
    match container::status(&container_name) {
        container::ContainerStatus::Running => {
            info!("Stopping container {container_name}...");
            if let Err(e) = container::stop(&container_name) {
                warn!("container stop failed: {e}");
            }
            info!("Removing container {container_name}...");
            if let Err(e) = container::destroy(&container_name) {
                warn!("container remove failed: {e}");
            }
        }
        container::ContainerStatus::Stopped => {
            info!("Removing container {container_name}...");
            if let Err(e) = container::destroy(&container_name) {
                warn!("container remove failed: {e}");
            }
        }
        container::ContainerStatus::Missing => {}
    }

    // Remove clone (only for non-explicit-path workspaces)
    if entry.path.is_none() && clone::workspace_exists(&workspace_root, &entry.repo, &entry.branch)
    {
        info!("Removing clone...");
        if let Err(e) = clone::remove_workspace(&workspace_root, &entry.repo, &entry.branch) {
            error!("failed to remove clone: {e}");
            return 1;
        }
    }

    // Remove from state
    st.remove_workspace(&entry.repo, &entry.branch);
    if let Err(e) = state::save(&st) {
        warn!("failed to save state: {e}");
    }

    info!("Workspace '{workspace}' destroyed.");
    0
}

/// Open workspace services in the default browser.
fn cmd_open(workspace: Option<String>) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    let url_groups = proxy::workspace_urls(&st);
    if url_groups.is_empty() {
        info!("No URLs configured. Add 'ports' to .dual.toml in your repo.");
        return 0;
    }

    // Filter by workspace if specified
    let filtered: Vec<_> = match &workspace {
        Some(ws) => url_groups.into_iter().filter(|(id, _)| id == ws).collect(),
        None => url_groups,
    };

    if filtered.is_empty() {
        if let Some(ws) = &workspace {
            error!("no URLs for workspace '{ws}'");
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
            info!("Opening {http_url}");
        }
    }

    0
}

/// Show workspace URLs.
fn cmd_urls(workspace: Option<String>) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    let url_groups = proxy::workspace_urls(&st);
    if url_groups.is_empty() {
        info!("No URLs configured. Add 'ports' to .dual.toml in your repo.");
        return 0;
    }

    // Filter by workspace if specified
    let filtered: Vec<_> = match &workspace {
        Some(ws) => url_groups.into_iter().filter(|(id, _)| id == ws).collect(),
        None => url_groups,
    };

    for (workspace_id, urls) in &filtered {
        info!("{workspace_id}");
        for url in urls {
            info!("{url}");
        }
        info!("");
    }

    0
}

/// Start the reverse proxy.
fn cmd_proxy() -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(proxy::start(&st)) {
        Ok(()) => 0,
        Err(e) => {
            error!("proxy failed: {e}");
            1
        }
    }
}

/// Output shell RC for a container (used by `eval "$(dual shell-rc <name>)"`).
fn cmd_shell_rc(container_name: &str) -> i32 {
    print!("{}", shell::generate_rc(container_name, &[]));
    0
}

/// Sync shared config files for a workspace.
fn cmd_sync(workspace_arg: Option<String>) -> i32 {
    let st = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    // Resolve which workspace we're syncing
    let entry = if let Some(ws) = workspace_arg {
        match st.resolve_workspace(&ws) {
            Some(e) => e.clone(),
            None => {
                error!("unknown workspace '{ws}'");
                return 1;
            }
        }
    } else {
        match detect_workspace(&st) {
            Some(e) => e,
            None => {
                error!("not inside a dual workspace");
                info!("Usage: dual sync [workspace]");
                return 1;
            }
        }
    };

    // Load hints
    let workspace_dir = st.workspace_dir(&entry);
    let hints = config::load_hints(&workspace_dir).unwrap_or_default();
    let shared_config = match &hints.shared {
        Some(s) if !s.files.is_empty() => s,
        _ => {
            error!("no [shared] section in .dual.toml (or files list is empty)");
            return 1;
        }
    };

    let shared_dir = match shared::ensure_shared_dir(&entry.repo) {
        Ok(d) => d,
        Err(e) => {
            error!("{e}");
            return 1;
        }
    };

    let is_main = entry.path.is_some();

    if is_main {
        // Main workspace: init shared dir, then prompt to sync all branches
        match shared::init_from_main(&workspace_dir, &shared_dir, &shared_config.files) {
            Ok(moved) => {
                for f in &moved {
                    info!("  moved {f} → shared/");
                }
            }
            Err(e) => {
                error!("{e}");
                return 1;
            }
        }

        // Prompt to sync all branches
        let branches: Vec<_> = st
            .workspaces_for_repo(&entry.repo)
            .into_iter()
            .filter(|ws| ws.path.is_none())
            .collect();

        if branches.is_empty() {
            info!("No branch workspaces to sync.");
            return 0;
        }

        // Interactive prompt — use println! directly since this is user interaction
        println!(
            "\nSync shared files to ALL {} branch workspace(s)? [y/N]",
            branches.len()
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap_or(0);
        if !input.trim().eq_ignore_ascii_case("y") {
            info!("Cancelled.");
            return 0;
        }

        for branch_entry in &branches {
            let branch_dir = st.workspace_dir(branch_entry);
            if !branch_dir.exists() {
                continue; // Not yet cloned
            }
            let ws_id = config::workspace_id(&branch_entry.repo, &branch_entry.branch);
            match shared::copy_to_branch(&branch_dir, &shared_dir, &shared_config.files) {
                Ok(copied) => {
                    info!("{ws_id}: synced {} file(s)", copied.len());
                }
                Err(e) => error!("{ws_id}: {e}"),
            }
        }
    } else {
        // Branch workspace: copy from shared dir
        match shared::copy_to_branch(&workspace_dir, &shared_dir, &shared_config.files) {
            Ok(copied) => {
                if copied.is_empty() {
                    info!(
                        "No shared files available yet. Run `dual sync` in the main workspace first."
                    );
                } else {
                    for f in &copied {
                        info!("  synced {f}");
                    }
                }
            }
            Err(e) => {
                error!("{e}");
                return 1;
            }
        }
    }

    0
}

/// Detect the repo name from the current working directory.
///
/// Matches the git remote URL of the cwd against known workspace URLs in state.
fn detect_repo_from_cwd(st: &state::WorkspaceState) -> Option<String> {
    let (_, url, _) = detect_git_repo().ok()?;

    // Match against known workspace URLs
    for ws in st.all_workspaces() {
        if ws.url == url {
            return Some(ws.repo.clone());
        }
    }

    // Also try matching by git root path against workspace paths
    let cwd = std::env::current_dir().ok()?;
    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))?;

    for ws in st.all_workspaces() {
        let ws_dir = st.workspace_dir(ws);
        if ws_dir == root || ws_dir == cwd {
            return Some(ws.repo.clone());
        }
    }

    None
}

/// Detect which workspace the current directory belongs to.
fn detect_workspace(st: &state::WorkspaceState) -> Option<state::WorkspaceEntry> {
    let cwd = std::env::current_dir().ok()?;

    // Try git root first (handles being in subdirectories)
    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or(cwd);

    for ws in st.all_workspaces() {
        let ws_dir = st.workspace_dir(ws);
        if ws_dir == root {
            return Some(ws.clone());
        }
    }
    None
}

/// Print workspace status grouped by repo.
fn print_workspace_status(st: &state::WorkspaceState, backend: &dyn MultiplexerBackend) {
    let workspace_root = st.workspace_root();

    // Collect unique repo names in order of first appearance
    let mut repos: Vec<String> = Vec::new();
    for ws in st.all_workspaces() {
        if !repos.contains(&ws.repo) {
            repos.push(ws.repo.clone());
        }
    }

    for repo in &repos {
        println!("{repo}");
        for ws in st.workspaces_for_repo(repo) {
            let container_name = config::container_name(&ws.repo, &ws.branch);
            let session_name = config::session_name(&ws.repo, &ws.branch);

            let clone_exists = if ws.path.is_some() {
                ws.path
                    .as_ref()
                    .map(|p| PathBuf::from(p).join(".git").exists())
                    .unwrap_or(false)
            } else {
                clone::workspace_exists(&workspace_root, &ws.repo, &ws.branch)
            };
            let container_st = container::status(&container_name);
            let tmux_alive = backend.is_alive(&session_name);

            let (icon, status_text) = match (&container_st, tmux_alive) {
                (container::ContainerStatus::Running, true) => {
                    ("\u{25cf}", "running  (container: up, tmux: attached)")
                }
                (container::ContainerStatus::Running, false) => {
                    ("\u{25cf}", "running  (container: up, tmux: none)")
                }
                (container::ContainerStatus::Stopped, true) => (
                    "\u{25cb}",
                    "stopped  (container: stopped, tmux: background)",
                ),
                (container::ContainerStatus::Stopped, false) => {
                    ("\u{25cb}", "stopped  (container: stopped, tmux: none)")
                }
                (container::ContainerStatus::Missing, _) if clone_exists => {
                    ("\u{25cb}", "stopped  (not launched)")
                }
                (container::ContainerStatus::Missing, _) => {
                    ("\u{25cc}", "lazy     (not cloned yet)")
                }
            };

            let branch_display = config::decode_branch(&config::encode_branch(&ws.branch));
            println!("  {branch_display:<24} {icon} {status_text}");
        }
        println!();
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
            assert_eq!(workspace.as_deref(), Some("lightfast-main"));
        } else {
            panic!("expected Launch command");
        }
    }

    #[test]
    fn launch_no_workspace() {
        let cli = Cli::parse_from(["dual", "launch"]);
        if let Some(Command::Launch { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Launch command");
        }
    }

    #[test]
    fn destroy_subcommand() {
        let cli = Cli::parse_from(["dual", "destroy", "lightfast-main"]);
        if let Some(Command::Destroy { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("lightfast-main"));
        } else {
            panic!("expected Destroy command");
        }
    }

    #[test]
    fn destroy_no_workspace() {
        let cli = Cli::parse_from(["dual", "destroy"]);
        if let Some(Command::Destroy { workspace }) = cli.command {
            assert!(workspace.is_none());
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
        let cli = Cli::parse_from(["dual", "create", "feat/auth", "--repo", "lightfast"]);
        if let Some(Command::Create { branch, repo }) = cli.command {
            assert_eq!(branch, "feat/auth");
            assert_eq!(repo.as_deref(), Some("lightfast"));
        } else {
            panic!("expected Create command");
        }
    }

    #[test]
    fn create_no_repo() {
        let cli = Cli::parse_from(["dual", "create", "feat/auth"]);
        if let Some(Command::Create { branch, repo }) = cli.command {
            assert_eq!(branch, "feat/auth");
            assert!(repo.is_none());
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
    fn sync_subcommand_no_args() {
        let cli = Cli::parse_from(["dual", "sync"]);
        if let Some(Command::Sync { workspace }) = cli.command {
            assert!(workspace.is_none());
        } else {
            panic!("expected Sync command");
        }
    }

    #[test]
    fn sync_subcommand_with_workspace() {
        let cli = Cli::parse_from(["dual", "sync", "lightfast-feat__auth"]);
        if let Some(Command::Sync { workspace }) = cli.command {
            assert_eq!(workspace.as_deref(), Some("lightfast-feat__auth"));
        } else {
            panic!("expected Sync command");
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
