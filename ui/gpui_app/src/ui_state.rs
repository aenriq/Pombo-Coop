use crate::color_system::ThemeSelection;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const UI_STATE_DIR: &str = ".agent-manager";
const UI_STATE_FILE: &str = "ui-state.toml";
const DEFAULT_LEFT_PANE_WIDTH_PX: f32 = 260.0;
const DEFAULT_RIGHT_PANE_WIDTH_PX: f32 = 320.0;
const MIN_SIDE_PANE_WIDTH_PX: f32 = 180.0;
const MAX_SIDE_PANE_WIDTH_PX: f32 = 640.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UiState {
    pub panes: PaneSizes,
    pub last_review_mode: ReviewMode,
    pub theme: ThemeSelection,
}

impl UiState {
    pub fn load() -> Result<Self, UiStateError> {
        Self::load_or_default(resolve_ui_state_path())
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, UiStateError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| UiStateError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let state: Self = toml::from_str(&raw).map_err(|source| UiStateError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

        state.validate()?;
        Ok(state)
    }

    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, UiStateError> {
        let path = path.as_ref();
        if !path.exists() {
            let state = Self::default();
            state.save_to_path(path)?;
            return Ok(state);
        }

        Self::load_from_path(path)
    }

    pub fn save(&self) -> Result<(), UiStateError> {
        self.save_to_path(resolve_ui_state_path())
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), UiStateError> {
        let path = path.as_ref();
        self.validate()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| UiStateError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let raw = toml::to_string_pretty(self).map_err(UiStateError::Serialize)?;
        fs::write(path, raw).map_err(|source| UiStateError::Write {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(())
    }

    pub fn validate(&self) -> Result<(), UiStateError> {
        self.panes.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneSizes {
    pub left_px: f32,
    pub right_px: f32,
}

impl Default for PaneSizes {
    fn default() -> Self {
        Self {
            left_px: DEFAULT_LEFT_PANE_WIDTH_PX,
            right_px: DEFAULT_RIGHT_PANE_WIDTH_PX,
        }
    }
}

impl PaneSizes {
    pub fn validate(&self) -> Result<(), UiStateError> {
        Self::validate_width_px("left_px", self.left_px)?;
        Self::validate_width_px("right_px", self.right_px)?;

        Ok(())
    }

    fn validate_width_px(name: &str, width_px: f32) -> Result<(), UiStateError> {
        if !width_px.is_finite() {
            return Err(UiStateError::Validation(format!(
                "pane '{name}' width must be finite"
            )));
        }

        if width_px < MIN_SIDE_PANE_WIDTH_PX {
            return Err(UiStateError::Validation(format!(
                "pane '{name}' width must be at least {MIN_SIDE_PANE_WIDTH_PX:.1}px, got {width_px:.1}px"
            )));
        }

        if width_px > MAX_SIDE_PANE_WIDTH_PX {
            return Err(UiStateError::Validation(format!(
                "pane '{name}' width must be at most {MAX_SIDE_PANE_WIDTH_PX:.1}px, got {width_px:.1}px"
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    #[default]
    Unified,
    Split,
}

pub fn resolve_ui_state_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(UI_STATE_DIR)
        .join(UI_STATE_FILE)
}

#[derive(Debug)]
pub enum UiStateError {
    Validation(String),
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Serialize(toml::ser::Error),
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for UiStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiStateError::Validation(message) => write!(f, "validation error: {message}"),
            UiStateError::Read { path, source } => {
                write!(f, "failed to read UI state at {}: {source}", path.display())
            }
            UiStateError::Parse { path, source } => {
                write!(
                    f,
                    "failed to parse UI state at {}: {source}",
                    path.display()
                )
            }
            UiStateError::Serialize(source) => {
                write!(f, "failed to serialize UI state: {source}")
            }
            UiStateError::CreateDir { path, source } => write!(
                f,
                "failed to create UI state parent directory {}: {source}",
                path.display()
            ),
            UiStateError::Write { path, source } => {
                write!(
                    f,
                    "failed to write UI state at {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for UiStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UiStateError::Validation(_) => None,
            UiStateError::Read { source, .. } => Some(source),
            UiStateError::Parse { source, .. } => Some(source),
            UiStateError::Serialize(source) => Some(source),
            UiStateError::CreateDir { source, .. } => Some(source),
            UiStateError::Write { source, .. } => Some(source),
        }
    }
}
