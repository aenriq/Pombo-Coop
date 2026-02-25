use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, ChatRole, ChatSubpanel, RESIZE_MODIFIER_LABEL};
use crate::theme::UiColors;
use crate::ui::components::pane_chrome::PaneChrome;

pub fn render_left_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let colors = app.ui_colors();
    WorktreesPane::new(focused, colors).draw(frame, app, area);
}

pub fn render_middle_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let colors = app.ui_colors();
    ChatPane::new(focused, app.chat_subpanel(), colors).draw(frame, app, area);
}

pub fn render_right_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let colors = app.ui_colors();
    ChangedFilesPane::new(focused, colors).draw(frame, app, area);
}

struct WorktreesPane {
    chrome: PaneChrome,
    colors: UiColors,
}

impl WorktreesPane {
    fn new(focused: bool, colors: UiColors) -> Self {
        Self {
            chrome: pane_chrome(focused, colors),
            colors,
        }
    }

    fn draw(self, frame: &mut Frame, app: &App, area: Rect) {
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
            .style(panel_surface_style(self.colors))
            .block(base_panel_block(Borders::ALL, self.chrome, self.colors).title("Worktrees"))
            .highlight_style(
                Style::default()
                    .bg(self.colors.list_highlight_background)
                    .fg(self.colors.list_highlight_foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(app.selected_worktree_idx()));
        frame.render_stateful_widget(list, area, &mut state);
    }
}

struct ChatPane {
    focused: bool,
    subpanel: ChatSubpanel,
    colors: UiColors,
}

impl ChatPane {
    fn new(focused: bool, subpanel: ChatSubpanel, colors: UiColors) -> Self {
        Self {
            focused,
            subpanel,
            colors,
        }
    }

    fn draw(self, frame: &mut Frame, app: &App, area: Rect) {
        let selected = app.selected_worktree();
        let transcript_focused = self.focused && self.subpanel == ChatSubpanel::Transcript;
        let composer_focused = self.focused && self.subpanel == ChatSubpanel::Composer;

        let chat_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(7)])
            .split(area);

        let transcript = build_chat_transcript_lines(app, self.colors);
        let transcript_viewport_rows = chat_layout[0].height.saturating_sub(2) as usize;
        let transcript_max_scroll = transcript.len().saturating_sub(transcript_viewport_rows) as u16;
        let transcript_scroll = app.chat_scroll().min(transcript_max_scroll);
        let transcript_title = format!("Chat · {} / {}", selected.repo, selected.name);

        let transcript_panel = Paragraph::new(transcript)
            .style(panel_surface_style(self.colors))
            .scroll((transcript_scroll, 0))
            .block(
                base_panel_block(
                    Borders::TOP | Borders::LEFT | Borders::RIGHT,
                    pane_chrome(transcript_focused, self.colors),
                    self.colors,
                )
                .title(transcript_title),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(transcript_panel, chat_layout[0]);

        let composer_text = if app.chat_draft().is_empty() {
            vec![
                Line::from(vec![
                    Span::styled("█", Style::default().fg(self.colors.panel_foreground)),
                    Span::raw(" "),
                    Span::styled(
                        "Press Ctrl+C to exit",
                        Style::default()
                            .fg(self.colors.muted_text)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(format!(
                    "Movement: j/k scroll transcript | Ctrl+Left/Right (or Ctrl+h/l) switch panels | {0}+h/j/k/l resize",
                    RESIZE_MODIFIER_LABEL
                )),
                Line::from("Ctrl+Up/Down or Ctrl+j/k switches between transcript and composer."),
            ]
        } else {
            vec![Line::from(app.chat_draft())]
        };

        let composer_title = Line::from(vec![Span::styled(
            app.active_model_label().to_string(),
            Style::default()
                .fg(self.colors.model_title)
                .add_modifier(Modifier::BOLD),
        )]);
        let composer = Paragraph::new(composer_text)
            .style(panel_surface_style(self.colors))
            .block(
                base_panel_block(Borders::ALL, pane_chrome(composer_focused, self.colors), self.colors)
                    .title(composer_title),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(composer, chat_layout[1]);
    }
}

struct ChangedFilesPane {
    chrome: PaneChrome,
    colors: UiColors,
}

impl ChangedFilesPane {
    fn new(focused: bool, colors: UiColors) -> Self {
        Self {
            chrome: pane_chrome(focused, colors),
            colors,
        }
    }

    fn draw(self, frame: &mut Frame, app: &App, area: Rect) {
        let selected = app.selected_worktree();
        let changed_files_len = selected.changed_files.len();

        if changed_files_len == 0 {
            let panel = Paragraph::new("No changed files.")
                .style(panel_surface_style(self.colors))
                .block(base_panel_block(Borders::ALL, self.chrome, self.colors).title("Changed Files"))
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
                        Style::default().fg(self.colors.added),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("-{}", change.deletions),
                        Style::default().fg(self.colors.removed),
                    ),
                ]))
            })
            .collect::<Vec<_>>();

        let right_panel = List::new(items)
            .style(panel_surface_style(self.colors))
            .block(base_panel_block(Borders::ALL, self.chrome, self.colors).title("Changed Files"))
            .highlight_style(
                Style::default()
                    .bg(self.colors.list_highlight_background)
                    .fg(self.colors.list_highlight_foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("» ");

        let mut state = ListState::default();
        state.select(Some(app.right_selected_idx().min(changed_files_len - 1)));
        frame.render_stateful_widget(right_panel, area, &mut state);
    }
}

fn pane_chrome(focused: bool, colors: UiColors) -> PaneChrome {
    PaneChrome::new(
        focused,
        Style::default().fg(colors.border_focused),
        Style::default().fg(colors.border_default),
    )
}

fn base_panel_block<'a>(borders: Borders, chrome: PaneChrome, colors: UiColors) -> Block<'a> {
    Block::default()
        .borders(borders)
        .border_style(chrome.border_style())
        .border_type(chrome.border_type())
        .style(panel_surface_style(colors))
}

fn panel_surface_style(colors: UiColors) -> Style {
    Style::default()
        .bg(colors.panel_background)
        .fg(colors.panel_foreground)
}

fn build_chat_transcript_lines(app: &App, colors: UiColors) -> Vec<Line<'static>> {
    let selected = app.selected_worktree();
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Context ",
                Style::default()
                    .fg(colors.context_label)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{} / {} · {} · #{}",
                selected.repo, selected.name, selected.branch, selected.pr_number
            )),
        ]),
        Line::from(vec![
            Span::styled("Summary ", Style::default().fg(colors.summary_label)),
            Span::raw(selected.summary),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("186", Style::default().fg(colors.line_number)),
            Span::raw("      "),
            Span::raw("shockwaves: this._shockwaves,"),
        ]),
        Line::from(vec![
            Span::styled("190", Style::default().fg(colors.line_added_number)),
            Span::raw("   +  "),
            Span::styled(
                "shockwaves: this.widget.shockwaves ?? this._internalShockwaves,",
                Style::default().fg(colors.line_added_text),
            ),
        ]),
        Line::from(""),
    ];

    for message in app.chat_messages() {
        let (label, style) = match message.role {
            ChatRole::Agent => (
                "agent",
                Style::default()
                    .fg(colors.role_agent)
                    .add_modifier(Modifier::BOLD),
            ),
            ChatRole::User => (
                "you",
                Style::default()
                    .fg(colors.role_user)
                    .add_modifier(Modifier::BOLD),
            ),
            ChatRole::System => (
                "system",
                Style::default()
                    .fg(colors.role_system)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        lines.push(Line::from(vec![Span::styled(label, style), Span::raw(" ")]));

        for entry_line in message.content.lines() {
            if let Some(path) = entry_line.strip_prefix("Edit ") {
                lines.push(Line::from(vec![
                    Span::styled("  Edit ", Style::default().fg(colors.edit_prefix)),
                    Span::styled(
                        path.to_string(),
                        Style::default()
                            .fg(colors.edit_path)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![Span::raw(format!("  {entry_line}"))]));
            }
        }
        lines.push(Line::from(""));
    }

    lines
}
