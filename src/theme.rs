use ratatui::style::Color;

use crate::config::UiColorsConfig;

#[derive(Debug, Clone, Copy)]
pub struct UiColors {
    pub panel_background: Color,
    pub panel_foreground: Color,
    pub border_default: Color,
    pub border_focused: Color,
    pub list_highlight_background: Color,
    pub list_highlight_foreground: Color,
    pub model_title: Color,
    pub added: Color,
    pub removed: Color,
    pub context_label: Color,
    pub summary_label: Color,
    pub line_number: Color,
    pub line_added_number: Color,
    pub line_added_text: Color,
    pub role_agent: Color,
    pub role_user: Color,
    pub role_system: Color,
    pub edit_prefix: Color,
    pub edit_path: Color,
    pub muted_text: Color,
    pub link: Color,
    pub status_text: Color,
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            panel_background: Color::Indexed(234),
            panel_foreground: Color::Indexed(252),
            border_default: Color::Indexed(240),
            border_focused: Color::Indexed(44),
            list_highlight_background: Color::Indexed(238),
            list_highlight_foreground: Color::Indexed(255),
            model_title: Color::Indexed(180),
            added: Color::Indexed(78),
            removed: Color::Indexed(203),
            context_label: Color::Indexed(75),
            summary_label: Color::Indexed(245),
            line_number: Color::Indexed(243),
            line_added_number: Color::Indexed(180),
            line_added_text: Color::Indexed(110),
            role_agent: Color::Indexed(114),
            role_user: Color::Indexed(81),
            role_system: Color::Indexed(180),
            edit_prefix: Color::Indexed(180),
            edit_path: Color::Indexed(75),
            muted_text: Color::Indexed(245),
            link: Color::Indexed(117),
            status_text: Color::Indexed(245),
        }
    }
}

impl UiColors {
    pub fn from_config(config: &UiColorsConfig) -> Self {
        let mut colors = Self::default();
        colors.panel_background =
            override_color(colors.panel_background, config.panel_background.as_deref());
        colors.panel_foreground =
            override_color(colors.panel_foreground, config.panel_foreground.as_deref());
        colors.border_default =
            override_color(colors.border_default, config.border_default.as_deref());
        colors.border_focused =
            override_color(colors.border_focused, config.border_focused.as_deref());
        colors.list_highlight_background = override_color(
            colors.list_highlight_background,
            config.list_highlight_background.as_deref(),
        );
        colors.list_highlight_foreground = override_color(
            colors.list_highlight_foreground,
            config.list_highlight_foreground.as_deref(),
        );
        colors.model_title = override_color(colors.model_title, config.model_title.as_deref());
        colors.added = override_color(colors.added, config.added.as_deref());
        colors.removed = override_color(colors.removed, config.removed.as_deref());
        colors.context_label =
            override_color(colors.context_label, config.context_label.as_deref());
        colors.summary_label =
            override_color(colors.summary_label, config.summary_label.as_deref());
        colors.line_number = override_color(colors.line_number, config.line_number.as_deref());
        colors.line_added_number = override_color(
            colors.line_added_number,
            config.line_added_number.as_deref(),
        );
        colors.line_added_text =
            override_color(colors.line_added_text, config.line_added_text.as_deref());
        colors.role_agent = override_color(colors.role_agent, config.role_agent.as_deref());
        colors.role_user = override_color(colors.role_user, config.role_user.as_deref());
        colors.role_system = override_color(colors.role_system, config.role_system.as_deref());
        colors.edit_prefix = override_color(colors.edit_prefix, config.edit_prefix.as_deref());
        colors.edit_path = override_color(colors.edit_path, config.edit_path.as_deref());
        colors.muted_text = override_color(colors.muted_text, config.muted_text.as_deref());
        colors.link = override_color(colors.link, config.link.as_deref());
        colors.status_text = override_color(colors.status_text, config.status_text.as_deref());
        colors
    }
}

fn override_color(default: Color, raw: Option<&str>) -> Color {
    raw.and_then(parse_color).unwrap_or(default)
}

fn parse_color(raw: &str) -> Option<Color> {
    let color = raw.trim();
    if color.is_empty() {
        return None;
    }

    if let Some(hex) = color.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some(index) = color
        .strip_prefix("indexed:")
        .or_else(|| color.strip_prefix("index:"))
        .and_then(|value| value.parse::<u8>().ok())
    {
        return Some(Color::Indexed(index));
    }

    let normalized = color
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_");

    match normalized.as_str() {
        "reset" => Some(Color::Reset),
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" | "teal" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "light_red" => Some(Color::LightRed),
        "light_green" => Some(Color::LightGreen),
        "light_yellow" => Some(Color::LightYellow),
        "light_blue" => Some(Color::LightBlue),
        "light_magenta" => Some(Color::LightMagenta),
        "light_cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(red, green, blue))
}
