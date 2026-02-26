use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.ui_colors();
    let provider = app.active_provider_descriptor();
    let widths = app.effective_panel_widths(area.width);
    let focus_expand = app.panel_focus_expand_mode_summary(area.width);
    let subpanel_label = if app.focused_panel() == 1 {
        app.chat_subpanel_name()
    } else {
        "-"
    };
    let status_text = format!(
        "Provider: {} ({}) | Focus: {} | Subpanel: {} | Widths L/M/R: {}/{}/{} | Focus expand: {} | {}",
        provider.display_name,
        provider.id,
        app.focused_panel_name(),
        subpanel_label,
        widths[0],
        widths[1],
        widths[2],
        focus_expand,
        app.status_message()
    );
    let max_chars = area.width.saturating_sub(1) as usize;
    let status_text = truncate_for_bar(&status_text, max_chars);
    let status = Paragraph::new(status_text)
        .style(
            Style::default()
                .fg(colors.status_text)
                .bg(colors.panel_background),
        )
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(colors.border_default)),
        );
    frame.render_widget(status, area);
}

fn truncate_for_bar(value: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars == 1 {
        return "…".to_string();
    }

    let keep = max_chars - 1;
    let prefix = value.chars().take(keep).collect::<String>();
    format!("{prefix}…")
}
