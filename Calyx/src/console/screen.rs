// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::part::Part;
use crate::scene::{Entry, Inline, Live, Quiet, Scene};
use crossterm::{cursor, queue, style::Print, terminal};

use super::program_status::Report;

static ALTERNATE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Written {
    Shown,
    Lost,
}

pub(crate) struct QuietLines<'a> {
    pub(crate) url: &'a str,
    pub(crate) code: &'a str,
    pub(crate) key: Option<&'a str>,
    pub(crate) transient: Option<&'a str>,
}

#[derive(Debug, Default)]
pub(crate) struct Screen {
    lost: bool,
    erase: u16,
    previous: Option<Part>,
    frame: Vec<u8>,
    refused: Option<crate::Refused>,
}

impl Screen {
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

    pub(crate) fn give_back(&mut self) -> Written {
        let left = self.leave_quiet();
        let erased = self.frame(&[], None, size());
        let ended = self.write(|out| {
            plain(out)?;
            queue!(out, cursor::SetCursorStyle::DefaultUserShape, cursor::Show)
        });
        drop(terminal::disable_raw_mode());
        worst(worst(left, erased), ended)
    }

    pub(crate) fn is_quiet(&self) -> bool {
        ALTERNATE.load(Ordering::SeqCst)
    }

    pub(crate) fn cut(&mut self) {
        self.lost = true;
    }

    pub(crate) fn frame(
        &mut self,
        entries: &[Entry],
        live: Option<Live<'_>>,
        size: (u16, u16),
    ) -> Written {
        if self.lost || self.is_quiet() {
            return Written::Shown;
        }
        if let Err(reason) = self.render(entries, live, size) {
            if self.refused == Some(reason) {
                return Written::Shown;
            }
            self.refused = Some(reason);
            let erase = self.erase;
            self.erase = 0;
            return self.write(|out| {
                queue!(
                    out,
                    cursor::MoveToColumn(0),
                    cursor::MoveUp(erase),
                    terminal::Clear(terminal::ClearType::FromCursorDown)
                )?;
                write!(
                    out,
                    "{reason}; transcript in refused frames is discarded; read replies in the WebUI\r\n"
                )
            });
        }
        let bytes = std::mem::take(&mut self.frame);
        let written = self.write(|out| out.write_all(&bytes));
        self.frame = bytes;
        written
    }

    fn render(
        &mut self,
        entries: &[Entry],
        live: Option<Live<'_>>,
        size: (u16, u16),
    ) -> Result<(), crate::Refused> {
        let scene = Scene::inline(&Inline {
            columns: size.0,
            rows: size.1,
            erase: self.erase,
            previous: self.previous,
            entries,
            live,
        });
        let drawn = crate::draw(&scene, &mut self.frame)?;
        self.erase = drawn.cursor_row;
        self.refused = None;
        if let Some(last) = entries.last() {
            self.previous = Some(last.part());
        }
        Ok(())
    }

    pub(crate) fn quiet(&mut self, lines: &QuietLines<'_>) -> Written {
        let entered = if ALTERNATE.load(Ordering::SeqCst) {
            Written::Shown
        } else {
            let erased = self.frame(&[], None, size());
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
        if crate::draw(&scene, &mut self.frame).is_err() {
            return entered;
        }
        let bytes = std::mem::take(&mut self.frame);
        let shown = self.write(|out| out.write_all(&bytes));
        self.frame = bytes;
        worst(entered, shown)
    }

    pub(crate) fn report(&mut self, report: Report) -> Written {
        self.write(|out| queue!(out, report))
    }

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

pub(crate) fn size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}

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

#[cfg(not(unix))]
fn enhance(_out: &mut std::io::Stdout) -> std::io::Result<()> {
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_frame_does_not_accumulate_or_block_later_transcript() {
        let mut screen = Screen::default();
        let oversized = [Entry::Reply {
            said: "x".repeat(24 * 1024 * 1024),
        }];
        for _ in 0..3 {
            assert_eq!(
                screen.render(&oversized, None, (2, 24)),
                Err(crate::Refused::TooLong)
            );
            assert_eq!(screen.previous, None);
        }
        let next = [Entry::Reply { said: "ok".into() }];
        assert_eq!(screen.render(&next, None, (2, 24)), Ok(()));
        assert_eq!(screen.previous, Some(next[0].part()));
        let bytes = screen.frame.clone();
        let mut fresh = Screen::default();
        assert_eq!(fresh.render(&next, None, (2, 24)), Ok(()));
        assert_eq!(bytes, fresh.frame);
    }
}
