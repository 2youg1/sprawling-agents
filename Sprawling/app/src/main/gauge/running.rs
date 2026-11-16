// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use kernel::{AxCode, AxError};
use sprawling::audience::Audience;
use sprawling::monitor::spread::Spread;
use sprawling::monitor::tree::Tree;
use sprawling::serving::standing::monotonic_now;

use super::{ChildPeaks, Every, Host, Run, Running, Watched, emit, lines};

pub(super) fn run(
    running: &Running,
    audience: Audience,
    out: &mut impl Write,
) -> Result<(), AxError> {
    let host = Host::here();
    let mut run_and_tell = |index| -> Result<(Duration, u32), AxError> {
        let run = run_once(running, index)?;
        emit(out, &lines::run_line(&run, audience))?;
        Ok((run.wall, u32::from(run.exit != Some(0))))
    };
    let (head, mut failed) = run_and_tell(0)?;
    let mut tail = Vec::new();
    for index in 1..running.samples.get() {
        let (wall, failure) = run_and_tell(index)?;
        tail.push(wall);
        failed = failed.saturating_add(failure);
    }
    let spread = Spread::of(head, tail);
    emit(out, &lines::spread_line(&spread, failed, host, audience))
}

#[expect(
    clippy::disallowed_methods,
    reason = "gauge runs the person's command in the foreground of the person's terminal, where Ctrl+C must reach it (child D4)"
)]
fn run_once(running: &Running, index: u32) -> Result<Run, AxError> {
    let began = monotonic_now();
    let mut child = Command::new(&running.program)
        .args(&running.args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(std::io::stderr()))
        .stderr(Stdio::from(std::io::stderr()))
        .spawn()
        .map_err(|err| not_started(&running.program, &err))?;
    let beating = match Beating::start(child.id(), running.every) {
        Ok(beating) => beating,
        Err(err) => {

            drop(child.kill().and_then(|()| child.wait()));
            return Err(err);
        }
    };
    let status = child.wait();
    let wall = monotonic_now().saturating_duration_since(began);
    let peaks = child_peaks(&child);
    let watched = beating.stop();
    let status = status.map_err(|err| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "wait for the measured program",
            format!("{}: {err}", running.program.to_string_lossy()),
        )
        .with_recovery("run the measurement again")
    })?;
    Ok(Run {
        index,
        exit: status.code(),
        wall,
        watched,
        child: peaks,
    })
}

struct Beating {
    stop: mpsc::Sender<()>,
    thread: std::thread::JoinHandle<Watched>,
}

impl Beating {
    fn start(root: u32, every: Every) -> Result<Beating, AxError> {
        let (stop, stopped) = mpsc::channel();
        std::thread::Builder::new()
            .name("sprawling-gauge".to_owned())
            .spawn(move || beat_until_stopped(root, every, &stopped))
            .map(|thread| Beating { stop, thread })
            .map_err(|err| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "start the gauge's beat thread",
                    err.to_string(),
                )
                .with_recovery("check process thread limits")
            })
    }

    fn stop(self) -> Watched {

        match self.stop.send(()) {
            Ok(()) | Err(mpsc::SendError(())) => {}
        }

        self.thread.join().unwrap_or(Watched {
            beats: 0,
            seen: None,
            read_cost: Duration::ZERO,
        })
    }
}

fn beat_until_stopped(root: u32, every: Every, stopped: &mpsc::Receiver<()>) -> Watched {
    let mut tree = Tree::open(root);
    let mut beats = 0_u64;
    let mut read_cost = Duration::ZERO;
    let mut previous = monotonic_now();
    while let Err(mpsc::RecvTimeoutError::Timeout) = stopped.recv_timeout(every.0) {
        let before = monotonic_now();
        let reading = tree.read(before.saturating_duration_since(previous));
        read_cost = read_cost.saturating_add(monotonic_now().saturating_duration_since(before));
        previous = before;
        match reading {
            Some(_) => beats = beats.saturating_add(1),
            None => break,
        }
    }
    Watched {
        beats,
        seen: tree.seen(),
        read_cost,
    }
}

#[cfg(windows)]
fn child_peaks(child: &Child) -> ChildPeaks {
    use std::os::windows::io::AsRawHandle;
    let widen = |bytes: usize| Some(u64::try_from(bytes).unwrap_or(u64::MAX));
    let Ok(handle) = isize::try_from(child.as_raw_handle().addr()) else {
        return ChildPeaks::UNREAD;
    };
    match win32job::utils::get_process_memory_info(handle) {
        Ok(counters) => ChildPeaks {
            private_bytes: widen(counters.peak_pagefile_usage),
            working_set_bytes: widen(counters.peak_working_set_size),
        },

        Err(_) => ChildPeaks::UNREAD,
    }
}

#[cfg(not(windows))]
fn child_peaks(_child: &Child) -> ChildPeaks {
    ChildPeaks::UNREAD
}

impl ChildPeaks {
    const UNREAD: ChildPeaks = ChildPeaks {
        private_bytes: None,
        working_set_bytes: None,
    };
}

fn not_started(program: &std::ffi::OsStr, err: &std::io::Error) -> AxError {
    let name = program.to_string_lossy();
    if err.kind() == std::io::ErrorKind::NotFound {
        AxError::failure(AxCode::PathNotFound, "start the measured program", name)
            .with_recovery("check PATH, or write the program's full path")
    } else {
        AxError::failure(
            AxCode::ToolUnavailable,
            "start the measured program",
            format!("{name}: {err}"),
        )
        .with_recovery("check that this shell can run the program")
    }
}
