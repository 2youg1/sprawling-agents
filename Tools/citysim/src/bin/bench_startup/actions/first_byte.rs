// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use kernel::{AxCode, AxError};

use super::super::samples::Samples;
use super::super::stamp;

const POLL: Duration = Duration::from_millis(2);

const WITHIN: Duration = Duration::from_secs(300);

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

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
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

const PROOF_ENDS: [&str; 2] = ["the history is proved", "the ledger stopped taking writes"];

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

fn first_answer(child: &mut Child, at: SocketAddr) -> Result<(), AxError> {
    let started = stamp();
    while started.elapsed() < WITHIN {
        if let Ok(mut stream) = TcpStream::connect_timeout(&at, POLL) {
            let mut byte = [0u8; 1];

            let request = format!("GET / HTTP/1.1\r\nHost: {at}\r\nConnection: close\r\n\r\n");
            let asked = stream
                .write_all(request.as_bytes())
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
