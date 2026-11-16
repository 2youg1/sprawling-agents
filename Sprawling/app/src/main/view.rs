// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::city::report;
use super::grammar::{Arguments, nearest};
use kernel::event::record::{CheckpointCommitted, CheckpointFinding, SecretEgressBlocked};
use kernel::{AxError, EventKind, EventRecord, RunId, Seq};
use runtime::clock::{UtcSpan, parse_iso};
use serde::Serialize;
use std::io::{BufWriter, ErrorKind, Write};
use std::path::Path;
use std::process::ExitCode;

#[derive(Debug, Default)]
pub(super) struct Selection {
    pub(super) tail: Option<usize>,
    pub(super) from: Option<Seq>,
    pub(super) run: Option<RunId>,
    pub(super) kind: Option<EventKind>,
    pub(super) who: Option<String>,
    pub(super) grep: Option<String>,

    pub(super) span: UtcSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Audience {
    Agent,
    Person,
}

impl Audience {
    fn of_stdout() -> Audience {
        match sprawling::audience::Audience::of_stdout() {
            sprawling::audience::Audience::Person => Audience::Person,
            sprawling::audience::Audience::Agent => Audience::Agent,
        }
    }

    fn write_line(self, line: &[u8], out: &mut impl Write) -> std::io::Result<()> {
        let label = match self {
            Audience::Agent => String::new(),
            Audience::Person => chain_label(line, HashWidth::Whole),
        };
        out.write_all(label.as_bytes())?;
        out.write_all(line)?;
        out.write_all(b"\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HashWidth {
    Whole,

    Glance,
}

const GLANCE_DIGITS: usize = 12;

fn chain_label(line: &[u8], width: HashWidth) -> String {
    let hex = kernel::ledger::chain_hash(line).to_string();
    let kept = match width {
        HashWidth::Whole => hex.len(),
        HashWidth::Glance => GLANCE_DIGITS,
    };
    let digits: String = hex.chars().take(kept).collect();
    format!("{digits}  ")
}

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

impl From<storage::StorageError> for ViewError {
    fn from(err: storage::StorageError) -> ViewError {
        ViewError::Ledger(err.into_ax())
    }
}

impl From<std::io::Error> for ViewError {
    fn from(err: std::io::Error) -> ViewError {
        ViewError::Write(err)
    }
}

pub(super) fn verb(read: &Arguments) -> ExitCode {
    let Some(city) = read.positional(1) else {
        eprintln!("usage: sprawling view <city-dir> [--runs | --shells | filters]");
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
    let audience = Audience::of_stdout();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let written = if read.has("--runs") {
        write_runs(&dir, &mut out)
    } else if read.has("--shells") {
        shells::write_shells(&dir, &mut out)
    } else if chosen.is_everything() && audience == Audience::Person {
        terminal::show(&dir)
    } else {
        write_records(&dir, &chosen, audience, &mut out)
    };
    match written.and_then(|()| out.flush().map_err(ViewError::Write)) {
        Ok(()) => ExitCode::SUCCESS,

        Err(ViewError::Write(err)) if err.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(ViewError::Write(err)) => {
            eprintln!("sprawling: view: stdout: {err}");
            ExitCode::FAILURE
        }
        Err(ViewError::Ledger(err)) => report(err),
    }
}

impl Selection {
    fn is_everything(&self) -> bool {
        matches!(
            self,
            Selection {
                tail: None,
                from: None,
                run: None,
                kind: None,
                who: None,
                grep: None,
                span: _
            }
        ) && self.span == UtcSpan::default()
    }

    fn read(read: &Arguments) -> Result<Selection, String> {
        Ok(Selection {
            tail: whole_number(read, "--tail")?
                .map(|n| usize::try_from(n).map_err(|_| format!("--tail {n} is too large")))
                .transpose()?,
            from: seq_flag(read, "--from")?,
            run: run_flag(read)?,
            kind: read.value("--kind").map(kind_named).transpose()?,
            who: read.value("--who").map(str::to_owned),
            grep: read.value("--grep").map(str::to_owned),
            span: utc_span(read)?,
        })
    }

    fn admits(&self, line: &[u8]) -> Result<bool, AxError> {
        if let Some(text) = &self.grep
            && !text.is_empty()
            && !line
                .windows(text.len())
                .any(|window| window == text.as_bytes())
        {
            return Ok(false);
        }
        if self.kind.is_none() && self.who.is_none() && self.span == UtcSpan::default() {
            return Ok(true);
        }
        let record = EventRecord::parse_line(line)?;
        let when_holds = self.span.contains(record.t());
        let kind_holds = self.kind.is_none_or(|kind| record.kind() == kind);
        let who_holds = self.who.as_deref().is_none_or(|prefix| {
            record
                .addr()
                .is_some_and(|addr| addr.as_str().starts_with(prefix))
        });
        Ok(when_holds && kind_holds && who_holds)
    }
}

fn whole_number(read: &Arguments, flag: &str) -> Result<Option<u64>, String> {
    read.value(flag)
        .map(|raw| {
            raw.parse::<u64>()
                .map_err(|_| format!("{flag} wants a whole number, not '{raw}'"))
        })
        .transpose()
}

fn utc_span(read: &Arguments) -> Result<UtcSpan, String> {
    let moment = |flag: &str| {
        read.value(flag)
            .map(|raw| parse_iso(raw).map_err(|err| format!("{flag} '{raw}': {}", err.recovery())))
            .transpose()
    };
    UtcSpan::new(moment("--since")?, moment("--until")?)
        .map_err(|err| format!("--since and --until: {}", err.recovery()))
}

pub(super) fn seq_flag(read: &Arguments, flag: &str) -> Result<Option<Seq>, String> {
    whole_number(read, flag).map(|seq| seq.map(Seq::new))
}

pub(super) fn run_flag(read: &Arguments) -> Result<Option<RunId>, String> {
    read.value("--run")
        .map(|raw| RunId::parse(raw).map_err(|_| format!("'{raw}' is not a run id")))
        .transpose()
}

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

pub(super) fn write_records(
    dir: &Path,
    chosen: &Selection,
    audience: Audience,
    out: &mut impl Write,
) -> Result<(), ViewError> {
    let index = storage::LedgerIndex::rebuild(dir)?;
    let mut reader = index.reader(dir);
    let mut emit = |line: &[u8]| audience.write_line(line, &mut *out);
    match chosen.run {
        Some(run) => {
            let mut seqs: Vec<Seq> = index.run_seqs_before(run, None).collect();
            seqs.reverse();
            write_walk(&mut reader, seqs.into_iter(), chosen, &mut emit)
        }
        None => write_walk(&mut reader, index.seqs(), chosen, &mut emit),
    }
}

fn write_walk(
    reader: &mut storage::LineReader<'_>,
    walk: impl DoubleEndedIterator<Item = Seq>,
    chosen: &Selection,
    emit: &mut impl FnMut(&[u8]) -> std::io::Result<()>,
) -> Result<(), ViewError> {
    let from = chosen.from;
    let walk = walk.filter(|seq| from.is_none_or(|from| *seq >= from));
    match chosen.tail {
        None => {
            for seq in walk {
                let line = reader.line_at(seq)?;
                if chosen.admits(&line)? {
                    emit(&line)?;
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
                emit(line)?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum FindingOutcome {
    Committed,
    Refused,
}

#[derive(Serialize)]
struct FindingLine<'a> {
    run: RunId,
    seq: Seq,
    outcome: FindingOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'a str>,
    scope: &'a [String],
    #[serde(flatten)]
    finding: &'a CheckpointFinding,
}

struct FindingBatch<'a> {
    outcome: FindingOutcome,
    code: Option<&'a str>,
    scope: &'a [String],
    findings: &'a [CheckpointFinding],
}

pub(super) fn write_secret_findings(dir: &Path, out: &mut impl Write) -> Result<(), ViewError> {
    let index = storage::LedgerIndex::rebuild(dir)?;
    let mut reader = index.reader(dir);
    for seq in index.seqs() {
        let record = EventRecord::parse_line(&reader.line_at(seq)?)?;

        if record.kind() == EventKind::CheckpointCommitted
            && let CheckpointCommitted::Committed(commit) =
                record.data().read::<CheckpointCommitted>()?
        {
            write_finding_lines(
                &record,
                FindingBatch {
                    outcome: FindingOutcome::Committed,
                    code: None,
                    scope: &commit.scope,
                    findings: &commit.withheld,
                },
                out,
            )?;
        }
        if record.kind() == EventKind::SecretEgressBlocked {
            let refused = record.data().read::<SecretEgressBlocked>()?;
            write_finding_lines(
                &record,
                FindingBatch {
                    outcome: FindingOutcome::Refused,
                    code: Some(&refused.code),
                    scope: &refused.scope,
                    findings: &refused.withheld,
                },
                out,
            )?;
        }
    }
    Ok(())
}

fn write_finding_lines(
    record: &EventRecord,
    batch: FindingBatch<'_>,
    out: &mut impl Write,
) -> Result<(), ViewError> {
    for finding in batch.findings {
        serde_json::to_writer(
            &mut *out,
            &FindingLine {
                run: record.run(),
                seq: record.seq(),
                outcome: batch.outcome,
                code: batch.code,
                scope: batch.scope,
                finding,
            },
        )
        .map_err(std::io::Error::from)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

pub(super) fn secrets(read: &Arguments) -> ExitCode {
    let Some(city) = read.positional(1) else {
        eprintln!("usage: sprawling secrets <city>");
        return ExitCode::from(2);
    };
    let dir = kernel::layout::CityLayout::new(Path::new(city)).ledger();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let written =
        write_secret_findings(&dir, &mut out).and_then(|()| out.flush().map_err(ViewError::Write));
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(ViewError::Write(err)) if err.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(ViewError::Write(err)) => {
            eprintln!("sprawling: secrets: stdout: {err}");
            ExitCode::FAILURE
        }
        Err(ViewError::Ledger(err)) => report(err),
    }
}

pub(super) fn write_runs(dir: &Path, out: &mut impl Write) -> Result<(), ViewError> {
    for line in accounting::lineage::lineage_of(dir)?.lines() {
        serde_json::to_writer(&mut *out, &line.to_json()).map_err(std::io::Error::from)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

#[path = "view/arrange.rs"]
mod arrange;
#[path = "view/detail.rs"]
mod detail;
#[path = "view/follow.rs"]
mod follow;
#[path = "view/frame.rs"]
mod frame;
#[cfg(test)]
#[path = "view/frame_tests.rs"]
mod frame_tests;
#[path = "view/keys.rs"]
mod keys;
#[path = "view/list.rs"]
mod list;
#[path = "view/rounds.rs"]
mod rounds;
#[path = "view/shells.rs"]
mod shells;
#[cfg(test)]
#[path = "view/span_tests.rs"]
mod span_tests;
#[path = "view/terminal.rs"]
mod terminal;
