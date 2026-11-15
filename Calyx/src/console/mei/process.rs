// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Child, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;

use super::super::ui::Show;
use super::MeiEvent;

/// One adapter failure: the stable code, the rule it broke, the exact
/// repair, and what failed — subject, full path and offending value.
pub(crate) struct AdapterFailure {
    pub(crate) code: &'static str,
    pub(crate) action: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) recovery: &'static str,
    pub(crate) subject: String,
    pub(crate) path: String,
    pub(crate) value: String,
}

/// Encodes text as a TOML string rather than Rust debug syntax, whose
/// control-character escapes are not valid TOML.
fn toml_string(text: &str) -> String {
    toml::Value::from(text).to_string()
}

impl AdapterFailure {
    /// The structured diagnostic the repository rule asks for: a `toml`
    /// fenced block carrying every field, ready for the terminal boundary.
    pub(crate) fn render(&self) -> String {
        let description = format!("{}: {}", self.action, self.subject);
        let cause = format!(
            "{}\npath = {}\nvalue = {}",
            self.rule, self.path, self.value
        );
        format!(
            "```toml\ncode = {}\ndescription = {}\ncause = {}\nresolution = {}\n```",
            toml_string(self.code),
            toml_string(&description),
            toml_string(&cause),
            toml_string(self.recovery),
        )
    }
}

struct Signals {
    done: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

/// One running child and the driver which reaps it. The two signals travel
/// together so Esc and the driver agree on the terminal state.
pub(crate) struct MeiProcess {
    #[cfg(unix)]
    pid: u32,
    #[cfg(windows)]
    job: Option<win32job::Job>,
    signals: Signals,
    driver: Option<std::thread::JoinHandle<()>>,
}

impl MeiProcess {
    #[cfg(test)]
    pub(crate) fn finished_for_test() -> MeiProcess {
        MeiProcess {
            #[cfg(unix)]
            pid: 0,
            #[cfg(windows)]
            job: None,
            signals: Signals {
                done: Arc::new(AtomicBool::new(true)),
                cancelled: Arc::new(AtomicBool::new(false)),
            },
            driver: None,
        }
    }

    pub(crate) fn spawn(
        prompt: &str,
        resume: Option<&Path>,
        to: SyncSender<Show>,
    ) -> Result<MeiProcess, String> {
        let program = mei_program();
        MeiProcess::spawn_with_program_and_resume(prompt, resume, to, &program)
    }

    #[cfg(test)]
    pub(crate) fn spawn_with_program(
        prompt: &str,
        to: SyncSender<Show>,
        program: &std::ffi::OsStr,
    ) -> Result<MeiProcess, String> {
        MeiProcess::spawn_with_program_and_resume(prompt, None, to, program)
    }

    pub(super) fn spawn_with_program_and_resume(
        prompt: &str,
        resume: Option<&Path>,
        to: SyncSender<Show>,
        program: &std::ffi::OsStr,
    ) -> Result<MeiProcess, String> {
        let mut command = mei_command(program, resume);
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                let program = program.to_string_lossy().into_owned();
                AdapterFailure {
                    code: "E_MEI_CONFIG",
                    action: "start Mei",
                    rule: "MEI_BIN must name an executable Mei binary",
                    recovery: "set MEI_BIN to the built binary, for example `MEI_BIN=target/debug/mei`, put `mei` beside `calyx`, or put `mei` on PATH",
                    subject: err.to_string(),
                    path: program.clone(),
                    value: program,
                }
                .render()
            })?;
        #[cfg(unix)]
        let pid = child.id();
        #[cfg(windows)]
        let job = match assign_job(&child) {
            Ok(job) => Some(job),
            Err(err) => {
                let mut child = child;
                if let Err(kill) = child.kill() {
                    return Err(format!("{err}; also failed to stop Mei: {kill}"));
                }
                if let Err(wait) = child.wait() {
                    return Err(format!("{err}; also failed to reap Mei: {wait}"));
                }
                return Err(err);
            }
        };
        let signals = Signals {
            done: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let driver_signals = Signals {
            done: Arc::clone(&signals.done),
            cancelled: Arc::clone(&signals.cancelled),
        };
        let prompt = prompt.to_owned();
        let driver = std::thread::spawn(move || {
            drive(child, prompt, to, &driver_signals);
        });
        Ok(MeiProcess {
            #[cfg(unix)]
            pid,
            #[cfg(windows)]
            job,
            signals,
            driver: Some(driver),
        })
    }

    /// Requests group termination without waiting for the UI's event loop.
    pub(crate) fn request_stop(&mut self) -> Result<(), String> {
        if !self.signals.done.swap(true, Ordering::SeqCst) {
            self.signals.cancelled.store(true, Ordering::SeqCst);
            self.terminate_tree()
        } else {
            Ok(())
        }
    }

    /// Stops the process tree, then joins the driver.
    #[cfg(test)]
    pub(crate) fn stop(&mut self) -> Result<(), String> {
        let stopped = self.request_stop();
        let reaped = self.reap();
        stopped.and(reaped)
    }

    #[cfg(unix)]
    fn terminate_tree(&mut self) -> Result<(), String> {
        stop_group(self.pid)
    }

    #[cfg(windows)]
    fn terminate_tree(&mut self) -> Result<(), String> {
        drop(self.job.take());
        Ok(())
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.driver
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    pub(crate) fn reap(&mut self) -> Result<(), String> {
        let Some(driver) = self.driver.take() else {
            return Ok(());
        };
        driver
            .join()
            .map_err(|_| "the Mei driver panicked".to_owned())
    }
}

fn mei_program() -> std::ffi::OsString {
    if let Some(bin) = std::env::var_os("MEI_BIN") {
        return bin;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let sibling = dir.join(if cfg!(windows) { "mei.exe" } else { "mei" });
        if sibling.is_file() {
            return sibling.into_os_string();
        }
    }
    std::ffi::OsString::from("mei")
}

pub(super) fn mei_command(
    program: &std::ffi::OsStr,
    resume: Option<&Path>,
) -> std::process::Command {
    let mut command = child::command(program);
    if let Some(path) = resume {
        command.arg("--resume").arg(path);
    }
    command
}

impl Drop for MeiProcess {
    fn drop(&mut self) {
        if let Err(err) = self.request_stop() {
            eprintln!("stop Mei process group: {err}");
        }
        #[cfg(windows)]
        {
            drop(self.job.take());
        }
        if let Err(err) = self.reap() {
            eprintln!("reap Mei process group: {err}");
        }
    }
}

fn drive(mut child: Child, prompt: String, to: SyncSender<Show>, signals: &Signals) {
    let result = drive_output(&mut child, &prompt, &to);
    let (status, answer) = match result {
        Ok((status, answer)) => (status, answer),
        Err(error) => {
            if let Err(stop) = stop_after_io_failure(&mut child) {
                eprintln!("stop Mei after an I/O failure: {stop}");
            }
            if let Err(wait) = child.wait() {
                eprintln!("reap Mei after an I/O failure: {wait}");
            }
            (None, Err(error))
        }
    };
    let result = match (status, answer) {
        (Some(status), Ok(answer)) if status.success() => Ok(answer),
        (Some(status), Ok(_)) => Err(format!("Mei exited with {status}")),
        (_, Err(error)) => Err(error),
        (None, Ok(answer)) => Ok(answer),
    };
    signals.done.store(true, Ordering::SeqCst);
    drop(to.send(Show::Mei(MeiEvent::Finished {
        result,
        cancelled: signals.cancelled.load(Ordering::SeqCst),
    })));
}

fn drive_output(
    child: &mut Child,
    prompt: &str,
    to: &SyncSender<Show>,
) -> Result<(Option<ExitStatus>, Result<String, String>), String> {
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| "open Mei stdin".to_owned())?;
    input
        .write_all(prompt.as_bytes())
        .map_err(|err| format!("write the prompt: {err}"))?;
    drop(input);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "open Mei stdout".to_owned())?;
    let reader = std::thread::spawn(move || -> Result<String, String> {
        let mut bytes = Vec::new();
        let mut stdout = stdout;
        stdout
            .read_to_end(&mut bytes)
            .map_err(|err| format!("read Mei stdout: {err}"))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    });
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "open Mei stderr".to_owned())?;
    for line in std::io::BufReader::new(stderr).lines() {
        let line = line.map_err(|err| format!("read Mei stderr: {err}"))?;
        if to.send(Show::Mei(MeiEvent::Line(line))).is_err() {
            return Err("the console stopped listening".to_owned());
        }
    }
    let status = child.wait().map_err(|err| format!("wait for Mei: {err}"))?;
    let answer = reader
        .join()
        .map_err(|_| "the stdout reader panicked".to_owned())?;
    Ok((Some(status), answer))
}

#[cfg(unix)]
fn stop_after_io_failure(child: &mut Child) -> Result<(), String> {
    stop_group(child.id())
}

#[cfg(windows)]
fn stop_after_io_failure(child: &mut Child) -> Result<(), String> {
    child.kill().map_err(|err| err.to_string())
}

#[cfg(unix)]
fn stop_group(pid: u32) -> Result<(), String> {
    let pid = i32::try_from(pid).map_err(|_| "invalid Mei process id".to_owned())?;
    let pid =
        rustix::process::Pid::from_raw(pid).ok_or_else(|| "invalid Mei process id".to_owned())?;
    rustix::process::kill_process_group(pid, rustix::process::Signal::KILL)
        .map_err(|err| format!("terminate Mei process group {pid}: {err}"))
}

#[cfg(windows)]
fn assign_job(child: &Child) -> Result<win32job::Job, String> {
    use std::os::windows::io::AsRawHandle as _;

    let mut limits = win32job::ExtendedLimitInfo::new();
    limits.limit_kill_on_job_close();
    let job = win32job::Job::create_with_limit_info(&limits)
        .map_err(|err| format!("create Mei job: {err}"))?;
    let handle = isize::try_from(child.as_raw_handle().addr())
        .map_err(|err| format!("Mei process handle: {err}"))?;
    job.assign_process(handle)
        .map_err(|err| format!("assign Mei to job: {err}"))?;
    Ok(job)
}
