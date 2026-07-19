use std::{error::Error, fmt, io, io::Write};

use crossterm::{
    QueueableCommand,
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use punctum_grid::{GridSize, PatchKind, Surface, SurfaceError, diff};

use punctum_terminal::{
    TerminalCell, TerminalColor, TerminalPlanError, plan_patch, validate_cell_width,
};

#[derive(Debug)]
pub enum TerminalPresentError {
    Plan(TerminalPlanError),
    Surface(SurfaceError),
    Io(io::Error),
}

impl fmt::Display for TerminalPresentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => write!(formatter, "terminal frame is invalid: {error}"),
            Self::Surface(error) => write!(formatter, "terminal surface is invalid: {error}"),
            Self::Io(error) => write!(formatter, "terminal output failed: {error}"),
        }
    }
}

impl Error for TerminalPresentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            Self::Surface(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<TerminalPlanError> for TerminalPresentError {
    fn from(error: TerminalPlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<SurfaceError> for TerminalPresentError {
    fn from(error: SurfaceError) -> Self {
        Self::Surface(error)
    }
}

impl From<io::Error> for TerminalPresentError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct TerminalPresenter<W> {
    writer: W,
    previous: Option<Surface<TerminalCell>>,
    cell_width: u16,
}

impl<W> TerminalPresenter<W>
where
    W: Write,
{
    pub fn new(writer: W, cell_width: u16) -> Result<Self, TerminalPlanError> {
        validate_cell_width(cell_width)?;
        Ok(Self {
            writer,
            previous: None,
            cell_width,
        })
    }

    pub fn present(&mut self, frame: &Surface<TerminalCell>) -> Result<(), TerminalPresentError> {
        let empty = Surface::from_cells(GridSize::new(0, 0), Vec::new())?;
        let patch = diff(self.previous.as_ref().unwrap_or(&empty), frame);
        let runs = plan_patch(&patch, self.cell_width)?;

        if patch.kind() == PatchKind::Replace {
            self.writer.queue(Clear(ClearType::All))?;
        }
        for run in runs.runs() {
            self.writer.queue(MoveTo(run.col(), run.row()))?;
            let mut overflow_into_continuation = 0;
            for cell in run.cells() {
                self.writer
                    .queue(SetForegroundColor(color(cell.foreground())))?
                    .queue(SetBackgroundColor(color(cell.background())))?;
                if let Some(grapheme) = cell.grapheme() {
                    self.writer.queue(Print(grapheme))?;
                    let width = unicode_width::UnicodeWidthStr::width(grapheme);
                    let cell_width = usize::from(self.cell_width);
                    for _ in width..cell_width {
                        self.writer.queue(Print(' '))?;
                    }
                    overflow_into_continuation = width.saturating_sub(cell_width);
                } else {
                    let remaining =
                        usize::from(self.cell_width).saturating_sub(overflow_into_continuation);
                    for _ in 0..remaining {
                        self.writer.queue(Print(' '))?;
                    }
                    overflow_into_continuation =
                        overflow_into_continuation.saturating_sub(usize::from(self.cell_width));
                }
            }
        }
        let (cursor_col, cursor_row) = runs.final_cursor();
        self.writer
            .queue(ResetColor)?
            .queue(MoveTo(cursor_col, cursor_row))?;
        self.writer.flush()?;
        self.previous = Some(frame.clone());
        Ok(())
    }

    pub fn invalidate(&mut self) {
        self.previous = None;
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

fn color(color: TerminalColor) -> Color {
    match color {
        TerminalColor::Default => Color::Reset,
        TerminalColor::Black => Color::Black,
        TerminalColor::Gray => Color::DarkGrey,
        TerminalColor::White => Color::White,
        TerminalColor::Red => Color::Red,
        TerminalColor::Green => Color::Green,
        TerminalColor::Yellow => Color::Yellow,
        TerminalColor::Blue => Color::Blue,
        TerminalColor::Magenta => Color::Magenta,
        TerminalColor::Cyan => Color::Cyan,
        TerminalColor::Rgb { red, green, blue } => Color::Rgb {
            r: red,
            g: green,
            b: blue,
        },
    }
}

pub struct TerminalSession {
    active: bool,
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        if let Err(error) = execute!(
            io::stdout(),
            EnterAlternateScreen,
            Clear(ClearType::All),
            Hide
        ) {
            let _ = terminal::disable_raw_mode();
            return Err(error);
        }

        Ok(Self { active: true })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            let _ = execute!(io::stdout(), ResetColor, Show, LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();
            self.active = false;
        }
    }
}
