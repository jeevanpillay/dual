use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use super::app::App;

pub enum Action {
    Quit,
    Select(String), // workspace_id
}

pub fn handle_events(app: &mut App) -> Result<Option<Action>, Box<dyn std::error::Error>> {
    if event::poll(std::time::Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        return Ok(handle_key(app, key));
    }
    Ok(None)
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
            None
        }
        KeyCode::Enter => {
            if let Some(workspace_id) = app.selected_workspace_id() {
                Some(Action::Select(workspace_id))
            } else {
                // Selected a repo header — toggle expand/collapse
                app.toggle_expand();
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_app() -> App {
        use crate::state::{WorkspaceEntry, WorkspaceState};
        use crate::tmux_backend::TmuxBackend;

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

        let backend = TmuxBackend::new();
        App::new(&state, &backend)
    }

    #[test]
    fn quit_on_q() {
        let mut app = make_app();
        let action = handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn quit_on_esc() {
        let mut app = make_app();
        let action = handle_key(&mut app, key(KeyCode::Esc));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn navigate_down() {
        let mut app = make_app();
        let initial = app.selected();
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.selected(), initial + 1);
    }

    #[test]
    fn navigate_up_at_top() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.selected(), 0);
    }

    #[test]
    fn enter_on_repo_header_toggles() {
        let mut app = make_app();
        // First item should be a repo header
        let action = handle_key(&mut app, key(KeyCode::Enter));
        assert!(action.is_none()); // toggle, not select
    }

    #[test]
    fn j_and_k_navigate() {
        let mut app = make_app();
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.selected(), 1);
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.selected(), 0);
    }
}
