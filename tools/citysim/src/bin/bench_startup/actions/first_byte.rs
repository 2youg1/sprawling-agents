// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! First byte: `sprawling serve` spawned to the first byte of `GET /`
//! (`tools/citysim/spec/BenchStartup.lean` §8-5-1).
//!
//! Shape: adapter. The end of the boundary is a byte of the page rather
//! than the port opening, because a person sees the page, and a server
//! that accepts a connection it cannot answer yet is still closed to
//! them. Each sample serves on a port borrowed from the system a moment
//! before and is stopped once its byte has arrived and the serve has said
//! whether it proved the history it opened from. What that serve wrote
//! to standard error is kept in the log file the caller names, so the
//! phases the product timed for itself - the opening line and the proof
//! line, the moment commands are taken (sprawling-SPEC.md 8-122) - sit
//! beside the reading.

use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use kernel::{AxCode, AxError};

use super::super::samples::Samples;
use super::super::stamp;

/// One attempt to reach the server, and the pause after a refused one.
const POLL: Duration = Duration::from_millis(2);
/// How long one sample may take before it is reported: a city with
/// 400,000 records folds its whole Ledger before it accepts, which is
/// tens of seconds on a slow disk.
const WITHIN: Duration = Duration::from_secs(300);

/// `samples` spawns of `serve` over `city`, each timed to its first byte;
/// each sample's standard error replaces the last one's in `log`.
///
/// # Errors
/// Refuses a sample whose server exited or never answered, and
/// propagates the operating system's refusal to spawn or stop it, or to
/// open `log`.
pub fn first_byte(
    binary: &Path,
    city: &Path,
    samples: usize,
    log: &Path,
) -> Result<Samples, AxError> {
    let head = one(binary, city, log)?;
    let tail = (1..samples)
        .map(|_| one(binary, city, log))
        .collect::<Result<Vec<Duration>, AxError>>()?;
    Ok(Samples::of(head, tail))
}

fn one(binary: &Path, city: &Path, log: &Path) -> Result<Duration, AxError> {
    let said = std::fs::File::create(log)
        .map_err(|err| refused("keep the serve's log", &log.display().to_string(), &err))?;
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map_err(|err| refused("borrow a loopback port", "127.0.0.1:0", &err))?
        .port();
    let boundary = stamp();
    let mut child = Command::new(binary)
        .arg("serve")
        .arg(city)
        .arg(format!("127.0.0.1:{port}"))
        .args(["--no-console", "--no-open"])
        .env("SPRAWLING_OPEN", "never")
        .stdout(Stdio::null())
        .stderr(Stdio::from(said))
        .spawn()
        .map_err(|err| {
            refused(
                "serve the fixture city",
                &binary.display().to_string(),
                &err,
            )
        })?;
    let answered = first_answer(&mut child, SocketAddr::from(([127, 0, 0, 1], port)));
    let took = boundary.elapsed();
    let settled = match &answered {
        Ok(()) => proof_settled(&mut child, log),
        Err(_) => Ok(()),
    };
    let stopped = child
        .kill()
        .and_then(|()| child.wait())
        .map_err(|err| refused("stop the served city", &binary.display().to_string(), &err));
    answered?;
    settled?;
    stopped?;
    Ok(took)
}

/// The lines a served city writes when the proof of its history ends:
/// whole, or the ledger stopped (sprawling-SPEC.md 8-90).
const PROOF_ENDS: [&str; 2] = ["the history is proved", "the ledger stopped taking writes"];

/// Waits until the serve's log says how the proof of its history ended,
/// so the log a reader is pointed at carries the moment commands were
/// taken; the first byte is already timed.
fn proof_settled(child: &mut Child, log: &Path) -> Result<(), AxError> {
    let started = stamp();
    while started.elapsed() < WITHIN {
        let said = std::fs::read_to_string(log)
            .map_err(|err| refused("read the serve's log", &log.display().to_string(), &err))?;
        if PROOF_ENDS.iter().any(|end| said.contains(end)) {
            return Ok(());
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "serve the fixture city",
                format!("the server exited before it proved its history, {status}"),
            )
            .with_recovery("read the serve's log beside the fixture city"));
        }
        std::thread::sleep(POLL);
    }
    Err(AxError::failure(
        AxCode::ToolUnavailable,
        "serve the fixture city",
        "the proof of the history said nothing within 300 s",
    )
    .with_recovery("read the serve's log beside the fixture city"))
}

/// Polls until the page's first byte arrives.
fn first_answer(child: &mut Child, at: SocketAddr) -> Result<(), AxError> {
    let started = stamp();
    while started.elapsed() < WITHIN {
        if let Ok(mut stream) = TcpStream::connect_timeout(&at, POLL) {
            let mut byte = [0u8; 1];
            let asked = stream
                .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                .and_then(|()| stream.set_read_timeout(Some(WITHIN)))
                .and_then(|()| stream.read(&mut byte));
            if matches!(asked, Ok(1)) {
                return Ok(());
            }
        }
        match child.try_wait() {
            Ok(None) => {}
            Ok(Some(status)) => {
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "serve the fixture city",
                    format!("the server exited before its first byte, {status}"),
                )
                .with_recovery("run the same `sprawling serve` by hand to read what it says"));
            }
            Err(err) => return Err(refused("watch the served city", "serve", &err)),
        }
        std::thread::sleep(POLL);
    }
    Err(AxError::failure(
        AxCode::ToolUnavailable,
        "serve the fixture city",
        "no byte arrived within 300 s",
    )
    .with_recovery("run the same `sprawling serve` by hand to see where it stops"))
}

fn refused(action: &'static str, subject: &str, err: &std::io::Error) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, action, format!("{subject}: {err}"))
        .with_recovery("run just bench-startup, which builds the binary it measures first")
}
