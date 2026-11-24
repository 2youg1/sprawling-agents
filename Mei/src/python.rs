// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use crate::capture::Store as CaptureStore;
use crate::doctor::Environment;
use crate::error::{Code, Diagnostic, Error, Result};

mod frame;
mod preview;

pub use crate::capture::INLINE_BYTES;

const BOOT: &str = include_str!("boot.py");

const INLINE_KIB: u64 = INLINE_BYTES / 1024;

pub const IMAGE_CALL_LIMIT: usize = 2;
pub const IMAGE_MIB: usize = 5;

pub fn tool_description(environment: &Environment) -> String {
    format!(
        concat!(
            "<python version=\"{}\" uv_version=\"{}\">\nA persistent python interpreter. Variables, imports, and the working directory (cwd) persist across calls. ",
            "An exception prints its traceback without ending the session, so you can recover and continue without rebuilding state. ",
            "Expressions do not auto-echo, so explicitly print any values you need to inspect. ",
            "If combined UTF-8 stdout/stderr is up to {} KiB, the tool returns it inline. ",
            "Larger or non-UTF-8 output returns a notice and the full path, or a Python expression that losslessly reconstructs a non-Unicode native path; ",
            "filter that file, or re-run with narrower output, in the next python call. ",
            "Run shell commands and CLIs through subprocess, capture their output, and check return codes. ",
            "Start child interpreters with sys.executable, so they inherit the active environment. ",
            "Call show_image(path) to attach a PNG, JPEG, GIF, or WebP picture as visual tool-result content; a picture is not text output. ",
            "Attach at most {} pictures per call and at most {} MiB per picture, on the call's own thread; a picture from another thread or a later call is refused with how to read it. ",
            "Because capabilities like web_search, web_fetch or sub-agent delegation are not exposed as built-in tools, ",
            "python serves as your interface to invoke external APIs, CLIs, headless tools, and more directly via subprocess.\n</python>"
        ),
        environment.python.version, environment.uv.version, INLINE_KIB, IMAGE_CALL_LIMIT, IMAGE_MIB
    )
}

const CLOSE_POLL: Duration = Duration::from_millis(20);
const CLOSE_GRACE: Duration = Duration::from_secs(5);

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
) -> Error {
    Error::Diagnostic(Diagnostic::new(Code::Python, action, subject, recovery))
}

pub(crate) struct Execution {
    pub content: serde_json::Value,
    pub failed: bool,
    pub previews: Vec<String>,
}

pub struct Py {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    captures: CaptureStore,
}

impl Py {
    pub fn spawn() -> Result<Py> {
        Py::with_boot(BOOT, None)
    }

    pub fn spawn_in(environment: &Environment) -> Result<Py> {
        Py::with_boot(BOOT, Some(environment))
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "an independent harness process starts its own interpreter, not a city child"
    )]
    fn with_boot(boot: &str, environment: Option<&Environment>) -> Result<Py> {
        let mut command = match environment {
            Some(environment) => Command::new(&environment.python.executable),
            None => Command::new(python_program()),
        };
        if let Some(environment) = environment {
            command.env("PATH", environment.child_path()?);
        }
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
        let captures = CaptureStore::create()?;
        command.env("MEI_CAPTURE_DIR", captures.root());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                let mut captures = captures;
                if let Err(cleanup) = captures.discard() {
                    eprintln!("{cleanup}");
                }
                return Err(failure(
                    "start Python for Mei",
                    err.to_string(),
                    "install Python 3 on PATH as python3 or python",
                ));
            }
        };
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
            captures,
        })
    }

    pub fn run(&mut self, code: &str) -> Result<serde_json::Value> {
        Ok(self.run_observed(code)?.content)
    }

    pub(crate) fn run_observed(&mut self, code: &str) -> Result<Execution> {
        let frame = serde_json::to_vec(code).map_err(|err| {
            failure(
                "serialize a Python control frame",
                err.to_string(),
                "pass code as a UTF-8 Rust string",
            )
        })?;
        self.write_frame(&frame)?;
        let frame = self.read_frame()?;
        let text = self.captures.observe(frame.capture)?;
        let previews = if std::env::var_os("MEI_CAPTURE_CREATED").is_some_and(|value| value == "1")
        {
            frame
                .images
                .iter()
                .filter_map(|image| preview::thumbnail(&image.media_type, &image.data))
                .collect()
        } else {
            Vec::new()
        };
        Ok(Execution {
            content: frame::content(text, frame.images),
            failed: frame.failed,
            previews,
        })
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

    fn read_frame(&mut self) -> Result<frame::Frame> {
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
        frame::parse(&line)
    }

    pub fn close(&mut self) -> Result<()> {
        self.close_within(CLOSE_GRACE)?;
        if let Err(err) = self.captures.complete() {
            eprintln!("{err}");
        }
        Ok(())
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
            Ok(None) => self.terminate_process(),
            Err(err) => Err(failure(
                "close and wait for Python",
                err.to_string(),
                "if the interpreter does not exit, kill it and wait again",
            )),
        }
    }

    pub fn kill_and_wait(&mut self) -> Result<()> {
        let process = self.terminate_process();
        let captures = self.captures.discard();
        match (process, captures) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(())) | (Ok(()), Err(err)) => Err(err),
            (Err(process), Err(captures)) => Err(failure(
                "clean up a failed Python run",
                format!("{process}; {captures}"),
                "stop remaining processes and remove the named capture directory before retrying",
            )),
        }
    }

    fn terminate_process(&mut self) -> Result<()> {
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
    if cfg!(windows) { "python" } else { "python3" }
}

impl Drop for Py {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {
                if let Err(err) = self.captures.abandon() {
                    eprintln!("{err}");
                }
            }
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

    fn text(value: serde_json::Value) -> String {
        value.as_str().unwrap().to_owned()
    }

    fn read_path(path: &str) -> String {
        std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
    }

    #[test]
    fn state_and_small_outputs_survive_calls() {
        let mut python = Py::spawn().unwrap();
        assert_eq!(python.run("x = 40").unwrap(), "");
        assert_eq!(python.run("40 + 2").unwrap(), "");
        assert!(
            text(python.run("raise ValueError('recoverable')").unwrap())
                .contains("ValueError: recoverable")
        );
        assert_eq!(
            text(python.run("print(x)").unwrap()).replace("\r\n", "\n"),
            "40\n"
        );
        assert_eq!(
            text(python.run("x += 2\nprint(x)").unwrap()).replace("\r\n", "\n"),
            "42\n"
        );
        assert!(text(python.run("raise SystemExit(3)").unwrap()).contains("SystemExit: 3"));
        assert_eq!(
            text(python.run("print(x)").unwrap()).replace("\r\n", "\n"),
            "42\n"
        );
        assert_eq!(python.run("pass").unwrap(), "");
        python.close().unwrap();
    }

    #[test]
    fn dropping_an_exited_interpreter_reclaims_uncommitted_captures() {
        let mut python = Py::spawn().unwrap();
        let captures = python.captures.root().to_path_buf();
        assert!(python.run("import os\nos._exit(0)").is_err());
        for _ in 0..500 {
            if python.child.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(python.child.try_wait().unwrap().is_some());
        drop(python);
        assert!(!captures.exists());
    }

    #[test]
    fn tool_description_uses_the_inline_budget() {
        let environment = Environment::new(
            crate::doctor::Program::new("3.12.13", std::path::PathBuf::from("python")),
            crate::doctor::Program::new("0.12.3", std::path::PathBuf::from("uv")),
        );
        let description = tool_description(&environment);
        assert!(description.contains(&format!("up to {INLINE_KIB} KiB")));
        assert!(description.contains(&format!(
            "at most {IMAGE_CALL_LIMIT} pictures per call and at most {IMAGE_MIB} MiB"
        )));
        assert!(description.starts_with("<python version=\"3.12.13\" uv_version=\"0.12.3\">"));
    }

    #[test]
    fn boot_py_enforces_the_limits_the_description_states() {
        assert!(BOOT.contains("IMAGE_CALL_LIMIT = 2\n"));
        assert!(BOOT.contains("IMAGE_LIMIT = 5 * 1024 * 1024\n"));
        assert_eq!(IMAGE_CALL_LIMIT.to_string(), "2");
        assert_eq!(IMAGE_MIB.to_string(), "5");
    }

    #[test]
    fn imports_and_cwd_survive_calls() {
        let directory = tempfile::tempdir().unwrap();
        let path = serde_json::to_string(&directory.path().to_string_lossy()).unwrap();
        let mut python = Py::spawn().unwrap();
        let first = python
            .run(&format!("import os\nos.chdir({path})\nprint(os.getcwd())"))
            .unwrap();
        let second = text(python.run("print(os.getcwd())").unwrap());
        assert_eq!(first, second);
        assert_eq!(
            std::path::Path::new(second.trim()).canonicalize().unwrap(),
            directory.path().canonicalize().unwrap()
        );
        python.close().unwrap();
    }

    #[test]
    fn sys_executable_starts_a_captured_child_python() {
        let mut python = Py::spawn().unwrap();
        let output = text(python
            .run("import subprocess, sys\nchild = subprocess.run([sys.executable, '-c', 'print(42)'], capture_output=True, text=True, check=True)\nprint(child.stdout, end='')")
            .unwrap());
        assert_eq!(output.replace("\r\n", "\n"), "42\n");
        python.close().unwrap();
    }

    #[test]
    fn large_output_is_a_notice_and_full_file_path() {
        let mut python = Py::spawn().unwrap();
        let notice = text(python.run("print('x' * 50000)").unwrap());
        assert!(notice.starts_with("Output exceeds 16384 bytes."));
        let path = notice.lines().last().unwrap();
        assert_eq!(read_path(path).len(), 50001);
        python.close().unwrap();
    }

    #[test]
    fn the_inline_budget_is_exact() {
        let mut python = Py::spawn().unwrap();
        let inline = python.run("import os\nos.write(1, b'x' * 16384)").unwrap();
        assert_eq!(text(inline).len(), 16384);
        let notice = text(python.run("os.write(1, b'x' * 16385)").unwrap());
        assert!(notice.starts_with("Output exceeds 16384 bytes."));
        python.close().unwrap();
    }

    #[test]
    fn non_utf8_output_is_a_notice_and_full_file_path() {
        let mut python = Py::spawn().unwrap();
        let notice = text(python.run("import os\nos.write(1, b'\\xff')").unwrap());
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
        let output = text(python
            .run("import threading\nthreading.Thread(target=threading.Event().wait).start()\nprint('answered')")
            .unwrap());
        assert_eq!(output.replace("\r\n", "\n"), "answered\n");
        python
            .close_within(Duration::from_millis(200))
            .expect("the close escalates instead of hanging");
    }
}
