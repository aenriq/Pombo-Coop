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
