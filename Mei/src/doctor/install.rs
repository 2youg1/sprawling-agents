// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::disallowed_methods,
    reason = "Mei doctor starts its own installers outside city child management"
)]

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use super::{Paths, Program, failure};
use crate::error::{Result, remove_file_instruction, remove_tree_instruction};

pub(super) const BUDGET: Duration = Duration::from_secs(900);

const BUDGET_SECONDS: u64 = 900;
const CONNECT_TIMEOUT_SECONDS: u64 = 10;

pub(super) struct Deadline {
    at: Instant,
}

impl Deadline {
    pub(super) fn start() -> Result<Deadline> {
        Deadline::after(BUDGET)
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "Mei's installer adapter samples monotonic time for its process deadline"
    )]
    pub(super) fn after(duration: Duration) -> Result<Deadline> {
        let at = Instant::now().checked_add(duration).ok_or_else(|| {
            failure(
                "set the shared repair deadline",
                format!("the duration {duration:?} is not representable"),
                "use a finite repair deadline",
            )
        })?;
        Ok(Deadline { at })
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "Mei's installer adapter reads its monotonic deadline"
    )]
    fn remaining(&self) -> Duration {
        self.at.saturating_duration_since(Instant::now())
    }
}

pub fn uv(paths: &Paths, deadline: &Deadline) -> Result<Program> {
    std::fs::create_dir_all(&paths.bin).map_err(|err| {
        failure(
            "create Mei's uv directory",
            err.to_string(),
            format!("keep it writable: {}", remove_tree_instruction(&paths.bin)),
        )
        .at(&paths.bin)
    })?;
    let script = paths.root.join(if cfg!(windows) {
        "uv-install.ps1"
    } else {
        "uv-install.sh"
    });
    if cfg!(windows) {
        run(
            deadline,
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &format!(
                        concat!(
                            "$ErrorActionPreference = 'Stop'; ",
                            "Invoke-RestMethod -TimeoutSec {0} -Uri 'https://astral.sh/uv/install.ps1' ",
                            "-OutFile $env:MEI_UV_INSTALL_SCRIPT; & $env:MEI_UV_INSTALL_SCRIPT"
                        ),
                        BUDGET_SECONDS
                    ),
                ])
                .env("MEI_UV_INSTALL_SCRIPT", &script)
                .env("UV_UNMANAGED_INSTALL", &paths.bin),
            "install uv with the official PowerShell installer",
            &script,
        )?;
    } else {
        run(
            deadline,
            Command::new("sh")
                .args([
                    "-c",
                    &format!(
                        concat!(
                            "if command -v curl >/dev/null 2>&1; then ",
                            "curl --fail --silent --show-error --location ",
                            "--connect-timeout {0} --max-time {1} ",
                            "--output \"$MEI_UV_INSTALL_SCRIPT\" https://astral.sh/uv/install.sh; ",
                            "elif command -v wget >/dev/null 2>&1; then ",
                            "wget --tries=1 --timeout={1} -q -O \"$MEI_UV_INSTALL_SCRIPT\" https://astral.sh/uv/install.sh; ",
                            "else echo 'curl or wget is required' >&2; exit 127; fi && sh \"$MEI_UV_INSTALL_SCRIPT\""
                        ),
                        CONNECT_TIMEOUT_SECONDS, BUDGET_SECONDS
                    ),
                ])
                .env("MEI_UV_INSTALL_SCRIPT", &script)
                .env("UV_UNMANAGED_INSTALL", &paths.bin),
            "install uv with the official shell installer",
            &script,
        )?;
    }
    let executable = paths.bin.join(if cfg!(windows) { "uv.exe" } else { "uv" });
    super::probe_uv(&executable)?.ok_or_else(|| {
        failure(
            "verify the installed uv",
            "the private uv executable did not return a numeric version",
            format!(
                "remove the half-installed tool and retry: {}",
                remove_file_instruction(&executable)
            ),
        )
        .at(&executable)
    })
}

pub fn python(paths: &Paths, uv: &Program, deadline: &Deadline) -> Result<Program> {
    for reinstall in [false, true] {
        run(
            deadline,
            &mut python_install_command(paths, uv, reinstall),
            "install the latest stable Python through uv",
            &uv.executable,
        )?;

        let output = run(
            deadline,
            &mut python_find_command(paths, uv),
            "locate the installed Python",
            &uv.executable,
        )?;
        let text = String::from_utf8(output.stdout).map_err(|err| {
            failure(
                "read the installed Python path",
                err.to_string(),
                "install into a UTF-8 doctor directory or set MEI_DOCTOR_DIR to one",
            )
            .at(&paths.root)
        })?;
        let path = text.trim();
        if path.is_empty() {
            return Err(failure(
                "locate the installed Python",
                "uv returned an empty path",
                format!(
                    "remove the incomplete private installation and retry: {}",
                    remove_tree_instruction(&paths.python)
                ),
            )
            .at(&paths.root));
        }
        if let Some(program) = super::probe_python(Path::new(path).as_os_str())? {
            return Ok(program);
        }
    }
    Err(failure(
        "verify the installed Python after one reinstall",
        "the managed interpreter did not return its version and executable",
        format!(
            "remove the incomplete private installation and retry: {}",
            remove_tree_instruction(&paths.python)
        ),
    )
    .at(&paths.python))
}

pub(super) fn python_install_command(paths: &Paths, uv: &Program, reinstall: bool) -> Command {
    let mut command = uv_command(paths, uv);
    command
        .args(["python", "install"])
        .env("UV_HTTP_TIMEOUT", BUDGET_SECONDS.to_string());
    if cfg!(windows) {
        command.arg("--no-registry");
    }
    if reinstall {
        command.arg("--reinstall");
    }
    command
}

pub(super) fn python_find_command(paths: &Paths, uv: &Program) -> Command {
    let mut command = uv_command(paths, uv);
    command.args(["python", "find", "--managed-python", "--no-project"]);
    command
}

fn uv_command(paths: &Paths, uv: &Program) -> Command {
    let mut command = Command::new(&uv.executable);
    command
        .args(["--no-config", "--no-progress"])
        .current_dir(&paths.root)
        .env("UV_PYTHON_INSTALL_DIR", &paths.python)
        .env("UV_PYTHON_BIN_DIR", &paths.python_bin)
        .env("UV_CACHE_DIR", &paths.uv_cache)
        .env_remove("UV_PYTHON");
    command
}

pub(super) fn run(
    deadline: &Deadline,
    command: &mut Command,
    action: &str,
    subject: &Path,
) -> Result<Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| {
        failure(
            action,
            err.to_string(),
            "keep the installer available and retry with network access",
        )
        .at(subject)
    })?;
    let stdout = child.stdout.take().map(read_pipe);
    let stderr = child.stderr.take().map(read_pipe);
    let status = match crate::child_exit::wait(&mut child, deadline.remaining()) {
        Ok(Some(status)) => status,
        Ok(None) => {
            stop(&mut child, action, subject)?;
            return Err(failure(
                action,
                format!("the shared repair deadline of {BUDGET:?} elapsed"),
                "repair network access or the named installer, then start Mei again; no successful cache was written",
            )
            .at(subject));
        }
        Err(err) => {
            stop(&mut child, action, subject)?;
            return Err(failure(
                action,
                err.to_string(),
                "inspect the installer process and retry",
            )
            .at(subject));
        }
    };
    let stdout = match stdout {
        Some(reader) => receive_pipe(reader, deadline, action, subject)?,
        None => Vec::new(),
    };
    let stderr = match stderr {
        Some(reader) => receive_pipe(reader, deadline, action, subject)?,
        None => Vec::new(),
    };
    check_status(
        Output {
            status,
            stdout,
            stderr,
        },
        action,
        subject,
    )
}

fn read_pipe(
    mut pipe: impl Read + Send + 'static,
) -> std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
        drop(sender.send(result));
    });
    receiver
}

fn receive_pipe(
    reader: std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
    deadline: &Deadline,
    action: &str,
    subject: &Path,
) -> Result<Vec<u8>> {
    let remaining = deadline.remaining();
    if remaining.is_zero() {
        return Err(failure(
            action,
            format!(
                "the shared repair deadline of {BUDGET:?} elapsed while reading installer output"
            ),
            "repair the installer and retry; no successful cache was written",
        )
        .at(subject));
    }
    reader
        .recv_timeout(remaining)
        .map_err(|err| {
            failure(
                action,
                format!("installer output did not arrive before the shared deadline: {err}"),
                "repair the installer and retry; no successful cache was written",
            )
            .at(subject)
        })?
        .map_err(|err| {
            failure(
                action,
                err.to_string(),
                "keep the installer output channel readable",
            )
            .at(subject)
        })
}

fn stop(child: &mut std::process::Child, action: &str, subject: &Path) -> Result<()> {
    use crate::child_exit::TerminateError;

    crate::child_exit::terminate(child).map_err(|error| {
        let (stage, err) = match error {
            TerminateError::Kill(err) => ("kill the installer", err),
            TerminateError::Wait(err) => ("wait for the killed installer", err),
        };
        failure(
            action,
            format!("{stage}: {err}"),
            "reap the installer before retrying",
        )
        .at(subject)
    })
}

fn check_status(output: Output, action: &str, subject: &Path) -> Result<Output> {
    if output.status.success() {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(failure(
        action,
        format!("exit status {}", output.status),
        format!(
            "repair network access or the named installer, then start Mei again; no successful cache was written: {}",
            remove_file_instruction(subject)
        ),
    )
    .at_with_value(subject, stderr.trim()))
}
