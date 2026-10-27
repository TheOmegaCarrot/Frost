//! The terminal as a grid of cells, redrawn by difference.

use std::io::{self, Write};

use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{QueueableCommand, terminal};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) ch: char,
    pub(crate) fg: Color,
    pub(crate) bg: Color,
}

impl Cell {
    const BLANK: Cell = Cell {
        ch: ' ',
        fg: Color::Reset,
        bg: Color::Reset,
    };
}

/// A frame being drawn: every cell of the terminal.
pub(crate) struct Frame {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Frame {
    pub(crate) fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::BLANK; usize::from(width) * usize::from(height)],
        }
    }

    pub(crate) fn width(&self) -> u16 {
        self.width
    }

    pub(crate) fn height(&self) -> u16 {
        self.height
    }

    /// Set the cell at `x`, `y`, if it is on the frame.
    pub(crate) fn set(&mut self, x: i32, y: i32, cell: Cell) {
        if let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y))
            && x < self.width
            && y < self.height
        {
            self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)] = cell;
        }
    }

    /// The characters of the frame, a line per row, without colors.
    pub(crate) fn to_text(&self) -> String {
        self.cells
            .chunks(usize::from(self.width).max(1))
            .map(|row| row.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Write `text` from `x`, `y` rightward, one cell per character, clipped to
    /// the frame.
    pub(crate) fn text(&mut self, x: i32, y: i32, text: &str, fg: Color, bg: Color) {
        for (ch, x) in text.chars().zip(x..) {
            self.set(x, y, Cell { ch, fg, bg });
        }
    }
}

/// The terminal, and what it shows now.
pub(crate) struct Screen {
    out: io::BufWriter<io::Stdout>,
    shown: Option<Frame>,
}

impl Screen {
    pub(crate) fn new() -> Self {
        Self {
            out: io::BufWriter::new(io::stdout()),
            shown: None,
        }
    }

    /// A blank frame the size of the terminal.
    pub(crate) fn frame(&self) -> io::Result<Frame> {
        let (width, height) = terminal::size()?;
        Ok(Frame::new(width, height))
    }

    /// Show `frame`, writing only the cells that differ from what is shown.
    pub(crate) fn show(&mut self, frame: Frame) -> io::Result<()> {
        let resized = self
            .shown
            .as_ref()
            .is_none_or(|shown| (shown.width, shown.height) != (frame.width, frame.height));
        if resized {
            self.out
                .queue(SetBackgroundColor(Color::Reset))?
                .queue(Clear(ClearType::All))?;
        }
        // The cursor and colors where the last write left them, to skip
        // commands that would change nothing.
        let mut cursor = None;
        let mut colors = None;
        for (index, cell) in frame.cells.iter().enumerate() {
            let unchanged = !resized
                && self
                    .shown
                    .as_ref()
                    .is_some_and(|shown| shown.cells[index] == *cell);
            if unchanged {
                continue;
            }
            let x = u16::try_from(index % usize::from(frame.width)).expect("a column fits a u16");
            let y = u16::try_from(index / usize::from(frame.width)).expect("a row fits a u16");
            if cursor != Some((x, y)) {
                self.out.queue(MoveTo(x, y))?;
            }
            if colors != Some((cell.fg, cell.bg)) {
                self.out
                    .queue(SetForegroundColor(cell.fg))?
                    .queue(SetBackgroundColor(cell.bg))?;
                colors = Some((cell.fg, cell.bg));
            }
            self.out.queue(Print(cell.ch))?;
            cursor = Some((x + 1, y));
        }
        self.out.flush()?;
        self.shown = Some(frame);
        Ok(())
    }
}
