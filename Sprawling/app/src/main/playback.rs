// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use accounting::playback::{
    Asked, BUNDLE_MAX_BYTES, City, Confidential, Cutoff, PAGE_MAX_BYTES, Place, Reader, Report,
    Request, Selection, Window,
};
use kernel::{Address, AxCode, AxError};

use super::city::report;
use super::exit::Exit;
use super::grammar::Arguments;
use super::refusal::{Form, written};
use super::view::{run_flag, seq_flag};

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
    let made =
        accounting::playback::export(Path::new(city), &request).and_then(|bundle| {
            match read.value("--page") {
                Some(template) => {
                    let template = read_capped(Path::new(template), PAGE_MAX_BYTES, "a template")?;
                    accounting::playback::embed(&template, &bundle)
                }
                None => Ok(bundle.bytes().to_vec()),
            }
        });
    let landed = made.and_then(|bytes| match read.value("--out") {
        Some(out) => accounting::playback::land(Place::Chosen(Path::new(out)), &bytes),
        None => write_stdout(&bytes),
    });
    match landed {
        Ok(()) => Exit::Done.into(),
        Err(err) => report(err),
    }
}

pub(super) fn check(read: &Arguments) -> ExitCode {
    let Some(path) = read.positional(1) else {
        return Exit::Line.into();
    };
    let files =
        read_capped(Path::new(path), PAGE_MAX_BYTES, "a bundle or a page").and_then(|file| {
            let other = read
                .value("--bundle")
                .map(|other| read_capped(Path::new(other), BUNDLE_MAX_BYTES, "a bundle"))
                .transpose()?;
            let observed = read
                .value("--observed")
                .map(|seen| read_capped(Path::new(seen), PAGE_MAX_BYTES, "an observation"))
                .transpose()?;
            Ok((file, other, observed))
        });
    let (file, other, observed) = match files {
        Ok(files) => files,
        Err(err) => return report(err),
    };
    let found = accounting::playback::check(
        &file,
        &Asked {
            bundle: other.as_deref(),
            city: read.value("--city").map(|city| City {
                root: Path::new(city),
                reader: person(read),
            }),
            observed: observed.as_deref(),
        },
    );
    let (line, exit) = outcome(&found);
    println!("{line}");
    exit.into()
}

#[derive(Debug)]
enum Refused {

    Line(String),

    Selection(AxError),
}

fn request_of(read: &Arguments) -> Result<Request, Refused> {
    let building = read
        .value("--building")
        .map(|raw| Address::parse(raw).map_err(|_| format!("'{raw}' is not a building's address")))
        .transpose()
        .map_err(Refused::Line)?;
    let span = Window {
        since: read.value("--since"),
        until: read.value("--until"),
        day: read.value("--day"),
    }
    .span()
    .map_err(Refused::Selection)?;
    let selection = Selection::new(
        seq_flag(read, "--from").map_err(Refused::Line)?,
        seq_flag(read, "--through").map_err(Refused::Line)?,
        run_flag(read).map_err(Refused::Line)?,
        building,
    )
    .map_err(Refused::Selection)?
    .during(span);
    Ok(Request {
        selection,
        reader: person(read),
        cutoff: Cutoff::Latest,
    })
}

fn person(read: &Arguments) -> Reader {
    Reader::Person(if read.has("--include-confidential") {
        Confidential::Included
    } else {
        Confidential::Withheld
    })
}

fn outcome(found: &Report) -> (serde_json::Value, Exit) {
    let exit = if found.holds() {
        Exit::Done
    } else {
        Exit::Refused
    };
    (found.line(), exit)
}

fn write_stdout(bytes: &[u8]) -> Result<(), AxError> {
    let mut out = std::io::stdout().lock();
    out.write_all(bytes)
        .and_then(|()| out.flush())
        .map_err(|err| {
            AxError::failure(AxCode::StorageFatal, "write a playback export", "stdout")
                .with_recovery(format!(
                    "stdout closed before the export was whole ({err}); write it with --out \
                     instead of a pipe that stops early"
                ))
        })
}

fn read_capped(path: &Path, cap: usize, what: &str) -> Result<Vec<u8>, AxError> {
    let unreadable = |err: std::io::Error| {
        AxError::failure(
            AxCode::PathNotFound,
            "read a playback file",
            path.display().to_string(),
        )
        .with_recovery(format!("name {what} that exists and can be read ({err})"))
    };
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    let over = match usize::try_from(size) {
        Ok(size) => size > cap,
        Err(_) => true,
    };
    if over {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a playback file",
            format!("{} is {size} bytes, over {cap}", path.display()),
        )
        .with_recovery(format!(
            "name {what} that sprawling playback wrote; none is this large"
        )));
    }
    std::fs::read(path).map_err(unreadable)
}

#[cfg(test)]
#[path = "playback_tests.rs"]
mod tests;
