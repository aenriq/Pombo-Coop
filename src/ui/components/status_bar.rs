use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.ui_colors();
    let provider = app.active_provider_descriptor();
    let subpanel_label = if app.focused_panel() == 1 {
        app.chat_subpanel_name()
    } else {
        "-"
    };
    let status_text = format!(
        "Provider: {} ({}) | Focus: {} | Subpanel: {} | Widths L/M/R: {}/{}/{} | {}",
        provider.display_name,
        provider.id,
        app.focused_panel_name(),
        subpanel_label,
        app.panel_widths()[0],
        app.panel_widths()[1],
        app.panel_widths()[2],
        app.status_message()
    );
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(colors.status_text).bg(colors.panel_background))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(colors.border_default)),
        );
    frame.render_widget(status, area);
}
