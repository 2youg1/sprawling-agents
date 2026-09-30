// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A route that is a command the person wrote, run for as long as the
//! route is open (remote_access-SPEC.md §8-9).
//!
//! The command learns the loopback address from `SPRAWLING_REMOTE_LOCAL`
//! and, once outside can reach it, prints one line
//! `{"url": "https://…"}`. Lines before it are noise.
//!
//! **The reader is a thread.** A blocking read on a pipe has no deadline,
//! and a command that never prints must not hold the caller forever; the
//! thread turns the read into a channel the caller waits on with one, the
//! same reason `agent_protocols::mcp::stdio` gives. After the address it
//! keeps reading and discards, so a command that goes on printing never
//! blocks on a full pipe; the pipe closes when the command ends, and the
//! thread returns.

use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::{Opened, Permanence, PublicUrl, Route, Running};

/// The environment variable that carries `local` to the command.
pub const LOCAL_ENV: &str = "SPRAWLING_REMOTE_LOCAL";

/// The longest part of a wrong line quoted back in a refusal.
const QUOTED_CHARS: usize = 120;

/// The command, as the person configured it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// The command route.
pub struct CommandRoute {
    command: RouteCommand,
    permanence: Permanence,
    patience: TimeoutMs,
    running: Option<Running>,
}

/// One line of the command's output, as this route reads it.
enum Line {
    Noise,
    Address(Result<PublicUrl, AxError>),
}

impl CommandRoute {
    /// A closed route that runs `command`, answers `permanence` because
    /// the person stated it, and waits up to `patience` for the address.
    #[must_use]
    pub fn new(command: RouteCommand, permanence: Permanence, patience: TimeoutMs) -> Self {
        Self {
            command,
            permanence,
            patience,
            running: None,
        }
    }
}

impl Route for CommandRoute {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError> {
        self.close()?;
        let mut running = Running::start(
            Command::new(&self.command.program)
                .args(&self.command.args)
                .env(LOCAL_ENV, local.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null()),
            "check the route command's program and arguments; the city starts it, it does not \
             install it",
        )?;
        let address = watch(&mut running)?;
        let program = running.program().to_owned();
        match address.recv_timeout(Duration::from_millis(self.patience.0)) {
            Ok(Ok(url)) => {
                self.running = Some(running);
                Ok(Opened {
                    url,
                    permanence: self.permanence,
                })
            }
            Ok(Err(refusal)) => Err(refusal),
            Err(RecvTimeoutError::Timeout) => Err(AxError::failure(
                AxCode::Timeout,
                "open a route command",
                format!("{program} printed no address within {} ms", self.patience.0),
            )
            .with_recovery(
                r#"the command was stopped; it prints {"url": "https://..."} once outside can reach the address"#,
            )),
            Err(RecvTimeoutError::Disconnected) => Err(AxError::failure(
                AxCode::ToolUnavailable,
                "open a route command",
                format!("{program} ended before it printed its address"),
            )
            .with_recovery(
                "run the command by hand to see why it stopped; it keeps running while the route is open",
            )),
        }
    }

    fn close(&mut self) -> Result<(), AxError> {
        self.running
            .take()
            .map_or(Ok(()), |mut running| running.stop())
    }
}

/// Starts the thread that reads the command's output, and answers the
/// channel its address line arrives on.
fn watch(running: &mut Running) -> Result<Receiver<Result<PublicUrl, AxError>>, AxError> {
    let refused = |subject: String| {
        AxError::failure(AxCode::ToolUnavailable, "open a route command", subject)
            .with_recovery("the machine refused what reading the command needs; try again")
    };
    let stdout = running
        .stdout()
        .ok_or_else(|| refused(format!("{}: no output pipe", running.program())))?;
    let (sender, address) = mpsc::channel();
    std::thread::Builder::new()
        .name("route-command".to_owned())
        .spawn(move || read(stdout, &sender))
        .map_err(|err| refused(format!("{}: a reader thread: {err}", running.program())))?;
    Ok(address)
}

/// Sends the first address line, then drains the pipe until it closes.
fn read(stdout: ChildStdout, sender: &mpsc::Sender<Result<PublicUrl, AxError>>) {
    let mut lines = BufReader::new(stdout).split(b'\n');
    for bytes in lines.by_ref() {
        let Ok(bytes) = bytes else { return };
        match classify(&String::from_utf8_lossy(&bytes)) {
            Line::Noise => {}
            Line::Address(found) => {
                if sender.send(found).is_err() {
                    return;
                }
                break;
            }
        }
    }
    for bytes in lines {
        if bytes.is_err() {
            return;
        }
    }
}

/// A line is the address when it opens with `{`. It must then be exactly
/// `{"url": "<address>"}`, with blanks allowed around the parts and no
/// backslash in the address: a subset of JSON, so no JSON parser is
/// needed for one line, and a line that meant to give the address but got
/// it wrong is refused rather than skipped.
fn classify(text: &str) -> Line {
    let line = text.trim();
    if !line.starts_with('{') {
        return Line::Noise;
    }
    let value = line
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .map(str::trim)
        .and_then(|member| member.strip_prefix(r#""url""#))
        .map(str::trim_start)
        .and_then(|rest| rest.strip_prefix(':'))
        .map(str::trim)
        .and_then(|rest| rest.strip_prefix('"'))
        .and_then(|rest| rest.strip_suffix('"'));
    Line::Address(match value {
        Some(url) if !url.contains(['\\', '"']) => PublicUrl::parse(url),
        Some(_) | None => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read a route command's address",
            line.chars().take(QUOTED_CHARS).collect::<String>(),
        )
        .with_recovery(
            r#"print one line {"url": "https://host"} once the address is reachable, with no backslash in it"#,
        )),
    })
}
