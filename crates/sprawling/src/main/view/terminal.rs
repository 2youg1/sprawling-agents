// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The terminal the person's face of `sprawling view` runs in
//! (sprawling-SPEC.md 8-91): read the city once, then read keys, apply
//! the action each names, and draw the frame the face returns. Every
//! decision is in `keys` and `frame`; this file only moves bytes.

use std::io::{BufWriter, Write};
use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, queue, style, terminal};
use kernel::EventRecord;
use sprawling::lineage::Lineage;

use super::ViewError;
use super::frame::{Face, Row, Size};
use super::keys::{Key, action_for};

/// Shows the city whose ledger is kept in `dir` until the person quits.
/// The terminal is given back as it was found, whether or not the loop
/// failed.
pub(super) fn show(dir: &Path) -> Result<(), ViewError> {
    let (runs, records) = read_city(dir)?;
    let (columns, rows) = terminal::size()?;
    let mut face = Face::open(&runs, records, size_of(columns, rows));
    let mut out = BufWriter::new(std::io::stdout());
    terminal::enable_raw_mode()?;
    let looped = execute!(out, terminal::EnterAlternateScreen, cursor::Hide)
        .and_then(|()| run_loop(&mut face, &mut out));
    let restored = execute!(out, cursor::Show, terminal::LeaveAlternateScreen)
        .and_then(|()| terminal::disable_raw_mode());
    looped.and(restored).map_err(ViewError::Write)
}

/// One pass over the ledger: the lineage fold and the `records` lens.
fn read_city(dir: &Path) -> Result<(Vec<sprawling::lineage::RunLine>, Vec<Row>), ViewError> {
    let index = memory::LedgerIndex::rebuild(dir)?;
    let mut reader = index.reader(dir);
    let mut lineage = Lineage::default();
    let mut records = Vec::with_capacity(index.seqs().len());
    for &seq in index.seqs() {
        let line = reader.line_at(seq)?;
        let record = EventRecord::parse_line(&line)?;
        lineage.apply(&record)?;
        records.push(Row {
            seq,
            run: record.run(),
            // The line just parsed as JSON, which is UTF-8 by definition,
            // so the lossy conversion never replaces a byte.
            line: String::from_utf8_lossy(&line).into_owned(),
        });
    }
    Ok((lineage.lines().collect(), records))
}

fn run_loop(face: &mut Face, out: &mut impl Write) -> std::io::Result<()> {
    while !face.is_closed() {
        draw(face, out)?;
        match crossterm::event::read()? {
            Event::Key(pressed) => {
                if let Some(action) = key_of(pressed).and_then(action_for) {
                    face.apply(action);
                }
            }
            Event::Resize(columns, rows) => face.resize(size_of(columns, rows)),
            Event::FocusGained | Event::FocusLost | Event::Mouse(_) => {}
        }
    }
    Ok(())
}

fn draw(face: &Face, out: &mut impl Write) -> std::io::Result<()> {
    queue!(out, cursor::MoveTo(0, 0))?;
    for line in face.frame() {
        queue!(
            out,
            style::Print(line),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1)
        )?;
    }
    queue!(out, terminal::Clear(terminal::ClearType::FromCursorDown))?;
    out.flush()
}

fn size_of(columns: u16, rows: u16) -> Size {
    Size {
        columns: usize::from(columns),
        rows: usize::from(rows),
    }
}

/// The key a press names; a release or repeat report, which Windows
/// sends as well, names none.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "KeyCode is non-exhaustive, and every key the table does not name asks for nothing"
)]
fn key_of(pressed: KeyEvent) -> Option<Key> {
    if pressed.kind != KeyEventKind::Press {
        return None;
    }
    match pressed.code {
        KeyCode::Char('c') if pressed.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Key::Interrupt)
        }
        KeyCode::Char(typed) => Some(Key::Char(typed)),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        _ => None,
    }
}
