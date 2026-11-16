// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::BufRead;
use std::process::{Child, Command, ExitCode};

fn main() -> ExitCode {
    match serve(std::env::args_os().skip(1).collect()) {
        Ok(code) => code,
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
enum Failure {

    Usage,

    Start(std::io::Error),

    Close(String),

    Wait(std::io::Error),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => write!(
                f,
                "E_LAUNCHER_USAGE: cannot serve without a program\nrecovery: run serve_grouped <sprawling> serve <city> <address> --no-console"
            ),
            Self::Start(error) => write!(
                f,
                "E_LAUNCHER_START: cannot start the city in a group of its own: {error}\nrecovery: check that the named binary exists and can run"
            ),
            Self::Close(why) => write!(
                f,
                "E_LAUNCHER_CLOSE: cannot ask the city to close in order: {why}\nrecovery: the walk terminates the launcher, and the city ends with it"
            ),
            Self::Wait(error) => write!(
                f,
                "E_LAUNCHER_WAIT: cannot wait for the city: {error}\nrecovery: terminate the launcher; the city ends with it"
            ),
        }
    }
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn serve(arguments: Vec<std::ffi::OsString>) -> Result<ExitCode, Failure> {
    let (program, rest) = arguments.split_first().ok_or(Failure::Usage)?;
    let (mut city, _group) = grouped::start(Command::new(program).args(rest))?;
    let asked = await_request(&mut city);
    let status = city.wait().map_err(Failure::Wait)?;
    asked?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn await_request(city: &mut Child) -> Result<(), Failure> {
    for line in std::io::stdin().lock().lines() {
        if line.map_err(Failure::Wait)?.trim() == "close" {
            return grouped::close(city);
        }
    }
    city.kill().map_err(Failure::Wait)
}

#[cfg(windows)]
mod grouped {
    use super::Failure;
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command};

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    pub(super) fn start(command: &mut Command) -> Result<(Child, win32job::Job), Failure> {
        let city = command
            .creation_flags(CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(Failure::Start)?;
        let mut limits = win32job::ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        let job = win32job::Job::create_with_limit_info(&limits)
            .map_err(|error| Failure::Start(error.into()))?;
        let handle = isize::try_from(city.as_raw_handle().addr())
            .map_err(|error| Failure::Start(std::io::Error::other(error)))?;
        job.assign_process(handle)
            .map_err(|error| Failure::Start(error.into()))?;
        Ok((city, job))
    }

    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    pub(super) fn close(city: &Child) -> Result<(), Failure> {
        let id = city.id();
        let script = format!(
            "Add-Type -Namespace Grouped -Name Console -MemberDefinition '[DllImport(\"kernel32.dll\")] public static extern bool GenerateConsoleCtrlEvent(uint e, uint g);'; if (-not [Grouped.Console]::GenerateConsoleCtrlEvent(1, {id})) {{ exit 1 }}"
        );
        let sent = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .map_err(|error| Failure::Close(error.to_string()))?;
        if sent.success() {
            Ok(())
        } else {
            Err(Failure::Close(format!(
                "Ctrl-Break to group {id} was refused ({sent}); the launcher may have no console"
            )))
        }
    }
}

#[cfg(not(windows))]
mod grouped {
    use super::Failure;
    use std::process::{Child, Command};

    pub(super) fn start(command: &mut Command) -> Result<(Child, ()), Failure> {
        command
            .spawn()
            .map(|city| (city, ()))
            .map_err(Failure::Start)
    }

    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    pub(super) fn close(city: &Child) -> Result<(), Failure> {
        let id = city.id();
        let sent = Command::new("kill")
            .args(["-s", "INT", &id.to_string()])
            .status()
            .map_err(|error| Failure::Close(error.to_string()))?;
        if sent.success() {
            Ok(())
        } else {
            Err(Failure::Close(format!("kill -s INT {id} refused ({sent})")))
        }
    }
}
