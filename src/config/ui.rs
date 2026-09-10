//! Configuration of the user interface output.

use serde::Deserialize;
use serde::Serialize;

use crate::colors::Color;
use crate::colors::ColoredText;

/// Configuration for the user interface.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct UiConfig {
    /// Prefix for hint logs.
    #[serde(default = "UiConfig::default_hint")]
    pub hint: ColoredText,

    /// Prefix for information logs.
    #[serde(default = "UiConfig::default_warning")]
    pub warning: ColoredText,

    /// Prefix for error logs.
    #[serde(default = "UiConfig::default_error")]
    pub error: ColoredText,
}

#[allow(clippy::missing_docs_in_private_items)]
impl UiConfig {
    fn default_hint() -> ColoredText {
        ColoredText::new("Hint", Color::cyan())
    }

    fn default_warning() -> ColoredText {
        ColoredText::new("Warning", Color::ansi_color(166))
    }

    fn default_error() -> ColoredText {
        ColoredText::new("Error", Color::red())
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            hint: Self::default_hint(),
            warning: Self::default_warning(),
            error: Self::default_error(),
        }
    }
}
