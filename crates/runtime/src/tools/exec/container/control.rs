// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Bounded daemon requests (`crates/runtime/spec/Tools/Exec/Container.lean`).

use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use kernel::AxError;

use super::denied;

const POLLS: u32 = 250;
const INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);
const MAX_OUTPUT: u64 = 1 << 20;
static REQUESTS: AtomicU64 = AtomicU64::new(0);

pub(super) fn output(command: &mut Command) -> Result<Output, AxError> {
    let files = Files::open()?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            std::fs::File::create(files.0.join("out"))
                .map_err(|err| denied("keep daemon output", err.to_string()))?,
        ))
        .stderr(Stdio::from(
            std::fs::File::create(files.0.join("err"))
                .map_err(|err| denied("keep daemon errors", err.to_string()))?,
        ));
    let mut child = command
        .spawn()
        .map_err(|err| denied("contact the container daemon", err.to_string()))?;
    for _ in 0..POLLS {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(Output {
                    status,
                    stdout: files.read("out")?,
                    stderr: files.read("err")?,
                });
            }
            Ok(None) => std::thread::sleep(INTERVAL),
            Err(err) => {
                stop(&mut child)?;
                return Err(denied("wait for the container daemon", err.to_string()));
            }
        }
    }
    stop(&mut child)?;
    Err(denied(
        "contact the container daemon",
        "the bounded daemon request timed out",
    ))
}

pub(super) fn checked(command: &mut Command) -> Result<Vec<u8>, AxError> {
    let answer = output(command)?;
    if !answer.status.success() {
        return Err(denied(
            "contact the container daemon",
            format!(
                "{}: {}",
                answer.status,
                String::from_utf8_lossy(&answer.stderr).trim()
            ),
        ));
    }
    Ok(answer.stdout)
}

fn stop(child: &mut std::process::Child) -> Result<(), AxError> {
    child
        .kill()
        .map_err(|err| denied("stop the daemon client", err.to_string()))?;
    child
        .wait()
        .map_err(|err| denied("reap the daemon client", err.to_string()))?;
    Ok(())
}

struct Files(std::path::PathBuf);

impl Files {
    fn open() -> Result<Self, AxError> {
        let dir = std::env::temp_dir().join(format!(
            "sprawling-container-request-{}-{}",
            std::process::id(),
            REQUESTS.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).map_err(|err| denied("keep daemon output", err.to_string()))?;
        Ok(Self(dir))
    }

    fn read(&self, name: &str) -> Result<Vec<u8>, AxError> {
        use std::io::Read;
        let file = std::fs::File::open(self.0.join(name))
            .map_err(|err| denied("read daemon output", err.to_string()))?;
        if file
            .metadata()
            .map_err(|err| denied("read daemon output", err.to_string()))?
            .len()
            > MAX_OUTPUT
        {
            return Err(denied(
                "read daemon output",
                "daemon response exceeds the bounded output size",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_OUTPUT)
            .read_to_end(&mut bytes)
            .map_err(|err| denied("read daemon output", err.to_string()))?;
        Ok(bytes)
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_dir_all(&self.0) {
            eprintln!("container request output cleanup failed: {err}");
        }
    }
}
