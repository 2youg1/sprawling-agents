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
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use accounting::playback::{
    Against, BUNDLE_MAX_BYTES, Confidential, Cutoff, Reader, Report, Request, Selection, Verdict,
};
use kernel::{Address, AxCode, AxError, PROTECTED_METADATA};

use super::city::report;
use super::exit::Exit;
use super::grammar::Arguments;
use super::refusal::{Form, written};
use super::view::{run_flag, seq_flag};

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
    let building = read
        .value("--building")
        .map(|raw| Address::parse(raw).map_err(|_| format!("'{raw}' is not a building's address")))
        .transpose()
        .map_err(Refused::Line)?;
    let selection = Selection::new(
        seq_flag(read, "--from").map_err(Refused::Line)?,
        seq_flag(read, "--through").map_err(Refused::Line)?,
        run_flag(read).map_err(Refused::Line)?,
        building,
    )
    .map_err(Refused::Selection)?;
    Ok(Request {
        selection,
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
fn outcome(found: &Report) -> (serde_json::Value, Exit) {
    let mut line = serde_json::Map::new();
    line.insert("digest".to_owned(), found.digest.to_string().into());
    line.insert("events".to_owned(), found.events.to_string().into());
    let (verdict, exit) = match &found.verdict {
        Verdict::Consistent => ("consistent", Exit::Done),
        Verdict::Same => ("same", Exit::Done),
        Verdict::Differs { section } => {
            line.insert("section".to_owned(), (*section).into());
            ("differs", Exit::Refused)
        }
        Verdict::CannotReproduce { why } => {
            line.insert("why".to_owned(), why.as_str().into());
            ("cannot_reproduce", Exit::Refused)
        }
    };
    line.insert("verdict".to_owned(), verdict.into());
    (serde_json::Value::Object(line), exit)
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
fn write_new(target: &Path, bytes: &[u8]) -> Result<(), AxError> {
    let refuse = |code: AxCode, recovery: String| {
        AxError::failure(
            code,
            "write a playback bundle",
            target.display().to_string(),
        )
        .with_recovery(recovery)
    };
    let (Some(name), parent) = (target.file_name(), target.parent()) else {
        return Err(refuse(
            AxCode::InvalidArgs,
            "name a file for --out, not a directory".to_owned(),
        ));
    };
    let parent = parent.filter(|parent| !parent.as_os_str().is_empty());
    let resolved = std::fs::canonicalize(parent.unwrap_or(Path::new("."))).map_err(|err| {
        refuse(
            AxCode::PathNotFound,
            format!("create the directory first ({err})"),
        )
    })?;
    let landing = resolved.join(name);
    if protected(&landing) {
        return Err(refuse(
            AxCode::OutsideWriteDomain,
            "write the bundle outside `.sprawling` and `.git`: the ledger and git's own \
             metadata take no export"
                .to_owned(),
        ));
    }
    if std::fs::symlink_metadata(&landing).is_ok() {
        return Err(refuse(
            AxCode::InvalidArgs,
            "choose a file name that does not exist yet; a playback export never overwrites"
                .to_owned(),
        ));
    }
    let staged = staged_beside(&landing);
    let landed = stage(&staged, bytes).and_then(|()| std::fs::hard_link(&staged, &landing));
    let cleared = std::fs::remove_file(&staged);
    match (landed, cleared) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), _) => Err(refuse(
            AxCode::StorageFatal,
            format!("the bundle did not land ({err}); nothing was left at the target"),
        )),
        (Ok(()), Err(err)) => Err(refuse(
            AxCode::StorageFatal,
            format!(
                "the bundle landed, and its staged copy {} could not be removed ({err}); \
                 delete it by hand",
                staged.display()
            ),
        )),
    }
}

/// Whether any segment of `path` is protected metadata, compared without
/// ASCII case the way `Address::is_reserved` compares it.
fn protected(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(segment) => segment.to_str().is_some_and(|segment| {
            PROTECTED_METADATA
                .iter()
                .any(|name| segment.eq_ignore_ascii_case(name))
        }),
        Component::Prefix(_) | Component::RootDir | Component::CurDir | Component::ParentDir => {
            false
        }
    })
}

/// The staged file beside `landing`, named for this process so two
/// exports to one directory never share one.
fn staged_beside(landing: &Path) -> PathBuf {
    let mut name = landing.as_os_str().to_owned();
    name.push(format!(".partial-{}", std::process::id()));
    PathBuf::from(name)
}

/// Writes `bytes` to a file that must not exist yet, through to the disk.
fn stage(staged: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staged)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
#[path = "playback_tests.rs"]
mod tests;
