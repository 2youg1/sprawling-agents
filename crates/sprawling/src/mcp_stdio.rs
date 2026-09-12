// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An MCP server run as a child process, spoken to one line at a time.
//!
//! The protocol crate knows what to say; this module knows where the
//! bytes go, how long they may take, and who reclaims the process. All
//! three belong to the assembly layer, because they are the parts that
//! touch this machine.
//!
//! Two decisions are worth reading before changing anything here.
//!
//! **The reader is a thread, and the deadline is real.** A synchronous
//! read on a pipe has no deadline of its own, so a server that never
//! answers would hold the run's only worker forever — the exact failure
//! this project has already met once from the other direction, when a
//! provider went silent after `model_called` and nothing timed out. The
//! thread turns a blocking read into a channel this side can wait on
//! with a deadline. It cannot leak: killing the child closes the pipe,
//! the reader sees end of input, and it returns.
//!
//! **A deadline that passes kills the child.** A late answer arriving
//! after its call gave up would be read as the answer to the next call,
//! and two calls swapped is worse than a refusal. Killing ends that
//! possibility rather than guarding against it.

use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use crate::mcp_redeeming::Redeemed;

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
    answers: Receiver<String>,
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
        let (sender, answers) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name(format!("mcp-{command}"))
            .spawn(move || {
                for line in std::io::BufReader::new(stdout).lines() {
                    // Either end finishing ends the reader: a closed pipe
                    // means the server is gone, and a closed channel
                    // means this city stopped listening.
                    let Ok(text) = line else { break };
                    if sender.send(text).is_err() {
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

impl std::fmt::Debug for StdioServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.try_lock() {
            Ok(connection) => write!(f, "StdioServer({})", connection.program),
            Err(_in_flight) => f.write_str("StdioServer(answering)"),
        }
    }
}

impl protocol::Outbound for StdioServer {
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
            Ok(answer) => Ok(answer),
            Err(RecvTimeoutError::Timeout) => {
                self.reclaim();
                Err(AxError::failure(
                    AxCode::Timeout,
                    "call an mcp server",
                    format!("{}: no answer within {} ms", self.program, patience.0),
                )
                .with_recovery(
                    "the server was stopped so a late answer cannot be read as the next one; \
                     start the run again once the server responds",
                ))
            }
            Err(RecvTimeoutError::Disconnected) => Err(AxError::failure(
                AxCode::ToolUnavailable,
                "call an mcp server",
                format!("{}: the server closed its output", self.program),
            )
            .with_recovery("check the server's own logs; this city sees only its answers")),
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
/// judgment. It lives outside the test module so the assembly's own
/// tests can start the same child; it exists in no other build.
#[cfg(test)]
pub(crate) fn echoing(answer: &str) -> (String, Vec<String>) {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
