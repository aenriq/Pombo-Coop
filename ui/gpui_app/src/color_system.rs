use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    #[default]
    Stone,
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
            (ThemeName::Stone, ThemeMode::Light) => ThemePalette::stone_light(),
            (ThemeName::Stone, ThemeMode::Dark) => ThemePalette::stone_dark(),
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
    fn stone_light() -> Self {
        Self {
            app_bg: 0xffffff,
            panel_bg: 0xfafaf9,
            left_panel_bg: 0xfafaf9,
            header_bg: 0xf5f5f4,
            card_bg: 0xffffff,
            border: 0xe7e5e4,
            border_strong: 0xd6d3d1,
            text_primary: 0x1c1917,
            text_muted: 0x78716c,
            primary: 0x292524,
            primary_foreground: 0xfafaf9,
            secondary: 0xf5f5f4,
            secondary_foreground: 0x292524,
            muted: 0xf5f5f4,
            accent: 0xf5f5f4,
            accent_foreground: 0x292524,
            muted_foreground: 0x78716c,
            destructive: 0xdc2626,
            destructive_foreground: 0xfef2f2,
            success: 0x16a34a,
            success_foreground: 0xf0fdf4,
            warning: 0xd97706,
            warning_foreground: 0xfffbeb,
            splitter_hover: 0xd6d3d1,
        }
    }

    fn stone_dark() -> Self {
        Self {
            app_bg: 0x1c1917,
            panel_bg: 0x1c1917,
            left_panel_bg: 0x292524,
            header_bg: 0x292524,
            card_bg: 0x1f1b19,
            border: 0x44403c,
            border_strong: 0x57534e,
            text_primary: 0xfafaf9,
            text_muted: 0xa8a29e,
            primary: 0xe7e5e4,
            primary_foreground: 0x292524,
            secondary: 0x44403c,
            secondary_foreground: 0xfafaf9,
            muted: 0x44403c,
            accent: 0x44403c,
            accent_foreground: 0xfafaf9,
            muted_foreground: 0xa8a29e,
            destructive: 0xef4444,
            destructive_foreground: 0xfef2f2,
            success: 0x22c55e,
            success_foreground: 0x052e16,
            warning: 0xf59e0b,
            warning_foreground: 0x451a03,
            splitter_hover: 0x57534e,
        }
    }
}
