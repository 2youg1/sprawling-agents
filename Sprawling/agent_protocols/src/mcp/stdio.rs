// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};

use super::reading::answer_unread;
use super::redeeming::Redeemed;
use super::{Heard, Lines};

#[derive(Clone)]
pub(crate) struct StdioServer {
    inner: Arc<Mutex<Connection>>,
}

struct Connection {
    program: String,
    child: std::process::Child,
    requests: std::process::ChildStdin,
    answers: Lines,
}

impl StdioServer {

    pub(crate) fn start(
        command: &str,
        args: &[String],
        env: &[Redeemed],
        cwd: &Path,
    ) -> Result<StdioServer, AxError> {
        let mut child = child::command(command)
            .args(args)
            .envs(env.iter().map(|pair| (pair.name(), pair.plaintext())))
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())

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
        let answers = Lines::over(std::io::BufReader::new(stdout), command)?;
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

    fn tell(&mut self, line: &str) -> Result<(), AxError> {
        self.framed(line)?;
        writeln!(self.requests, "{line}").map_err(|err| self.broken("write to", &err))?;
        self.requests
            .flush()
            .map_err(|err| self.broken("flush", &err))
    }

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
        match self.answers.next(Duration::from_millis(patience.0)) {
            Ok(Heard::Message(answer)) => Ok(answer),

            Err(refused) => {
                self.reclaim();
                Err(answer_unread(&refused))
            }
            Ok(Heard::Silent) => {
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
            Ok(Heard::Ended) => Err(AxError::failure(
                AxCode::ToolUnavailable,
                "call an mcp server",
                format!("{}: the server closed its output", self.program),
            )
            .effect_unknown()
            .with_recovery(
                "the server closed its output after taking the call, and may have acted on it; \
                 check what it was asked to do, then dispatch again",
            )),
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

    fn reclaim(&mut self) {
        if self.child.kill().is_ok() {
            match self.child.wait() {
                Ok(_status) => {}

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

#[cfg(any(test, feature = "conformance"))]
pub fn echoing(answer: &str) -> (String, Vec<String>) {

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
