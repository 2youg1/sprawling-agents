// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::process::{Command, Stdio};
use std::time::Duration;

use crate::capture::Store as CaptureStore;
use crate::doctor::Environment;
use crate::error::{Code, Diagnostic, Error, Result};

mod frame;
mod preview;
mod process;

use process::Interpreter;

pub use crate::capture::INLINE_BYTES;

const BOOT: &str = include_str!("boot.py");

const INLINE_KIB: u64 = INLINE_BYTES / 1024;

pub const IMAGE_CALL_LIMIT: usize = 2;
pub const IMAGE_MIB: usize = 5;

pub fn tool_description(environment: &Environment) -> String {
    format!(
        concat!(
            "<python version=\"{}\" uv_version=\"{}\">\nA persistent python interpreter. Variables, imports, and the working directory (cwd) persist across calls. ",
            "Send calls you can already specify together in one response. They run in order and each returns its own result. ",
            "Wait for a result when it determines the next call. ",
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
    interpreter: Interpreter,
    captures: CaptureStore,
}

impl Py {
    pub fn spawn() -> Result<Py> {
        let mut python = Py::prepare(None)?;
        python.interpreter.start()?;
        Ok(python)
    }

    pub fn spawn_in(environment: &Environment) -> Result<Py> {
        let mut python = Py::prepare_in(environment)?;
        python.interpreter.start()?;
        Ok(python)
    }

    pub fn prepare_in(environment: &Environment) -> Result<Py> {
        Py::prepare(Some(environment))
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "Mei prepares its independent interpreter command"
    )]
    fn prepare(environment: Option<&Environment>) -> Result<Py> {
        let mut command = match environment {
            Some(environment) => Command::new(&environment.python.executable),
            None => Command::new(python_program()),
        };
        if let Some(environment) = environment {
            command.env("PATH", environment.child_path()?);
        }
        command.current_dir(std::env::current_dir().map_err(|err| {
            failure(
                "bind the Python working directory",
                err.to_string(),
                "keep the session working directory accessible before starting Mei",
            )
        })?);
        command
            .args(["-u", "-c", BOOT])
            .arg(INLINE_BYTES.to_string());
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
        Ok(Py {
            interpreter: Interpreter::Ready(command),
            captures,
        })
    }

    pub fn run(&mut self, code: &str) -> Result<serde_json::Value> {
        Ok(self.run_observed(code)?.content)
    }

    pub(crate) fn run_observed(&mut self, code: &str) -> Result<Execution> {
        let length = u64::try_from(code.len())
            .map_err(|err| {
                failure(
                    "size a Python control frame",
                    err.to_string(),
                    "send a smaller Python tool call",
                )
            })?
            .to_le_bytes();
        let mut frame = Vec::with_capacity(length.len().saturating_add(code.len()));
        frame.extend_from_slice(&length);
        frame.extend_from_slice(code.as_bytes());
        let process = self.interpreter.start()?;
        process.write_frame(&frame)?;
        let frame = process.read_frame()?;
        let text = match frame.capture {
            frame::Capture::Text(text) => {
                self.captures.reclaim_inline()?;
                text
            }
            frame::Capture::File(identifier) => self.captures.observe(identifier)?,
        };
        let previews = if std::env::var_os("MEI_CAPTURE_CREATED").is_some_and(|value| value == "1")
        {
            frame
                .images
                .iter()
                .filter_map(|image| preview::thumbnail(image.media_type, &image.data))
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

    pub fn close(&mut self) -> Result<()> {
        self.interpreter.close_within(CLOSE_GRACE)?;
        if let Err(err) = self.captures.complete() {
            eprintln!("{err}");
        }
        Ok(())
    }

    pub fn kill_and_wait(&mut self) -> Result<()> {
        let process = self.interpreter.terminate();
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
}

fn python_program() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

impl Drop for Py {
    fn drop(&mut self) {
        if let Err(err) = self.kill_and_wait() {
            eprintln!("{err}");
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
        let child = &mut python.interpreter.start().unwrap().child;
        for _ in 0..500 {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(child.try_wait().unwrap().is_some());
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
    fn retained_output_descriptors_still_write_only_their_own_capture() {
        let mut python = Py::spawn().unwrap();
        python.run("import os").unwrap();
        assert_eq!(
            python
                .run("inline = os.dup(1); print('seen', end='')")
                .unwrap(),
            "seen"
        );
        let notice = text(python.run("os.write(inline, b'late'); os.close(inline); spilled = os.dup(2); os.write(2, b'x' * 16385)").unwrap());
        let path = notice.lines().last().unwrap();
        assert_eq!(
            python
                .run("os.write(spilled, b'after'); os.close(spilled); print('next', end='')")
                .unwrap(),
            "next"
        );
        python.close().unwrap();
        let output = std::fs::read(path).unwrap();
        assert_eq!(output.len(), 16390);
        assert!(output.ends_with(b"after"));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(std::path::Path::new(path).parent().unwrap()).unwrap();
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
        let process = python.interpreter.start().unwrap();
        process
            .write_frame(b"\x01\x00\x00\x00\x00\x00\x00\x00\xff")
            .unwrap();
        assert!(process.read_frame().is_err());
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
            .interpreter
            .close_within(Duration::from_millis(200))
            .expect("the close escalates instead of hanging");
    }
}
