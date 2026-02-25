use ratatui::style::{Modifier, Style};
use ratatui::widgets::BorderType;

#[derive(Clone, Copy)]
pub struct PaneChrome {
    focused: bool,
    focused_border_style: Style,
    unfocused_border_style: Style,
}

impl PaneChrome {
    pub fn new(focused: bool, focused_border_style: Style, unfocused_border_style: Style) -> Self {
        Self {
            focused,
            focused_border_style,
            unfocused_border_style,
        }
    }

    pub fn border_style(self) -> Style {
        if self.focused {
            return self.focused_border_style.add_modifier(Modifier::BOLD);
        }

        self.unfocused_border_style
    }

    pub fn border_type(self) -> BorderType {
        if self.focused {
            return BorderType::Thick;
        }

        BorderType::Plain
    }
}
