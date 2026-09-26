// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The read-only verb that shows one city's Ledger from disk
//! (sprawling-SPEC.md 8-93).
//!
//! What it writes is what an agent already parses: Ledger lines byte for
//! byte, or with `--runs` one JSON line per run from
//! `sprawling::lineage`. Nothing here decides what a record means, so a
//! new event kind needs no line in this file.

use super::city::report;
use super::grammar::{Arguments, nearest};
use kernel::{AxError, EventKind, EventRecord, RunId, Seq};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::Path;
use std::process::ExitCode;

/// Which Ledger lines the `records` lens writes. Every condition given
/// must hold; `tail` then keeps the last lines that passed.
#[derive(Debug, Default)]
pub(super) struct Selection {
    pub(super) tail: Option<usize>,
    pub(super) from: Option<Seq>,
    pub(super) run: Option<RunId>,
    pub(super) kind: Option<EventKind>,
    pub(super) who: Option<String>,
    pub(super) grep: Option<String>,
}

/// Why the verb could not finish writing.
#[derive(Debug)]
pub(super) enum ViewError {
    Ledger(AxError),
    Write(std::io::Error),
}

impl From<AxError> for ViewError {
    fn from(err: AxError) -> ViewError {
        ViewError::Ledger(err)
    }
}

impl From<memory::MemoryError> for ViewError {
    fn from(err: memory::MemoryError) -> ViewError {
        ViewError::Ledger(err.into_ax())
    }
}

impl From<std::io::Error> for ViewError {
    fn from(err: std::io::Error) -> ViewError {
        ViewError::Write(err)
    }
}

/// Exit codes: 0 written, 1 the ledger could not be read, 2 this
/// command line.
pub(super) fn verb(read: &Arguments) -> ExitCode {
    let Some(city) = read.positional(1) else {
        eprintln!("usage: sprawling view <city-dir> [--runs | filters]");
        return ExitCode::from(2);
    };
    let chosen = match Selection::read(read) {
        Ok(chosen) => chosen,
        Err(line) => {
            eprintln!("sprawling: view: {line}");
            return ExitCode::from(2);
        }
    };
    let dir = kernel::layout::CityLayout::new(Path::new(city)).ledger();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let written = if read.has("--runs") {
        write_runs(&dir, &mut out)
    } else {
        write_records(&dir, &chosen, &mut out)
    };
    match written.and_then(|()| out.flush().map_err(ViewError::Write)) {
        Ok(()) => ExitCode::SUCCESS,
        // A reader that stopped early (`| head`) has what it asked for.
        Err(ViewError::Write(err)) if err.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(ViewError::Write(err)) => {
            eprintln!("sprawling: view: stdout: {err}");
            ExitCode::FAILURE
        }
        Err(ViewError::Ledger(err)) => report(err),
    }
}

impl Selection {
    fn read(read: &Arguments) -> Result<Selection, String> {
        let number = |flag: &str| -> Result<Option<u64>, String> {
            read.value(flag)
                .map(|raw| {
                    raw.parse::<u64>()
                        .map_err(|_| format!("{flag} wants a whole number, not '{raw}'"))
                })
                .transpose()
        };
        Ok(Selection {
            tail: number("--tail")?
                .map(|n| usize::try_from(n).map_err(|_| format!("--tail {n} is too large")))
                .transpose()?,
            from: number("--from")?.map(Seq::new),
            run: read
                .value("--run")
                .map(|raw| RunId::parse(raw).map_err(|_| format!("'{raw}' is not a run id")))
                .transpose()?,
            kind: read.value("--kind").map(kind_named).transpose()?,
            who: read.value("--who").map(str::to_owned),
            grep: read.value("--grep").map(str::to_owned),
        })
    }

    /// Whether one raw Ledger line passes the conditions that read its
    /// bytes; the run and the lower seq bound are settled by the walk.
    fn admits(&self, line: &[u8]) -> Result<bool, AxError> {
        if let Some(text) = &self.grep
            && !text.is_empty()
            && !line
                .windows(text.len())
                .any(|window| window == text.as_bytes())
        {
            return Ok(false);
        }
        if self.kind.is_none() && self.who.is_none() {
            return Ok(true);
        }
        let record = EventRecord::parse_line(line)?;
        let kind_holds = self.kind.is_none_or(|kind| record.kind() == kind);
        let who_holds = self.who.as_deref().is_none_or(|prefix| {
            record
                .addr()
                .is_some_and(|addr| addr.as_str().starts_with(prefix))
        });
        Ok(kind_holds && who_holds)
    }
}

/// The event kind spelled `raw`, or the line that says it is not one
/// and which spellings are close.
fn kind_named(raw: &str) -> Result<EventKind, String> {
    let names: Vec<String> = EventKind::ALL
        .iter()
        .filter_map(|kind| match serde_json::to_value(kind) {
            Ok(serde_json::Value::String(name)) => Some(name),
            Ok(_) | Err(_) => None,
        })
        .collect();
    serde_json::from_value(serde_json::Value::String(raw.to_owned())).map_err(|_| {
        let close = nearest(raw, names.iter().map(String::as_str));
        match close.as_slice() {
            [] => format!("no event kind '{raw}'"),
            close => format!(
                "no event kind '{raw}'. Did you mean '{}'?",
                close.join("', '")
            ),
        }
    })
}

/// Writes the lines `chosen` selects, oldest first, each ending in `\n`.
pub(super) fn write_records(
    dir: &Path,
    chosen: &Selection,
    out: &mut impl Write,
) -> Result<(), ViewError> {
    let index = memory::LedgerIndex::rebuild(dir)?;
    let mut reader = index.reader(dir);
    match chosen.run {
        Some(run) => {
            let mut seqs: Vec<Seq> = index.run_seqs_before(run, None).collect();
            seqs.reverse();
            write_walk(&mut reader, seqs.into_iter(), chosen, out)
        }
        None => write_walk(&mut reader, index.seqs(), chosen, out),
    }
}

/// Writes the lines of `walk`, an ascending run of seqs, that `chosen`
/// admits.
fn write_walk(
    reader: &mut memory::LineReader<'_>,
    walk: impl DoubleEndedIterator<Item = Seq>,
    chosen: &Selection,
    out: &mut impl Write,
) -> Result<(), ViewError> {
    let from = chosen.from;
    let walk = walk.filter(|seq| from.is_none_or(|from| *seq >= from));
    match chosen.tail {
        None => {
            for seq in walk {
                let line = reader.line_at(seq)?;
                if chosen.admits(&line)? {
                    out.write_all(&line)?;
                    out.write_all(
                        b"
",
                    )?;
                }
            }
        }
        Some(count) => {
            let mut kept = Vec::new();
            for seq in walk.rev() {
                if kept.len() >= count {
                    break;
                }
                let line = reader.line_at(seq)?;
                if chosen.admits(&line)? {
                    kept.push(line);
                }
            }
            for line in kept.iter().rev() {
                out.write_all(line)?;
                out.write_all(
                    b"
",
                )?;
            }
        }
    }
    Ok(())
}

/// Writes one JSON line per run, oldest first.
pub(super) fn write_runs(dir: &Path, out: &mut impl Write) -> Result<(), ViewError> {
    for line in sprawling::lineage::lineage_of(dir)?.lines() {
        serde_json::to_writer(&mut *out, &line.to_json()).map_err(std::io::Error::from)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
