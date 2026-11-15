// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The persistent Python adapter (`Mei/Spec.lean` §6, §10-§12).
//! Each `run` returns the path of the file that captured stdout+stderr.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use crate::error::{Code, Diagnostic, Error, Result};

const BOOT: &str = include_str!("boot.py");

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

    /// Executes one code string; returns the path of the capture file.
    ///
    /// # Errors
    /// Channel, frame and process failures. Tracebacks live in the file.
    pub fn run(&mut self, code: &str) -> Result<String> {
        let frame = serde_json::to_vec(code).map_err(|err| {
            failure(
                "serialize a Python control frame",
                err.to_string(),
                "pass code as a UTF-8 Rust string",
            )
        })?;
        self.write_frame(&frame)?;
        self.read_frame()
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

fn python_program() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
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
        std::fs::read_to_string(path)
            .unwrap()
            .replace("\r\n", "\n")
    }

    #[test]
    fn state_and_path_handles_survive_calls() {
        let mut python = Py::spawn().unwrap();
        let path = python.run("x = 40\nprint(x)").unwrap();
        assert_eq!(read_path(&path), "40\n");
        let path = python.run("x += 2\nprint(x)").unwrap();
        assert_eq!(read_path(&path), "42\n");
        let path = python.run("pass").unwrap();
        assert_eq!(read_path(&path), "");
        assert!(
            read_path(&python.run("raise SystemExit(3)").unwrap()).contains("SystemExit: 3")
        );
        python.close().unwrap();
    }

    #[test]
    fn large_output_is_a_full_file_path() {
        let mut python = Py::spawn().unwrap();
        let path = python.run("print('x' * 50000)").unwrap();
        assert_eq!(read_path(&path).len(), 50001);
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
        let path = python
            .run("import threading\nthreading.Thread(target=threading.Event().wait).start()\nprint('answered')")
            .unwrap();
        assert_eq!(read_path(&path), "answered\n");
        python
            .close_within(Duration::from_millis(200))
            .expect("the close escalates instead of hanging");
    }
}

