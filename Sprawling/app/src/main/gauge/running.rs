// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A command run n times under `gauge`: each run timed from before its
//! spawn until `wait` sees it exit, its process tree read on a beat
//! thread meanwhile, and the spread of the runs' wall times at the end
//! (`crates/sprawling/spec/Main.lean` §8-129-4).

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

/// Runs the command `running.samples` times, one line per run, then the
/// spread of their wall times. The first run is kept: it is the cold
/// one, and the spread shows what it cost.
///
/// # Errors
/// The program cannot be started or waited for; a failed write to `out`.
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

/// One run, from before the spawn until `wait` sees the exit. The
/// command reads nothing and writes to this process's stderr, so stdout
/// carries readings alone.
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
            // The refusal to report is the thread's; a child that
            // already exited, or will not be stopped, changes nothing
            // about it.
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

/// The `sprawling-gauge` thread, reading the tree under one run's
/// process every beat until told to stop or the process is gone.
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

    /// Stops the beats and takes what they saw.
    fn stop(self) -> Watched {
        // A thread that already ended, its root gone, dropped the
        // receiver; it has nothing more to be told.
        match self.stop.send(()) {
            Ok(()) | Err(mpsc::SendError(())) => {}
        }
        // A beat thread that died measured nothing this run can trust,
        // which the lines write as null (8-129-4, decision 7).
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

/// The direct child's peak commit and working set, which Windows keeps
/// until the child's handle closes; read after `wait`, so exact rather
/// than a beat's sample.
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
        // A refused read is a peak nobody measured, written null rather
        // than 0 (8-129-4, decision 7).
        Err(_) => ChildPeaks::UNREAD,
    }
}

/// Elsewhere the platform keeps no peak this crate can read safely.
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
