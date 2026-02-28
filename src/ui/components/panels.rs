use std::collections::{BTreeSet, VecDeque};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};

use crate::app::{App, ChatRole, ChatSubpanel, FileChangeKind, ModelChoice};
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
    if app.model_picker_active() {
        render_model_picker_overlay(frame, app, area, colors);
    }
}

pub fn render_right_panel(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let colors = app.ui_colors();
    ChangedFilesPane::new(focused, colors).draw(frame, app, area);
}

struct WorktreesPane {
    chrome: PaneChrome,
    focused: bool,
    colors: UiColors,
}

impl WorktreesPane {
    fn new(focused: bool, colors: UiColors) -> Self {
        Self {
            chrome: pane_chrome(focused, colors),
            focused,
            colors,
        }
    }

    fn draw(self, frame: &mut Frame, app: &App, area: Rect) {
        let panel_block =
            base_panel_block(Borders::ALL, self.chrome, self.colors).title("Worktrees");
        let panel_inner = panel_block.inner(area);
        frame.render_widget(panel_block, area);

        if panel_inner.width == 0 || panel_inner.height == 0 {
            return;
        }

        let footer_height = if panel_inner.height > 2 { 2 } else { 0 };
        let (list_area, footer_area) = if footer_height > 0 {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(footer_height)])
                .split(panel_inner);
            (split[0], Some(split[1]))
        } else {
            (panel_inner, None)
        };

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
            .highlight_style(
                Style::default()
                    .bg(self.colors.list_highlight_background)
                    .fg(self.colors.list_highlight_foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(app.selected_worktree_idx()));
        frame.render_stateful_widget(list, list_area, &mut state);
        render_worktrees_footer(
            frame,
            footer_area,
            self.focused,
            app.worktree_name_prompt_active(),
            app.agent_rename_prompt_active(),
            app.selected_agent_can_rename(),
            self.colors,
        );

        if app.worktree_name_prompt_active() {
            render_worktree_name_overlay(frame, app, area, self.colors);
        }
        if app.agent_rename_prompt_active() {
            render_agent_rename_overlay(frame, app, area, self.colors);
        }
    }
}

fn render_worktrees_footer(
    frame: &mut Frame,
    area: Option<Rect>,
    focused: bool,
    worktree_prompt_active: bool,
    rename_prompt_active: bool,
    can_rename_selected: bool,
    colors: UiColors,
) {
    let Some(area) = area else {
        return;
    };

    let footer_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(if focused {
            colors.border_focused
        } else {
            colors.border_default
        }))
        .style(panel_surface_style(colors));
    let footer_inner = footer_block.inner(area);
    frame.render_widget(footer_block, area);

    if footer_inner.width == 0 || footer_inner.height == 0 {
        return;
    }

    let hotkey_style = Style::default()
        .fg(colors.panel_foreground)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.muted_text);
    let line = if worktree_prompt_active || rename_prompt_active {
        let action = if rename_prompt_active {
            " rename"
        } else {
            " create"
        };
        Line::from(vec![
            Span::styled("Enter", hotkey_style),
            Span::styled(action, text_style),
            Span::styled("  ", text_style),
            Span::styled("Esc", hotkey_style),
            Span::styled(" cancel", text_style),
        ])
    } else {
        let mut spans = vec![
            Span::styled("A", hotkey_style),
            Span::styled(" new agent  ", text_style),
            Span::styled("W", hotkey_style),
            Span::styled(" new worktree", text_style),
        ];
        if can_rename_selected {
            spans.extend([
                Span::styled("  ", text_style),
                Span::styled("R", hotkey_style),
                Span::styled(" rename agent", text_style),
            ]);
        }
        Line::from(spans)
    };

    frame.render_widget(
        Paragraph::new(line)
            .style(panel_surface_style(colors))
            .wrap(Wrap { trim: false }),
        footer_inner,
    );
}

fn render_worktree_name_overlay(frame: &mut Frame, app: &App, area: Rect, colors: UiColors) {
    if area.width < 24 || area.height < 8 {
        return;
    }

    let popup_width = area.width.saturating_sub(4).min(52);
    let popup_height = 7;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height.min(area.height.saturating_sub(1)),
    };

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title("New Worktree")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(colors.border_focused)
                .add_modifier(Modifier::BOLD),
        )
        .style(panel_surface_style(colors));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let prompt = app.worktree_name_prompt_value();
    let lines = vec![
        Line::from(Span::styled(
            "Type worktree name. Agent name is auto-generated.",
            Style::default().fg(colors.muted_text),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                if prompt.is_empty() { " " } else { prompt },
                Style::default().fg(colors.panel_foreground),
            ),
        ]),
        Line::from(Span::styled(
            "Enter to create • Esc to cancel",
            Style::default().fg(colors.muted_text),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .style(panel_surface_style(colors))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_agent_rename_overlay(frame: &mut Frame, app: &App, area: Rect, colors: UiColors) {
    if area.width < 24 || area.height < 8 {
        return;
    }

    let popup_width = area.width.saturating_sub(4).min(52);
    let popup_height = 7;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height.min(area.height.saturating_sub(1)),
    };

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title("Rename Agent")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(colors.border_focused)
                .add_modifier(Modifier::BOLD),
        )
        .style(panel_surface_style(colors));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let prompt = app.agent_rename_prompt_value();
    let lines = vec![
        Line::from(Span::styled(
            "Rename selected non-worktree agent.",
            Style::default().fg(colors.muted_text),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                if prompt.is_empty() { " " } else { prompt },
                Style::default().fg(colors.panel_foreground),
            ),
        ]),
        Line::from(Span::styled(
            "Enter to rename • Esc to cancel",
            Style::default().fg(colors.muted_text),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .style(panel_surface_style(colors))
            .wrap(Wrap { trim: false }),
        inner,
    );
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

        let (transcript_column_area, transcript_scrollbar_area) = if chat_layout[0].width > 1 {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(chat_layout[0]);
            (split[0], Some(split[1]))
        } else {
            (chat_layout[0], None)
        };

        let transcript_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(transcript_column_area);
        let transcript_text_area = transcript_layout[0];
        let transcript_indicator_area = Rect {
            x: chat_layout[0].x,
            y: transcript_layout[1].y,
            width: chat_layout[0].width,
            height: transcript_layout[1].height,
        };

        let transcript = build_chat_transcript_lines(app, self.colors);
        let transcript_viewport_rows = transcript_text_area.height as usize;
        let transcript_total_rows =
            wrapped_line_count(&transcript, transcript_text_area.width as usize);
        let transcript_max_scroll = transcript_total_rows
            .saturating_sub(transcript_viewport_rows)
            .min(u16::MAX as usize) as u16;
        app.update_chat_scroll_max(transcript_max_scroll);
        let transcript_scroll = app.chat_scroll().min(transcript_max_scroll);

        let transcript_panel = Paragraph::new(transcript)
            .style(panel_surface_style(self.colors))
            .scroll((transcript_scroll, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(transcript_panel, transcript_text_area);

        if transcript_max_scroll > 0 {
            if let Some(scrollbar_area) = transcript_scrollbar_area {
                let scrollbar_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(scrollbar_area);
                let scrollbar_content_area = scrollbar_layout[0];
                let scrollbar_position = if transcript_max_scroll == 0 {
                    0
                } else {
                    let max_thumb_position = transcript_total_rows.saturating_sub(1) as f64;
                    ((transcript_scroll as f64 / transcript_max_scroll as f64) * max_thumb_position)
                        .round() as usize
                };
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .thumb_symbol("█")
                    .thumb_style(Style::default().fg(self.colors.border_focused))
                    .track_symbol(None)
                    .begin_symbol(None)
                    .end_symbol(None);
                let mut scrollbar_state = ScrollbarState::new(transcript_total_rows.max(1))
                    .position(scrollbar_position)
                    .viewport_content_length(transcript_viewport_rows.max(1));
                frame.render_stateful_widget(
                    scrollbar,
                    scrollbar_content_area,
                    &mut scrollbar_state,
                );
            }
        }

        render_transcript_indicator_row(
            frame,
            transcript_indicator_area,
            transcript_focused,
            transcript_scroll,
            transcript_max_scroll,
            self.colors,
        );

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
                "Type a message (Enter to send, Shift+Enter for newline)",
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

fn wrapped_line_count(lines: &[Line<'_>], wrap_width: usize) -> usize {
    if wrap_width == 0 {
        return 0;
    }

    lines
        .iter()
        .map(|line| wrapped_rows_for_line(line, wrap_width))
        .sum()
}

fn wrapped_rows_for_line(line: &Line<'_>, wrap_width: usize) -> usize {
    if wrap_width == 0 {
        return 0;
    }

    // Ratatui wraps on word boundaries; using plain ceil(width/wrap_width) can undercount rows
    // and make the transcript's last wrapped lines unreachable by scroll.
    let mut line_width = 0usize;
    let mut word_width = 0usize;
    let mut whitespace_width = 0usize;
    let mut pending_whitespace = VecDeque::<usize>::new();
    let mut non_whitespace_previous = false;
    let mut wrapped_rows = 0usize;
    let mut saw_symbol = false;

    for span in &line.spans {
        for ch in span.content.chars() {
            let symbol_width = display_width(ch);
            if symbol_width > wrap_width {
                continue;
            }

            saw_symbol = true;
            let is_whitespace = ch.is_whitespace();
            let word_found = non_whitespace_previous && is_whitespace;
            let untrimmed_overflow =
                line_width == 0 && word_width + whitespace_width + symbol_width > wrap_width;

            if word_found || untrimmed_overflow {
                line_width += whitespace_width;
                line_width += word_width;
                pending_whitespace.clear();
                whitespace_width = 0;
                word_width = 0;
            }

            let line_full = line_width >= wrap_width;
            let pending_word_overflow =
                symbol_width > 0 && line_width + whitespace_width + word_width >= wrap_width;

            if line_full || pending_word_overflow {
                let mut remaining_width = wrap_width.saturating_sub(line_width);
                while let Some(width) = pending_whitespace.front().copied() {
                    if width > remaining_width {
                        break;
                    }

                    whitespace_width = whitespace_width.saturating_sub(width);
                    remaining_width = remaining_width.saturating_sub(width);
                    pending_whitespace.pop_front();
                }

                wrapped_rows += 1;
                line_width = 0;

                // First whitespace on a wrapped line is dropped from the next segment.
                if is_whitespace && pending_whitespace.is_empty() {
                    non_whitespace_previous = false;
                    continue;
                }
            }

            if is_whitespace {
                whitespace_width += symbol_width;
                pending_whitespace.push_back(symbol_width);
            } else {
                word_width += symbol_width;
            }

            non_whitespace_previous = !is_whitespace;
        }
    }

    if !saw_symbol {
        return 1;
    }

    if line_width == 0 && word_width == 0 && whitespace_width > 0 {
        wrapped_rows += 1;
    } else {
        line_width += whitespace_width;
        line_width += word_width;
        if line_width > 0 {
            wrapped_rows += 1;
        }
    }

    wrapped_rows.max(1)
}

fn display_width(ch: char) -> usize {
    if ch == '\t' {
        4
    } else if ch.is_control() {
        0
    } else {
        1
    }
}

fn render_transcript_indicator_row(
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    scroll: u16,
    max_scroll: u16,
    colors: UiColors,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let can_scroll_up = scroll > 0;
    let can_scroll_down = scroll < max_scroll;
    let mut cells = vec!['─'; area.width as usize];
    let mut up_idx = None;
    let mut down_idx = None;
    if !cells.is_empty() {
        if can_scroll_up {
            cells[0] = '↑';
            up_idx = Some(0usize);
        }
        let last = cells.len() - 1;
        if can_scroll_down {
            cells[last] = '↓';
            down_idx = Some(last);
        } else if !can_scroll_up {
            cells[last] = ' ';
        }
    }

    let base_style = Style::default()
        .bg(colors.panel_background)
        .fg(if focused {
            colors.border_focused
        } else {
            colors.border_default
        })
        .add_modifier(if focused {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let arrow_style = base_style.fg(Color::Indexed(221));
    let indicator_line = Line::from(
        cells
            .into_iter()
            .enumerate()
            .map(|(idx, ch)| {
                let style = if Some(idx) == up_idx || Some(idx) == down_idx {
                    arrow_style
                } else {
                    base_style
                };
                Span::styled(ch.to_string(), style)
            })
            .collect::<Vec<_>>(),
    );

    frame.render_widget(Paragraph::new(indicator_line), area);
}

fn render_model_picker_overlay(frame: &mut Frame, app: &App, area: Rect, colors: UiColors) {
    if area.width < 30 || area.height < 10 {
        return;
    }

    let popup_width = area.width.saturating_sub(6).min(96);
    let popup_height = area.height.saturating_sub(6).min(16);
    if popup_width < 24 || popup_height < 8 {
        return;
    }

    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title("Select Model")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(colors.border_focused)
                .add_modifier(Modifier::BOLD),
        )
        .style(panel_surface_style(colors));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.width < 4 || inner.height < 5 {
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "Use /model in composer to open this selector.",
            Style::default().fg(colors.muted_text),
        )),
        Line::from(Span::styled(
            "Enter: apply model    Esc: dismiss",
            Style::default().fg(colors.muted_text),
        )),
    ])
    .style(panel_surface_style(colors))
    .wrap(Wrap { trim: false });
    frame.render_widget(header, layout[0]);

    let items = app
        .model_picker_options()
        .iter()
        .enumerate()
        .map(|(idx, choice)| render_model_choice_item(idx, choice, colors))
        .collect::<Vec<_>>();
    let list = List::new(items)
        .style(panel_surface_style(colors))
        .highlight_style(
            Style::default()
                .bg(colors.list_highlight_background)
                .fg(colors.list_highlight_foreground)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    let mut state = ListState::default();
    state.select(Some(
        app.model_picker_selected()
            .min(app.model_picker_options().len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(list, layout[1], &mut state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Current: ", Style::default().fg(colors.muted_text)),
        Span::styled(
            app.active_model_label().to_owned(),
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(panel_surface_style(colors))
    .wrap(Wrap { trim: false });
    frame.render_widget(footer, layout[2]);
}

fn render_model_choice_item(
    idx: usize,
    choice: &ModelChoice,
    colors: UiColors,
) -> ListItem<'static> {
    let mut line = vec![
        Span::styled(
            format!("{}. ", idx + 1),
            Style::default().fg(colors.muted_text),
        ),
        Span::styled(
            choice.id.clone(),
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if choice.is_current {
        line.push(Span::styled(
            " (current)",
            Style::default()
                .fg(colors.role_agent)
                .add_modifier(Modifier::BOLD),
        ));
    }
    line.push(Span::raw("  "));
    line.push(Span::styled(
        choice.description.clone(),
        Style::default().fg(colors.muted_text),
    ));

    ListItem::new(Line::from(line))
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

        let footer_height = if self.focused { 3 } else { 0 };
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
            let empty_message = if app.selected_worktree_has_git_repository() {
                "No changed files."
            } else {
                "No git repository found."
            };
            let panel = Paragraph::new(empty_message)
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
            app.right_multi_selected(),
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
            app.right_multi_selected(),
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
    multi_selected: &BTreeSet<usize>,
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
        let marked = multi_selected.contains(idx);

        let status_width = 4usize;
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
            if marked { "* " } else { "  " },
            Style::default().fg(colors.context_label),
        )];
        row.push(Span::styled(
            format!("{} ", change.kind.code()),
            Style::default()
                .fg(change_kind_color(change.kind, colors))
                .add_modifier(Modifier::BOLD),
        ));

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
    let clear_hotkey_style = if has_query {
        Style::default()
            .fg(colors.panel_foreground)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.muted_text)
    };

    let command_line = Line::from(vec![
        Span::styled(
            "Space",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" mark  ", Style::default().fg(colors.muted_text)),
        Span::styled(
            "X",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" clear marks  ", Style::default().fg(colors.muted_text)),
        Span::styled(
            "A",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" (un)stage", Style::default().fg(colors.muted_text)),
    ]);

    let mut search_line = vec![
        Span::styled(
            "/",
            Style::default()
                .fg(colors.panel_foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" search  ", Style::default().fg(colors.muted_text)),
        Span::styled("C", clear_hotkey_style),
        Span::styled(" clear query", Style::default().fg(colors.muted_text)),
    ];
    if has_query || search_active {
        search_line.extend([
            Span::styled("  |  ", Style::default().fg(colors.muted_text)),
            Span::styled("Query: ", Style::default().fg(colors.muted_text)),
            Span::styled("/", Style::default().fg(colors.panel_foreground)),
            Span::styled(
                trimmed_query.to_string(),
                Style::default().fg(colors.panel_foreground),
            ),
        ]);
        if search_active {
            search_line.push(Span::styled(
                "█",
                Style::default().fg(colors.panel_foreground),
            ));
        }
    }
    let lines = vec![command_line, Line::from(search_line)];

    let footer = Paragraph::new(lines).style(panel_surface_style(colors));
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

    if let Some(wave) = app.thinking_wave() {
        lines.push(Line::from(vec![
            Span::styled(
                "agent",
                Style::default()
                    .fg(colors.role_agent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("thinking ", Style::default().fg(colors.muted_text)),
            Span::styled(
                wave,
                Style::default()
                    .fg(colors.role_agent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines
}
