use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let provider = app.active_provider_descriptor();
    let status_text = format!(
        "Provider: {} ({}) | Focus: {} | Widths L/M/R: {}/{}/{} | {}",
        provider.display_name,
        provider.id,
        app.focused_panel_name(),
        app.panel_widths()[0],
        app.panel_widths()[1],
        app.panel_widths()[2],
        app.status_message()
    );
    let status = Paragraph::new(status_text).block(Block::default().borders(Borders::TOP));
    frame.render_widget(status, area);
}
