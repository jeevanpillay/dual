use crate::backend::MultiplexerBackend;
use crate::clone;
use crate::config;
use crate::container;
use crate::state::WorkspaceState;

use super::event;
use super::ui;

/// A single item in the flattened display list.
pub struct DisplayItem {
    /// Text to show
    pub display: String,
    /// Workspace status (None = repo header)
    pub status: Option<WorkspaceStatus>,
    /// Workspace id if this is a branch item
    pub workspace_id: Option<String>,
}

#[derive(Clone, Copy)]
pub enum WorkspaceStatus {
    /// tmux session alive
    Running,
    /// Container stopped, no tmux
    Stopped,
    /// Not cloned yet
    Lazy,
}

struct RepoGroup {
    name: String,
    workspaces: Vec<WorkspaceItem>,
    expanded: bool,
}

struct WorkspaceItem {
    branch: String,
    workspace_id: String,
    status: WorkspaceStatus,
}

pub struct App {
    repos: Vec<RepoGroup>,
    selected: usize,
    item_count: usize,
}

impl App {
    /// Build app state from workspace state and live backend status.
    pub fn new(state: &WorkspaceState, backend: &dyn MultiplexerBackend) -> Self {
        let workspace_root = state.workspace_root();

        // Group workspaces by repo (preserve insertion order)
        let mut repo_names: Vec<String> = Vec::new();
        for ws in state.all_workspaces() {
            if !repo_names.contains(&ws.repo) {
                repo_names.push(ws.repo.clone());
            }
        }

        let repos: Vec<RepoGroup> = repo_names
            .iter()
            .map(|repo| {
                let workspaces = state
                    .workspaces_for_repo(repo)
                    .into_iter()
                    .map(|ws| {
                        let session_name = config::session_name(&ws.repo, &ws.branch);
                        let container_name = config::container_name(&ws.repo, &ws.branch);
                        let clone_exists = if ws.path.is_some() {
                            ws.path
                                .as_ref()
                                .is_some_and(|p| std::path::PathBuf::from(p).join(".git").exists())
                        } else {
                            clone::workspace_exists(&workspace_root, &ws.repo, &ws.branch)
                        };

                        let status = if backend.is_alive(&session_name) {
                            WorkspaceStatus::Running
                        } else if matches!(
                            container::status(&container_name),
                            container::ContainerStatus::Stopped
                        ) || clone_exists
                        {
                            WorkspaceStatus::Stopped
                        } else {
                            WorkspaceStatus::Lazy
                        };

                        WorkspaceItem {
                            branch: ws.branch.clone(),
                            workspace_id: config::workspace_id(&ws.repo, &ws.branch),
                            status,
                        }
                    })
                    .collect();

                RepoGroup {
                    name: repo.clone(),
                    workspaces,
                    expanded: true,
                }
            })
            .collect();

        let item_count = Self::count_items(&repos);
        App {
            repos,
            selected: 0,
            item_count,
        }
    }

    fn count_items(repos: &[RepoGroup]) -> usize {
        repos
            .iter()
            .map(|r| 1 + if r.expanded { r.workspaces.len() } else { 0 })
            .sum()
    }

    /// Flatten repos into display items for rendering.
    pub fn flatten_items(&self) -> Vec<DisplayItem> {
        let mut items = Vec::new();
        for repo in &self.repos {
            let chevron = if repo.expanded { "▼" } else { "▶" };
            items.push(DisplayItem {
                display: format!("{chevron} {}", repo.name),
                status: None,
                workspace_id: None,
            });
            if repo.expanded {
                for ws in &repo.workspaces {
                    let icon = match ws.status {
                        WorkspaceStatus::Running => "●",
                        WorkspaceStatus::Stopped => "○",
                        WorkspaceStatus::Lazy => "◌",
                    };
                    let label = match ws.status {
                        WorkspaceStatus::Running => "running",
                        WorkspaceStatus::Stopped => "stopped",
                        WorkspaceStatus::Lazy => "lazy",
                    };
                    let branch_display = config::decode_branch(&config::encode_branch(&ws.branch));
                    items.push(DisplayItem {
                        display: format!("  {branch_display:<24} {icon} {label}"),
                        status: Some(ws.status),
                        workspace_id: Some(ws.workspace_id.clone()),
                    });
                }
            }
        }
        items
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.item_count {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Get the workspace_id of the currently selected item, or None if it's a repo header.
    pub fn selected_workspace_id(&self) -> Option<String> {
        let items = self.flatten_items();
        items
            .get(self.selected)
            .and_then(|item| item.workspace_id.clone())
    }

    /// Toggle expand/collapse of the repo group at the current selection.
    pub fn toggle_expand(&mut self) {
        // Find which repo group this index belongs to
        let mut idx = 0;
        for repo in &mut self.repos {
            if idx == self.selected {
                repo.expanded = !repo.expanded;
                self.item_count = Self::count_items_from_repos(&self.repos);
                // Clamp selected if it now exceeds item count
                if self.selected >= self.item_count {
                    self.selected = self.item_count.saturating_sub(1);
                }
                return;
            }
            idx += 1;
            if repo.expanded {
                idx += repo.workspaces.len();
            }
        }
    }

    fn count_items_from_repos(repos: &[RepoGroup]) -> usize {
        Self::count_items(repos)
    }
}

/// Main TUI entry point.
///
/// Returns `Ok(Some(workspace_id))` if user selected a workspace to launch.
/// Returns `Ok(None)` if user quit.
pub fn run(
    state: &WorkspaceState,
    backend: &dyn MultiplexerBackend,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let mut app = App::new(state, backend);

    let result = loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(action) = event::handle_events(&mut app)? {
            match action {
                event::Action::Quit => break Ok(None),
                event::Action::Select(workspace_id) => {
                    break Ok(Some(workspace_id));
                }
            }
        }
    };

    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{WorkspaceEntry, WorkspaceState};
    use crate::tmux_backend::TmuxBackend;

    fn test_state() -> WorkspaceState {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".into(),
                url: "url".into(),
                branch: "main".into(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".into(),
                url: "url".into(),
                branch: "feat/auth".into(),
                path: None,
            })
            .unwrap();
        state
            .add_workspace(WorkspaceEntry {
                repo: "agent-os".into(),
                url: "url".into(),
                branch: "main".into(),
                path: None,
            })
            .unwrap();
        state
    }

    #[test]
    fn app_groups_by_repo() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);

        assert_eq!(app.repos.len(), 2);
        assert_eq!(app.repos[0].name, "lightfast");
        assert_eq!(app.repos[0].workspaces.len(), 2);
        assert_eq!(app.repos[1].name, "agent-os");
        assert_eq!(app.repos[1].workspaces.len(), 1);
    }

    #[test]
    fn flatten_items_expanded() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);
        let items = app.flatten_items();

        // 2 repo headers + 3 workspaces = 5
        assert_eq!(items.len(), 5);
        assert!(items[0].status.is_none()); // repo header
        assert!(items[0].display.contains("lightfast"));
        assert!(items[1].status.is_some()); // workspace
        assert!(items[1].display.contains("main"));
        assert!(items[3].status.is_none()); // repo header
        assert!(items[3].display.contains("agent-os"));
    }

    #[test]
    fn navigation_bounds() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let mut app = App::new(&state, &backend);

        assert_eq!(app.selected(), 0);
        app.move_up(); // should stay at 0
        assert_eq!(app.selected(), 0);

        // Move to the end
        for _ in 0..10 {
            app.move_down();
        }
        assert_eq!(app.selected(), 4); // 5 items, max index = 4
    }

    #[test]
    fn selected_workspace_id_returns_none_for_header() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);
        // First item is a repo header
        assert!(app.selected_workspace_id().is_none());
    }

    #[test]
    fn selected_workspace_id_returns_id_for_branch() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let mut app = App::new(&state, &backend);
        app.move_down(); // move to first workspace
        assert_eq!(
            app.selected_workspace_id(),
            Some("lightfast-main".to_string())
        );
    }

    #[test]
    fn toggle_collapse_reduces_items() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let mut app = App::new(&state, &backend);

        assert_eq!(app.item_count, 5);
        app.toggle_expand(); // collapse "lightfast"
        assert_eq!(app.item_count, 3); // 2 headers + 1 workspace (agent-os/main)
    }

    #[test]
    fn toggle_expand_restores_items() {
        let state = test_state();
        let backend = TmuxBackend::new();
        let mut app = App::new(&state, &backend);

        app.toggle_expand(); // collapse
        assert_eq!(app.item_count, 3);
        app.toggle_expand(); // expand again
        assert_eq!(app.item_count, 5);
    }

    #[test]
    fn empty_state_produces_no_items() {
        let state = WorkspaceState::new();
        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);
        assert_eq!(app.flatten_items().len(), 0);
        assert_eq!(app.item_count, 0);
    }
}
