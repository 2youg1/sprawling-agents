// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The persistent Python adapter (`Mei/Spec.lean` §6, §10-§12).
//! Python exceptions are tool feedback; channel and process failures are not.

#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
#[cfg(unix)]
use std::time::Duration;

use crate::error::{Code, Diagnostic, Error, Result};

#[cfg(unix)]
const BOOT: &str = include_str!("boot.py");

/// How often a closing interpreter is checked for exit. The close grace is
/// a multiple of this poll, so the bounded wait needs no clock of its own.
#[cfg(unix)]
const CLOSE_POLL: Duration = Duration::from_millis(20);

/// How long a closing interpreter may take to exit before it is killed and
/// reaped. A clean interpreter exits in moments; model code that leaves
/// non-daemon threads behind would otherwise hold the final answer hostage
/// forever, because Python refuses to exit while any of them lives.
#[cfg(unix)]
const CLOSE_GRACE: Duration = Duration::from_secs(5);

#[cfg(unix)]
fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
) -> Error {
    Error::Diagnostic(Diagnostic::new(Code::Python, action, subject, recovery))
}

/// The direct child and its private one-line JSON channel.
#[cfg(unix)]
pub struct Py {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

/// A non-POSIX placeholder which always refuses to start.
#[cfg(not(unix))]
pub struct Py;

#[cfg(unix)]
impl Py {
    /// Starts `python3 -u -c BOOT`, preserving an existing `HARNESS` wrapper.
    ///
    /// # Errors
    /// The interpreter cannot be started or its private pipes cannot be opened.
    pub fn spawn() -> Result<Py> {
        Py::with_boot(BOOT)
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "an independent Archetype process starts its own interpreter, not a city child"
    )]
    fn with_boot(boot: &str) -> Result<Py> {
        let mut command = Command::new("python3");
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
                "start python3",
                err.to_string(),
                "install python3 on PATH or run Mei in its POSIX container",
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

    /// Executes one code string and returns its captured stdout+stderr.
    ///
    /// # Errors
    /// Only serialization, channel, frame and process failures are errors.
    /// Python tracebacks and best-effort timeouts return ordinary text.
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

    /// Closes control input and reaps a normally idle interpreter.
    ///
    /// The wait is bounded by [`CLOSE_GRACE`]: model code may leave
    /// non-daemon threads behind, and Python refuses to exit while any of
    /// them lives, so after the grace the interpreter is killed and reaped.
    /// A forced cleanup is ordinary shutdown, not a run failure; the model
    /// already returned its final answer.
    ///
    /// # Errors
    /// Waiting on or killing/reaping the interpreter fails. On an active
    /// or broken run use [`Py::kill_and_wait`] instead.
    pub fn close(&mut self) -> Result<()> {
        self.close_within(CLOSE_GRACE)
    }

    /// Polls the interpreter for exit at most `grace` long, then escalates.
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

    /// Kills the direct interpreter and always attempts to reap it.
    ///
    /// # Errors
    /// Either the kill or wait failed; both are attempted before returning.
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

#[cfg(unix)]
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

#[cfg(not(unix))]
impl Py {
    /// Native Windows is outside Mei's POSIX boundary.
    ///
    /// # Errors
    /// Always returns `E_MEI_UNSUPPORTED`.
    pub fn spawn() -> Result<Py> {
        Err(Error::Diagnostic(Diagnostic::new(
            Code::Unsupported,
            "start Python for Mei",
            "native Windows has no SIGALRM or POSIX fd semantics",
            "run Mei inside a POSIX container, VM, bwrap or dedicated POSIX account",
        )))
    }

    /// Always returns the same unsupported-platform error as [`Py::spawn`].
    ///
    /// # Errors
    /// Always returns `E_MEI_UNSUPPORTED`.
    pub fn run(&mut self, _code: &str) -> Result<String> {
        Py::spawn().map(|_| String::new())
    }

    /// No interpreter exists on this platform, so cleanup succeeds.
    pub fn close(&mut self) -> Result<()> {
        Ok(())
    }

    /// No interpreter exists on this platform, so cleanup succeeds.
    pub fn kill_and_wait(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(all(test, unix))]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn state_subprocesses_and_encoding_survive_calls() {
        let mut python = Py::spawn().unwrap();
        assert_eq!(python.run("x = 40\nprint(x)").unwrap(), "40\n");
        assert_eq!(python.run("x += 2\nprint(x)").unwrap(), "42\n");
        let output = python.run("import subprocess, sys\nsubprocess.run([sys.executable, '-c', 'print(7)'])\nprint(subprocess.run(['cat'], capture_output=True, text=True).stdout == '')").unwrap();
        assert_eq!(output, "7\nTrue\n");
        assert!(python.run("input()").unwrap().contains("EOFError"));
        assert_eq!(python.run("print('仍 works')").unwrap(), "仍 works\n");
        assert_eq!(python.run("import os\nos.write(1, b'\\xff')").unwrap(), "�");
        assert_eq!(python.run("pass").unwrap(), "(no output)");
        assert!(
            python
                .run("raise SystemExit(3)")
                .unwrap()
                .contains("SystemExit: 3")
        );
        python.close().unwrap();
    }

    #[test]
    fn the_output_window_boundary_is_exact() {
        let mut python = Py::spawn().unwrap();
        let output = python.run("import os\nos.write(1, b'x' * 16000)").unwrap();
        assert_eq!(output.len(), 16000);
        assert!(!output.contains("truncated"));
        let output = python.run("os.write(1, b'x' * 16001)").unwrap();
        assert!(output.contains("truncated; full output:"));
        let path = output
            .split("full output: ")
            .nth(1)
            .unwrap()
            .split(" ...]")
            .next()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        python.close().unwrap();
    }

    #[test]
    fn large_output_is_windowed_without_losing_the_full_file() {
        let mut python = Py::spawn().unwrap();
        let output = python.run("print('x' * 50000)").unwrap();
        assert!(output.starts_with(&"x".repeat(8000)));
        assert!(output.contains("truncated; full output:"));
        assert!(output.ends_with(&format!("{}\n", "x".repeat(7999))));
        let path = output
            .split("full output: ")
            .nth(1)
            .unwrap()
            .split(" ...]")
            .next()
            .unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap().len(), 50001);
        std::fs::remove_file(path).unwrap();
        python.close().unwrap();
    }

    #[test]
    fn background_output_is_a_snapshot_at_each_call_boundary() {
        let mut python = Py::spawn().unwrap();
        let first = python
            .run(
                "import threading\nready = threading.Event()\ndef background():\n    ready.wait()\n    print('late', flush=True)\nt = threading.Thread(target=background)\nt.start()\nprint('early')",
            )
            .unwrap();
        assert_eq!(first, "early\n");
        let second = python.run("ready.set()\nt.join()\nprint('next')").unwrap();
        assert_eq!(second, "late\nnext\n");
        python.close().unwrap();
    }

    #[test]
    fn a_timeout_returns_feedback_and_the_next_call_reuses_state() {
        let boot = BOOT.replace("250s", "1s").replace("alarm(250)", "alarm(1)");
        let mut python = Py::with_boot(&boot).unwrap();
        let output = python.run("x = 42\nwhile True: pass").unwrap();
        assert!(
            output.contains("TimeoutError: timeout after 1s"),
            "{output}"
        );
        assert_eq!(python.run("print(x)").unwrap(), "42\n");
        python.close().unwrap();
    }

    #[test]
    fn an_invalid_control_frame_is_fatal_not_a_tool_traceback() {
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
    fn leftover_background_threads_cannot_hold_the_final_answer_hostage() {
        let mut python = Py::spawn().unwrap();
        python
            .run("import threading\nthreading.Thread(target=threading.Event().wait).start()\nprint('answered')")
            .unwrap();
        // A non-daemon thread never finishes: the bounded close kills and
        // reaps the interpreter instead of waiting forever.
        python
            .close_within(Duration::from_millis(200))
            .expect("the close escalates instead of hanging");
    }

    #[test]
    fn a_background_writer_during_snapshot_cannot_overwrite_or_extend_it() {
        let mut python = Py::spawn().unwrap();
        let first = python
            .run(
                "import os, threading\nprint('head', flush=True)\nthreading.Thread(target=lambda: [os.write(1, b'x') for _ in range(50000)]).start()",
            )
            .unwrap();
        // The snapshot is read through an independent handle, bounded to the
        // size captured before the read: the head survives and late background
        // appends cannot prolong it (they land past the bound).
        assert!(first.starts_with("head\n"), "{first}");
        assert!(first.len() < 64, "the snapshot stays bounded: {first}");
        python.kill_and_wait().unwrap();
    }
}
