// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fresh-process CPython calls and their completion protocol. The timing ends
//! when the reader receives the flushed line, not on a later poll or process
//! teardown (`crates/citysim/spec/Bench.lean` §8-6-1).

use std::io::BufRead as _;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use kernel::B3Hash;

use super::{Guest, Sample, fixture, prerequisite};

pub(super) const COLD_SAMPLES: usize = 5;
pub(crate) const COLD_COMMAND: &str = "python-cold";
pub(super) const COMPLETE_LINE: &str = "python-cold-complete\n";
const COLD_PATIENCE: Duration = Duration::from_secs(120);
const EXIT_COMPLETION_PATIENCE: Duration = Duration::from_secs(1);
const PROCESS_POLL: Duration = Duration::from_millis(10);

type Completion = Result<(String, Duration), String>;

pub(super) fn calls(guest: &Guest) -> Result<Vec<Sample>, String> {
    (0..COLD_SAMPLES)
        .map(|index| {
            call(guest.digest, &guest.path, &guest.stdlib.host).map(|took| Sample {
                took,
                round: index.saturating_add(1),
                metric: "latency",
            })
        })
        .collect()
}

#[expect(
    clippy::disallowed_methods,
    reason = "a developer bench spawns its own executable; it is not a city child"
)]
fn call(guest: B3Hash, snapshot: &Path, stdlib: &Path) -> Result<Duration, String> {
    let started = super::super::stamp();
    let executable = std::env::current_exe()
        .map_err(|why| cold_process("locate the cold benchmark executable", why.to_string()))?;
    let mut child = Command::new(executable)
        .arg(COLD_COMMAND)
        .arg(guest.to_string())
        .arg(snapshot)
        .arg(stdlib)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| cold_process("start the cold CPython benchmark child", why.to_string()))?;
    let Some(stdout) = child.stdout.take() else {
        return Err(stop_child(
            &mut child,
            "the cold child has no completion pipe".to_owned(),
        ));
    };
    let (send, received) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut line = String::new();
        let read = std::io::BufReader::new(stdout)
            .read_line(&mut line)
            .map(|_| {
                let took = started.elapsed();
                (line, took)
            })
            .map_err(|why| format!("read the cold child's completion: {why}"));
        send.send(read).map_err(|why| why.to_string())
    });
    match wait_for_cold(&mut child, received) {
        Err(why) => {
            // A failed kill can leave the reader blocked on a live child's
            // stdout. Returning without joining lets process exit reclaim it;
            // the error already says that the child may still be running.
            Err(why)
        }
        Ok((completed, status)) => {
            let read = reader
                .join()
                .map_err(|_| fixture("the cold completion reader panicked"))
                .and_then(|result| result);
            read?;
            let output = child.wait_with_output().map_err(|why| why.to_string())?;
            if !status.success() {
                return Err(prerequisite(format!(
                    "the cold CPython child exited with {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            Ok(completed)
        }
    }
}

fn wait_for_cold(
    child: &mut std::process::Child,
    received: mpsc::Receiver<Completion>,
) -> Result<(Duration, ExitStatus), String> {
    let started = super::super::stamp();
    let mut completed = None;
    loop {
        match received.try_recv() {
            Ok(message) => match completion_took(message) {
                Ok(took) => completed = Some(took),
                Err(why) => return Err(stop_child(child, why)),
            },
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) if completed.is_some() => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                return match child.try_wait() {
                    Ok(Some(status)) => Err(prerequisite(format!(
                        "the cold child exited with {status} without a completion record"
                    ))),
                    Ok(None) => Err(stop_child(
                        child,
                        "the cold child's completion pipe closed while it was running".to_owned(),
                    )),
                    Err(why) => Err(stop_child(child, why.to_string())),
                };
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if completed.is_none() {
                    match received.recv_timeout(EXIT_COMPLETION_PATIENCE) {
                        Ok(message) => {
                            completed = Some(completion_took(message)?);
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            return Err(prerequisite(format!(
                                "the cold child exited with {status} but supplied no completion record within {} ms",
                                EXIT_COMPLETION_PATIENCE.as_millis()
                            )));
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return Err(prerequisite(format!(
                                "the cold child exited with {status} and closed its completion pipe"
                            )));
                        }
                    }
                }
                let Some(completed) = completed else {
                    return Err(prerequisite(
                        "the cold child exited without a measurable completion".to_owned(),
                    ));
                };
                return Ok((completed, status));
            }
            Ok(None) => {}
            Err(why) => {
                return Err(stop_child(
                    child,
                    format!("wait for a cold CPython bench child: {why}"),
                ));
            }
        }
        if started.elapsed() >= COLD_PATIENCE {
            return Err(stop_child(
                child,
                format!(
                    "the cold CPython child did not finish within {} seconds",
                    COLD_PATIENCE.as_secs()
                ),
            ));
        }
        std::thread::sleep(PROCESS_POLL);
    }
}

fn completion_took(message: Completion) -> Result<Duration, String> {
    let (line, took) = message?;
    if line != COMPLETE_LINE {
        return Err(format!(
            "the cold child wrote an unexpected protocol line: {line:?}"
        ));
    }
    Ok(took)
}

fn stop_child(child: &mut std::process::Child, detail: String) -> String {
    let stopped = child
        .kill()
        .map_err(|why| format!("kill the cold CPython bench child: {why}"))
        .and_then(|()| {
            child
                .wait()
                .map_err(|why| format!("reap the cold CPython bench child: {why}"))
        });
    match stopped {
        Ok(_) => prerequisite(detail),
        Err(cleanup) => prerequisite(format!(
            "{detail}; {cleanup}; the child may still be running"
        )),
    }
}

fn cold_process(action: &str, detail: String) -> String {
    kernel::AxError::failure(
        kernel::AxCode::ToolUnavailable,
        action,
        detail,
    )
    .with_recovery(
        "check that the bench executable can start a child process, then rerun; no cold reading is produced",
    )
    .to_string()
}
