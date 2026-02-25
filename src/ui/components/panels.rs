use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, ChatRole, ChatSubpanel, FileChangeKind};
use crate::theme::UiColors;
use crate::ui::components::pane_chrome::PaneChrome;

pub fn render_left_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let colors = app.ui_colors();
    WorktreesPane::new(focused, colors).draw(frame, app, area);
}

pub fn render_middle_panel(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    focused: bool,
    show_caret: bool,
) {
    let colors = app.ui_colors();
    ChatPane::new(focused, app.chat_subpanel(), colors, show_caret).draw(frame, app, area);
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
    show_caret: bool,
}

impl ChatPane {
    fn new(focused: bool, subpanel: ChatSubpanel, colors: UiColors, show_caret: bool) -> Self {
        Self {
            focused,
            subpanel,
            colors,
            show_caret,
        }
    }

    fn draw(self, frame: &mut Frame, app: &App, area: Rect) {
        let transcript_focused = self.focused && self.subpanel == ChatSubpanel::Transcript;
        let composer_focused = self.focused && self.subpanel == ChatSubpanel::Composer;

        let panel_title = "Chat";
        let outer_block = base_panel_block(
            Borders::ALL,
            pane_chrome(self.focused, self.colors),
            self.colors,
        )
        .title(panel_title);
        let outer_inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        if outer_inner.width < 3 || outer_inner.height < 4 {
            return;
        }

        let chat_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(7)])
            .split(outer_inner);

        let transcript = build_chat_transcript_lines(app, self.colors);
        let transcript_viewport_rows = chat_layout[0].height as usize;
        let transcript_max_scroll =
            transcript.len().saturating_sub(transcript_viewport_rows) as u16;
        let transcript_scroll = app.chat_scroll().min(transcript_max_scroll);

        let transcript_panel = Paragraph::new(transcript)
            .style(panel_surface_style(self.colors))
            .scroll((transcript_scroll, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(transcript_panel, chat_layout[0]);

        if transcript_focused {
            let divider = Block::default().borders(Borders::BOTTOM).border_style(
                Style::default()
                    .fg(self.colors.border_focused)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(divider, chat_layout[0]);
        }

        let textarea_title = Line::from(vec![Span::styled(
            "Message".to_string(),
            Style::default()
                .fg(self.colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        )]);

        let textarea_block = base_panel_block(
            Borders::ALL,
            pane_chrome(composer_focused, self.colors),
            self.colors,
        )
        .title(textarea_title);
        let textarea_inner = textarea_block.inner(chat_layout[1]);
        frame.render_widget(textarea_block, chat_layout[1]);

        if textarea_inner.width == 0 || textarea_inner.height == 0 {
            return;
        }

        if textarea_inner.height < 2 {
            let compact = Paragraph::new(app.chat_draft())
                .style(panel_surface_style(self.colors))
                .wrap(Wrap { trim: false });
            frame.render_widget(compact, textarea_inner);
            return;
        }

        let textarea_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(textarea_inner);
        let textarea_input_area = textarea_layout[0];
        let textarea_footer_area = textarea_layout[1];

        let draft_lines = if app.chat_draft().is_empty() {
            vec![Line::from(Span::styled(
                "Type a message (Ctrl+Enter to send)",
                Style::default().fg(self.colors.muted_text),
            ))]
        } else {
            app.chat_draft()
                .split('\n')
                .map(|line| Line::from(line.to_owned()))
                .collect::<Vec<_>>()
        };

        let (cursor_line, cursor_column) = app.chat_cursor_line_column();
        let viewport_rows = textarea_input_area.height as usize;
        let scroll = cursor_line.saturating_add(1).saturating_sub(viewport_rows) as u16;
        let textarea = Paragraph::new(draft_lines)
            .style(panel_surface_style(self.colors))
            .scroll((scroll, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(textarea, textarea_input_area);

        let model_footer = Paragraph::new(Line::from(vec![
            Span::styled("Model: ", Style::default().fg(self.colors.muted_text)),
            Span::styled(
                app.active_model_label().to_string(),
                Style::default().fg(self.colors.muted_text),
            ),
        ]))
        .style(panel_surface_style(self.colors))
        .wrap(Wrap { trim: false });
        frame.render_widget(model_footer, textarea_footer_area);

        if self.show_caret && app.composer_is_focused() {
            let visible_line = cursor_line.saturating_sub(scroll as usize);
            if visible_line < viewport_rows {
                let cursor_x = textarea_input_area.x
                    + (cursor_column.min(textarea_input_area.width.saturating_sub(1) as usize)
                        as u16);
                let cursor_y = textarea_input_area.y + visible_line as u16;
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

struct ChangedFilesPane {
    focused: bool,
    chrome: PaneChrome,
    colors: UiColors,
}

impl ChangedFilesPane {
    fn new(focused: bool, colors: UiColors) -> Self {
        Self {
            focused,
            chrome: pane_chrome(focused, colors),
            colors,
        }
    }

    fn draw(self, frame: &mut Frame, app: &App, area: Rect) {
        let selected = app.selected_worktree();
        let panel_block =
            base_panel_block(Borders::ALL, self.chrome, self.colors).title("Changed Files");
        let panel_inner = panel_block.inner(area);
        frame.render_widget(panel_block, area);

        if panel_inner.width == 0 || panel_inner.height == 0 {
            return;
        }

        let footer_height = if self.focused {
            if app.right_search_visible() { 3 } else { 2 }
        } else {
            0
        };
        let (list_area, footer_area) = if footer_height > 0 && panel_inner.height > footer_height {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(footer_height)])
                .split(panel_inner);
            (split[0], Some(split[1]))
        } else {
            (panel_inner, None)
        };

        if selected.changed_files.is_empty() {
            let panel = Paragraph::new("No changed files.")
                .style(panel_surface_style(self.colors))
                .wrap(Wrap { trim: false });
            frame.render_widget(panel, list_area);
            render_changed_files_footer(
                frame,
                footer_area,
                self.colors,
                app.right_search_active(),
                app.right_search_query(),
            );
            return;
        }

        let (unstaged, staged) = app.changed_file_sections();

        let mut items = Vec::new();
        let mut selected_row = None;
        let row_width = list_area.width.saturating_sub(4) as usize;
        let query = app.right_search_query().trim();
        if !query.is_empty() {
            let match_count = unstaged.len() + staged.len();
            let noun = if match_count == 1 { "file" } else { "files" };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{match_count} {noun} match query "),
                    Style::default().fg(self.colors.muted_text),
                ),
                Span::styled(
                    format!("/{query}"),
                    Style::default()
                        .fg(self.colors.panel_foreground)
                        .add_modifier(Modifier::BOLD),
                ),
            ])));
        }
        push_changed_file_section(
            &mut items,
            "Unstaged",
            &unstaged,
            selected,
            app.right_selected_idx(),
            &mut selected_row,
            false,
            row_width,
            self.colors,
        );
        push_changed_file_section(
            &mut items,
            "Staged",
            &staged,
            selected,
            app.right_selected_idx(),
            &mut selected_row,
            true,
            row_width,
            self.colors,
        );

        let right_panel = List::new(items)
            .style(panel_surface_style(self.colors))
            .highlight_style(
                Style::default()
                    .bg(self.colors.list_highlight_background)
                    .fg(self.colors.list_highlight_foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("» ");

        let mut state = ListState::default();
        state.select(selected_row);
        frame.render_stateful_widget(right_panel, list_area, &mut state);
        render_changed_files_footer(
            frame,
            footer_area,
            self.colors,
            app.right_search_active(),
            app.right_search_query(),
        );
    }
}

fn push_changed_file_section(
    items: &mut Vec<ListItem<'static>>,
    title: &str,
    file_indices: &[usize],
    worktree: &crate::app::Worktree,
    selected_file_idx: usize,
    selected_row: &mut Option<usize>,
    fill_header_separator: bool,
    row_width: usize,
    colors: UiColors,
) {
    let mut header = vec![
        Span::styled(
            title.to_string(),
            Style::default()
                .fg(colors.muted_text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({})", file_indices.len()),
            Style::default().fg(colors.muted_text),
        ),
    ];
    if fill_header_separator {
        let title_len = title.chars().count();
        let count_len = file_indices.len().to_string().chars().count();
        let base_len = title_len + 3 + count_len; // "Title (n)"
        let separator_len = row_width.saturating_sub(base_len + 2).max(1);
        header.push(Span::raw(" "));
        header.push(Span::styled(
            "-".repeat(separator_len),
            Style::default().fg(colors.muted_text),
        ));
    }
    items.push(ListItem::new(Line::from(header)));

    if file_indices.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "  (none)",
            Style::default().fg(colors.muted_text),
        )])));
        return;
    }

    for idx in file_indices {
        let change = &worktree.changed_files[*idx];
        if *idx == selected_file_idx {
            *selected_row = Some(items.len());
        }

        let status_width = 2usize;
        let min_gap = 2usize;
        let plus_text = format!("+{}", change.additions);
        let minus_text = format!("-{}", change.deletions);
        let counts_width = plus_text.chars().count() + 1 + minus_text.chars().count();
        let name_budget = row_width
            .saturating_sub(status_width + counts_width + min_gap)
            .max(1);
        let (path_prefix, file_name) = split_display_path(change.path, name_budget);
        let name_width = path_prefix.chars().count()
            + if path_prefix.is_empty() { 0 } else { 1 }
            + file_name.chars().count();
        let spacer_width = row_width
            .saturating_sub(status_width + name_width + counts_width)
            .max(1);

        let mut row = vec![Span::styled(
            format!("{} ", change.kind.code()),
            Style::default()
                .fg(change_kind_color(change.kind, colors))
                .add_modifier(Modifier::BOLD),
        )];

        if !path_prefix.is_empty() {
            row.push(Span::styled(
                path_prefix,
                Style::default().fg(colors.muted_text),
            ));
            row.push(Span::raw(" "));
        }
        row.push(Span::styled(
            file_name,
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ));
        row.push(Span::raw(" ".repeat(spacer_width)));
        row.extend([
            Span::styled(plus_text, Style::default().fg(colors.added)),
            Span::raw(" "),
            Span::styled(minus_text, Style::default().fg(colors.removed)),
        ]);

        items.push(ListItem::new(Line::from(row)));
    }
}

fn split_display_path(path: &str, max_chars: usize) -> (String, String) {
    let Some((parent, file_name)) = path.rsplit_once('/') else {
        return (String::new(), truncate_tail(path, max_chars));
    };

    let file_name_len = file_name.chars().count();
    if file_name_len + 1 >= max_chars {
        return (String::new(), truncate_tail(file_name, max_chars));
    }

    let prefix_budget = max_chars.saturating_sub(file_name_len + 1);
    let prefix = truncate_tail(&format!("{parent}/"), prefix_budget);
    (prefix, file_name.to_string())
}

fn truncate_tail(value: &str, max_chars: usize) -> String {
    let length = value.chars().count();
    if length <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let keep = max_chars - 3;
    let prefix = value.chars().take(keep).collect::<String>();
    format!("{prefix}...")
}

fn change_kind_color(kind: FileChangeKind, colors: UiColors) -> Color {
    match kind {
        // VSCode-like hues tuned for 256-color terminals.
        FileChangeKind::Modified => Color::Indexed(180),
        FileChangeKind::Added => Color::Indexed(78),
        FileChangeKind::Deleted => Color::Indexed(203),
        FileChangeKind::Renamed => Color::Indexed(110),
        FileChangeKind::Copied => Color::Indexed(75),
        FileChangeKind::TypeChanged => Color::Indexed(176),
        FileChangeKind::Unmerged => Color::Indexed(204),
        FileChangeKind::Untracked => Color::Indexed(114),
        FileChangeKind::Ignored => colors.muted_text,
    }
}

fn render_changed_files_footer(
    frame: &mut Frame,
    area: Option<Rect>,
    colors: UiColors,
    search_active: bool,
    search_query: &str,
) {
    let Some(area) = area else {
        return;
    };

    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(colors.border_default))
        .style(panel_surface_style(colors));
    let footer_inner = footer_block.inner(area);
    frame.render_widget(footer_block, area);

    if footer_inner.width == 0 || footer_inner.height == 0 {
        return;
    }

    let trimmed_query = search_query.trim();
    let has_query = !trimmed_query.is_empty();
    let show_search_row = search_active || has_query;
    let clear_hotkey_style = if has_query {
        Style::default()
            .fg(colors.panel_foreground)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.muted_text)
    };

    let mut lines = vec![Line::from(vec![
        Span::styled(
            "A",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" stage  ", Style::default().fg(colors.muted_text)),
        Span::styled(
            "R",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" unstage  ", Style::default().fg(colors.muted_text)),
        Span::styled(
            "/",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" search  ", Style::default().fg(colors.muted_text)),
        Span::styled("C", clear_hotkey_style),
        Span::styled(" clear", Style::default().fg(colors.muted_text)),
    ])];

    if show_search_row && footer_inner.height > 1 {
        let mut search_line = vec![
            Span::styled("Search: ", Style::default().fg(colors.muted_text)),
            Span::styled("/", Style::default().fg(colors.panel_foreground)),
            Span::styled(
                trimmed_query.to_string(),
                Style::default().fg(colors.panel_foreground),
            ),
        ];
        if search_active {
            search_line.push(Span::styled(
                "█",
                Style::default().fg(colors.panel_foreground),
            ));
        }
        lines.push(Line::from(search_line));
    }

    let footer = Paragraph::new(lines)
        .style(panel_surface_style(colors))
        .wrap(Wrap { trim: false });
    frame.render_widget(footer, footer_inner);
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
