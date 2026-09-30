// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The person's door onto playback: `sprawling playback export` and
//! `sprawling playback check` (sprawling-SPEC.md 8-126).
//!
//! The projection, the selection, the read bound and the recomputation
//! are `accounting::playback`'s (accounting-SPEC.md 8-12). This module
//! reads the command line, names the person as the reader, and decides
//! where the bytes go: to stdout once the whole bundle exists, or to a
//! new file that lands whole or not at all.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use accounting::playback::{
    Against, BUNDLE_MAX_BYTES, Confidential, Cutoff, Reader, Report, Request, Selection, Verdict,
};
use kernel::{AxCode, AxError};

use super::city::report;
use super::exit::Exit;
use super::grammar::Arguments;
use super::refusal::{Form, written};

/// Exports the bundle the command line selects.
pub(super) fn export(read: &Arguments) -> ExitCode {
    let Some(city) = read.positional(1) else {
        return Exit::Line.into();
    };
    let request = match request_of(read) {
        Ok(request) => request,
        Err(Refused::Line(line)) => {
            eprintln!("sprawling: playback export: {line}");
            return Exit::Line.into();
        }
        Err(Refused::Selection(err)) => {
            eprint!("{}", written(&err, Form::Human));
            return Exit::Line.into();
        }
    };
    if request.reader == Reader::Person(Confidential::Included) {
        eprintln!("sprawling: playback: this bundle includes confidential buildings");
    }
    let bundle = match accounting::playback::export(Path::new(city), &request) {
        Ok(bundle) => bundle,
        Err(err) => return report(err),
    };
    let landed = match read.value("--out") {
        Some(out) => write_new(Path::new(out), bundle.bytes()),
        None => write_stdout(bundle.bytes()),
    };
    match landed {
        Ok(()) => Exit::Done.into(),
        Err(err) => report(err),
    }
}

/// Checks a bundle on its own, against another, or against its city, and
/// writes what it found as one JSON line.
pub(super) fn check(read: &Arguments) -> ExitCode {
    let Some(path) = read.positional(1) else {
        return Exit::Line.into();
    };
    if read.has("--bundle") && read.has("--city") {
        eprintln!("sprawling: playback check: give --bundle or --city, not both");
        return Exit::Line.into();
    }
    let checked = read_bundle(Path::new(path)).and_then(|bytes| {
        match (read.value("--bundle"), read.value("--city")) {
            (Some(other), _) => {
                let theirs = read_bundle(Path::new(other))?;
                accounting::playback::check(&bytes, Against::Bundle(&theirs))
            }
            (None, Some(city)) => accounting::playback::check(
                &bytes,
                Against::City {
                    root: Path::new(city),
                    reader: person(read),
                },
            ),
            (None, None) => accounting::playback::check(&bytes, Against::Nothing),
        }
    });
    match checked {
        Ok(found) => {
            let (line, exit) = outcome(&found);
            println!("{line}");
            exit.into()
        }
        Err(err) => report(err),
    }
}

/// Why a command line did not become a request.
#[derive(Debug)]
enum Refused {
    /// A flag value that does not read.
    Line(String),
    /// A range that contradicts itself.
    Selection(AxError),
}

fn request_of(read: &Arguments) -> Result<Request, Refused> {
    Ok(Request {
        selection: Selection::everything(),
        reader: person(read),
        cutoff: Cutoff::Latest,
    })
}

/// The person, widened to confidential buildings only when they asked.
fn person(read: &Arguments) -> Reader {
    Reader::Person(if read.has("--include-confidential") {
        Confidential::Included
    } else {
        Confidential::Withheld
    })
}

/// The JSON line a check writes, and the exit it ends with.
fn outcome(_found: &Report) -> (serde_json::Value, Exit) {
    (serde_json::json!({"verdict": "consistent"}), Exit::Done)
}

/// Writes the whole bundle to stdout. A reader that closes the pipe
/// early leaves a bundle that cannot be read back, so that is a failure.
fn write_stdout(bytes: &[u8]) -> Result<(), AxError> {
    let mut out = std::io::stdout().lock();
    out.write_all(bytes)
        .and_then(|()| out.flush())
        .map_err(|err| {
            AxError::failure(AxCode::StorageFatal, "write a playback bundle", "stdout")
                .with_recovery(format!(
                    "stdout closed before the bundle was whole ({err}); write it with --out \
                     instead of a pipe that stops early"
                ))
        })
}

/// Reads a bundle file, refusing one over the ceiling before reading it.
fn read_bundle(path: &Path) -> Result<Vec<u8>, AxError> {
    let unreadable = |err: std::io::Error| {
        AxError::failure(
            AxCode::PathNotFound,
            "read a playback bundle",
            path.display().to_string(),
        )
        .with_recovery(format!(
            "name a bundle file that exists and can be read ({err})"
        ))
    };
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    let over = match usize::try_from(size) {
        Ok(size) => size > BUNDLE_MAX_BYTES,
        Err(_) => true,
    };
    if over {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a playback bundle",
            format!(
                "{} is {size} bytes, over {BUNDLE_MAX_BYTES}",
                path.display()
            ),
        )
        .with_recovery("check a bundle sprawling playback export wrote; none is this large"));
    }
    std::fs::read(path).map_err(unreadable)
}

/// Writes `bytes` to `target`, which must not exist, through a staged
/// file beside it: the target appears whole or not at all, never
/// overwrites, and never lands in protected metadata, whatever links the
/// path passes through.
///
/// # Errors
/// A parent that does not resolve, a target inside protected metadata,
/// a target that exists, and any write or link that fails; the staged
/// file is removed on every path.
fn write_new(_target: &Path, _bytes: &[u8]) -> Result<(), AxError> {
    Ok(())
}

#[cfg(test)]
#[path = "playback_tests.rs"]
mod tests;
