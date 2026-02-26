use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;
use crate::config::config_path;
use crate::provider::AuthStrategy;

pub fn render_auth_onboarding(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.ui_colors();
    let provider = app.active_provider_descriptor();
    let strategy_label = match provider.auth_strategy {
        AuthStrategy::Link => "Link login",
        AuthStrategy::ApiKey => "API key",
    };

    let onboarding_lines = vec![
        Line::from(vec![Span::styled(
            "Codex Sign-In Required",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(format!(
            "Active provider: {} ({})",
            provider.display_name, provider.id
        )),
        Line::from(format!("Auth strategy: {strategy_label}")),
        Line::from(""),
        Line::from("1. Press O to open the login link in your browser:"),
        Line::from(vec![Span::styled(
            provider.login_url,
            Style::default()
                .fg(colors.link)
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("2. Complete login in the browser."),
        Line::from("3. Return here and press Enter to continue."),
        Line::from(""),
        Line::from("Future-ready config fields are included for:"),
        Line::from("- custom base URL"),
        Line::from("- API key env var"),
        Line::from("- preferred model"),
        Line::from(""),
        Line::from(
            "Keys: O open link, Enter confirm login, R refresh CLI auth, T/Y/F8 test connection, P next provider, Q quit",
        ),
        Line::from(format!("Config file: {}", config_path().display())),
    ];

    let modal_area = centered_rect(74, 75, area);
    frame.render_widget(Clear, modal_area);
    let modal = Paragraph::new(onboarding_lines)
        .style(
            Style::default()
                .bg(colors.panel_background)
                .fg(colors.panel_foreground),
        )
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title("Provider Setup")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border_default)),
        );
    frame.render_widget(modal, modal_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}
