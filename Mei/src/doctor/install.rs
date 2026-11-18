// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::disallowed_methods,
    reason = "Mei doctor starts its own installers outside city child management"
)]

use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

use super::{Paths, Program, failure};
use crate::error::{Result, remove_file_command, remove_tree_command};

/// The wall-clock budget one installer may spend.
///
/// The doctor lock is held for the whole repair, so this is also what a second
/// Mei may legitimately wait: `lock::LOCK_WAIT` reads this value, and a change
/// here moves that ceiling with it.
pub(super) const BUDGET: Duration = Duration::from_secs(900);

const BUDGET_SECONDS: u64 = 900;
const CONNECT_TIMEOUT_SECONDS: u64 = 10;

pub fn uv(paths: &Paths) -> Result<Program> {
    std::fs::create_dir_all(&paths.bin).map_err(|err| {
        failure(
            "create Mei's uv directory",
            err.to_string(),
            format!("keep it writable: {}", remove_tree_command(&paths.bin)),
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
                remove_file_command(&executable)
            ),
        )
        .at(&executable)
    })
}

pub fn python(paths: &Paths, uv: &Program) -> Result<Program> {
    let installed = uv_command(paths, uv)
        .args(["python", "install"])
        .env("UV_HTTP_TIMEOUT", BUDGET_SECONDS.to_string())
        .output();
    run_output(
        installed,
        "install the latest stable Python through uv",
        &uv.executable,
    )?;

    let found = uv_command(paths, uv)
        .args(["python", "find", "--managed-python"])
        .output();
    let output = run_output(found, "locate the installed Python", &uv.executable)?;
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
                remove_tree_command(&paths.python)
            ),
        )
        .at(&paths.root));
    }
    super::probe_python(Path::new(path).as_os_str())?.ok_or_else(|| {
        failure(
            "verify the installed Python",
            "the managed interpreter did not return its version and executable",
            format!(
                "remove the incomplete private installation and retry: {}",
                remove_tree_command(&paths.python)
            ),
        )
        .at_with_value(Path::new(path), path)
    })
}

fn uv_command(paths: &Paths, uv: &Program) -> Command {
    let mut command = Command::new(&uv.executable);
    command
        .args(["--no-config", "--no-progress"])
        .current_dir(&paths.root)
        .env("UV_PYTHON_INSTALL_DIR", &paths.python)
        .env("UV_PYTHON_BIN_DIR", &paths.python_bin)
        .env("UV_CACHE_DIR", &paths.uv_cache);
    command
}

fn run(command: &mut Command, action: &str, subject: &Path) -> Result<Output> {
    run_output(command.output(), action, subject)
}

fn run_output(output: std::io::Result<Output>, action: &str, subject: &Path) -> Result<Output> {
    let output = output.map_err(|err| {
        failure(
            action,
            err.to_string(),
            "keep the installer available and retry with network access",
        )
        .at(subject)
    })?;
    if output.status.success() {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(failure(
        action,
        format!("exit status {}", output.status),
        format!(
            "repair network access or the named installer, then start Mei again; no successful cache was written: {}",
            remove_file_command(subject)
        ),
    )
    .at_with_value(subject, stderr.trim()))
}
