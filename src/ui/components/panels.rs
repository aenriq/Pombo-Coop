use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, RESIZE_MODIFIER_LABEL};

pub fn render_left_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let list_items = app
        .worktrees()
        .iter()
        .map(|worktree| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(worktree.repo, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" / "),
                    Span::raw(worktree.name),
                ]),
                Line::from(format!(
                    "{} · {} · #{}",
                    worktree.branch, worktree.status, worktree.pr_number
                )),
                Line::from(""),
            ])
        })
        .collect::<Vec<_>>();

    let list = List::new(list_items)
        .block(panel_block("Worktrees", focused))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_worktree_idx()));
    frame.render_stateful_widget(list, area, &mut state);
}

pub fn render_middle_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let selected = app.selected_worktree();
    let provider = app.active_provider_descriptor();
    let content = vec![
        Line::from(vec![
            Span::styled("Worktree: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.name),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.repo),
        ]),
        Line::from(vec![
            Span::styled("Branch: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.branch),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.status),
        ]),
        Line::from(vec![
            Span::styled("Provider: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(provider.display_name),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Summary",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(selected.summary),
        Line::from(""),
        Line::from("Panel focus: Ctrl+h/j/k/l or Ctrl+Arrows"),
        Line::from(format!(
            "Panel resize: {0}+h/j/k/l or {0}+Arrows",
            RESIZE_MODIFIER_LABEL
        )),
        Line::from("Movement keys (j/k or Up/Down) act on focused panel."),
        Line::from("- Worktrees panel: change selected worktree"),
        Line::from("- Details panel: scroll this content"),
        Line::from("- Changed Files panel: move changed-file selection"),
        Line::from("q to quit"),
    ];

    let viewport_rows = area.height.saturating_sub(2) as usize;
    let max_scroll = content.len().saturating_sub(viewport_rows) as u16;
    let scroll = app.details_scroll().min(max_scroll);

    let panel = Paragraph::new(content)
        .scroll((scroll, 0))
        .block(panel_block("Details", focused))
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

pub fn render_right_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let selected = app.selected_worktree();
    let changed_files_len = selected.changed_files.len();

    if changed_files_len == 0 {
        let panel = Paragraph::new("No changed files.")
            .block(panel_block("Changed Files", focused))
            .wrap(Wrap { trim: false });
        frame.render_widget(panel, area);
        return;
    }

    let items = selected
        .changed_files
        .iter()
        .map(|change| {
            ListItem::new(Line::from(vec![
                Span::raw(change.path),
                Span::raw("  "),
                Span::styled(
                    format!("+{}", change.additions),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("-{}", change.deletions),
                    Style::default().fg(Color::Red),
                ),
            ]))
        })
        .collect::<Vec<_>>();

    let right_panel = List::new(items)
        .block(panel_block("Changed Files", focused))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    let mut state = ListState::default();
    state.select(Some(app.right_selected_idx().min(changed_files_len - 1)));
    frame.render_stateful_widget(right_panel, area, &mut state);
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let mut block = Block::default().title(title).borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    }
    block
}
