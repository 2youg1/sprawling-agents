// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An MCP server run as a child process, spoken to one line at a time.
//!
//! The handshake and the tool calls know what to say; this module knows
//! where the bytes go, how long they may take, and who reclaims the
//! process.
//!
//! Two decisions are worth reading before changing anything here.
//!
//! **The reader is a thread, and the deadline is real.** A synchronous
//! read on a pipe has no deadline of its own, so a server that never
//! answers would hold the run's only worker forever — the exact failure
//! this project has already met once from the other direction, when a
//! provider went silent after `model_called` and nothing timed out. The
//! thread turns a blocking read into a channel this side can wait on
//! with a deadline. It cannot outlive the connection: killing the child
//! closes the pipe, and the reader's last word, end of input, is taken
//! by the next call or refused once the connection is dropped.
//!
//! **The reader holds one message at a time.** It reads through
//! `read_one_message`, so one line is at most `MESSAGE_CEILING` bytes,
//! and it hands each message over a channel with no queue, so it reads
//! the next only once a call has taken this one. Whatever the server
//! writes beyond that waits in the pipe, which is the server's buffer
//! rather than this city's (agent_protocols-SPEC.md 8-15). A line past
//! the ceiling is refused to the call that was waiting, and the child is
//! stopped, because the rest of that line cannot be told apart from the
//! next message.
//!
//! **A deadline that passes kills the child.** A late answer arriving
//! after its call gave up would be read as the answer to the next call,
//! and two calls swapped is worse than a refusal. Killing ends that
//! possibility rather than guarding against it.

use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::redeeming::Redeemed;
use super::{Received, read_one_message};

/// A handle on one running server. Cloning gives a second handle on the
/// same process, which is what a server offering several tools needs:
/// one child, one connection, one place that answers.
#[derive(Clone)]
pub(crate) struct StdioServer {
    inner: Arc<Mutex<Connection>>,
}

/// The process, its pipes, and the name to use when refusing.
struct Connection {
    program: String,
    child: std::process::Child,
    requests: std::process::ChildStdin,
    answers: Receiver<Result<Received, AxError>>,
}

impl StdioServer {
    /// Starts `command` and keeps it running until the last handle on it
    /// is dropped.
    ///
    /// `env` is added to what this process already has, name by name, so
    /// a server still finds the search path and the home directory it
    /// needs. **A name handed to a child cannot be taken back**, which
    /// is why a credential arrives here already redeemed and exists as
    /// plaintext only inside this call.
    ///
    /// # Errors
    /// Refuses a program this machine cannot start, and a child whose
    /// pipes the operating system did not hand back.
    pub(crate) fn start(
        command: &str,
        args: &[String],
        env: &[Redeemed],
        cwd: &Path,
    ) -> Result<StdioServer, AxError> {
        let mut child = std::process::Command::new(command)
            .args(args)
            .envs(env.iter().map(|pair| (pair.name(), pair.plaintext())))
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            // The server's own diagnostics stay on its stderr and out of
            // this city: they are not answers, and reading them here
            // would make them look like answers.
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|err| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "start an mcp server",
                    format!("{command}: {err}"),
                )
                .with_recovery(
                    "check the command in `[[mcp]]`; the city starts it, it does not install it",
                )
            })?;
        let requests = child.stdin.take().ok_or_else(|| pipes_missing(command))?;
        let stdout = child.stdout.take().ok_or_else(|| pipes_missing(command))?;
        let (sender, answers) = std::sync::mpsc::sync_channel(0);
        let named = command.to_owned();
        std::thread::Builder::new()
            .name(format!("mcp-{command}"))
            .spawn(move || {
                let mut reader = std::io::BufReader::new(stdout);
                loop {
                    let read = read_one_message(&mut reader, &named);
                    // Either end finishing ends the reader: the end of
                    // input or a refused message is the last thing it has
                    // to say, and a closed channel means this city
                    // stopped listening.
                    let last = !matches!(read, Ok(Received::Message(_)));
                    if sender.send(read).is_err() || last {
                        break;
                    }
                }
            })
            .map_err(|err| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "start an mcp server",
                    format!("{command}: {err}"),
                )
                .with_recovery("the machine refused a thread to read this server's answers")
            })?;
        Ok(StdioServer {
            inner: Arc::new(Mutex::new(Connection {
                program: command.to_owned(),
                child,
                requests,
                answers,
            })),
        })
    }
}

impl StdioServer {
    /// Whether the child has exited. A connection left locked by a dead
    /// thread, or a child whose state the platform will not report,
    /// counts as ended: the caller starts a new one, which is the one
    /// recovery either case has.
    pub(crate) fn has_ended(&self) -> bool {
        match self.inner.lock() {
            Ok(mut connection) => match connection.child.try_wait() {
                Ok(None) => false,
                Ok(Some(_)) | Err(_) => true,
            },
            Err(_poisoned) => true,
        }
    }
}

impl std::fmt::Debug for StdioServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.try_lock() {
            Ok(connection) => write!(f, "StdioServer({})", connection.program),
            Err(_in_flight) => f.write_str("StdioServer(answering)"),
        }
    }
}

impl crate::Outbound for StdioServer {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError> {
        let mut connection = self.inner.lock().map_err(|_| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "call an mcp server",
                "the connection was left locked by a thread that died".to_owned(),
            )
            .with_recovery("restart this city; the server is started again with it")
        })?;
        connection.exchange(line, patience)
    }

    /// A notification goes down the same pipe and nothing is read back,
    /// because the far end will not answer it. Writing it is the whole
    /// of the delivery this transport can promise.
    fn notify(&mut self, line: &str, _patience: TimeoutMs) -> Result<(), AxError> {
        let mut connection = self.inner.lock().map_err(|_| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "tell an mcp server",
                "the connection was left locked by a thread that died".to_owned(),
            )
            .with_recovery("restart this city; the server is started again with it")
        })?;
        connection.tell(line)
    }
}

impl Connection {
    /// Writes one message and does not wait. Shares the framing check
    /// with `exchange`, because a newline splits a notification into two
    /// messages exactly as it splits a request.
    fn tell(&mut self, line: &str) -> Result<(), AxError> {
        self.framed(line)?;
        writeln!(self.requests, "{line}").map_err(|err| self.broken("write to", &err))?;
        self.requests
            .flush()
            .map_err(|err| self.broken("flush", &err))
    }

    /// The transport is line delimited, so a newline inside a message
    /// would silently become two messages. The framing is this module's
    /// contract, so it is checked here rather than trusted.
    fn framed(&self, line: &str) -> Result<(), AxError> {
        if line.contains('\n') {
            return Err(AxError::failure(
                AxCode::WireMismatch,
                "call an mcp server",
                format!("{}: a request carried a newline", self.program),
            )
            .with_recovery("send one message per line; the transport frames on newlines"));
        }
        Ok(())
    }

    fn exchange(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError> {
        self.framed(line)?;
        writeln!(self.requests, "{line}").map_err(|err| self.broken("write to", &err))?;
        self.requests
            .flush()
            .map_err(|err| self.broken("flush", &err))?;
        match self.answers.recv_timeout(Duration::from_millis(patience.0)) {
            Ok(Ok(Received::Message(answer))) => Ok(answer),
            // The server answered with bytes this city will not read. It
            // took the call, so what it did is unknown, and the rest of
            // that answer is still in the pipe, so the child goes.
            Ok(Err(refused)) => {
                self.reclaim();
                Err(answer_unread(&refused))
            }
            Err(RecvTimeoutError::Timeout) => {
                self.reclaim();
                Err(AxError::failure(
                    AxCode::Timeout,
                    "call an mcp server",
                    format!("{}: no answer within {} ms", self.program, patience.0),
                )
                .effect_unknown()
                .with_recovery(
                    "the server was stopped so a late answer cannot be read as the next one; \
                     it may have acted on the call before it went quiet, so check what it was \
                     asked to do before asking again",
                ))
            }
            Ok(Ok(Received::EndOfInput)) | Err(RecvTimeoutError::Disconnected) => {
                Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "call an mcp server",
                    format!("{}: the server closed its output", self.program),
                )
                .effect_unknown()
                .with_recovery(
                    "the server closed its output after taking the call, and may have acted on it; \
                 check what it was asked to do, then dispatch again",
                ))
            }
        }
    }

    fn broken(&self, action: &str, err: &std::io::Error) -> AxError {
        AxError::failure(
            AxCode::ToolUnavailable,
            "call an mcp server",
            format!("{}: cannot {action} the server: {err}", self.program),
        )
        .with_recovery("the server is no longer reachable; this run continues without it")
    }

    /// Ends the process. Failure here means it had already ended, which
    /// is the state this asks for, so there is nothing left to report.
    fn reclaim(&mut self) {
        if self.child.kill().is_ok() {
            match self.child.wait() {
                Ok(_status) => {}
                // A child that cannot be waited for was reaped by the
                // platform; either way it is not running.
                Err(_unwaitable) => {}
            }
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.reclaim();
    }
}

/// The refusal a reader gave an answer, as the call that was waiting
/// reads it: the server took the call and answered, so what it did is
/// unknown (agent_protocols-SPEC.md 8-15). Both line-framed transports
/// hand their callers this one.
pub(super) fn answer_unread(refused: &AxError) -> AxError {
    AxError::failure(*refused.code(), refused.action(), refused.subject())
        .effect_unknown()
        .with_recovery(refused.recovery())
}

fn pipes_missing(command: &str) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "start an mcp server",
        format!("{command}: the child was started without pipes"),
    )
    .with_recovery("this build asked for piped stdin and stdout; the platform gave neither")
}

/// A server that answers every line with the same result, built from a
/// program every supported platform has.
///
/// One fixed answer is enough to drive discover, list and call, because
/// what these tests hold is the transport rather than a server's
/// judgment. It lives outside the test module, behind `conformance`, so
/// the assembly's own tests can start the same child; it exists in no
/// other build.
#[cfg(any(test, feature = "conformance"))]
pub fn echoing(answer: &str) -> (String, Vec<String>) {
    // A notification is passed over in silence, because a real server
    // does not answer one. A fake that answered everything would leave
    // one unread line in the pipe, and every later call would read the
    // answer to the message before it.
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                format!(
                    "while($l=[Console]::In.ReadLine()){{if($l -notmatch 'notifications/')\
                     {{Write-Output '{answer}'}}}}"
                ),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec![
                "-c".to_owned(),
                format!(
                    "while IFS= read -r l; do case \"$l\" in *notifications/*) ;; \
                     *) printf '%s\\n' '{answer}';; esac; done"
                ),
            ],
        )
    }
}

/// The server [`counting_starts`] builds, which after its start line
/// reads nothing until `gate` exists, so a test holds a handshake open
/// for as long as it needs and sees, from `starts`, that it has begun.
#[cfg(feature = "conformance")]
pub fn gated(answer: &str, starts: &Path, gate: &Path) -> (String, Vec<String>) {
    let (command, mut args) = counting_starts(answer, starts);
    let wait = if cfg!(windows) {
        format!(
            "while(!(Test-Path -LiteralPath '{}')){{Start-Sleep -Milliseconds 5}}; ",
            gate.display()
        )
    } else {
        format!(
            "while [ ! -e '{}' ]; do sleep 0.005; done; ",
            gate.display()
        )
    };
    if let Some(script) = args.last_mut() {
        let after_mark = script.find("; ").map_or(0, |at| at.saturating_add(2));
        script.insert_str(after_mark, &wait);
    }
    (command, args)
}

/// The server [`echoing`] builds, which also writes one line to
/// `starts` each time it is started, so a test can count how many
/// children a sequence of dispatches cost.
#[cfg(feature = "conformance")]
pub fn counting_starts(answer: &str, starts: &Path) -> (String, Vec<String>) {
    let (command, mut args) = echoing(answer);
    let mark = if cfg!(windows) {
        format!("Add-Content -LiteralPath '{}' -Value s; ", starts.display())
    } else {
        format!("echo s >> '{}'; ", starts.display())
    };
    if let Some(script) = args.last_mut() {
        script.insert_str(0, &mark);
    }
    (command, args)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
