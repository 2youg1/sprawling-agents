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

use super::{
    cleanup::{Cleanup, Confirmation},
    control, denied,
};
use serde::{Deserialize, Serialize};

const ARGUMENT: &str = "__container-guardian";
const READY: &[u8] = b"ready\n";
const CREATE: &[u8] = b"create\n";
const DONE: &[u8] = b"done\n";
const POLLS: usize = 200;
const INTERVAL: Duration = Duration::from_millis(10);
const RETRY: Duration = Duration::from_secs(5);

pub(super) struct Launch<'a> {
    pub(super) cleanup: &'a Cleanup,
    pub(super) command: &'a Command,
}

#[derive(Serialize, Deserialize)]
struct Ownership {
    #[serde(flatten)]
    cleanup: Cleanup,
    arguments: Vec<std::ffi::OsString>,
    answer: std::path::PathBuf,
}

pub(super) struct Guard {
    child: Child,
    answer: std::path::PathBuf,
}

impl Guard {
    pub(super) fn spawn(
        harness: &Path,
        launch: Launch<'_>,
        ready: &Path,
        errors: std::fs::File,
    ) -> Result<Self, AxError> {
        let answer = ready.with_extension("created");
        std::fs::File::create(&answer)
            .map_err(|err| denied("prepare container creation response", err))?;
        let record = serde_json::to_vec(&Ownership {
            cleanup: launch.cleanup.clone(),
            arguments: launch
                .command
                .get_args()
                .map(std::ffi::OsStr::to_os_string)
                .collect(),
            answer: answer.clone(),
        })
        .map_err(|err| denied("encode container guardian ownership", err))?;
        let ownership = ready.with_extension("ownership");
        std::fs::write(&ownership, record)
            .map_err(|err| denied("keep container guardian ownership", err))?;
        let output = std::fs::File::create(ready)
            .map_err(|err| denied("prepare container guardian readiness", err))?;
        let child = Command::new(harness)
            .arg(ARGUMENT)
            .arg(ownership)
            .stdin(Stdio::piped())
            .stdout(output)
            .stderr(errors)
            .spawn()
            .map_err(|err| denied("start the container guardian", err))?;
        let mut guard = Self { child, answer };
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

    pub(super) fn create(&mut self) -> Result<(), AxError> {
        self.child
            .stdin
            .as_mut()
            .ok_or_else(|| denied("request container creation", "guardian channel closed"))?
            .write_all(CREATE)
            .map_err(|err| denied("request container creation", err))?;
        for _ in 0..control::POLLS {
            let bytes = std::fs::read(&self.answer)
                .map_err(|err| denied("read container creation response", err))?;
            if bytes.last() == Some(&b'\n') {
                return serde_json::from_slice::<Result<(), AxError>>(&bytes)
                    .map_err(|err| denied("decode container creation response", err))?;
            }
            std::thread::sleep(control::INTERVAL);
        }
        Err(denied(
            "wait for container creation",
            "guardian retains the in-flight request",
        ))
    }

    pub(super) fn finish_cleanup(&mut self) -> Result<(), AxError> {
        drop(self.child.stdin.take());
        match self
            .child
            .try_wait()
            .map_err(|err| denied("reap the container guardian", err))?
        {
            Some(status) if status.success() => Ok(()),
            Some(status) => Err(denied("reap the container guardian", status)),
            None => Err(denied(
                "wait for guardian cleanup",
                "guardian still owns creation and removal",
            )),
        }
    }

    pub(super) fn disarm(&mut self) -> Result<(), AxError> {
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
    let bytes =
        std::fs::read(record).map_err(|err| denied("read container guardian ownership", err))?;
    let ownership: Ownership = serde_json::from_slice(&bytes)
        .map_err(|err| denied("decode container guardian ownership", err))?;
    std::io::stdout()
        .lock()
        .write_all(READY)
        .map_err(|err| denied("confirm container guardian readiness", err))?;
    let mut input = std::io::stdin().lock();
    let mut confirmation = Confirmation::Absence;
    let request = read_command(&mut input);
    match request {
        Ok(answer) if answer == CREATE => {
            let mut create = Command::new(&ownership.cleanup.program);
            create.args(&ownership.arguments);
            let created =
                control::output(&mut create, control::Wait::Creation).and_then(|answer| {
                    if answer.status.success() {
                        Ok(())
                    } else {
                        Err(denied(
                            "create the owned container",
                            String::from_utf8_lossy(&answer.stderr),
                        ))
                    }
                });
            if created.is_err() {
                confirmation = Confirmation::Deletion;
            }
            let published = serde_json::to_vec(&created)
                .map_err(|err| denied("encode container creation response", err))
                .and_then(|mut bytes| {
                    bytes.push(b'\n');
                    std::fs::write(&ownership.answer, bytes)
                        .map_err(|err| denied("publish container creation response", err))
                });
            if let Err(err) = published {
                eprintln!("{err}; guardian takes cleanup responsibility");
            } else {
                match read_command(&mut input) {
                    Ok(answer) if answer == DONE && created.is_ok() => return Ok(()),
                    Ok(_) => {}
                    Err(err) => eprintln!("{err}; guardian takes cleanup responsibility"),
                }
            }
        }
        Ok(answer) if answer == DONE => return Ok(()),
        Ok(_) => {}
        Err(err) => eprintln!("{err}; guardian takes cleanup responsibility"),
    }
    loop {
        match ownership.cleanup.remove(confirmation).and_then(|()| {
            confirmation = Confirmation::Absence;
            ownership.cleanup.release_copy()
        }) {
            Ok(()) => return Ok(()),
            Err(err) => {
                eprintln!(
                    "{err}; container {} remains owned by its guardian",
                    ownership.cleanup.name
                );
                std::thread::sleep(RETRY);
            }
        }
    }
}

fn read_command(input: &mut impl Read) -> Result<Vec<u8>, AxError> {
    let mut answer = Vec::with_capacity(CREATE.len());
    for _ in 0..CREATE.len() {
        let mut byte = [0_u8; 1];
        let count = input
            .read(&mut byte)
            .map_err(|err| denied("read guardian command", err))?;
        if count == 0 {
            break;
        }
        answer.extend_from_slice(&byte);
        if byte == [b'\n'] {
            break;
        }
    }
    Ok(answer)
}
