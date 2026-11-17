// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The persistent Python3 adapter (`Mei/Spec.lean` §6, §10-§12).
//! Small UTF-8 captures are returned whole; large captures stay at their paths.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use crate::error::{Code, Diagnostic, Error, Result};

const BOOT: &str = include_str!("boot.py");
const INLINE_BYTES: u64 = 16 * 1024;

const CLOSE_POLL: Duration = Duration::from_millis(20);
const CLOSE_GRACE: Duration = Duration::from_secs(5);

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
) -> Error {
    Error::Diagnostic(Diagnostic::new(Code::Python, action, subject, recovery))
}

/// The direct child and its private one-line JSON channel.
pub struct Py {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Py {
    /// Starts `python3`/`python` with BOOT, preserving an existing `HARNESS`.
    ///
    /// # Errors
    /// The interpreter cannot be started or its private pipes cannot be opened.
    pub fn spawn() -> Result<Py> {
        Py::with_boot(BOOT)
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "an independent harness process starts its own interpreter, not a city child"
    )]
    fn with_boot(boot: &str) -> Result<Py> {
        let mut command = Command::new(python_program());
        command.args(["-u", "-c", boot]);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());
        if std::env::var_os("HARNESS").is_none() {
            let harness = std::env::current_exe().map_err(|err| {
                failure(
                    "resolve current_exe for HARNESS",
                    err.to_string(),
                    "set HARNESS to the Mei executable before starting it",
                )
            })?;
            command.env("HARNESS", harness);
        }
        #[cfg(test)]
        command.env("LC_ALL", "C").env("PYTHONUTF8", "0");
        let mut child = command.spawn().map_err(|err| {
            failure(
                "start Python for Mei",
                err.to_string(),
                "install Python 3 on PATH as python3 or python",
            )
        })?;
        let input = child.stdin.take().ok_or_else(|| {
            failure(
                "open the Python control input",
                "stdin was not piped",
                "keep the Python control channel piped",
            )
        })?;
        let output = child.stdout.take().ok_or_else(|| {
            failure(
                "open the Python control output",
                "stdout was not piped",
                "keep the Python control channel piped",
            )
        })?;
        Ok(Py {
            child,
            input: Some(input),
            output: BufReader::new(output),
        })
    }

    /// Executes one code string and returns its observable tool result.
    ///
    /// # Errors
    /// Channel, frame, capture-file and process failures.
    pub fn run(&mut self, code: &str) -> Result<String> {
        let frame = serde_json::to_vec(code).map_err(|err| {
            failure(
                "serialize a Python control frame",
                err.to_string(),
                "pass code as a UTF-8 Rust string",
            )
        })?;
        self.write_frame(&frame)?;
        let path = self.read_frame()?;
        observable_output(&path)
    }

    fn write_frame(&mut self, frame: &[u8]) -> Result<()> {
        let input = self.input.as_mut().ok_or_else(|| {
            failure(
                "write a Python control frame",
                "the control input is closed",
                "start a new Mei process instead of reusing a closed interpreter",
            )
        })?;
        input
            .write_all(frame)
            .and_then(|()| input.write_all(b"\n"))
            .and_then(|()| input.flush())
            .map_err(|err| {
                failure(
                    "write a Python control frame",
                    err.to_string(),
                    "the interpreter or private input channel died; start a new Mei process",
                )
            })
    }

    fn read_frame(&mut self) -> Result<String> {
        let mut line = String::new();
        let count = self.output.read_line(&mut line).map_err(|err| {
            failure(
                "read a Python response frame",
                err.to_string(),
                "the private output channel must emit UTF-8 JSON strings",
            )
        })?;
        if count == 0 {
            let status = self.child.try_wait().map_err(|err| {
                failure(
                    "reap Python after a closed channel",
                    err.to_string(),
                    "inspect the interpreter failure before starting a new process",
                )
            })?;
            return Err(failure(
                "read a Python response frame",
                format!("the private channel closed; interpreter status: {status:?}"),
                "the interpreter or private channel died; start a new Mei process",
            ));
        }
        let value: serde_json::Value = serde_json::from_str(&line).map_err(|err| {
            failure(
                "parse a Python response frame",
                err.to_string(),
                "the private channel must contain exactly one JSON string per response",
            )
        })?;
        value.as_str().map(str::to_owned).ok_or_else(|| {
            failure(
                "parse a Python response frame",
                "the JSON value is not a string",
                "the private channel must contain JSON strings, not objects",
            )
        })
    }

    /// Closes control input and reaps within [`CLOSE_GRACE`], else kills.
    ///
    /// # Errors
    /// Waiting on or killing/reaping the interpreter fails.
    pub fn close(&mut self) -> Result<()> {
        self.close_within(CLOSE_GRACE)
    }

    fn close_within(&mut self, grace: Duration) -> Result<()> {
        drop(self.input.take());
        let Some(polls) = grace.as_millis().checked_div(CLOSE_POLL.as_millis()) else {
            return Err(failure(
                "close and wait for Python",
                "the close poll has no length",
                "keep the close poll positive",
            ));
        };
        for _ in 0..polls {
            match self.child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => {}
                Err(err) => {
                    return Err(failure(
                        "close and wait for Python",
                        err.to_string(),
                        "if the interpreter does not exit, kill it and wait again",
                    ));
                }
            }
            std::thread::sleep(CLOSE_POLL);
        }
        match self.child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => self.kill_and_wait(),
            Err(err) => Err(failure(
                "close and wait for Python",
                err.to_string(),
                "if the interpreter does not exit, kill it and wait again",
            )),
        }
    }

    /// Kills the interpreter and reaps it.
    ///
    /// # Errors
    /// Kill or wait failed.
    pub fn kill_and_wait(&mut self) -> Result<()> {
        let killed = self.child.kill();
        let waited = self.child.wait();
        match (killed, waited) {
            (Ok(()), Ok(_)) => Ok(()),
            (Err(err), _) => Err(failure(
                "kill Python",
                err.to_string(),
                "reap the process before starting another Mei run",
            )),
            (Ok(()), Err(err)) => Err(failure(
                "wait for killed Python",
                err.to_string(),
                "reap the process before starting another Mei run",
            )),
        }
    }
}

fn output_too_large(path: &str) -> String {
    format!(
        "Output exceeds {INLINE_BYTES} bytes. Use Python3 to filter and print the needed result.\n{path}"
    )
}

fn observable_output(path: &str) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|err| {
        failure(
            "open Python3 output",
            format!("{path}: {err}"),
            "keep the capture file available until the tool result is read",
        )
    })?;
    let bytes = file
        .metadata()
        .map_err(|err| {
            failure(
                "read Python3 output metadata",
                format!("{path}: {err}"),
                "keep the capture file readable until the tool result is read",
            )
        })?
        .len();
    if bytes > INLINE_BYTES {
        return Ok(output_too_large(path));
    }
    let mut captured = Vec::new();
    file.take(INLINE_BYTES.saturating_add(1))
        .read_to_end(&mut captured)
        .map_err(|err| {
            failure(
                "read Python3 output",
                format!("{path}: {err}"),
                "keep the capture file readable until the tool result is read",
            )
        })?;
    if u64::try_from(captured.len()).unwrap_or(u64::MAX) > INLINE_BYTES {
        return Ok(output_too_large(path));
    }
    match String::from_utf8(captured) {
        Ok(text) => Ok(text),
        Err(_) => Ok(format!(
            "Output is not UTF-8. Process it with Python3.\n{path}"
        )),
    }
}

fn python_program() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

impl Drop for Py {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                if let Err(err) = self.kill_and_wait() {
                    eprintln!("{err}");
                }
            }
        }
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
mod tests {
    use super::*;

    fn read_path(path: &str) -> String {
        std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
    }

    #[test]
    fn state_and_small_outputs_survive_calls() {
        let mut python = Py::spawn().unwrap();
        assert_eq!(python.run("x = 40").unwrap(), "");
        assert_eq!(
            python.run("print(x)").unwrap().replace("\r\n", "\n"),
            "40\n"
        );
        assert_eq!(
            python
                .run("x += 2\nprint(x)")
                .unwrap()
                .replace("\r\n", "\n"),
            "42\n"
        );
        assert_eq!(python.run("pass").unwrap(), "");
        assert!(
            python
                .run("raise SystemExit(3)")
                .unwrap()
                .contains("SystemExit: 3")
        );
        python.close().unwrap();
    }

    #[test]
    fn large_output_is_a_notice_and_full_file_path() {
        let mut python = Py::spawn().unwrap();
        let notice = python.run("print('x' * 50000)").unwrap();
        assert!(notice.starts_with("Output exceeds 16384 bytes."));
        let path = notice.lines().last().unwrap();
        assert_eq!(read_path(path).len(), 50001);
        python.close().unwrap();
    }

    #[test]
    fn the_inline_budget_is_exact() {
        let mut python = Py::spawn().unwrap();
        let inline = python.run("import os\nos.write(1, b'x' * 16384)").unwrap();
        assert_eq!(inline.len(), 16384);
        let notice = python.run("os.write(1, b'x' * 16385)").unwrap();
        assert!(notice.starts_with("Output exceeds 16384 bytes."));
        python.close().unwrap();
    }

    #[test]
    fn non_utf8_output_is_a_notice_and_full_file_path() {
        let mut python = Py::spawn().unwrap();
        let notice = python.run("import os\nos.write(1, b'\\xff')").unwrap();
        assert!(notice.starts_with("Output is not UTF-8."));
        let path = notice.lines().last().unwrap();
        assert_eq!(std::fs::read(path).unwrap(), [255]);
        python.close().unwrap();
    }

    #[test]
    fn an_invalid_control_frame_is_fatal() {
        let mut python = Py::spawn().unwrap();
        python.write_frame(b"{}").unwrap();
        assert!(python.read_frame().is_err());
        python.kill_and_wait().unwrap();
    }

    #[test]
    fn an_interpreter_death_is_an_error() {
        let mut python = Py::spawn().unwrap();
        assert!(python.run("import os\nos._exit(7)").is_err());
        python.kill_and_wait().unwrap();
    }

    #[test]
    fn leftover_threads_cannot_hold_the_answer() {
        let mut python = Py::spawn().unwrap();
        let output = python
            .run("import threading\nthreading.Thread(target=threading.Event().wait).start()\nprint('answered')")
            .unwrap();
        assert_eq!(output.replace("\r\n", "\n"), "answered\n");
        python
            .close_within(Duration::from_millis(200))
            .expect("the close escalates instead of hanging");
    }
}
