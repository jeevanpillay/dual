use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::app::{App, WorkspaceStatus};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(3),    // workspace list
        Constraint::Length(1), // footer
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" dual", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  workspace browser"),
    ]));
    frame.render_widget(header, chunks[0]);

    // Workspace list
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let items = app.flatten_items();

    if items.is_empty() {
        let empty = Paragraph::new("  No workspaces. Run `dual add` in a repo to get started.")
            .block(block);
        frame.render_widget(empty, chunks[1]);
    } else {
        let list_items: Vec<ListItem> = items
            .iter()
            .map(|item| {
                let style = match item.status {
                    Some(WorkspaceStatus::Running) => Style::default().fg(Color::Green),
                    Some(WorkspaceStatus::Stopped) => Style::default().fg(Color::Yellow),
                    Some(WorkspaceStatus::Lazy) => Style::default().fg(Color::DarkGray),
                    None => Style::default().add_modifier(Modifier::BOLD), // repo header
                };
                ListItem::new(Line::from(Span::styled(&item.display, style)))
            })
            .collect();

        let list = List::new(list_items)
            .block(block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        let mut list_state = ListState::default();
        list_state.select(Some(app.selected()));

        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // Footer keybindings
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" j/k", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" navigate  "),
        Span::styled("enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" launch  "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{WorkspaceEntry, WorkspaceState};
    use crate::tmux_backend::TmuxBackend;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn render_does_not_panic_empty() {
        let state = WorkspaceState::new();
        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);

        let test_backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(test_backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
    }

    #[test]
    fn render_does_not_panic_with_workspaces() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".into(),
                url: "url".into(),
                branch: "main".into(),
                path: None,
            })
            .unwrap();

        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);

        let test_backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(test_backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
    }

    #[test]
    fn render_does_not_panic_small_terminal() {
        let mut state = WorkspaceState::new();
        state
            .add_workspace(WorkspaceEntry {
                repo: "lightfast".into(),
                url: "url".into(),
                branch: "main".into(),
                path: None,
            })
            .unwrap();

        let backend = TmuxBackend::new();
        let app = App::new(&state, &backend);

        let test_backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(test_backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
    }
}
