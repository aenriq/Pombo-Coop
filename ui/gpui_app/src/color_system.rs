use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    #[serde(alias = "stone")]
    #[default]
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ThemeSelection {
    pub name: ThemeName,
    pub mode: ThemeMode,
}

impl ThemeSelection {
    pub fn toggled_mode(self) -> Self {
        Self {
            name: self.name,
            mode: self.mode.toggled(),
        }
    }

    pub fn palette(self) -> ThemePalette {
        match (self.name, self.mode) {
            (ThemeName::Neutral, ThemeMode::Light) => ThemePalette::neutral_light(),
            (ThemeName::Neutral, ThemeMode::Dark) => ThemePalette::neutral_dark(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub app_bg: u32,
    pub panel_bg: u32,
    pub left_panel_bg: u32,
    pub header_bg: u32,
    pub card_bg: u32,
    pub border: u32,
    pub border_strong: u32,
    pub text_primary: u32,
    pub text_muted: u32,
    pub primary: u32,
    #[allow(dead_code)]
    pub primary_foreground: u32,
    pub secondary: u32,
    pub secondary_foreground: u32,
    pub muted: u32,
    pub accent: u32,
    pub accent_foreground: u32,
    pub muted_foreground: u32,
    pub destructive: u32,
    pub destructive_foreground: u32,
    pub success: u32,
    pub success_foreground: u32,
    pub warning: u32,
    pub warning_foreground: u32,
    pub splitter_hover: u32,
}

impl ThemePalette {
    fn neutral_light() -> Self {
        Self {
            app_bg: 0xffffff,
            panel_bg: 0xffffff,
            left_panel_bg: 0xfafafa,
            header_bg: 0xf5f5f5,
            card_bg: 0xffffff,
            border: 0xe5e5e5,
            border_strong: 0xd4d4d4,
            text_primary: 0x171717,
            text_muted: 0x737373,
            primary: 0x171717,
            primary_foreground: 0xfafafa,
            secondary: 0xf5f5f5,
            secondary_foreground: 0x171717,
            muted: 0xf5f5f5,
            accent: 0xf5f5f5,
            accent_foreground: 0x171717,
            muted_foreground: 0x737373,
            destructive: 0xdc2626,
            destructive_foreground: 0xfef2f2,
            success: 0x16a34a,
            success_foreground: 0xf0fdf4,
            warning: 0xd97706,
            warning_foreground: 0xfffbeb,
            splitter_hover: 0xd4d4d4,
        }
    }

    fn neutral_dark() -> Self {
        Self {
            app_bg: 0x0a0a0a,
            panel_bg: 0x0a0a0a,
            left_panel_bg: 0x171717,
            header_bg: 0x171717,
            card_bg: 0x101010,
            border: 0x262626,
            border_strong: 0x404040,
            text_primary: 0xf5f5f5,
            text_muted: 0xa3a3a3,
            primary: 0xf5f5f5,
            primary_foreground: 0x171717,
            secondary: 0x262626,
            secondary_foreground: 0xf5f5f5,
            muted: 0x262626,
            accent: 0x262626,
            accent_foreground: 0xf5f5f5,
            muted_foreground: 0xa3a3a3,
            destructive: 0xef4444,
            destructive_foreground: 0xfef2f2,
            success: 0x22c55e,
            success_foreground: 0x052e16,
            warning: 0xf59e0b,
            warning_foreground: 0x451a03,
            splitter_hover: 0x404040,
        }
    }
}
