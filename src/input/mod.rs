use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if app.auth_required() {
        handle_auth_key_event(app, key);
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

    if let Some(direction) = panel_move_direction(&key) {
        app.move_within_focused_panel(direction);
        return;
    }

    if key.code == KeyCode::Char('q') {
        app.request_quit();
    }
}

fn handle_auth_key_event(app: &mut App, key: KeyEvent) {
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

fn panel_focus_direction(key: &KeyEvent) -> Option<i8> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT)
    {
        return None;
    }

    match key.code {
        KeyCode::Left | KeyCode::Char('h') => Some(-1),
        KeyCode::Right | KeyCode::Char('l') => Some(1),
        _ => None,
    }
}

fn subpanel_focus_direction(key: &KeyEvent) -> Option<i8> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT)
    {
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
