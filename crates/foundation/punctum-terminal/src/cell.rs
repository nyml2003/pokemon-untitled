use std::{error::Error, fmt};

use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    #[default]
    Default,
    Black,
    Gray,
    White,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Rgb {
        red: u8,
        green: u8,
        blue: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TerminalCell {
    content: TerminalCellContent,
    foreground: TerminalColor,
    background: TerminalColor,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TerminalCellContent {
    Grapheme(String),
    Continuation,
}

impl TerminalCell {
    pub fn new(symbol: char, foreground: TerminalColor, background: TerminalColor) -> Self {
        Self {
            content: TerminalCellContent::Grapheme(symbol.into()),
            foreground,
            background,
        }
    }

    pub fn from_grapheme(
        grapheme: impl Into<String>,
        foreground: TerminalColor,
        background: TerminalColor,
    ) -> Result<Self, TerminalCellError> {
        let grapheme = grapheme.into();
        let mut graphemes = grapheme.graphemes(true);
        if graphemes.next().is_none() {
            return Err(TerminalCellError::EmptyGrapheme);
        }
        if graphemes.next().is_some() {
            return Err(TerminalCellError::MultipleGraphemes);
        }

        Ok(Self {
            content: TerminalCellContent::Grapheme(grapheme),
            foreground,
            background,
        })
    }

    pub fn grapheme(&self) -> Option<&str> {
        match &self.content {
            TerminalCellContent::Grapheme(grapheme) => Some(grapheme),
            TerminalCellContent::Continuation => None,
        }
    }

    pub const fn foreground(&self) -> TerminalColor {
        self.foreground
    }

    pub const fn background(&self) -> TerminalColor {
        self.background
    }

    pub const fn is_continuation(&self) -> bool {
        matches!(self.content, TerminalCellContent::Continuation)
    }

    pub(crate) const fn continuation(foreground: TerminalColor, background: TerminalColor) -> Self {
        Self {
            content: TerminalCellContent::Continuation,
            foreground,
            background,
        }
    }
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self::new(' ', TerminalColor::Default, TerminalColor::Default)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalCellError {
    EmptyGrapheme,
    MultipleGraphemes,
}

impl fmt::Display for TerminalCellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrapheme => formatter.write_str("terminal cell grapheme must not be empty"),
            Self::MultipleGraphemes => {
                formatter.write_str("terminal cell must contain exactly one grapheme")
            }
        }
    }
}

impl Error for TerminalCellError {}
