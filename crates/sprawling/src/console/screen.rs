// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the terminal's one writer draws, and how the terminal is put
//! back (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! A frame is drawn by the console's renderer (`console_ffi`), which
//! turns a scene into bytes; this module keeps what one frame leaves for
//! the next - how far below the live region's first row the cursor
//! stands, and the last transcript line - and writes the bytes.
//!
//! Every write the console makes passes [`Screen::write`], the one place
//! that decides what a failed write means: the terminal is gone, nothing
//! is written from then on, and the caller hears it once as
//! [`Written::Lost`]. The alternate screen is wiped before it is left,
//! because a terminal setting can keep it in the scrollback, and the
//! panic hook does the same before the process aborts.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use console_ffi::part::Part;
use console_ffi::scene::{Entry, Inline, Live, Quiet, Scene};
use crossterm::{cursor, queue, style::Print, terminal};

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

/// The quiet host's lines, as the writer hands them over.
pub(crate) struct QuietLines<'a> {
    pub(crate) url: &'a str,
    pub(crate) code: &'a str,
    pub(crate) key: Option<&'a str>,
    pub(crate) transient: Option<&'a str>,
}

/// The terminal as the writer last left it.
#[derive(Debug, Default)]
pub(crate) struct Screen {
    lost: bool,
    /// Rows between the live region's first row and the cursor, as the
    /// last frame left them.
    erase: u16,
    /// The last transcript line drawn.
    previous: Option<Part>,
    /// The bytes of the last frame, kept for their capacity.
    frame: Vec<u8>,
}

impl Screen {
    /// Raw mode on, the window titled, the cursor a bar, and the
    /// terminal put back if anything panics from here on. Where the
    /// terminal can say so (Unix), keys arrive unambiguous - Shift+Enter
    /// told from Enter - and a paste arrives whole.
    pub(crate) fn take(&mut self) -> Written {
        if terminal::enable_raw_mode().is_err() {
            self.lost = true;
            return Written::Lost;
        }
        restore_on_panic();
        self.write(|out| {
            queue!(
                out,
                terminal::SetTitle("sprawling · closing this window stops it"),
                cursor::SetCursorStyle::SteadyBar
            )?;
            enhance(out)
        })
    }

    /// The terminal handed back: the live region erased, the alternate
    /// screen left, the cursor as the person had it, raw mode off.
    pub(crate) fn give_back(&mut self) -> Written {
        let left = self.leave_quiet();
        let erased = self.frame(&[], None, size().0);
        let ended = self.write(|out| {
            plain(out)?;
            queue!(out, cursor::SetCursorStyle::DefaultUserShape, cursor::Show)
        });
        drop(terminal::disable_raw_mode());
        worst(worst(left, erased), ended)
    }

    /// Whether the alternate screen is showing.
    pub(crate) fn is_quiet(&self) -> bool {
        ALTERNATE.load(Ordering::SeqCst)
    }

    /// Nothing is written from now on: the terminal is gone.
    pub(crate) fn cut(&mut self) {
        self.lost = true;
    }

    /// One frame on the main screen: the live region drawn last time
    /// erased, `entries` written into the scrollback, and `live` drawn
    /// under them, `columns` wide.
    pub(crate) fn frame(
        &mut self,
        entries: &[Entry],
        live: Option<Live<'_>>,
        columns: u16,
    ) -> Written {
        if self.lost || self.is_quiet() {
            return Written::Shown;
        }
        let scene = Scene::inline(&Inline {
            columns,
            erase: self.erase,
            previous: self.previous,
            entries,
            live,
        });
        match console_ffi::draw(&scene, &mut self.frame) {
            Ok(drawn) => {
                self.erase = drawn.cursor_row;
                if let Some(last) = entries.last() {
                    self.previous = Some(last.part());
                }
            }
            // A frame the renderer refused draws nothing; the next one
            // starts from the same place.
            Err(_) => return Written::Shown,
        }
        let bytes = std::mem::take(&mut self.frame);
        let written = self.write(|out| out.write_all(&bytes));
        self.frame = bytes;
        written
    }

    /// The alternate screen, showing the quiet host and nothing else.
    pub(crate) fn quiet(&mut self, lines: &QuietLines<'_>) -> Written {
        let entered = if ALTERNATE.load(Ordering::SeqCst) {
            Written::Shown
        } else {
            let erased = self.frame(&[], None, size().0);
            ALTERNATE.store(true, Ordering::SeqCst);
            self.erase = 0;
            worst(
                erased,
                self.write(|out| queue!(out, terminal::EnterAlternateScreen, cursor::Hide)),
            )
        };
        let (columns, rows) = size();
        let scene = Scene::quiet(&Quiet {
            columns,
            rows,
            url: lines.url,
            code: lines.code,
            key: lines.key,
            transient: lines.transient,
        });
        if console_ffi::draw(&scene, &mut self.frame).is_err() {
            return entered;
        }
        let bytes = std::mem::take(&mut self.frame);
        let shown = self.write(|out| out.write_all(&bytes));
        self.frame = bytes;
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
                terminal::LeaveAlternateScreen,
                cursor::Show
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

/// The terminal's size, or the size every terminal starts at when it
/// cannot say.
pub(crate) fn size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}

/// Unambiguous keys and whole pastes, asked for where crossterm can ask
/// (Unix); a terminal that knows neither ignores the request.
#[cfg(unix)]
fn enhance(out: &mut std::io::Stdout) -> std::io::Result<()> {
    use crossterm::event::{
        EnableBracketedPaste, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    };
    queue!(
        out,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        EnableBracketedPaste
    )
}

/// Windows reads keys through the console API, which has neither.
#[cfg(not(unix))]
fn enhance(_out: &mut std::io::Stdout) -> std::io::Result<()> {
    Ok(())
}

/// The keys and pastes as the terminal had them before.
#[cfg(unix)]
fn plain(out: &mut std::io::Stdout) -> std::io::Result<()> {
    use crossterm::event::{DisableBracketedPaste, PopKeyboardEnhancementFlags};
    queue!(out, PopKeyboardEnhancementFlags, DisableBracketedPaste)
}

#[cfg(not(unix))]
fn plain(_out: &mut std::io::Stdout) -> std::io::Result<()> {
    Ok(())
}

fn worst(first: Written, second: Written) -> Written {
    match (first, second) {
        (Written::Shown, Written::Shown) => Written::Shown,
        (Written::Lost, _) | (_, Written::Lost) => Written::Lost,
    }
}

/// Puts the terminal back before a panic is reported: the alternate
/// screen wiped and left, the keys and the cursor as they were, raw mode
/// off. The process then aborts as the release profile says.
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
        drop(plain(&mut out));
        drop(crossterm::execute!(
            out,
            Print("\x1b[?2026l"),
            cursor::SetCursorStyle::DefaultUserShape,
            cursor::Show
        ));
        drop(terminal::disable_raw_mode());
        previous(info);
    }));
}
