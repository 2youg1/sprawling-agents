// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the terminal's one writer draws, and how the terminal is put
//! back (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! Every write the console makes passes [`Screen::write`], the one place
//! that decides what a failed write means: the terminal is gone, nothing
//! is written from then on, and the caller hears it once as
//! [`Written::Lost`]. The alternate screen is wiped before it is left,
//! because a terminal setting can keep it in the scrollback, and the
//! panic hook does the same before the process aborts.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::{cursor, queue, style::Print, terminal};

use super::editor::{Editor, PROMPT};
use super::program_status::Report;

/// Whether the alternate screen is showing, for the panic hook, which
/// has no other way to know whether clearing the screen would wipe the
/// person's scrollback.
static ALTERNATE: AtomicBool = AtomicBool::new(false);

/// What one draw came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Written {
    Shown,
    /// This draw found the terminal gone; every later draw is skipped
    /// and answers `Shown`, so the loss is heard once.
    Lost,
}

/// The terminal as the writer last left it.
#[derive(Debug, Default)]
pub(crate) struct Screen {
    lost: bool,
    /// Rows between the prompt's first row and the cursor, as last drawn.
    rows: u16,
}

impl Screen {
    /// Raw mode on, the window titled, and the terminal put back if
    /// anything panics from here on.
    pub(crate) fn take(&mut self) -> Written {
        if terminal::enable_raw_mode().is_err() {
            self.lost = true;
            return Written::Lost;
        }
        restore_on_panic();
        self.write(|out| {
            queue!(
                out,
                terminal::SetTitle("sprawling · closing this window stops it")
            )
        })
    }

    /// The terminal handed back: the alternate screen left, a fresh
    /// line, raw mode off.
    pub(crate) fn give_back(&mut self) -> Written {
        let left = self.leave_quiet();
        let ended = self.write(|out| queue!(out, Print("\r\n")));
        drop(terminal::disable_raw_mode());
        worst(left, ended)
    }

    /// Whether the alternate screen is showing.
    pub(crate) fn is_quiet(&self) -> bool {
        ALTERNATE.load(Ordering::SeqCst)
    }

    /// Nothing is written from now on: the terminal is gone.
    pub(crate) fn cut(&mut self) {
        self.lost = true;
    }

    /// One line above the prompt, and the prompt drawn again below it
    /// when `editor` is given.
    pub(crate) fn above(&mut self, line: &str, editor: Option<&Editor>) -> Written {
        let erased = self.erase_input();
        let text = line.replace('\n', "\r\n");
        let shown = self.write(|out| queue!(out, Print(&text), Print("\r\n")));
        let drawn = editor.map_or(Written::Shown, |editor| self.draw_input(editor));
        worst(worst(erased, shown), drawn)
    }

    /// The submitted line stays in the scrollback as it was drawn.
    pub(crate) fn submitted(&mut self) -> Written {
        self.rows = 0;
        self.write(|out| queue!(out, Print("\r\n")))
    }

    pub(crate) fn erase_input(&mut self) -> Written {
        let rows = std::mem::take(&mut self.rows);
        self.write(|out| {
            if rows > 0 {
                queue!(out, cursor::MoveUp(rows))?;
            }
            queue!(
                out,
                cursor::MoveToColumn(0),
                terminal::Clear(terminal::ClearType::FromCursorDown)
            )
        })
    }

    pub(crate) fn draw_input(&mut self, editor: &Editor) -> Written {
        let width = terminal::size().map_or(80, |(columns, _)| columns);
        let layout = editor.layout(width);
        let text = editor.text().replace('\n', "\r\n  ");
        let written = self.write(|out| {
            queue!(out, Print(PROMPT), Print(&text))?;
            let up = layout.end_row.saturating_sub(layout.cursor_row);
            if up > 0 {
                queue!(out, cursor::MoveUp(up))?;
            }
            queue!(out, cursor::MoveToColumn(layout.cursor_column))
        });
        self.rows = layout.cursor_row;
        written
    }

    /// The alternate screen, showing `lines` and nothing else.
    pub(crate) fn quiet(&mut self, lines: &[String]) -> Written {
        let entered = if ALTERNATE.swap(true, Ordering::SeqCst) {
            Written::Shown
        } else {
            let erased = self.erase_input();
            worst(
                erased,
                self.write(|out| queue!(out, terminal::EnterAlternateScreen)),
            )
        };
        let text = lines.join("\r\n");
        let shown = self.write(|out| {
            queue!(
                out,
                terminal::Clear(terminal::ClearType::All),
                cursor::MoveTo(0, 0),
                Print(&text)
            )
        });
        worst(entered, shown)
    }

    /// One program status report, which moves no cursor and draws
    /// nothing on either screen.
    pub(crate) fn report(&mut self, report: Report) -> Written {
        self.write(|out| queue!(out, report))
    }

    /// Back to the main screen, the alternate one wiped first.
    pub(crate) fn leave_quiet(&mut self) -> Written {
        if !ALTERNATE.swap(false, Ordering::SeqCst) {
            return Written::Shown;
        }
        self.write(|out| {
            queue!(
                out,
                terminal::Clear(terminal::ClearType::All),
                terminal::LeaveAlternateScreen
            )
        })
    }

    /// The one place a terminal write can fail.
    fn write(&mut self, draw: impl FnOnce(&mut std::io::Stdout) -> std::io::Result<()>) -> Written {
        if self.lost {
            return Written::Shown;
        }
        let mut out = std::io::stdout();
        match draw(&mut out).and_then(|()| out.flush()) {
            Ok(()) => Written::Shown,
            Err(_) => {
                self.lost = true;
                Written::Lost
            }
        }
    }
}

fn worst(first: Written, second: Written) -> Written {
    match (first, second) {
        (Written::Shown, Written::Shown) => Written::Shown,
        (Written::Lost, _) | (_, Written::Lost) => Written::Lost,
    }
}

/// Puts the terminal back before a panic is reported: the alternate
/// screen wiped and left, raw mode off. The process then aborts as the
/// release profile says.
fn restore_on_panic() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut out = std::io::stdout();
        if ALTERNATE.swap(false, Ordering::SeqCst) {
            drop(crossterm::execute!(
                out,
                terminal::Clear(terminal::ClearType::All),
                terminal::LeaveAlternateScreen
            ));
        }
        drop(terminal::disable_raw_mode());
        previous(info);
    }));
}
