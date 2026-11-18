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

use super::{Paths, Program, failure};
use crate::error::Result;

pub fn uv(paths: &Paths) -> Result<Program> {
    std::fs::create_dir_all(&paths.bin).map_err(|err| {
        failure(
            "create Mei's uv directory",
            err.to_string(),
            "keep the private doctor directory writable",
            Some(&paths.bin),
        )
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
                    "$ErrorActionPreference = 'Stop'; Invoke-RestMethod -Uri 'https://astral.sh/uv/install.ps1' -OutFile $env:MEI_UV_INSTALL_SCRIPT; & $env:MEI_UV_INSTALL_SCRIPT",
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
                    "if command -v curl >/dev/null 2>&1; then curl --fail --silent --show-error --location --connect-timeout 10 --max-time 300 --output \"$MEI_UV_INSTALL_SCRIPT\" https://astral.sh/uv/install.sh; elif command -v wget >/dev/null 2>&1; then wget -q -O \"$MEI_UV_INSTALL_SCRIPT\" https://astral.sh/uv/install.sh; else echo 'curl or wget is required' >&2; exit 127; fi && sh \"$MEI_UV_INSTALL_SCRIPT\"",
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
            "remove the private uv directory and retry with network access to astral.sh",
            Some(&executable),
        )
    })
}

pub fn python(paths: &Paths, uv: &Program) -> Result<Program> {
    let mut command = Command::new(&uv.executable);
    command
        .args(["--no-config", "--no-progress", "python", "install"])
        .current_dir(&paths.root)
        .env("UV_PYTHON_INSTALL_DIR", &paths.python)
        .env("UV_PYTHON_BIN_DIR", &paths.python_bin)
        .env("UV_CACHE_DIR", &paths.uv_cache);
    run(
        &mut command,
        "install the latest stable Python through uv",
        &uv.executable,
    )?;

    let output = run(
        Command::new(&uv.executable)
            .args([
                "--no-config",
                "--no-progress",
                "python",
                "find",
                "--managed-python",
            ])
            .current_dir(&paths.root)
            .env("UV_PYTHON_INSTALL_DIR", &paths.python)
            .env("UV_PYTHON_BIN_DIR", &paths.python_bin)
            .env("UV_CACHE_DIR", &paths.uv_cache),
        "locate the installed Python",
        &uv.executable,
    )?;
    let text = String::from_utf8(output.stdout).map_err(|err| {
        failure(
            "read the installed Python path",
            err.to_string(),
            "install into a UTF-8 doctor directory or set MEI_DOCTOR_DIR to one",
            Some(&paths.root),
        )
    })?;
    let path = text.trim();
    if path.is_empty() {
        return Err(failure(
            "locate the installed Python",
            "uv returned an empty path",
            "remove the incomplete private Python installation and retry",
            Some(&paths.root),
        ));
    }
    super::probe_python(Path::new(path).as_os_str())?.ok_or_else(|| {
        failure(
            "verify the installed Python",
            "the managed interpreter did not return its version and executable",
            "remove the incomplete private Python installation and retry",
            Some(Path::new(path)),
        )
    })
}

fn run(command: &mut Command, action: &str, subject: &Path) -> Result<Output> {
    let output = command.output().map_err(|err| {
        failure(
            action,
            err.to_string(),
            "keep the installer available and retry with network access",
            Some(subject),
        )
    })?;
    if output.status.success() {
        return Ok(output);
    }
    Err(failure(
        action,
        format!(
            "exit status {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        "repair network access or the named installer, then start Mei again; no successful cache was written",
        Some(subject),
    ))
}
