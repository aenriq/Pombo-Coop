pub mod components;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    if app.auth_required() {
        render_auth_view(frame, app);
    } else {
        render_dashboard(frame, app);
    }
}

fn render_dashboard(frame: &mut Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(frame.area());

    let widths = app.panel_widths();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(widths[0]),
            Constraint::Percentage(widths[1]),
            Constraint::Percentage(widths[2]),
        ])
        .split(layout[0]);

    components::panels::render_left_panel(frame, app, columns[0], app.focused_panel() == 0);
    components::panels::render_middle_panel(frame, app, columns[1], app.focused_panel() == 1);
    components::panels::render_right_panel(frame, app, columns[2], app.focused_panel() == 2);
    components::status_bar::render_status_bar(frame, app, layout[1]);
}

fn render_auth_view(frame: &mut Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(frame.area());

    components::onboarding::render_auth_onboarding(frame, app, layout[0]);
    components::status_bar::render_status_bar(frame, app, layout[1]);
}
