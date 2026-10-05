// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Independent cleanup on parent EOF (`crates/runtime/spec/Tools/Exec/Container.lean`, D52).

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use kernel::AxError;

use super::{cleanup::Cleanup, denied};

const ARGUMENT: &str = "__container-guardian";
const READY: &[u8] = b"ready\n";
const DONE: &[u8] = b"done\n";
const POLLS: usize = 200;
const INTERVAL: Duration = Duration::from_millis(10);
const RETRY: Duration = Duration::from_secs(5);

pub(super) struct Guard {
    child: Child,
}

impl Guard {
    pub(super) fn spawn(
        harness: &Path,
        cleanup: &Cleanup,
        ready: &Path,
        errors: std::fs::File,
    ) -> Result<Self, AxError> {
        let record = serde_json::to_string(cleanup)
            .map_err(|err| denied("encode container guardian ownership", err))?;
        let output = std::fs::File::create(ready)
            .map_err(|err| denied("prepare container guardian readiness", err))?;
        let child = Command::new(harness)
            .args([ARGUMENT, &record])
            .stdin(Stdio::piped())
            .stdout(output)
            .stderr(errors)
            .spawn()
            .map_err(|err| denied("start the container guardian", err))?;
        let mut guard = Self { child };
        let ready_budget = u64::try_from(READY.len().saturating_add(1))
            .map_err(|err| denied("read container guardian readiness", err))?;
        for _ in 0..POLLS {
            let mut bytes = Vec::with_capacity(READY.len().saturating_add(1));
            std::fs::File::open(ready)
                .and_then(|file| file.take(ready_budget).read_to_end(&mut bytes))
                .map_err(|err| denied("read container guardian readiness", err))?;
            if bytes == READY {
                return Ok(guard);
            }
            if let Some(status) = guard
                .child
                .try_wait()
                .map_err(|err| denied("poll container guardian readiness", err))?
            {
                return Err(denied(
                    "start the container guardian",
                    format!("helper ended before ready: {status}"),
                ));
            }
            std::thread::sleep(INTERVAL);
        }
        Err(denied(
            "start the container guardian",
            "helper did not confirm readiness within its poll budget",
        ))
    }

    pub(super) fn disarm(&mut self) -> Result<(), AxError> {
        if self
            .child
            .try_wait()
            .map_err(|err| denied("reap the container guardian", err))?
            .is_some()
        {
            return Ok(());
        }
        if let Some(mut input) = self.child.stdin.take() {
            input
                .write_all(DONE)
                .map_err(|err| denied("release the container guardian", err))?;
        }
        for _ in 0..POLLS {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|err| denied("reap the container guardian", err))?
            {
                return if status.success() {
                    Ok(())
                } else {
                    Err(denied("reap the container guardian", status))
                };
            }
            std::thread::sleep(INTERVAL);
        }
        Err(denied(
            "reap the container guardian",
            "helper remains responsible after the poll budget",
        ))
    }
}

/// Runs the private child protocol after the ordinary CLI grammar refuses it.
///
/// The caller supplies only argv from its own executable; models have no wire door to it.
///
/// # Errors
/// Returns malformed ownership and readiness failures before any target is created.
/// After parent EOF, removal failures retain the helper and retry until confirmed.
pub fn run_container_guard(args: &[String]) -> Option<Result<(), AxError>> {
    if args.first().map(String::as_str) != Some(ARGUMENT) {
        return None;
    }
    Some(run(args))
}

fn run(args: &[String]) -> Result<(), AxError> {
    let [_, record] = args else {
        return Err(denied(
            "read container guardian ownership",
            "expected one cleanup record",
        ));
    };
    let cleanup: Cleanup = serde_json::from_str(record)
        .map_err(|err| denied("read container guardian ownership", err))?;
    std::io::stdout()
        .lock()
        .write_all(READY)
        .map_err(|err| denied("confirm container guardian readiness", err))?;
    let mut answer = Vec::with_capacity(DONE.len());
    let length = u64::try_from(DONE.len()).map_err(|err| denied("read guardian release", err))?;
    let read = std::io::stdin()
        .lock()
        .take(length)
        .read_to_end(&mut answer);
    match read {
        Ok(_) => {
            if answer == DONE {
                return Ok(());
            }
        }
        Err(err) => {
            eprintln!("{err}; parent channel failed, guardian takes cleanup responsibility")
        }
    }
    loop {
        match cleanup.remove().and_then(|()| cleanup.release_copy()) {
            Ok(()) => return Ok(()),
            Err(err) => {
                eprintln!(
                    "{err}; container {} remains owned by its guardian",
                    cleanup.name
                );
                std::thread::sleep(RETRY);
            }
        }
    }
}
