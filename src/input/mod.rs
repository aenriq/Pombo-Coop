use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::{App, ChatSubpanel};

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if app.auth_required() {
        handle_auth_key_event(app, key);
        return;
    }

    if app.handle_model_picker_key(key) {
        return;
    }

    if app.handle_agent_rename_prompt_key(key) {
        return;
    }

    if app.handle_worktree_name_prompt_key(key) {
        return;
    }

    if is_connection_test_shortcut(&key) {
        app.run_connection_test();
        return;
    }

    if handle_chat_transcript_scroll_shortcuts(app, key) {
        return;
    }

    if app.handle_composer_key(key) {
        return;
    }

    if handle_right_search_shortcut(app, key) {
        return;
    }

    if app.handle_right_search_key(key) {
        return;
    }

    if app.handle_left_panel_shortcuts(key) {
        return;
    }

    if handle_right_search_clear_shortcut(app, key) {
        return;
    }

    if let Some(direction) = subpanel_focus_direction(&key) {
        app.focus_subpanel(direction);
        return;
    }

    if let Some(direction) = panel_focus_direction(&key) {
        if direction > 0 {
            app.focus_next_panel();
        } else {
            app.focus_previous_panel();
        }
        return;
    }

    if let Some(direction) = panel_resize_direction(&key) {
        app.resize_focused_panel(direction);
        return;
    }

    if handle_multi_select_shortcuts(app, key) {
        return;
    }

    if handle_stage_shortcuts(app, key) {
        return;
    }

    if let Some(direction) = panel_move_direction(&key) {
        app.move_within_focused_panel(direction);
        return;
    }

    if key.code == KeyCode::Char('q') {
        app.request_quit();
    }
}

fn handle_auth_key_event(app: &mut App, key: KeyEvent) {
    if is_connection_test_shortcut(&key)
        || matches!(
            key.code,
            KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Char('y') | KeyCode::Char('Y')
        )
    {
        app.run_connection_test();
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.request_quit(),
        KeyCode::Char('o') | KeyCode::Char('O') => app.open_provider_login(),
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('L') => app.complete_link_login(),
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.refresh_auth_from_local_cli(true);
        }
        KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Tab => app.cycle_provider(),
        _ => {}
    }
}

fn is_connection_test_shortcut(key: &KeyEvent) -> bool {
    if key.modifiers.is_empty() && matches!(key.code, KeyCode::F(8)) {
        return true;
    }

    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let has_super = key.modifiers.contains(KeyModifiers::SUPER);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    (has_ctrl || has_super)
        && !has_alt
        && matches!(
            key.code,
            KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Char('y') | KeyCode::Char('Y')
        )
}

fn panel_focus_direction(key: &KeyEvent) -> Option<i8> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }

    match key.code {
        KeyCode::Left | KeyCode::Char('h') => Some(-1),
        KeyCode::Right | KeyCode::Char('l') => Some(1),
        _ => None,
    }
}

fn subpanel_focus_direction(key: &KeyEvent) -> Option<i8> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(-1),
        KeyCode::Down | KeyCode::Char('j') => Some(1),
        _ => None,
    }
}

fn panel_resize_direction(key: &KeyEvent) -> Option<i8> {
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    if has_ctrl {
        return None;
    }

    if has_alt {
        return match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Down | KeyCode::Char('j') => Some(-1),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Up | KeyCode::Char('k') => Some(1),
            _ => None,
        };
    }

    #[cfg(target_os = "macos")]
    {
        // Some terminal profiles send Option+h/j/k/l as transformed symbols, not ALT-modified keys.
        return match key.code {
            KeyCode::Char('\u{02D9}') | KeyCode::Char('\u{2206}') => Some(-1),
            KeyCode::Char('\u{02DA}') | KeyCode::Char('\u{00AC}') => Some(1),
            _ => None,
        };
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn panel_move_direction(key: &KeyEvent) -> Option<i8> {
    if !key.modifiers.is_empty() {
        return None;
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') => Some(1),
        KeyCode::Up | KeyCode::Char('k') => Some(-1),
        _ => None,
    }
}

fn handle_chat_transcript_scroll_shortcuts(app: &mut App, key: KeyEvent) -> bool {
    if app.focused_panel() != 1 || !key.modifiers.is_empty() {
        return false;
    }

    let step = 6;
    match key.code {
        KeyCode::PageUp => {
            for _ in 0..step {
                app.scroll_chat_transcript(-1);
            }
            true
        }
        KeyCode::PageDown => {
            for _ in 0..step {
                app.scroll_chat_transcript(1);
            }
            true
        }
        _ => false,
    }
}

fn handle_stage_shortcuts(app: &mut App, key: KeyEvent) -> bool {
    if app.focused_panel() != 2 {
        return false;
    }

    if app.right_search_active() {
        return false;
    }

    if !key.modifiers.is_empty() {
        return false;
    }

    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.toggle_selected_changed_file_staging();
            true
        }
        _ => false,
    }
}

fn handle_multi_select_shortcuts(app: &mut App, key: KeyEvent) -> bool {
    if app.focused_panel() != 2 || app.right_search_active() {
        return false;
    }

    if !key.modifiers.is_empty() {
        return false;
    }

    match key.code {
        KeyCode::Char(' ') => {
            app.toggle_right_multi_selected();
            true
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            app.clear_right_multi_selected();
            true
        }
        _ => false,
    }
}

fn handle_right_search_shortcut(app: &mut App, key: KeyEvent) -> bool {
    if app.focused_panel() != 2 || app.right_search_active() {
        return false;
    }

    if !key.modifiers.is_empty() {
        return false;
    }

    if key.code == KeyCode::Char('/') {
        app.open_right_search();
        return true;
    }

    false
}

fn handle_right_search_clear_shortcut(app: &mut App, key: KeyEvent) -> bool {
    if app.focused_panel() != 2 || app.right_search_active() || !app.right_search_has_query() {
        return false;
    }

    if !key.modifiers.is_empty() {
        return false;
    }

    if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')) {
        app.clear_right_search();
        return true;
    }

    false
}

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent, terminal_area: Rect) -> bool {
    if app.auth_required() {
        return false;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(terminal_area);
    let widths = app.effective_panel_widths(layout[0].width);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(widths[0]),
            Constraint::Percentage(widths[1]),
            Constraint::Percentage(widths[2]),
        ])
        .split(layout[0]);
    let Some(panel_idx) = columns
        .iter()
        .position(|area| point_in_rect(*area, mouse.column, mouse.row))
    else {
        return false;
    };

    match mouse.kind {
        MouseEventKind::Down(_) => {
            let was_focused = app.focused_panel();
            app.focus_panel_by_index(panel_idx);
            let mut changed = was_focused != panel_idx;
            if panel_idx == 1 {
                changed |= focus_chat_subpanel_at_mouse_row(app, columns[1], mouse.row);
            }
            if panel_idx == 2 {
                changed |= select_changed_file_at_mouse_row(app, columns[2], mouse.row, true);
            }
            changed
        }
        MouseEventKind::Moved | MouseEventKind::Drag(_) => {
            if panel_idx != 2 {
                return false;
            }

            select_changed_file_at_mouse_row(app, columns[2], mouse.row, false)
        }
        MouseEventKind::ScrollUp => match panel_idx {
            1 => {
                let was_focused = app.focused_panel();
                app.focus_panel_by_index(1);
                app.focus_subpanel(-1);
                app.scroll_chat_transcript(-1);
                was_focused != 1 || !app.chat_messages().is_empty()
            }
            2 => {
                let was_focused = app.focused_panel();
                app.focus_panel_by_index(2);
                app.move_within_focused_panel(-1);
                was_focused != 2 || !app.selected_worktree().changed_files.is_empty()
            }
            _ => false,
        },
        MouseEventKind::ScrollDown => match panel_idx {
            1 => {
                let was_focused = app.focused_panel();
                app.focus_panel_by_index(1);
                app.focus_subpanel(-1);
                app.scroll_chat_transcript(1);
                was_focused != 1 || !app.chat_messages().is_empty()
            }
            2 => {
                let was_focused = app.focused_panel();
                app.focus_panel_by_index(2);
                app.move_within_focused_panel(1);
                was_focused != 2 || !app.selected_worktree().changed_files.is_empty()
            }
            _ => false,
        },
        _ => false,
    }
}

fn select_changed_file_at_mouse_row(
    app: &mut App,
    right_panel: Rect,
    mouse_row: u16,
    force_focus: bool,
) -> bool {
    if right_panel.width < 3 || right_panel.height < 3 {
        return false;
    }

    let mut content_area = Rect {
        x: right_panel.x.saturating_add(1),
        y: right_panel.y.saturating_add(1),
        width: right_panel.width.saturating_sub(2),
        height: right_panel.height.saturating_sub(2),
    };
    if app.focused_panel() == 2 {
        let footer_height = 3;
        if content_area.height > footer_height {
            content_area.height = content_area.height.saturating_sub(footer_height);
        }
    }

    if mouse_row < content_area.y || mouse_row >= content_area.y.saturating_add(content_area.height)
    {
        return false;
    }

    let row = mouse_row.saturating_sub(content_area.y) as usize;
    let mut changed = false;
    if let Some(file_idx) = app.changed_file_index_for_list_row(row) {
        if force_focus {
            let was_focused = app.focused_panel();
            app.focus_panel_by_index(2);
            changed |= was_focused != 2;
        }
        changed |= app.select_right_file(file_idx);
    }
    changed
}

fn focus_chat_subpanel_at_mouse_row(app: &mut App, middle_panel: Rect, mouse_row: u16) -> bool {
    let Some(target) = chat_subpanel_at_row(middle_panel, mouse_row) else {
        return false;
    };

    if app.chat_subpanel() == target {
        return false;
    }

    let direction = match target {
        ChatSubpanel::Transcript => -1,
        ChatSubpanel::Composer => 1,
    };
    app.focus_subpanel(direction);
    true
}

fn chat_subpanel_at_row(middle_panel: Rect, mouse_row: u16) -> Option<ChatSubpanel> {
    if middle_panel.width < 3 || middle_panel.height < 3 {
        return None;
    }

    let inner = Rect {
        x: middle_panel.x.saturating_add(1),
        y: middle_panel.y.saturating_add(1),
        width: middle_panel.width.saturating_sub(2),
        height: middle_panel.height.saturating_sub(2),
    };
    if mouse_row < inner.y || mouse_row >= inner.y.saturating_add(inner.height) {
        return None;
    }

    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(7)])
        .split(inner);

    if mouse_row >= split[1].y && mouse_row < split[1].y.saturating_add(split[1].height) {
        return Some(ChatSubpanel::Composer);
    }
    if mouse_row >= split[0].y && mouse_row < split[0].y.saturating_add(split[0].height) {
        return Some(ChatSubpanel::Transcript);
    }

    None
}

fn point_in_rect(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}
